use eframe::egui;
use crate::features::settings::logic::Settings;
use crate::global::ui::shared::DragGuard;

pub fn show(context: &egui::Context, settings: &mut Settings, drag_guard: &mut DragGuard) -> bool {
    let mut refresh_needed = false;
    let tabs = ["General", "Cats", "Enemies", "Mods", "Data", "Animation", "Add-Ons", "About"];

    egui::CentralPanel::default().show(context, |ui_container| {
        ui_container.horizontal(|ui_row| {
            ui_row.spacing_mut().item_spacing.x = 5.0;

            for tab_name in tabs {
                let is_selected = settings.runtime.active_tab == *tab_name;
                let background_color = if is_selected {
                    egui::Color32::from_rgb(31, 106, 165)
                } else {
                    egui::Color32::from_gray(60)
                };

                let tab_button = egui::Button::new(
                    egui::RichText::new(tab_name)
                        .color(egui::Color32::WHITE)
                        .size(14.0)
                )
                    .fill(background_color)
                    .min_size(egui::vec2(80.0, 30.0));

                if ui_row.add(tab_button).clicked() {
                    settings.runtime.active_tab = tab_name.to_string();
                    settings.runtime.show_ip_field = false;
                }
            }
        });

        ui_container.add_space(5.0);
        ui_container.separator();
        ui_container.add_space(10.0);

        egui::ScrollArea::vertical().show(ui_container, |scroll_ui| {
            let current_tab = settings.runtime.active_tab.clone();

            scroll_ui.push_id(&current_tab, |tab_ui| {
                let action_result = match current_tab.as_str() {
                    "General" => super::general::show(tab_ui, &mut settings.general, &mut settings.runtime),
                    "Cats" => super::cats::show(tab_ui, &mut settings.cat_data),
                    "Enemies" => super::enemies::show(tab_ui, &mut settings.enemy_data),
                    "Mods" => super::mods::show(tab_ui, drag_guard),
                    "Data" => super::data::show(tab_ui, &mut settings.game_data, &mut settings.runtime, drag_guard),
                    "Animation" => super::animation::show(tab_ui, &mut settings.animation),
                    "Add-Ons" => super::addons::show(tab_ui, drag_guard),
                    "About" => super::about::show(tab_ui),
                    _ => {
                        tab_ui.vertical_centered(|centered_ui| {
                            centered_ui.add_space(50.0);
                            centered_ui.label(egui::RichText::new("No settings available for this category.").weak().size(16.0));
                        });
                        false
                    }
                };

                if action_result { refresh_needed = true; }
            });
        });
    });

    refresh_needed
}

pub fn toggle_ui(ui_container: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let desired_size = ui_container.spacing().interact_size.y * egui::vec2(2.0, 1.0);
    let (allocation_rect, mut response) = ui_container.allocate_exact_size(desired_size, egui::Sense::click());

    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }

    if !ui_container.is_rect_visible(allocation_rect) {
        return response;
    }

    let animation_progress = ui_container.ctx().animate_bool(response.id, *on);
    let visuals = ui_container.style().interact_selectable(&response, *on);
    let expansion_rect = allocation_rect.expand(visuals.expansion);
    let radius = 0.5 * expansion_rect.height();

    ui_container.painter().rect(expansion_rect, radius, visuals.bg_fill, visuals.bg_stroke);

    let circle_x_pos = egui::lerp((expansion_rect.left() + radius)..=(expansion_rect.right() - radius), animation_progress);
    ui_container.painter().circle(egui::pos2(circle_x_pos, expansion_rect.center().y), 0.75 * radius, visuals.fg_stroke.color, visuals.fg_stroke);

    response
}