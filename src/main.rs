// Pas de console en mode release.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod config;
mod ffi;
mod gaming;
mod installer;
mod layout;
mod memory;
mod pages;
mod privacy;
mod procs;
mod restore;
mod startup;
mod style;

use eframe::egui;
use std::collections::{HashSet, VecDeque};
use std::path::PathBuf;
use std::sync::atomic::Ordering;
use std::sync::mpsc::{channel, Receiver, Sender};
use std::time::{Duration, Instant};

fn main() -> eframe::Result<()> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Éclair")
            .with_icon(eframe::icon_data::from_png_bytes(include_bytes!("../assets/eclair.png")).unwrap_or_default())
            .with_inner_size([1100.0, 720.0])
            .with_decorations(false)
            .with_resizable(false),
        ..Default::default()
    };
    eframe::run_native("Éclair", options, Box::new(|cc| {
        style::apply(&cc.egui_ctx);
        Ok(Box::new(App::new()))
    }))
}

fn fmt_bytes(b: u64) -> String {
    const UNITS: [&str; 5] = ["o", "Ko", "Mo", "Go", "To"];
    let mut v = b as f64;
    let mut i = 0;
    while v >= 1024.0 && i < UNITS.len() - 1 {
        v /= 1024.0;
        i += 1;
    }
    format!("{v:.1} {}", UNITS[i])
}

#[derive(PartialEq, Clone, Copy)]
enum Tab {
    Dashboard,
    Cleaning,
    Performance,
    Privacy,
    Processes,
    Gaming,
    Startup,
    Installation,
    Settings,
}

enum Msg {
    Log(String),
    Installed(HashSet<String>),
    Done,
}

#[derive(Default)]
struct Stats {
    cpu: f64,
    mem: ffi::MemInfo,
    disk: Option<(u64, u64)>,
    uptime_ms: u64,
    power_plan: i32,
    privacy: Vec<bool>,
    procs: Vec<procs::Proc>,
    /// (liste d'attente, mémoire libre)
    lists: Option<(u64, u64)>,
    /// (la plus grossière, la plus fine, actuelle) en 100 ns
    timer: Option<(u32, u32, u32)>,
    global_timer: bool,
    /// Utilisation 3D de la carte graphique, None si le compteur Windows manque.
    gpu: Option<f64>,
    gaming: Vec<bool>,
    startup: Vec<startup::Entry>,
}

struct App {
    tab: Tab,
    admin: bool,
    stats: Stats,
    last_refresh: Option<Instant>,
    clean_user_temp: bool,
    clean_win_temp: bool,
    clean_recycle: bool,
    clean_dism: bool,
    log: Vec<String>,
    task: Option<Receiver<Msg>>,
    sampler: procs::Sampler,
    engine: procs::Engine,
    config: config::Config,
    new_favorite: String,
    standby: memory::StandbyCleaner,
    /// Lancé par la tâche planifiée du mode persistant : démarrer réduit.
    start_minimized: bool,
    install_selected: HashSet<&'static str>,
    /// Identifiants winget installés ; None tant que la recherche n'est pas finie.
    installed: Option<HashSet<String>>,
    installed_searched: bool,
    show_log: bool,
    user: String,
    /// [cpu %, ram %, gpu %] des dernières secondes, pour le graphique.
    history: VecDeque<[f32; 3]>,
    /// Messages des travaux en arrière-plan hors `task` (point de restauration…).
    notes_tx: Sender<String>,
    notes_rx: Receiver<String>,
    /// Un point de restauration a été tenté pendant cette session.
    restore_done: bool,
    /// Fermeture voulue (bouton Quitter) : ne pas seulement cacher la fenêtre.
    quitting: bool,
    tray_hint_shown: bool,
}

const GLOBAL_TIMER_KEY: &str = r"SYSTEM\CurrentControlSet\Control\Session Manager\kernel";

impl App {
    fn new() -> Self {
        ffi::cpu_usage(); // amorce la mesure CPU
        let config = config::Config::load();
        let engine = procs::Engine::start(config::dir().map(|d| d.join("priorites.txt")));
        *engine.favorites.lock().unwrap() = config.favorites.clone();
        let standby = memory::StandbyCleaner::start();
        if config.persistent {
            engine.mode.store(config.mode, Ordering::Relaxed);
            engine.auto_gaming.store(config.auto_gaming, Ordering::Relaxed);
            let s = &standby.settings;
            s.auto_purge.store(config.auto_purge, Ordering::Relaxed);
            s.list_mb.store(config.list_mb, Ordering::Relaxed);
            s.free_mb.store(config.free_mb, Ordering::Relaxed);
            s.fine_timer.store(config.fine_timer, Ordering::Relaxed);
        }
        ffi::tray_start("Éclair");
        let (notes_tx, notes_rx) = channel();
        Self {
            tab: Tab::Dashboard,
            admin: ffi::is_admin(),
            stats: Stats::default(),
            last_refresh: None,
            clean_user_temp: true,
            clean_win_temp: true,
            clean_recycle: false,
            clean_dism: false,
            log: vec!["Prêt.".into()],
            task: None,
            sampler: procs::Sampler::default(),
            engine,
            config,
            new_favorite: String::new(),
            standby,
            start_minimized: std::env::args().any(|a| a == "--minimized"),
            install_selected: HashSet::new(),
            installed: None,
            installed_searched: false,
            show_log: false,
            user: std::env::var("USERNAME").unwrap_or_else(|_| "toi".into()),
            history: VecDeque::with_capacity(layout::HISTORY),
            notes_tx,
            notes_rx,
            restore_done: false,
            quitting: false,
            tray_hint_shown: false,
        }
    }

    fn refresh_stats(&mut self) {
        if self.last_refresh.is_some_and(|t| t.elapsed() < Duration::from_secs(1)) {
            return;
        }
        self.last_refresh = Some(Instant::now());
        let mut procs = if self.tab == Tab::Processes { self.sampler.sample() } else { Vec::new() };
        procs.sort_by(|a, b| b.cpu.total_cmp(&a.cpu));
        self.stats = Stats {
            cpu: ffi::cpu_usage(),
            mem: ffi::memory_info(),
            disk: ffi::disk_space("C:\\"),
            uptime_ms: ffi::uptime_ms(),
            power_plan: ffi::get_power_plan(),
            privacy: privacy::SETTINGS.iter().map(|s| s.is_on()).collect(),
            procs,
            lists: ffi::memory_lists(),
            timer: ffi::timer_resolution(),
            global_timer: ffi::reg_get_dword(true, GLOBAL_TIMER_KEY, "GlobalTimerResolutionRequests") == Some(1),
            gpu: ffi::gpu_usage(),
            gaming: gaming::TWEAKS.iter().map(|t| (t.get)()).collect(),
            startup: startup::list(),
        };
        if self.history.len() == layout::HISTORY {
            self.history.pop_front();
        }
        let gpu = self.stats.gpu.unwrap_or(0.0) as f32;
        self.history.push_back([self.stats.cpu as f32, self.stats.mem.load_percent as f32, gpu]);
    }

    /// Lance un travail en arrière-plan pour ne pas figer l'interface.
    fn spawn(&mut self, job: impl FnOnce(&Sender<Msg>) + Send + 'static) {
        let (tx, rx) = channel();
        self.task = Some(rx);
        std::thread::spawn(move || {
            job(&tx);
            let _ = tx.send(Msg::Done);
        });
    }

    fn poll_task(&mut self) {
        let Some(rx) = &self.task else { return };
        let mut done = false;
        for msg in rx.try_iter() {
            match msg {
                Msg::Log(s) => self.log.push(s),
                Msg::Installed(ids) => self.installed = Some(ids),
                Msg::Done => done = true,
            }
        }
        if done {
            self.task = None;
            self.last_refresh = None;
        }
    }

    fn temp_dirs(&self) -> Vec<(&'static str, PathBuf)> {
        let mut dirs = Vec::new();
        if self.clean_user_temp {
            dirs.push(("Fichiers temporaires utilisateur", std::env::temp_dir()));
        }
        if self.clean_win_temp {
            let root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".into());
            dirs.push(("Fichiers temporaires Windows", PathBuf::from(root).join("Temp")));
        }
        dirs
    }

    fn start_clean(&mut self, dry_run: bool) {
        let dirs = self.temp_dirs();
        let recycle = self.clean_recycle;
        let dism = self.clean_dism;
        self.log.push(if dry_run { "Analyse en cours…" } else { "Nettoyage en cours…" }.into());
        self.spawn(move |tx| {
            let log = |s: String| {
                let _ = tx.send(Msg::Log(s));
            };
            let mut total = 0;
            for (name, dir) in dirs {
                let r = ffi::clean_directory(&dir, dry_run);
                total += r.bytes;
                let verb = if dry_run { "à supprimer" } else { "supprimés" };
                let mut line = format!("{name} : {} fichiers {verb} ({})", r.files, fmt_bytes(r.bytes));
                if r.failed > 0 {
                    line += &format!(", {} en cours d'utilisation ignorés", r.failed);
                }
                log(line);
            }
            if recycle {
                let size = ffi::recycle_bin_size();
                if let Some((bytes, items)) = size {
                    total += bytes;
                    log(format!("Corbeille : {items} éléments ({})", fmt_bytes(bytes)));
                }
                if !dry_run {
                    log(if ffi::empty_recycle_bin() { "Corbeille vidée." } else { "Échec du vidage de la corbeille." }.into());
                }
            }
            if dism {
                if dry_run {
                    log("Mises à jour Windows : analyse DISM (1 à 2 minutes)…".into());
                    match run_dism("/AnalyzeComponentStore") {
                        // Sortie en anglais (/English) : seules ces lignes résument l'espace récupérable.
                        Some(out) => out
                            .lines()
                            .filter(|l| ["Actual Size", "Backups and Disabled", "Cache and Temporary", "Cleanup Recommended"].iter().any(|k| l.contains(k)))
                            .for_each(|l| log(format!("  {}", l.trim()))),
                        None => log("DISM a échoué (admin requis).".into()),
                    }
                } else {
                    log("Mises à jour Windows : nettoyage DISM (plusieurs minutes)…".into());
                    let free = || ffi::disk_space("C:\\").map_or(0, |(f, _)| f);
                    let before = free();
                    if run_dism("/StartComponentCleanup").is_some() {
                        let freed = free().saturating_sub(before);
                        total += freed;
                        log(format!("Mises à jour Windows : {} libérés.", fmt_bytes(freed)));
                    } else {
                        log("DISM a échoué (admin requis).".into());
                    }
                }
            }
            let what = if dry_run { "Espace récupérable" } else { "Espace libéré" };
            log(format!("{what} : {}", fmt_bytes(total)));
        });
    }


    fn trim_memory(&mut self) {
        let before = ffi::memory_info().avail_phys;
        self.log.push("Libération de la mémoire…".into());
        self.spawn(move |tx| {
            let n = ffi::trim_working_sets();
            let gained = ffi::memory_info().avail_phys.saturating_sub(before);
            let _ = tx.send(Msg::Log(format!("{n} processus allégés, {} libérés.", fmt_bytes(gained))));
        });
    }

    fn set_mode(&mut self, m: u8) {
        self.engine.mode.store(m, Ordering::Relaxed);
        self.config.mode = m;
        self.config.save();
        let name = match m {
            procs::MODE_BALANCE => "Équilibré",
            procs::MODE_GAMING => "Gaming",
            _ => "Désactivé",
        };
        self.log.push(format!("Mode {name}."));
    }

    /// Avant la première modification de Windows de la session : point de restauration (admin).
    fn before_change(&mut self) {
        if !self.restore_done && self.admin {
            self.create_restore_point();
        }
    }

    fn create_restore_point(&mut self) {
        self.restore_done = true;
        self.log.push("Création d'un point de restauration…".into());
        let tx = self.notes_tx.clone();
        std::thread::spawn(move || {
            let _ = tx.send(restore::message(&restore::create()).into());
        });
    }

    fn start_upgrade_all(&mut self) {
        self.log.push("Mise à jour de toutes les applis (plusieurs minutes)…".into());
        self.spawn(|tx| {
            let _ = tx.send(Msg::Log(match installer::upgrade_all() {
                Ok(()) => "✔ Applis à jour.".into(),
                Err(e) => format!("Mise à jour terminée avec des erreurs ({e})."),
            }));
            if let Some(ids) = installer::installed() {
                let _ = tx.send(Msg::Installed(ids));
            }
        });
    }

    /// Cache la fenêtre : Éclair continue (modes, purge) avec son icône près de l'horloge.
    fn hide_to_tray(&mut self, ctx: &egui::Context) {
        if !ffi::hide_main_window() {
            ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
        }
        if !std::mem::replace(&mut self.tray_hint_shown, true) {
            ffi::tray_notify("Éclair tourne en arrière-plan", "Clic sur l'icône pour rouvrir, clic droit pour quitter.");
        }
    }

    fn favorites_changed(&mut self) {
        *self.engine.favorites.lock().unwrap() = self.config.favorites.clone();
        self.config.save();
    }

    fn set_persistent(&mut self, on: bool) {
        self.config.persistent = on;
        if on {
            for exe in self.config.favorites.clone() {
                self.persist(&exe, Some(4));
            }
        } else {
            for exe in self.config.persisted.clone() {
                self.persist(&exe, None);
            }
        }
        self.config.save();
        if on {
            self.before_change();
        }
        self.log.push(
            if on { "Réglages persistants." } else { "Réglages temporaires : ce qui était enregistré dans Windows est effacé." }.into(),
        );
        self.log.push(if set_autostart(on) {
            if on { "Lancement automatique à l'ouverture de session activé." } else { "Lancement automatique retiré." }.into()
        } else {
            "Impossible de modifier le lancement automatique (admin requis).".into()
        });
    }

    /// Enregistre (ou efface avec None) la priorité d'un exécutable dans Windows.
    fn persist(&mut self, exe: &str, level: Option<u32>) {
        if !procs::persist_priority(exe, level) {
            self.log.push(format!("Impossible d'enregistrer la priorité de {exe} dans Windows (admin requis)."));
            return;
        }
        self.config.persisted.retain(|e| e != exe);
        if level.is_some() {
            self.config.persisted.push(exe.to_string());
        }
    }

    fn start_install(&mut self) {
        let apps: Vec<&'static installer::App> = installer::APPS.iter().filter(|a| self.install_selected.contains(a.id)).collect();
        self.install_selected.clear();
        self.spawn(move |tx| {
            let log = |s: String| {
                let _ = tx.send(Msg::Log(s));
            };
            for (i, a) in apps.iter().enumerate() {
                log(format!("[{}/{}] Installation de {}…", i + 1, apps.len(), a.name));
                match installer::install(a.id) {
                    Ok(()) => log(format!("✔ {} installé.", a.name)),
                    Err(e) => log(format!("✖ {} : échec ({e}).", a.name)),
                }
            }
            if let Some(ids) = installer::installed() {
                let _ = tx.send(Msg::Installed(ids));
            }
        });
    }
}

impl Drop for App {
    fn drop(&mut self) {
        ffi::tray_stop();
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if std::mem::take(&mut self.start_minimized) {
            self.tray_hint_shown = true; // pas de bulle à chaque démarrage de Windows
            self.hide_to_tray(ctx);
        }
        // La croix cache la fenêtre au lieu de quitter ; "Quitter" (menu de l'icône ou Réglages) ferme vraiment.
        if ctx.input(|i| i.viewport().close_requested()) && !self.quitting && !ffi::tray_quit_requested() {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.hide_to_tray(ctx);
        }
        self.poll_task();
        self.log.extend(self.notes_rx.try_iter());
        self.log.extend(self.engine.events.try_iter());
        self.log.extend(self.standby.events.try_iter());
        self.refresh_stats();

        self.ui_frame(ctx);

        let next = if self.task.is_some() { 100 } else { 1000 };
        ctx.request_repaint_after(Duration::from_millis(next));
    }
}

/// Lance Éclair caché près de l'horloge, en administrateur, à l'ouverture de session (comme ISLC).
fn set_autostart(on: bool) -> bool {
    let mut cmd = std::process::Command::new("schtasks");
    if on {
        let Ok(exe) = std::env::current_exe() else { return false };
        let tr = format!("\"{}\" --minimized", exe.display());
        cmd.args(["/Create", "/TN", "Eclair", "/SC", "ONLOGON", "/RL", "HIGHEST", "/F", "/TR", &tr]);
    } else {
        cmd.args(["/Delete", "/TN", "Eclair", "/F"]);
    }
    use std::os::windows::process::CommandExt;
    cmd.creation_flags(0x0800_0000) // CREATE_NO_WINDOW : pas de console qui clignote
        .output()
        .is_ok_and(|o| o.status.success())
}

/// Lance DISM sur l'image en cours ; Some(sortie) si la commande a réussi.
fn run_dism(action: &str) -> Option<String> {
    use std::os::windows::process::CommandExt;
    let out = std::process::Command::new("dism")
        .args(["/English", "/Online", "/Cleanup-Image", action])
        .creation_flags(0x0800_0000) // CREATE_NO_WINDOW
        .output()
        .ok()?;
    out.status.success().then(|| String::from_utf8_lossy(&out.stdout).into_owned())
}
