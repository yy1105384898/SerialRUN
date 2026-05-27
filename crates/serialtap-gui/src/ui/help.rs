use crate::state::{Language, T};
use eframe::egui;

pub fn render_help_panel(ui: &mut egui::Ui, lang: Language) {
    ui.add_space(4.0);

    // Quick Start
    ui.heading(T::quick_start(lang));
    ui.add_space(4.0);
    ui.label(T::step1(lang));
    ui.label(T::step2(lang));
    ui.label(T::step3(lang));
    ui.label(T::step4(lang));

    ui.add_space(12.0);

    // Features
    ui.heading(T::features(lang));
    ui.add_space(4.0);

    let features = [
        T::feature_send(lang),
        T::feature_log(lang),
        T::feature_chart(lang),
        T::feature_auto_reply(lang),
        T::feature_record(lang),
    ];
    for f in &features {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("•").color(egui::Color32::from_rgb(0, 180, 120)));
            ui.label(*f);
        });
    }

    ui.add_space(12.0);

    // Tips
    ui.heading(T::tips(lang));
    ui.add_space(4.0);

    let tips = [T::tip1(lang), T::tip2(lang), T::tip3(lang)];
    for t in &tips {
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new("💡").color(egui::Color32::from_rgb(255, 200, 0)));
            ui.label(*t);
        });
    }
}
