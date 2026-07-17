use eframe::egui;

use core::modules::settings::AnimSettings;

use super::tabs::toggle_ui;

pub(crate) fn show(ui: &mut egui::Ui, settings: &mut AnimSettings) -> bool {
    let refresh_needed = false;

    egui::ScrollArea::vertical()
        .id_salt("anim_view_scroll")
        .auto_shrink([false, true])
        .show(ui, |ui| {
            
            ui.heading("Viewer");
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                ui.label("Centering Behavior:");
                egui::ComboBox::from_id_salt("centering_behavior")
                    .width(80.0)
                    .selected_text(match settings.centering_behavior {
                        0 => "Unit",
                        1 => "Origin",
                        2 => "None",
                        _ => "Unit",
                    })
                    .show_ui(ui, |ui| {
                        ui.selectable_value(&mut settings.centering_behavior, 0, "Unit")
                            .on_hover_text("Automatically center the unit on load (Default)");
                        ui.selectable_value(&mut settings.centering_behavior, 1, "Origin")
                            .on_hover_text("Reset camera to (0,0) on load");
                        ui.selectable_value(&mut settings.centering_behavior, 2, "None")
                            .on_hover_text("Keep camera position/zoom when switching units/forms");
                    });
            });

            ui.horizontal(|ui| {
                if toggle_ui(ui, &mut settings.debug_view).changed() {
                    ui.ctx().request_repaint();
                }
                ui.label("Enable Debug View");
            });

            ui.add_space(20.0);
            ui.heading("Exporter");
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                let tooltip_tight = "Automatically crops out minor vfx and glow when calculating camera bounds";
                toggle_ui(ui, &mut settings.use_tight_bounds).on_hover_text(tooltip_tight);
                ui.label("Use Tight Bounds").on_hover_text(tooltip_tight);
            });

            ui.horizontal(|ui| {
                let tooltip_cam = "Automatically calculates a Units tight bounding box when exporting\nThis setting may cause lag spikes on some devices";
                toggle_ui(ui, &mut settings.auto_set_camera_region).on_hover_text(tooltip_cam);
                ui.label("Auto-Set Camera Region").on_hover_text(tooltip_cam);
            });

            ui.add_space(5.0);
            ui.label(egui::RichText::new("Showcase").strong());
            
            ui.horizontal(|ui| {
                ui.label("Default Walk Frames");
                ui.add(egui::DragValue::new(&mut settings.default_showcase_walk).speed(1.0).range(0..=1000));
            });
            ui.horizontal(|ui| {
                ui.label("Default Idle Frames");
                ui.add(egui::DragValue::new(&mut settings.default_showcase_idle).speed(1.0).range(0..=1000));
            });
            ui.horizontal(|ui| {
                ui.label("Default Kb Frames");
                ui.add(egui::DragValue::new(&mut settings.default_showcase_kb).speed(1.0).range(0..=1000));
            });
        });

    refresh_needed
}