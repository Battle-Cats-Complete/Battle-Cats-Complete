use eframe::egui;
use serde::{Deserialize, Serialize};

use crate::global::shared::DragGuard;
use crate::global::utils::process_markdown;

// NOTE: No notice will appear if NOTICE_CONTENT is empty
pub const NOTICE_TITLE: &str = "NOTICE";
pub const NOTICE_CONTENT: &str = r#"
Battle Cats Complete has officially hit version 1.0.0! Featuring Cat stats, Enemy stats, Animations, Stages, and Mods!

You will have to re-sort your database to allow stages to be parsed correctly. If you have a pre-existing database go to "Data" page, select "Raw" import type, change the drop down from "Archive" to "Folder" (if it isnt already), and select your "game" folder for import.

Updates will not cease entirely, the project will only slow down. Extra utilities will occasionally be added, as well as minor features. And of course, bugs will always be fixed. The backend may also be reworked a few times, which will allow to app to grow faster!

That doesnt mean Battle Cats Complete is "done" however. There are plans for more daunting features in the future. But for now, the developer is going to take a break and go in "Maintenance Mode" for a while.
"#;

#[derive(Serialize, Deserialize, Default)]
struct AppMeta {
    app_version: String,
}

pub fn check_and_show(ctx: &egui::Context, drag_guard: &mut DragGuard) {
    if NOTICE_CONTENT.trim().is_empty() {
        return;
    }

    let state_id = egui::Id::new("notice_state");
    let mut is_open = ctx.data(|d| d.get_temp::<Option<bool>>(state_id)).flatten();

    let current_version = env!("CARGO_PKG_VERSION").to_string();

    if is_open.is_none() {
        let needs_notice = match core::global::io::json::load::<AppMeta>("meta.json") {
            Some(meta) => meta.app_version != current_version,
            None => true,
        };

        is_open = Some(needs_notice);
        ctx.data_mut(|d| d.insert_temp(state_id, Some(needs_notice)));
    }

    let mut show_window = is_open.unwrap_or(false);

    if show_window {
        let window_id = egui::Id::new("NoticeWindow");
        let (allow_drag, fixed_pos) = drag_guard.assign_bounds(ctx, window_id);

        let mut window = egui::Window::new(NOTICE_TITLE)
            .id(window_id)
            .collapsible(false)
            .resizable(false)
            .constrain(false)
            .movable(allow_drag)
            .default_pos(ctx.screen_rect().center() - egui::vec2(250.0, 150.0));

        if let Some(pos) = fixed_pos { window = window.current_pos(pos); }

        window.show(ctx, |ui| {
            ui.set_max_size([500.0, 400.0].into());

            egui::ScrollArea::vertical().auto_shrink([false, true]).show(ui, |ui| {
                ui.spacing_mut().item_spacing.y = 0.0;
                
                process_markdown(ui, NOTICE_CONTENT);
            });

            ui.add_space(20.0);
            ui.vertical_centered(|ui| {
                if ui.add_sized([120.0, 35.0], egui::Button::new(egui::RichText::new("Acknowledge").size(16.0).strong())).clicked() {
                    show_window = false;
                    ctx.data_mut(|d| d.insert_temp(state_id, Some(false)));
                    
                    let new_meta = AppMeta { app_version: current_version.clone() };
                    core::global::io::json::save("meta.json", &new_meta);
                }
            });
        });
    }
}