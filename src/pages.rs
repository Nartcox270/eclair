//! Pages de l'onglet Optimisation et de l'installation.

use crate::style::{self, MUTED};
use crate::{ffi, fmt_bytes, gaming, installer, memory, privacy, procs, startup, App, Msg, GLOBAL_TIMER_KEY};
use eframe::egui::{self, RichText};
use std::sync::atomic::Ordering;

impl App {
    pub(crate) fn ui_cleaning(&mut self, ui: &mut egui::Ui) {
        style::page_header(ui, "Nettoyage", "Libère de l'espace disque. Les fichiers en cours d'utilisation sont ignorés.");
        style::section(ui, "À NETTOYER", |ui| {
            style::toggle_row(ui, "Fichiers temporaires utilisateur", "Dossier %TEMP% de ta session.", &mut self.clean_user_temp, true);
            style::toggle_row(ui, "Fichiers temporaires Windows", "Dossier Windows\\Temp (admin conseillé).", &mut self.clean_win_temp, true);
            style::toggle_row(ui, "Corbeille", "Vidée définitivement.", &mut self.clean_recycle, true);
            style::toggle_row(
                ui,
                "Anciens composants des mises à jour Windows",
                "Nettoyage DISM de Microsoft : admin requis, plusieurs minutes.",
                &mut self.clean_dism,
                true,
            );
        });
        ui.add_space(10.0);
        let busy = self.task.is_some();
        ui.horizontal(|ui| {
            if ui.add_enabled(!busy, egui::Button::new("🔍  Analyser").min_size(egui::vec2(150.0, 34.0))).clicked() {
                self.start_clean(true);
            }
            if ui.add_enabled(!busy, egui::Button::new("🗑  Nettoyer").min_size(egui::vec2(150.0, 34.0))).clicked() {
                self.start_clean(false);
            }
            if busy {
                ui.spinner();
            }
        });
    }

    pub(crate) fn ui_performance(&mut self, ui: &mut egui::Ui) {
        style::page_header(ui, "Performance", "Mémoire, réseau, alimentation et minuteur système.");
        let busy = self.task.is_some();
        style::section(ui, "ACTIONS RAPIDES", |ui| {
            ui.horizontal(|ui| {
                if ui.add_enabled(!busy, egui::Button::new("💾  Libérer la mémoire vive")).on_hover_text("Vide la mémoire de travail des processus.").clicked() {
                    self.trim_memory();
                }
                if ui.button("🌐  Vider le cache DNS").on_hover_text("Utile en cas de problème de connexion à un site.").clicked() {
                    let ok = ffi::flush_dns();
                    self.log.push(if ok { "Cache DNS vidé." } else { "Échec du vidage du cache DNS." }.into());
                }
            });
        });

        style::section(ui, "PLAN D'ALIMENTATION", |ui| {
            let current = self.stats.power_plan;
            if current == -1 {
                ui.label(RichText::new("Plan actuel : personnalisé").color(MUTED));
            }
            for (id, name) in [
                (3, "Performances max (coeurs jamais en veille, sur secteur)"),
                (1, "Performances élevées"),
                (0, "Équilibré"),
                (2, "Économie d'énergie"),
            ] {
                if ui.radio(current == id, name).clicked() && current != id {
                    self.before_change();
                    let msg = if ffi::set_power_plan(id) {
                        format!("Plan d'alimentation : {name}.")
                    } else {
                        format!("Impossible d'activer « {name} » (plan absent sur ce PC ?).")
                    };
                    self.log.push(msg);
                    self.last_refresh = None;
                }
            }
        });

        self.ui_standby(ui);
    }

    fn ui_standby(&mut self, ui: &mut egui::Ui) {
        let s = self.standby.settings.clone();
        let mut changed = false;
        let mut auto = s.auto_purge.load(Ordering::Relaxed);
        let (mut list_mb, mut free_mb) = (s.list_mb.load(Ordering::Relaxed), s.free_mb.load(Ordering::Relaxed));
        style::section(ui, "MÉMOIRE EN ATTENTE (STANDBY)", |ui| {
            match self.stats.lists {
                Some((standby, free)) => ui.label(format!("En attente : {}   ·   Libre : {}", fmt_bytes(standby), fmt_bytes(free))),
                None => ui.label(RichText::new("Lecture impossible : lance Éclair en administrateur.").color(MUTED)),
            };
            changed |= style::toggle_row(ui, "Purger automatiquement", "Réduit les saccades en jeu quand la mémoire libre manque.", &mut auto, true);
            ui.horizontal(|ui| {
                ui.label("quand la liste dépasse");
                changed |= ui.add(egui::DragValue::new(&mut list_mb).range(128..=65536).suffix(" Mo")).changed();
                ui.label("et que la mémoire libre est sous");
                changed |= ui.add(egui::DragValue::new(&mut free_mb).range(128..=65536).suffix(" Mo")).changed();
            });
            if ui.button("Purger maintenant").clicked() {
                self.log.push(memory::purge());
                self.last_refresh = None;
            }
        });

        style::section(ui, "MINUTEUR SYSTÈME", |ui| {
            if let Some((_, finest, current)) = self.stats.timer {
                ui.label(format!("Résolution actuelle : {:.3} ms", current as f64 / 10_000.0));
                let mut fine = s.fine_timer.load(Ordering::Relaxed);
                let title = format!("Forcer {:.2} ms", finest as f64 / 10_000.0);
                if style::toggle_row(ui, &title, "Tant qu'Éclair tourne : latence plus faible en jeu.", &mut fine, true) {
                    s.fine_timer.store(fine, Ordering::Relaxed);
                    changed = true;
                }
            }
            let mut global = self.stats.global_timer;
            let help = if self.admin { "Windows 11 : le minuteur s'applique à tout le système. Redémarrage requis." } else { "Admin requis." };
            if style::toggle_row(ui, "Résolution globale", help, &mut global, self.admin) {
                self.before_change();
                let ok = if global {
                    ffi::reg_set_dword(true, GLOBAL_TIMER_KEY, "GlobalTimerResolutionRequests", 1)
                } else {
                    ffi::reg_delete(true, GLOBAL_TIMER_KEY, "GlobalTimerResolutionRequests")
                };
                self.log.push(if ok { "Résolution globale du minuteur modifiée (redémarrage requis)." } else { "Impossible de modifier la résolution globale." }.into());
                self.last_refresh = None;
            }
        });

        if changed {
            s.auto_purge.store(auto, Ordering::Relaxed);
            s.list_mb.store(list_mb, Ordering::Relaxed);
            s.free_mb.store(free_mb, Ordering::Relaxed);
            self.config.auto_purge = auto;
            self.config.list_mb = list_mb;
            self.config.free_mb = free_mb;
            self.config.fine_timer = s.fine_timer.load(Ordering::Relaxed);
            self.config.save();
        }
    }

    pub(crate) fn ui_processes(&mut self, ui: &mut egui::Ui) {
        style::page_header(ui, "Processus", "Priorités des programmes : modes Équilibré et Gaming.");

        style::section(ui, "MODE", |ui| {
            let mode = self.engine.mode.load(Ordering::Relaxed);
            ui.horizontal(|ui| {
                for (m, name) in [(procs::MODE_OFF, "Désactivé"), (procs::MODE_BALANCE, "Équilibré"), (procs::MODE_GAMING, "Gaming")] {
                    if ui.radio(mode == m, name).clicked() && mode != m {
                        self.set_mode(m);
                    }
                }
            });
            ui.label(
                RichText::new(match mode {
                    procs::MODE_BALANCE => "Ralentit les programmes qui monopolisent le processeur quand le PC est chargé.",
                    procs::MODE_GAMING => "Priorité haute pour les jeux et applis prioritaires ; les autres passent en arrière-plan. La fenêtre au premier plan n'est jamais ralentie.",
                    _ => "Aucune priorité n'est modifiée automatiquement.",
                })
                .color(MUTED),
            );
            let mut auto = self.config.auto_gaming;
            let help = "Quand le mode est Désactivé : passe en Gaming dès qu'un jeu prioritaire est en plein écran, et revient 15 s après.";
            if style::toggle_row(ui, "Gaming automatique", help, &mut auto, true) {
                self.config.auto_gaming = auto;
                self.engine.auto_gaming.store(auto, Ordering::Relaxed);
                self.config.save();
                self.log.push(format!("Gaming automatique {}.", if auto { "activé" } else { "désactivé" }));
            }
            ui.label(RichText::new(format!("Programmes modifiés en ce moment : {}.", self.engine.changed_count())).size(11.5).color(MUTED));
        });

        let mut remove = None;
        let mut add = None;
        style::section(ui, "JEUX ET APPLIS PRIORITAIRES", |ui| {
            ui.horizontal_wrapped(|ui| {
                for f in &self.config.favorites {
                    if ui.button(format!("{f}  ✖")).on_hover_text("Retirer").clicked() {
                        remove = Some(f.clone());
                    }
                }
            });
            ui.horizontal(|ui| {
                let edit = ui.add(egui::TextEdit::singleline(&mut self.new_favorite).hint_text("jeu.exe").desired_width(180.0));
                let enter = edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter));
                if (ui.button("Ajouter").clicked() || enter) && !self.new_favorite.trim().is_empty() {
                    add = Some(std::mem::take(&mut self.new_favorite));
                }
                ui.label(RichText::new("ou ☆ dans la liste ci-dessous").color(MUTED));
            });
        });

        let mut change = None;
        style::section(ui, "PROGRAMMES", |ui| {
            egui::Grid::new("procs").striped(true).num_columns(5).spacing([16.0, 6.0]).show(ui, |ui| {
                ui.label("");
                ui.strong("Programme");
                ui.strong("Processeur");
                ui.strong("Mémoire");
                ui.strong("Priorité");
                ui.end_row();
                for p in &self.stats.procs {
                    let lower = p.name.to_lowercase();
                    let fav = self.config.favorites.contains(&lower);
                    let star = RichText::new(if fav { "★" } else { "☆" }).color(if fav { style::ACCENT } else { MUTED });
                    if ui.add(egui::Button::new(star).frame(false)).on_hover_text("Prioritaire en mode Gaming").clicked() {
                        if fav {
                            remove = Some(lower);
                        } else {
                            add = Some(p.name.clone());
                        }
                    }
                    ui.label(&p.name);
                    ui.label(format!("{:.1} %", p.cpu));
                    ui.label(fmt_bytes(p.mem));
                    let current = procs::PRIORITIES.get(p.priority as usize).copied().unwrap_or("Temps réel");
                    egui::ComboBox::from_id_salt(p.pid).selected_text(current).show_ui(ui, |ui| {
                        for (level, name) in procs::PRIORITIES.iter().enumerate() {
                            if ui.selectable_label(p.priority as usize == level, *name).clicked() {
                                change = Some((p.pid, p.name.clone(), level as u32, *name));
                            }
                        }
                    });
                    ui.end_row();
                }
            });
        });

        if let Some(exe) = add {
            let exe = exe.trim().to_lowercase();
            if !self.config.favorites.contains(&exe) {
                if self.config.persistent {
                    self.persist(&exe, Some(4));
                }
                self.log.push(format!("{exe} ajouté aux applis prioritaires."));
                self.config.favorites.push(exe);
                self.favorites_changed();
            }
        }
        if let Some(exe) = remove {
            self.config.favorites.retain(|f| *f != exe);
            if self.config.persisted.contains(&exe) {
                self.persist(&exe, None);
            }
            self.log.push(format!("{exe} retiré des applis prioritaires."));
            self.favorites_changed();
        }
        if let Some((pid, name, level, label)) = change {
            self.log.push(if ffi::set_priority(pid, level) {
                format!("{name} : priorité {label}.")
            } else {
                format!("Impossible de changer la priorité de {name} (admin requis ?).")
            });
            if self.config.persistent {
                self.persist(&name.to_lowercase(), Some(level));
                self.config.save();
            }
            self.last_refresh = None;
        }
    }

    pub(crate) fn ui_gaming(&mut self, ui: &mut egui::Ui) {
        style::page_header(ui, "Jeu", "Réglages dont l'effet est documenté par Microsoft ou mesurable. Activé = optimisation appliquée.");
        for (i, t) in gaming::TWEAKS.iter().enumerate() {
            let mut on = self.stats.gaming.get(i).copied().unwrap_or(false);
            let locked = t.admin && !self.admin;
            let mut help = t.help.to_string();
            if t.reboot {
                help += " Redémarrage requis.";
            }
            if locked {
                help += " Admin requis.";
            }
            if style::toggle_row(ui, t.label, &help, &mut on, !locked) {
                self.before_change();
                self.log.push(if (t.set)(on) {
                    format!("{} : optimisation {}.", t.label, if on { "appliquée" } else { "annulée" })
                } else {
                    format!("Impossible de modifier « {} ».", t.label)
                });
                self.last_refresh = None;
            }
        }
    }

    pub(crate) fn ui_privacy(&mut self, ui: &mut egui::Ui) {
        style::page_header(ui, "Confidentialité", "Coupe ce qui envoie des données à Microsoft. Certains changements prennent effet après un redémarrage.");
        let mut group = "";
        for (i, s) in privacy::SETTINGS.iter().enumerate() {
            if s.group != group {
                group = s.group;
                ui.add_space(10.0);
                ui.label(RichText::new(group.to_uppercase()).strong().size(12.5));
            }
            let mut on = self.stats.privacy.get(i).copied().unwrap_or(true);
            let locked = s.hklm && !self.admin;
            if style::toggle_row(ui, s.label, if locked { "Admin requis." } else { "" }, &mut on, !locked) {
                self.before_change();
                let state = if on { "activé" } else { "désactivé" };
                self.log.push(if s.set(on) {
                    format!("{} : {state}.", s.label)
                } else {
                    format!("Impossible de modifier « {} ».", s.label)
                });
                self.last_refresh = None;
            }
        }
    }

    pub(crate) fn ui_startup(&mut self, ui: &mut egui::Ui) {
        style::page_header(
            ui,
            "Démarrage",
            "Programmes lancés à l'ouverture de session. Les couper accélère le démarrage, sans les désinstaller (comme le Gestionnaire des tâches).",
        );
        let mut toggle = None;
        for (i, e) in self.stats.startup.iter().enumerate() {
            let mut on = e.enabled;
            let locked = e.hklm && !self.admin;
            let mut help = format!("{} · {}", if e.hklm { "Tous les utilisateurs" } else { "Ta session" }, e.command);
            if locked {
                help += " · Admin requis.";
            }
            if style::toggle_row(ui, &e.name, &help, &mut on, !locked) {
                toggle = Some((i, on));
            }
        }
        if self.stats.startup.is_empty() {
            ui.label("Aucun programme au démarrage.");
        }
        if let Some((i, on)) = toggle {
            self.before_change();
            let e = &self.stats.startup[i];
            self.log.push(if startup::set_enabled(e, on) {
                format!("{} : {} au démarrage.", e.name, if on { "activé" } else { "désactivé" })
            } else {
                format!("Impossible de modifier {} (admin requis ?).", e.name)
            });
            self.last_refresh = None;
        }
    }

    pub(crate) fn ui_installation(&mut self, ui: &mut egui::Ui) {
        style::page_header(
            ui,
            "Installation",
            "Coche les applis puis installe : version officielle la plus récente, sans fenêtre, via winget. Installer accepte la licence de chaque appli.",
        );
        if !self.installed_searched && self.task.is_none() {
            self.installed_searched = true;
            self.log.push("Recherche des applications déjà installées…".into());
            self.spawn(|tx| {
                let _ = tx.send(match installer::installed() {
                    Some(ids) => Msg::Installed(ids),
                    None => Msg::Log("winget introuvable : installe « App Installer » depuis le Microsoft Store.".into()),
                });
            });
        }

        let busy = self.task.is_some();
        ui.horizontal(|ui| {
            let n = self.install_selected.len();
            if ui.add_enabled(!busy && n > 0, egui::Button::new(format!("📥  Installer la sélection ({n})")).min_size(egui::vec2(0.0, 34.0))).clicked() {
                self.start_install();
            }
            if ui.add_enabled(!busy, egui::Button::new("⟳  Tout mettre à jour").min_size(egui::vec2(0.0, 34.0))).on_hover_text("Met à jour toutes les applis installées que winget connaît.").clicked() {
                self.start_upgrade_all();
            }
            if busy {
                ui.spinner();
            }
        });

        let mut categories: Vec<&str> = installer::APPS.iter().map(|a| a.category).collect();
        categories.dedup();
        for category in categories {
            style::section(ui, &category.to_uppercase(), |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing.x = 18.0;
                    for a in installer::APPS.iter().filter(|a| a.category == category) {
                        if self.installed.as_ref().is_some_and(|ids| ids.contains(a.id)) {
                            ui.add_enabled(false, egui::Checkbox::new(&mut true, format!("{} ✔", a.name))).on_disabled_hover_text("Déjà installé");
                            continue;
                        }
                        let mut on = self.install_selected.contains(a.id);
                        if ui.add_enabled(!busy, egui::Checkbox::new(&mut on, a.name)).changed() {
                            if on {
                                self.install_selected.insert(a.id);
                            } else {
                                self.install_selected.remove(a.id);
                            }
                        }
                    }
                });
            });
        }
    }
}
