use eframe::egui;
use crate::global::ui::shared::DragGuard;

pub fn show(ui_container: &mut egui::Ui, drag_guard: &mut DragGuard) -> bool {
    let context = ui_container.ctx().clone();

    egui::ScrollArea::vertical()
        .id_salt("mods_settings_scroll")
        .auto_shrink([false, true])
        .show(ui_container, |scroll_ui| {
            scroll_ui.heading("Identity");
            scroll_ui.add_space(5.0);

            let manage_pem_button = egui::Button::new("Manage PEM")
                .fill(egui::Color32::from_rgb(40, 90, 160));

            if scroll_ui.add_sized([180.0, 30.0], manage_pem_button).clicked() {
                crate::features::settings::ui::pem::open(&context);
            }

            scroll_ui.add_space(20.0);
        });

    crate::features::settings::ui::pem::show(&context, drag_guard);

    false
}