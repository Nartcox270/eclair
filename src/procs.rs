//! CPU par processus et moteur de priorités :
//! - Équilibré (ProBalance) : ralentit les programmes qui monopolisent le processeur ;
//! - Gaming : priorité haute pour les jeux et applis favoris, les autres passent en arrière-plan.

use crate::ffi;
use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU8, Ordering};
use std::sync::mpsc::{channel, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

pub const PRIORITIES: [&str; 5] = ["Inactive", "Sous la normale", "Normale", "Supérieure", "Haute"];
const BELOW_NORMAL: u32 = 1;
const NORMAL: u32 = 2;
const HIGH: u32 = 4;

pub const MODE_OFF: u8 = 0;
pub const MODE_BALANCE: u8 = 1;
pub const MODE_GAMING: u8 = 2;

// Jamais ralentis en mode Gaming : ralentir le bureau ou le son se sentirait en jeu.
const PROTECTED: [&str; 4] = ["explorer.exe", "dwm.exe", "audiodg.exe", "csrss.exe"];

pub struct Proc {
    pub pid: u32,
    pub name: String,
    pub priority: u32,
    /// Pourcentage de tout le processeur (100 % = tous les coeurs à fond).
    pub cpu: f64,
    pub mem: u64,
}

#[derive(Default)]
pub struct Sampler {
    last: HashMap<u32, u64>,
    at: Option<Instant>,
}

impl Sampler {
    /// Le CPU est mesuré depuis l'appel précédent ; le premier appel renvoie 0 partout.
    pub fn sample(&mut self) -> Vec<Proc> {
        let now = Instant::now();
        let cpus = std::thread::available_parallelism().map_or(1, |n| n.get()) as f64;
        let elapsed = self.at.map_or(0.0, |t| (now - t).as_secs_f64() * 1e7 * cpus); // en 100 ns
        self.at = Some(now);
        let prev = std::mem::take(&mut self.last);
        ffi::list_processes()
            .into_iter()
            .map(|p| {
                self.last.insert(p.pid, p.cpu_time);
                let cpu = match prev.get(&p.pid) {
                    Some(&before) if elapsed > 0.0 => p.cpu_time.saturating_sub(before) as f64 / elapsed * 100.0,
                    _ => 0.0,
                };
                Proc { pid: p.pid, name: p.name, priority: p.priority, cpu, mem: p.working_set }
            })
            .collect()
    }
}

const BUSY: f64 = 40.0; // % CPU total à partir duquel le PC est considéré chargé
const SUSTAIN: u32 = 2; // secondes consécutives au-dessus des seuils

/// (seuil pour ralentir, seuil pour rendre) en % du total. Plafonnés à une fraction d'un coeur :
/// sur 20 threads, un programme qui sature un coeur ne pèse que 5 % et passerait sous 10 %.
fn hog_thresholds(cpus: f64) -> (f64, f64) {
    let one_core = 100.0 / cpus;
    (10f64.min(0.75 * one_core), 5f64.min(0.3 * one_core))
}

/// Programme dont l'outil a changé la priorité, avec celle d'origine pour la rendre.
struct Changed {
    name: String,
    orig: u32,
}

type ChangedMap = HashMap<u32, Changed>;

/// Le Gaming automatique reste actif ce temps après la sortie du plein écran (alt-tab rapide).
const AUTO_GRACE: Duration = Duration::from_secs(15);

pub struct Engine {
    pub mode: Arc<AtomicU8>,
    /// Active le Gaming tout seul quand un jeu prioritaire est en plein écran.
    pub auto_gaming: Arc<AtomicBool>,
    /// Mode réellement appliqué (le Gaming automatique peut différer du mode choisi).
    pub effective: Arc<AtomicU8>,
    /// Noms d'exécutables, en minuscules.
    pub favorites: Arc<Mutex<Vec<String>>>,
    changed: Arc<Mutex<ChangedMap>>,
    journal: Option<PathBuf>,
    pub events: Receiver<String>,
}

impl Engine {
    /// `journal` : fichier où noter les priorités changées, pour les rendre au lancement suivant
    /// si l'outil est tué ou plante. None : pas de journal (tests).
    pub fn start(journal: Option<PathBuf>) -> Self {
        let (tx, events) = channel();
        if let Some(path) = &journal {
            let n = recover(path);
            if n > 0 {
                let _ = tx.send(format!("{n} priorités rendues après un arrêt inattendu."));
            }
        }
        let mode = Arc::new(AtomicU8::new(MODE_OFF));
        let auto_gaming = Arc::new(AtomicBool::new(false));
        let effective = Arc::new(AtomicU8::new(MODE_OFF));
        let favorites = Arc::new(Mutex::new(Vec::new()));
        let changed = Arc::new(Mutex::new(ChangedMap::new()));
        let shared = Shared {
            mode: mode.clone(),
            auto_gaming: auto_gaming.clone(),
            effective: effective.clone(),
            favorites: favorites.clone(),
            changed: changed.clone(),
        };
        let path = journal.clone();
        std::thread::spawn(move || run(&shared, path.as_deref(), &tx));
        Self { mode, auto_gaming, effective, favorites, changed, journal, events }
    }

    pub fn changed_count(&self) -> usize {
        self.changed.lock().unwrap().len()
    }
}

// Ne jamais laisser un programme modifié après la fermeture de l'outil.
impl Drop for Engine {
    fn drop(&mut self) {
        restore_all(&mut self.changed.lock().unwrap(), None);
        if let Some(path) = &self.journal {
            let _ = std::fs::remove_file(path);
        }
    }
}

fn restore(pid: u32, c: &Changed, tx: Option<&Sender<String>>) {
    if ffi::set_priority(pid, c.orig) {
        if let Some(tx) = tx {
            let _ = tx.send(format!("Priorité d'origine rendue à {}.", c.name));
        }
    }
}

fn restore_all(changed: &mut ChangedMap, tx: Option<&Sender<String>>) {
    for (pid, c) in changed.drain() {
        restore(pid, &c, tx);
    }
}

/// Une ligne par programme modifié : "pid<TAB>priorité d'origine<TAB>nom".
fn journal_format(changed: &ChangedMap) -> String {
    let mut lines: Vec<String> = changed.iter().map(|(pid, c)| format!("{pid}\t{}\t{}\n", c.orig, c.name)).collect();
    lines.sort();
    lines.concat()
}

fn journal_parse(s: &str) -> Vec<(u32, u32, String)> {
    s.lines()
        .filter_map(|l| {
            let mut f = l.splitn(3, '\t');
            Some((f.next()?.parse().ok()?, f.next()?.parse().ok()?, f.next()?.to_string()))
        })
        .collect()
}

/// Rend leur priorité aux programmes notés dans le journal (seulement si le pid porte toujours
/// le même nom : un pid réutilisé par un autre programme n'est pas touché). Renvoie le nombre rendu.
pub fn recover(path: &Path) -> usize {
    let Ok(text) = std::fs::read_to_string(path) else { return 0 };
    let alive: HashMap<u32, String> = ffi::list_processes().into_iter().map(|p| (p.pid, p.name)).collect();
    let n = journal_parse(&text)
        .into_iter()
        .filter(|(pid, orig, name)| alive.get(pid).is_some_and(|n| n.eq_ignore_ascii_case(name)) && ffi::set_priority(*pid, *orig))
        .count();
    let _ = std::fs::remove_file(path);
    n
}

struct Shared {
    mode: Arc<AtomicU8>,
    auto_gaming: Arc<AtomicBool>,
    effective: Arc<AtomicU8>,
    favorites: Arc<Mutex<Vec<String>>>,
    changed: Arc<Mutex<ChangedMap>>,
}

#[derive(Default)]
struct State {
    sampler: Sampler,
    over: HashMap<u32, u32>,
    failed: HashSet<u32>,
    last_mode: u8,
    auto_until: Option<Instant>,
}

fn run(shared: &Shared, journal: Option<&Path>, tx: &Sender<String>) {
    let mut state = State::default();
    let mut written = String::new();
    loop {
        std::thread::sleep(Duration::from_secs(1));
        let mut changed = shared.changed.lock().unwrap();
        state.step(shared, &mut changed, tx);
        if let Some(path) = journal {
            let text = journal_format(&changed);
            if text != written {
                let _ = if text.is_empty() { std::fs::remove_file(path) } else { std::fs::write(path, &text) };
                written = text;
            }
        }
    }
}

impl State {
    fn step(&mut self, shared: &Shared, changed: &mut ChangedMap, tx: &Sender<String>) {
        let chosen = shared.mode.load(Ordering::Relaxed);
        let auto = chosen == MODE_OFF && shared.auto_gaming.load(Ordering::Relaxed);
        let mut procs = if chosen != MODE_OFF || auto { self.sampler.sample() } else { Vec::new() };

        let mut m = chosen;
        if auto {
            let fs = ffi::fullscreen_foreground_pid();
            let favs = shared.favorites.lock().unwrap();
            if fs != 0 && procs.iter().any(|p| p.pid == fs && favs.contains(&p.name.to_lowercase())) {
                self.auto_until = Some(Instant::now() + AUTO_GRACE);
            }
            if self.auto_until.is_some_and(|t| Instant::now() < t) {
                m = MODE_GAMING;
            }
        } else {
            self.auto_until = None;
        }
        shared.effective.store(m, Ordering::Relaxed);

        if m != self.last_mode {
            if chosen == MODE_OFF && m == MODE_GAMING {
                let _ = tx.send("Jeu détecté : mode Gaming automatique.".into());
            } else if chosen == MODE_OFF && self.last_mode == MODE_GAMING && auto {
                let _ = tx.send("Jeu terminé : mode Gaming automatique coupé.".into());
            }
            // La liste a été lue avant de rendre les priorités : on la corrige, sinon le nouveau
            // mode prendrait une priorité modifiée pour la priorité d'origine.
            for p in &mut procs {
                if let Some(c) = changed.get(&p.pid) {
                    p.priority = c.orig;
                }
            }
            // Silencieux en Gaming : des dizaines de programmes en arrière-plan sont rendus.
            restore_all(changed, (self.last_mode == MODE_BALANCE).then_some(tx));
            self.over.clear();
            self.failed.clear();
            self.last_mode = m;
        }
        // Liste vide = capture ratée, pas "tout le monde est mort" : sinon on oublierait
        // de rendre leur priorité aux programmes modifiés.
        if m == MODE_OFF || procs.is_empty() {
            return;
        }
        let alive: HashSet<u32> = procs.iter().map(|p| p.pid).collect();
        changed.retain(|pid, _| alive.contains(pid));
        self.over.retain(|pid, _| alive.contains(pid));
        self.failed.retain(|pid| alive.contains(pid));
        let fg = ffi::foreground_pid();
        if m == MODE_BALANCE {
            balance(&procs, changed, &mut self.over, fg, tx);
        } else {
            let favs = shared.favorites.lock().unwrap().clone();
            gaming(&procs, changed, &mut self.failed, &favs, fg, tx);
        }
    }
}

fn balance(procs: &[Proc], changed: &mut ChangedMap, over: &mut HashMap<u32, u32>, fg: u32, tx: &Sender<String>) {
    let total: f64 = procs.iter().map(|p| p.cpu).sum();
    let me = std::process::id();
    let (hog_at, calm_at) = hog_thresholds(std::thread::available_parallelism().map_or(1, |n| n.get()) as f64);
    for p in procs {
        if let Some(c) = changed.get(&p.pid) {
            // Rendu dès qu'il se calme, que le PC se libère, ou que l'utilisateur passe dessus.
            if p.cpu < calm_at || total < BUSY / 2.0 || p.pid == fg {
                restore(p.pid, c, Some(tx));
                changed.remove(&p.pid);
            }
            continue;
        }
        let hog = total >= BUSY && p.cpu >= hog_at && p.priority == NORMAL && p.pid != fg && p.pid != me;
        let n = over.entry(p.pid).or_default();
        *n = if hog { *n + 1 } else { 0 };
        if *n >= SUSTAIN && ffi::set_priority(p.pid, BELOW_NORMAL) {
            *n = 0;
            changed.insert(p.pid, Changed { name: p.name.clone(), orig: NORMAL });
            let _ = tx.send(format!("Équilibré : {} ralenti ({:.0} % du processeur).", p.name, p.cpu));
        }
    }
}

/// Priorité voulue en mode Gaming ; None = ne pas toucher (ou rendre l'origine).
fn gaming_target(name: &str, pid: u32, fg: u32, favorites: &[String]) -> Option<u32> {
    let name = name.to_lowercase();
    if favorites.contains(&name) {
        Some(HIGH)
    } else if pid == fg || pid == std::process::id() || PROTECTED.contains(&name.as_str()) {
        None
    } else {
        Some(BELOW_NORMAL)
    }
}

fn gaming(procs: &[Proc], changed: &mut ChangedMap, failed: &mut HashSet<u32>, favorites: &[String], fg: u32, tx: &Sender<String>) {
    for p in procs {
        let target = gaming_target(&p.name, p.pid, fg, favorites);
        let orig = changed.get(&p.pid).map_or(p.priority, |c| c.orig);
        match target {
            None => {
                if let Some(c) = changed.remove(&p.pid) {
                    restore(p.pid, &c, None);
                }
            }
            // On ne ralentit que les programmes en priorité normale : un réglage manuel est respecté.
            Some(BELOW_NORMAL) if orig != NORMAL => {}
            Some(t) if t != p.priority && !failed.contains(&p.pid) => {
                if ffi::set_priority(p.pid, t) {
                    changed.insert(p.pid, Changed { name: p.name.clone(), orig });
                    if t == HIGH {
                        let _ = tx.send(format!("Gaming : {} en priorité haute.", p.name));
                    }
                } else {
                    failed.insert(p.pid); // protégé ou admin requis : inutile de réessayer
                }
            }
            _ => {}
        }
    }
}

// Priorité enregistrée dans Windows pour un exécutable : appliquée à chaque lancement,
// même sans l'optimiseur. Admin requis.
const IFEO: &str = r"SOFTWARE\Microsoft\Windows NT\CurrentVersion\Image File Execution Options";

/// level : 0..4 comme PRIORITIES ; None supprime le réglage.
pub fn persist_priority(exe: &str, level: Option<u32>) -> bool {
    let key = format!(r"{IFEO}\{exe}\PerfOptions");
    match level {
        // Codes CpuPriorityClass de Windows : 1 inactive, 5 sous la normale, 2 normale, 6 supérieure, 3 haute.
        Some(l) => ffi::reg_set_dword(true, &key, "CpuPriorityClass", [1, 5, 2, 6, 3][l.min(4) as usize]),
        None => ffi::reg_delete(true, &key, "CpuPriorityClass"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sampler_sees_own_cpu() {
        let mut s = Sampler::default();
        s.sample();
        let end = Instant::now() + Duration::from_millis(300);
        while Instant::now() < end {
            std::hint::black_box(0u64);
        }
        let me = std::process::id();
        let own = s.sample().into_iter().find(|p| p.pid == me).expect("processus de test listé");
        let cpus = std::thread::available_parallelism().unwrap().get() as f64;
        // Un coeur occupé à fond ≈ 100 / cpus % du total.
        assert!(own.cpu > 50.0 / cpus, "cpu mesuré : {}", own.cpu);
    }

    #[test]
    fn set_priority_round_trip() {
        // Un processus à part : changer celui du test ferait hériter sa priorité
        // aux processus lancés en parallèle par le stress test.
        let child = Kill(vec![std::process::Command::new("ping")
            .args(["-t", "127.0.0.1"])
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap()]);
        let pid = child.0[0].id();
        assert!(ffi::set_priority(pid, BELOW_NORMAL));
        assert_eq!(prio(pid), Some(BELOW_NORMAL));
        assert!(ffi::set_priority(pid, NORMAL));
        assert_eq!(prio(pid), Some(NORMAL));
        assert!(!ffi::set_priority(pid, 5), "temps réel refusé");
    }

    /// Tue les processus de charge même si le test échoue.
    struct Kill(Vec<std::process::Child>);
    impl Drop for Kill {
        fn drop(&mut self) {
            for c in &mut self.0 {
                let _ = c.kill();
            }
        }
    }

    fn wait_for(secs: u64, cond: impl Fn() -> bool) -> bool {
        let end = Instant::now() + Duration::from_secs(secs);
        while Instant::now() < end {
            if cond() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(250));
        }
        cond()
    }

    fn prio(pid: u32) -> Option<u32> {
        ffi::list_processes().into_iter().find(|p| p.pid == pid).map(|p| p.priority)
    }

    #[test]
    #[ignore = "stress : occupe la moitié des coeurs ~30 s et change des priorités sur le PC"]
    fn stress_modes() {
        use std::process::{Command, Stdio};
        // Moitié des coeurs : assez pour charger le PC, sans le figer quand ils passent en priorité haute.
        let n = (std::thread::available_parallelism().unwrap().get() / 2).max(1);
        let spawn = |prog: &str, args: &[&str]| Command::new(prog).args(args).stdout(Stdio::null()).spawn().unwrap();
        let mut procs = Kill((0..n).map(|_| spawn("powershell", &["-NoProfile", "-Command", "while($true){}"])).collect());
        let hogs: Vec<u32> = procs.0.iter().map(|c| c.id()).collect();
        procs.0.push(spawn("ping", &["-t", "127.0.0.1"]));
        let calm = procs.0.last().unwrap().id();
        let all = |level: u32| hogs.iter().all(|&h| prio(h) == Some(level));

        let engine = Engine::start(None);
        let t = Instant::now();
        engine.mode.store(MODE_BALANCE, Ordering::Relaxed);
        assert!(wait_for(15, || all(BELOW_NORMAL)), "Équilibré : {n} processus à 1 coeur chacun doivent être ralentis");
        assert_eq!(prio(calm), Some(NORMAL), "Équilibré : un programme calme n'est pas touché");
        println!("Équilibré : {n} gourmands ralentis en {:.1} s", t.elapsed().as_secs_f64());

        let t = Instant::now();
        *engine.favorites.lock().unwrap() = vec!["powershell.exe".into()];
        engine.mode.store(MODE_GAMING, Ordering::Relaxed);
        assert!(wait_for(10, || all(HIGH)), "Gaming : favoris en priorité haute");
        assert!(wait_for(10, || prio(calm) == Some(BELOW_NORMAL)), "Gaming : les autres en arrière-plan");
        println!("Gaming : favoris montés et autres baissés en {:.1} s", t.elapsed().as_secs_f64());

        // Beaucoup de processus qui naissent et meurent pendant que le moteur tourne.
        let t = Instant::now();
        for _ in 0..50 {
            let _ = Command::new("cmd").args(["/c", "exit"]).status();
        }
        println!("50 processus éphémères en {:.1} s, moteur toujours actif : {} programmes modifiés", t.elapsed().as_secs_f64(), engine.changed_count());

        let t = Instant::now();
        engine.mode.store(MODE_OFF, Ordering::Relaxed);
        let restored = wait_for(10, || all(NORMAL) && prio(calm) == Some(NORMAL));
        let state: Vec<_> = hogs.iter().chain([&calm]).map(|&p| prio(p)).collect();
        assert!(restored, "Désactivé : tout est rendu (priorités : {state:?}, attendu 2)");
        assert!(wait_for(5, || engine.changed_count() == 0));
        println!("Désactivé : tout rendu en {:.1} s", t.elapsed().as_secs_f64());
    }

    #[test]
    fn journal_recovers_after_crash() {
        let child = Kill(vec![std::process::Command::new("ping")
            .args(["-t", "127.0.0.1"])
            .stdout(std::process::Stdio::null())
            .spawn()
            .unwrap()]);
        let pid = child.0[0].id();
        assert!(ffi::set_priority(pid, BELOW_NORMAL));
        let mut changed = ChangedMap::new();
        changed.insert(pid, Changed { name: "PING.EXE".into(), orig: NORMAL });
        changed.insert(999_999, Changed { name: "disparu.exe".into(), orig: NORMAL });
        let text = journal_format(&changed);
        assert_eq!(journal_parse(&text).len(), 2);

        let path = std::env::temp_dir().join("eclair-test-journal.txt");
        std::fs::write(&path, text).unwrap();
        assert_eq!(recover(&path), 1, "seul le processus vivant au même nom est rendu");
        assert_eq!(prio(pid), Some(NORMAL));
        assert!(!path.exists(), "journal effacé après reprise");
    }

    #[test]
    fn thresholds_follow_core_count() {
        assert_eq!(hog_thresholds(4.0), (10.0, 5.0));
        assert_eq!(hog_thresholds(20.0), (3.75, 1.5), "un coeur saturé (5 %) doit être détecté");
    }

    #[test]
    fn gaming_targets() {
        let favs = vec!["discord.exe".to_string()];
        assert_eq!(gaming_target("Discord.exe", 10, 99, &favs), Some(HIGH));
        assert_eq!(gaming_target("chrome.exe", 11, 99, &favs), Some(BELOW_NORMAL));
        assert_eq!(gaming_target("chrome.exe", 99, 99, &favs), None, "premier plan épargné");
        assert_eq!(gaming_target("explorer.exe", 12, 99, &favs), None, "protégé");
    }
}
