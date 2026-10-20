//! Thème sombre à accent vert, cartes arrondies.

use eframe::egui::{self, Color32, FontId, Rounding, Stroke, TextStyle};

pub const BG: Color32 = Color32::from_rgb(0, 0, 0);
pub const SIDEBAR: Color32 = Color32::from_rgb(10, 10, 10);
pub const CARD: Color32 = Color32::from_rgb(20, 20, 20);
pub const BUTTON: Color32 = Color32::from_rgb(40, 40, 40);
pub const BORDER: Color32 = Color32::from_rgb(34, 34, 34);
pub const ACCENT: Color32 = Color32::from_rgb(34, 224, 122);
pub const RED: Color32 = Color32::from_rgb(235, 72, 72);
pub const TEXT: Color32 = Color32::from_rgb(236, 236, 236);
pub const MUTED: Color32 = Color32::from_rgb(140, 140, 140);

pub fn apply(ctx: &egui::Context) {
    let mut v = egui::Visuals::dark();
    v.panel_fill = BG;
    v.window_fill = CARD;
    v.extreme_bg_color = Color32::from_rgb(12, 12, 12);
    v.faint_bg_color = Color32::from_rgb(16, 16, 16);
    v.override_text_color = Some(TEXT);
    v.hyperlink_color = ACCENT;
    v.selection.bg_fill = ACCENT.gamma_multiply(0.35);
    v.selection.stroke = Stroke::new(1.0_f32, ACCENT);
    v.window_rounding = Rounding::same(12.0);
    for w in [&mut v.widgets.inactive, &mut v.widgets.hovered, &mut v.widgets.active, &mut v.widgets.open] {
        w.rounding = Rounding::same(8.0);
    }
    v.widgets.noninteractive.rounding = Rounding::same(8.0);
    v.widgets.inactive.weak_bg_fill = BUTTON;
    v.widgets.inactive.bg_fill = BUTTON;
    v.widgets.hovered.weak_bg_fill = Color32::from_rgb(52, 52, 52);
    v.widgets.hovered.bg_stroke = Stroke::new(1.0_f32, ACCENT.gamma_multiply(0.6));
    v.widgets.active.weak_bg_fill = ACCENT.gamma_multiply(0.5);
    v.widgets.noninteractive.bg_stroke = Stroke::new(1.0_f32, BORDER);
    ctx.set_visuals(v);

    ctx.style_mut(|s| {
        s.spacing.item_spacing = egui::vec2(8.0, 8.0);
        s.spacing.button_padding = egui::vec2(12.0, 6.0);
        s.text_styles.insert(TextStyle::Heading, FontId::proportional(24.0));
        s.text_styles.insert(TextStyle::Body, FontId::proportional(14.0));
        s.text_styles.insert(TextStyle::Button, FontId::proportional(14.0));
        s.text_styles.insert(TextStyle::Small, FontId::proportional(11.5));
    });
}

/// Carte arrondie sur fond sombre.
pub fn card<R>(ui: &mut egui::Ui, add: impl FnOnce(&mut egui::Ui) -> R) -> egui::InnerResponse<R> {
    egui::Frame::none().fill(CARD).rounding(12.0).inner_margin(14.0).show(ui, add)
}

/// Bouton plein, en capitales, comme dans les cartes du tableau de bord.
pub fn wide_button(ui: &mut egui::Ui, text: &str, width: f32) -> egui::Response {
    ui.add_sized([width, 30.0], egui::Button::new(egui::RichText::new(text).strong().size(12.0)))
}

/// Interrupteur on/off (tiré de la démo egui).
pub fn toggle(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let size = egui::vec2(40.0, 22.0);
    let (rect, mut resp) = ui.allocate_exact_size(size, egui::Sense::click());
    if resp.clicked() {
        *on = !*on;
        resp.mark_changed();
    }
    let t = ui.ctx().animate_bool_responsive(resp.id, *on);
    let enabled = ui.is_enabled();
    let track = if *on { ACCENT } else { Color32::from_rgb(60, 60, 60) };
    let track = if enabled { track } else { track.gamma_multiply(0.4) };
    let p = ui.painter();
    p.rect_filled(rect, rect.height() / 2.0, track);
    let x = egui::lerp((rect.left() + 11.0)..=(rect.right() - 11.0), t);
    let knob = if *on { Color32::BLACK } else { Color32::from_rgb(200, 200, 200) };
    p.circle_filled(egui::pos2(x, rect.center().y), 8.0, knob);
    resp
}

/// Ligne-carte : titre et aide à gauche, interrupteur à droite. Renvoie true si basculé.
pub fn toggle_row(ui: &mut egui::Ui, title: &str, help: &str, on: &mut bool, enabled: bool) -> bool {
    egui::Frame::none()
        .fill(CARD)
        .rounding(10.0)
        .inner_margin(egui::Margin::symmetric(16.0, 10.0))
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.vertical(|ui| {
                    let color = if enabled { TEXT } else { MUTED };
                    ui.label(egui::RichText::new(title).color(color));
                    if !help.is_empty() {
                        ui.label(egui::RichText::new(help).size(11.5).color(MUTED));
                    }
                });
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.add_enabled_ui(enabled, |ui| toggle(ui, on)).inner.changed()
                })
                .inner
            })
            .inner
        })
        .inner
}

/// Titre de page et sous-titre gris.
pub fn page_header(ui: &mut egui::Ui, title: &str, subtitle: &str) {
    ui.heading(title);
    ui.label(egui::RichText::new(subtitle).color(MUTED));
    ui.add_space(6.0);
}

/// Section : petit titre en capitales puis contenu dans une carte pleine largeur.
pub fn section<R>(ui: &mut egui::Ui, title: &str, add: impl FnOnce(&mut egui::Ui) -> R) -> R {
    ui.add_space(8.0);
    ui.label(egui::RichText::new(title).strong().size(12.5));
    card(ui, |ui| {
        ui.set_width(ui.available_width());
        add(ui)
    })
    .inner
}
