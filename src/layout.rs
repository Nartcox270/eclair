//! Cadre de la fenêtre (barre de titre, barre d'icônes, journal) et tableau de bord.

use crate::style::{self, ACCENT, BORDER, CARD, MUTED, RED, SIDEBAR, TEXT};
use crate::{fmt_bytes, gaming, privacy, procs, restore, App, Tab};
use eframe::egui::{self, Align2, Color32, FontId, Pos2, RichText, Sense, Stroke, Vec2};
use std::f32::consts::TAU;
use std::sync::atomic::Ordering;

const NAV: [(Tab, &str, &str); 8] = [
    (Tab::Dashboard, "🏠", "Tableau de bord"),
    (Tab::Performance, "🚀", "Performance"),
    (Tab::Gaming, "🎮", "Jeu"),
    (Tab::Cleaning, "🗑", "Nettoyage"),
    (Tab::Processes, "📊", "Processus"),
    (Tab::Startup, "▶", "Démarrage"),
    (Tab::Privacy, "🔒", "Confidentialité"),
    (Tab::Installation, "📦", "Installation"),
];

/// Bouton-icône de la barre latérale ; contour vert quand l'onglet est ouvert.
fn nav_button(ui: &mut egui::Ui, icon: &str, selected: bool) -> egui::Response {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(44.0), Sense::click());
    let p = ui.painter();
    if selected {
        p.rect_filled(rect, 10.0, ACCENT.gamma_multiply(0.08));
        p.rect_stroke(rect, 10.0, Stroke::new(1.5_f32, ACCENT));
    } else if resp.hovered() {
        p.rect_filled(rect, 10.0, Color32::from_rgb(28, 28, 28));
    }
    let color = if selected { ACCENT } else if resp.hovered() { TEXT } else { MUTED };
    p.text(rect.center(), Align2::CENTER_CENTER, icon, FontId::proportional(20.0), color);
    resp
}

impl App {
    pub fn ui_frame(&mut self, ctx: &egui::Context) {
        egui::SidePanel::left("nav")
            .exact_width(68.0)
            .resizable(false)
            .frame(egui::Frame::none().fill(SIDEBAR).stroke(Stroke::new(1.0_f32, BORDER)).inner_margin(12.0))
            .show(ctx, |ui| {
                ui.vertical_centered(|ui| {
                    ui.add_space(4.0);
                    ui.label(RichText::new("⚡").size(26.0).color(ACCENT));
                    ui.add_space(14.0);
                    for (tab, icon, name) in NAV {
                        if nav_button(ui, icon, self.tab == tab).on_hover_text(name).clicked() {
                            self.tab = tab;
                        }
                        ui.add_space(4.0);
                    }
                    ui.with_layout(egui::Layout::bottom_up(egui::Align::Center), |ui| {
                        ui.label(RichText::new(concat!("v", env!("CARGO_PKG_VERSION"))).size(10.0).color(MUTED));
                        if nav_button(ui, "⚙", self.tab == Tab::Settings).on_hover_text("Réglages").clicked() {
                            self.tab = Tab::Settings;
                        }
                    });
                });
            });

        // Barre de titre maison : la fenêtre n'a pas de bordure Windows.
        egui::TopBottomPanel::top("titlebar")
            .exact_height(34.0)
            .frame(egui::Frame::none().fill(style::BG).inner_margin(egui::Margin::symmetric(10.0, 4.0)))
            .show(ctx, |ui| {
                let drag = ui.interact(ui.max_rect(), ui.id().with("drag"), Sense::click_and_drag());
                if drag.drag_started() {
                    ctx.send_viewport_cmd(egui::ViewportCommand::StartDrag);
                }
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let close = ui.add(egui::Button::new(RichText::new("🗙").size(15.0)).frame(false));
                    if close.on_hover_text("Fermer").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                    let min = ui.add(egui::Button::new(RichText::new("🗕").size(15.0)).frame(false));
                    if min.on_hover_text("Réduire").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Minimized(true));
                    }
                    if !self.admin {
                        ui.label(RichText::new("Mode limité : lancez en administrateur").size(11.0).color(RED));
                    }
                });
            });

        // Journal replié : dernière ligne seulement, clic pour tout voir.
        egui::TopBottomPanel::bottom("log")
            .frame(egui::Frame::none().fill(SIDEBAR).inner_margin(egui::Margin::symmetric(16.0, 6.0)))
            .show(ctx, |ui| {
                ui.horizontal(|ui| {
                    let arrow = if self.show_log { "🔽 Journal" } else { "🔼 Journal" };
                    if ui.add(egui::Button::new(RichText::new(arrow).size(12.0)).frame(false)).clicked() {
                        self.show_log = !self.show_log;
                    }
                    if let Some(last) = self.log.last() {
                        ui.label(RichText::new(last).size(12.0).color(MUTED));
                    }
                    if self.task.is_some() {
                        ui.spinner();
                    }
                });
                if self.show_log {
                    egui::ScrollArea::vertical().max_height(150.0).stick_to_bottom(true).auto_shrink([false, true]).show(ui, |ui| {
                        for line in &self.log {
                            ui.label(RichText::new(line).monospace().size(12.0));
                        }
                    });
                }
            });

        egui::CentralPanel::default()
            .frame(egui::Frame::none().fill(style::BG).inner_margin(egui::Margin { left: 28.0, right: 28.0, top: 8.0, bottom: 16.0 }))
            .show(ctx, |ui| match self.tab {
                Tab::Dashboard => self.ui_dashboard(ui),
                Tab::Settings => self.ui_settings(ui),
                tab => {
                    egui::ScrollArea::vertical().auto_shrink(false).show(ui, |ui| match tab {
                        Tab::Cleaning => self.ui_cleaning(ui),
                        Tab::Performance => self.ui_performance(ui),
                        Tab::Privacy => self.ui_privacy(ui),
                        Tab::Processes => self.ui_processes(ui),
                        Tab::Gaming => self.ui_gaming(ui),
                        Tab::Startup => self.ui_startup(ui),
                        Tab::Installation => self.ui_installation(ui),
                        Tab::Dashboard | Tab::Settings => {}
                    });
                }
            });
    }

    /// (optimisations appliquées, total) : réglages jeu recommandés + confidentialité.
    fn health(&self) -> (usize, usize) {
        let tweaks = gaming::TWEAKS.iter().zip(&self.stats.gaming).filter(|(t, _)| t.recommended);
        let (mut done, mut total) = (0, 0);
        for (_, &on) in tweaks {
            done += on as usize;
            total += 1;
        }
        // Un réglage de confidentialité décoché = optimisé.
        done += self.stats.privacy.iter().filter(|on| !**on).count();
        total += privacy::SETTINGS.len();
        (done, total)
    }

    fn ui_dashboard(&mut self, ui: &mut egui::Ui) {
        let (done, total) = self.health();
        let score = if total == 0 { 0.0 } else { done as f32 / total as f32 };

        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 0.0;
                    ui.label(RichText::new("Bonjour, ").size(28.0).strong());
                    ui.label(RichText::new(&self.user).size(28.0).strong().color(ACCENT));
                    ui.label(RichText::new(" !").size(28.0).strong());
                });
                ui.label(RichText::new("Bienvenue dans Éclair.").color(MUTED));
            });
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| health_ring(ui, score));
        });
        ui.add_space(14.0);

        let gap = 12.0;
        let right_w = 230.0;
        let left_w = ui.available_width() - right_w - gap;
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing = Vec2::splat(gap);
            ui.vertical(|ui| {
                ui.set_width(left_w);
                let card_w = (left_w - 2.0 * gap) / 3.0;
                let startup_on = self.stats.startup.iter().filter(|e| e.enabled).count();
                let boosted = self.config.favorites.len();
                ui.horizontal(|ui| {
                    if stat_card(ui, card_w, "🚀", done.to_string(), ACCENT, "Optimisations actives", "OPTIMISATIONS") {
                        self.tab = Tab::Gaming;
                    }
                    // Beaucoup de programmes au démarrage ralentissent l'ouverture de session.
                    let color = if startup_on > 8 { RED } else { TEXT };
                    if stat_card(ui, card_w, "▶", startup_on.to_string(), color, "Programmes au démarrage", "DÉMARRAGE") {
                        self.tab = Tab::Startup;
                    }
                    if stat_card(ui, card_w, "🎮", boosted.to_string(), TEXT, "Jeux boostés", "JEUX") {
                        self.tab = Tab::Processes;
                    }
                });
                self.usage_chart(ui, left_w);
            });
            self.status_card(ui, right_w, score);
        });
        ui.add_space(4.0);

        let gaming_on = self.engine.mode.load(Ordering::Relaxed) == procs::MODE_GAMING;
        let auto_on = !gaming_on && self.engine.effective.load(Ordering::Relaxed) == procs::MODE_GAMING;
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = gap;
            if tile(ui, "🗑", "Nettoyage", false) {
                self.tab = Tab::Cleaning;
            }
            if tile(ui, "💾", "Libérer la RAM", false) && self.task.is_none() {
                self.trim_memory();
            }
            let label = if gaming_on { "Gaming : ON" } else if auto_on { "Gaming : auto" } else { "Mode Gaming" };
            if tile(ui, "🎮", label, gaming_on || auto_on) {
                let m = if gaming_on { procs::MODE_OFF } else { procs::MODE_GAMING };
                self.set_mode(m);
            }
            if tile(ui, "🛡", "Restauration", false) {
                self.tab = Tab::Settings;
            }
            if tile(ui, "📦", "Installation", false) {
                self.tab = Tab::Installation;
            }
        });
    }

    fn usage_chart(&self, ui: &mut egui::Ui, width: f32) {
        style::card(ui, |ui| {
            ui.set_width(width - 28.0);
            ui.horizontal(|ui| {
                ui.label(RichText::new("UTILISATION SYSTÈME").strong());
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    let s = &self.stats;
                    if s.gpu.is_some() {
                        legend(ui, "GPU", GPU);
                    }
                    legend(ui, "CPU", ACCENT);
                    legend(ui, "RAM", TEXT);
                    let gpu = s.gpu.map_or(String::new(), |g| format!("   GPU {g:.0} %"));
                    ui.label(RichText::new(format!("CPU {:.0} %   RAM {} %{gpu}", s.cpu, s.mem.load_percent)).size(11.0).color(MUTED));
                });
            });
            let (rect, _) = ui.allocate_exact_size(egui::vec2(ui.available_width(), 190.0), Sense::hover());
            let p = ui.painter();
            for i in 0..=4 {
                let y = rect.top() + rect.height() * i as f32 / 4.0;
                p.extend(egui::Shape::dashed_line(&[Pos2::new(rect.left() + 34.0, y), Pos2::new(rect.right(), y)], Stroke::new(1.0_f32, BORDER), 3.0, 5.0));
            }
            p.text(rect.left_top(), Align2::LEFT_TOP, "100%", FontId::proportional(11.0), MUTED);
            p.text(rect.left_bottom(), Align2::LEFT_BOTTOM, "0%", FontId::proportional(11.0), MUTED);
            let plot = rect.with_min_x(rect.left() + 38.0);
            let n = HISTORY.max(2) - 1;
            let mut series_list = vec![(1, TEXT), (0, ACCENT)];
            if self.stats.gpu.is_some() {
                series_list.insert(0, (2, GPU));
            }
            for (series, color) in series_list {
                let pts: Vec<Pos2> = self
                    .history
                    .iter()
                    .enumerate()
                    .map(|(i, v)| {
                        let x = plot.right() - (self.history.len() - 1 - i) as f32 * plot.width() / n as f32;
                        Pos2::new(x, plot.bottom() - v[series].clamp(0.0, 100.0) / 100.0 * plot.height())
                    })
                    .collect();
                p.add(egui::Shape::line(pts, Stroke::new(1.8_f32, color)));
            }
        });
    }

    fn status_card(&self, ui: &mut egui::Ui, width: f32, score: f32) {
        style::card(ui, |ui| {
            ui.set_width(width - 28.0);
            ui.set_height(346.0);
            ui.vertical_centered(|ui| {
                ui.label(RichText::new("STATUT DU PC").strong());
                ui.add_space(40.0);
                let (text, color) = match score {
                    s if s >= 0.75 => ("Optimisé", ACCENT),
                    s if s >= 0.4 => ("En forme", Color32::from_rgb(240, 190, 60)),
                    _ => ("À optimiser", RED),
                };
                let (rect, _) = ui.allocate_exact_size(Vec2::splat(140.0), Sense::hover());
                let p = ui.painter();
                p.circle_filled(rect.center(), 66.0, color.gamma_multiply(0.12));
                p.circle_stroke(rect.center(), 66.0, Stroke::new(2.0_f32, color.gamma_multiply(0.7)));
                p.text(rect.center(), Align2::CENTER_CENTER, "🖥", FontId::proportional(58.0), color);
                ui.add_space(18.0);
                ui.label(RichText::new(text).size(18.0).strong().color(color));
                let s = &self.stats;
                if let Some((free, _)) = s.disk {
                    ui.label(RichText::new(format!("{} libres sur C:", fmt_bytes(free))).size(12.0).color(MUTED));
                }
                let secs = s.uptime_ms / 1000;
                ui.label(RichText::new(format!("Allumé depuis {} h {} min", secs / 3600, secs / 60 % 60)).size(12.0).color(MUTED));
            });
        });
    }

    fn ui_settings(&mut self, ui: &mut egui::Ui) {
        ui.heading("Réglages");
        ui.add_space(8.0);
        style::card(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new("Application des réglages").strong());
            let persistent = self.config.persistent;
            if ui.radio(!persistent, "Temporaire : tout est annulé quand Éclair est quitté").clicked() && persistent {
                self.set_persistent(false);
            }
            if ui
                .radio(persistent, "Persistant : réglages rétablis à chaque démarrage de Windows, applis prioritaires enregistrées dans Windows")
                .clicked()
                && !persistent
            {
                self.set_persistent(true);
            }
        });
        ui.add_space(8.0);
        style::card(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new("Restauration").strong());
            ui.label(
                RichText::new("Un point de restauration est créé automatiquement avant le premier changement de chaque session. Il permet d'annuler tous les réglages d'Éclair depuis Windows.")
                    .color(MUTED),
            );
            ui.horizontal(|ui| {
                if ui.add_enabled(self.admin, egui::Button::new("🛡  Créer un point de restauration")).on_disabled_hover_text("Admin requis").clicked() {
                    self.create_restore_point();
                }
                if ui.button("↺  Ouvrir la Restauration du système").clicked() && !restore::open_system_restore() {
                    self.log.push("Impossible d'ouvrir la Restauration du système.".into());
                }
            });
        });
        ui.add_space(8.0);
        style::card(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.label(RichText::new("À propos").strong());
            ui.label(concat!("Éclair v", env!("CARGO_PKG_VERSION"), " — interface Rust, coeur C++."));
            ui.label(format!("Droits administrateur : {}", if self.admin { "oui" } else { "non (certaines actions sont limitées)" }));
            ui.label(RichText::new("Fermer la fenêtre laisse Éclair tourner près de l'horloge (modes, purge mémoire).").color(MUTED));
            if ui.button("Quitter Éclair").clicked() {
                self.quitting = true;
                ui.ctx().send_viewport_cmd(egui::ViewportCommand::Close);
            }
        });
    }
}

pub const HISTORY: usize = 60;
const GPU: Color32 = Color32::from_rgb(110, 160, 255);

fn legend(ui: &mut egui::Ui, name: &str, color: Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(22.0, 10.0), Sense::hover());
    ui.painter().line_segment([rect.left_center(), rect.right_center()], Stroke::new(1.8_f32, color));
    ui.label(RichText::new(name).size(11.0).color(MUTED));
}

/// Anneau "SANTÉ" : arc vert proportionnel au score.
fn health_ring(ui: &mut egui::Ui, score: f32) {
    let (rect, resp) = ui.allocate_exact_size(Vec2::splat(84.0), Sense::hover());
    resp.on_hover_text("Part des optimisations recommandées (Jeu et Confidentialité) déjà appliquées.");
    let p = ui.painter();
    let c = rect.center();
    p.circle_stroke(c, 38.0, Stroke::new(4.0_f32, Color32::from_rgb(40, 40, 40)));
    let steps = (64.0 * score).ceil() as usize;
    if steps > 0 {
        let pts: Vec<Pos2> = (0..=steps)
            .map(|i| {
                let a = -TAU / 4.0 + TAU * score * i as f32 / steps as f32;
                c + 38.0 * Vec2::angled(a)
            })
            .collect();
        p.add(egui::Shape::line(pts, Stroke::new(4.0_f32, ACCENT)));
    }
    p.text(c - Vec2::new(0.0, 12.0), Align2::CENTER_CENTER, "SANTÉ", FontId::proportional(10.0), MUTED);
    p.text(c + Vec2::new(0.0, 6.0), Align2::CENTER_CENTER, format!("{:.0}%", score * 100.0), FontId::proportional(22.0), TEXT);
}

/// Carte statistique : icône + nombre, libellé, bouton. Renvoie true si le bouton est cliqué.
fn stat_card(ui: &mut egui::Ui, width: f32, icon: &str, value: String, color: Color32, label: &str, button: &str) -> bool {
    style::card(ui, |ui| {
        ui.set_width(width - 28.0);
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.label(RichText::new(icon).size(22.0).color(MUTED));
                ui.label(RichText::new(value).size(22.0).strong().color(color));
            });
            ui.label(label);
            style::wide_button(ui, button, ui.available_width()).clicked()
        })
        .inner
    })
    .inner
}

/// Tuile d'action rapide (grande icône + libellé).
fn tile(ui: &mut egui::Ui, icon: &str, label: &str, active: bool) -> bool {
    let (rect, resp) = ui.allocate_exact_size(egui::vec2(112.0, 76.0), Sense::click());
    let p = ui.painter();
    let fill = if resp.hovered() { Color32::from_rgb(30, 30, 30) } else { CARD };
    p.rect_filled(rect, 12.0, fill);
    if active {
        p.rect_stroke(rect, 12.0, Stroke::new(1.5_f32, ACCENT));
    }
    let color = if active { ACCENT } else { TEXT };
    p.text(rect.center() - Vec2::new(0.0, 10.0), Align2::CENTER_CENTER, icon, FontId::proportional(26.0), color);
    p.text(rect.center() + Vec2::new(0.0, 22.0), Align2::CENTER_CENTER, label, FontId::proportional(12.0), color);
    resp.clicked()
}
