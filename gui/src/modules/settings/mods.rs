use eframe::egui;
use tracing::{debug, trace};

use core::modules::settings::{ExportBehavior, ModsSettings};

use crate::common::shared::DragGuard;

pub(crate) fn show(ui_container: &mut egui::Ui, settings: &mut ModsSettings, drag_guard: &mut DragGuard) -> bool {
    let context = ui_container.ctx().clone();
    let mut refresh_needed = false;

    egui::ScrollArea::vertical()
        .id_salt("mods_settings_scroll")
        .auto_shrink([false, true])
        .show(ui_container, |scroll_ui| {
            scroll_ui.heading("Export");
            scroll_ui.add_space(5.0);

            let manage_pem_button = egui::Button::new("Manage PEM")
                .fill(egui::Color32::from_rgb(40, 90, 160));

            if scroll_ui.add_sized([180.0, 30.0], manage_pem_button).clicked() {
                trace!("Manage PEM button clicked");
                crate::modules::settings::pem::open(&context);
            }

            scroll_ui.add_space(10.0);

            scroll_ui.horizontal(|ui| {
                let label_response = ui.label("Export Behavior");
                let tooltip_text = "Determines whether to scan and automatically choose, always create a new APK, or always overwrite the input APK.";
                label_response.on_hover_text(tooltip_text);

                egui::ComboBox::from_id_salt("export_behavior_combo")
                    .selected_text(match settings.export_behavior {
                        ExportBehavior::Automatic => "Automatic",
                        ExportBehavior::Create => "Create",
                        ExportBehavior::Update => "Update",
                    })
                    .show_ui(ui, |ui| {
                        if ui.selectable_value(&mut settings.export_behavior, ExportBehavior::Automatic, "Automatic").changed() {
                            debug!("Export behavior set to Automatic");
                            refresh_needed = true;
                        }
                        if ui.selectable_value(&mut settings.export_behavior, ExportBehavior::Create, "Create").changed() {
                            debug!("Export behavior set to Create");
                            refresh_needed = true;
                        }
                        if ui.selectable_value(&mut settings.export_behavior, ExportBehavior::Update, "Update").changed() {
                            debug!("Export behavior set to Update");
                            refresh_needed = true;
                        }
                    });
            });

            scroll_ui.add_space(20.0);
        });

    crate::modules::settings::pem::show(&context, drag_guard);

    refresh_needed
}