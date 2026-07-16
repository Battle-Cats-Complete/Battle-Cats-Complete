use eframe::egui;

use core::settings::logic::state::{SidebarBehavior, StageDataSettings};

pub fn show(ui_container: &mut egui::Ui, settings: &mut StageDataSettings) -> bool {
    let mut refresh_needed = false;

    egui::ScrollArea::vertical()
        .id_salt("stages_scroll")
        .auto_shrink([false, true])
        .show(ui_container, |scroll_ui| {
            scroll_ui.heading("Stage List");
            scroll_ui.add_space(5.0);

            scroll_ui.horizontal(|horizontal_ui| {
                horizontal_ui.label("Sidebar Behavior:");

                let original = settings.sidebar_behavior.clone();

                let selected_text = match settings.sidebar_behavior {
                    SidebarBehavior::Cover => "Cover",
                    SidebarBehavior::Push => "Push",
                };

                egui::ComboBox::from_id_salt("stage_sidebar_behavior")
                    .selected_text(selected_text)
                    .show_ui(horizontal_ui, |combo_ui| {
                        combo_ui.selectable_value(&mut settings.sidebar_behavior, SidebarBehavior::Cover, "Cover")
                            .on_hover_text("Allows the stage select list sidebar to cover stage contents");
                        combo_ui.selectable_value(&mut settings.sidebar_behavior, SidebarBehavior::Push, "Push")
                            .on_hover_text("Allows the stage select sidebar to push stage contents\nIt is recommended you use fullscreen or extend the apps\nwindow horizontally if you use this setting");
                    });

                if original != settings.sidebar_behavior {
                    refresh_needed = true;
                }
            });
        });

    refresh_needed
}