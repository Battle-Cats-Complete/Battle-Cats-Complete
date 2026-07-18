use eframe::egui;
use tracing::{debug, info};

use core::common::context::GlobalContext;

use crate::modules::cat::state::show as show_cats;
use crate::modules::data::manager::show as show_data;
use crate::modules::enemy::state::show as show_enemies;
use crate::modules::home;
use crate::modules::mods::frame::show as show_mods;
use crate::modules::settings::show as show_settings;
use crate::modules::stage::master::show as show_stages;

use super::{BattleCatsApp, Page, ALL_PAGES};

pub(crate) fn draw(app: &mut BattleCatsApp, ctx: &egui::Context) {
    let screen_rect = ctx.screen_rect();

    let sidebar_inner_width = 150.0;
    let sidebar_margin = 15.0;
    let total_sidebar_width = sidebar_inner_width + (sidebar_margin * 2.0);

    let target_open = if app.sidebar_open { 1.0 } else { 0.0 };
    let anim_id = egui::Id::new("sb_anim");
    let open_factor = ctx.animate_value_with_time(anim_id, target_open, 0.35);

    let visible_sidebar_width = total_sidebar_width * open_factor;
    let width_id = egui::Id::new("sidebar_visible_width");
    ctx.data_mut(|data| data.insert_temp(width_id, visible_sidebar_width));

    if open_factor > 0.0 && open_factor < 1.0 {
        ctx.request_repaint();
    }

    if app.mod_state.data.needs_rescan {
        info!("Mod state flagged needs_rescan, initiating full data reload");
        app.mod_state.data.needs_rescan = false;
        app.perform_full_data_reload();
        ctx.request_repaint();
    }

    let mut style = (*ctx.style()).clone();
    style.visuals.window_rounding = egui::Rounding::same(10.0);
    style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(10.0);
    style.visuals.widgets.inactive.rounding = egui::Rounding::same(10.0);
    style.visuals.widgets.hovered.rounding = egui::Rounding::same(10.0);
    style.visuals.widgets.active.rounding = egui::Rounding::same(10.0);
    style.spacing.item_spacing = egui::vec2(10.0, 10.0);
    style.visuals.window_fill = egui::Color32::from_rgb(33, 33, 33);
    style.visuals.panel_fill = egui::Color32::from_rgb(33, 33, 33);
    style.visuals.override_text_color = Some(egui::Color32::WHITE);
    ctx.set_style(style);

    let global_ctx = GlobalContext {
        param: &app.param,
        localizable: &app.localizable,
    };

    match app.current_page {
        Page::Home => home::show(ctx, &mut app.current_page, &mut app.settings, &mut app.drag_guard),
        Page::Cats => show_cats(ctx, &mut app.cat_list_state, &mut app.settings, global_ctx, &mut app.drag_guard),
        Page::Enemies => show_enemies(ctx, &mut app.enemy_list_state, &mut app.settings, global_ctx, &mut app.drag_guard),
        Page::Stages => show_stages(ctx, &mut app.stage_list_state, &mut app.settings, global_ctx),
        Page::Mods => show_mods(ctx, &mut app.mod_state, &mut app.settings),
        Page::Data => {
            egui::CentralPanel::default().show(ctx, |ui| {
                show_data(ui, &mut app.import_state, &mut app.settings);
            });
        },
        Page::Settings => {
            let refresh_needed = show_settings(ctx, &mut app.settings, &mut app.drag_guard);
            if refresh_needed {
                info!("Settings change requested a UI refresh");
                app.perform_full_data_reload();
                ctx.request_repaint();
            }
        }
    }

    let sidebar_x = screen_rect.width() - visible_sidebar_width;
    let button_gap = 10.0;
    let button_size = 40.0;
    let button_x = sidebar_x - button_gap - button_size;

    if open_factor > 0.0 {
        egui::Area::new("sidebar_area".into())
            .constrain(false)
            .fixed_pos(egui::pos2(sidebar_x, 0.0))
            .order(egui::Order::Middle)
            .show(ctx, |ui| {
                egui::Frame::none()
                    .fill(egui::Color32::from_rgb(20, 20, 20))
                    .inner_margin(15.0)
                    .rounding(egui::Rounding { nw: 10.0, sw: 10.0, ne: 0.0, se: 0.0 })
                    .show(ui, |ui| {
                        ui.set_min_size(egui::vec2(sidebar_inner_width, screen_rect.height()));
                        ui.vertical_centered_justified(|ui| {

                            for page_enum in ALL_PAGES {
                                ui.add_space(5.0);
                                let button_text = egui::RichText::new(page_enum.tab_name()).size(16.0);
                                let is_selected = app.current_page == *page_enum;

                                let background_color = if is_selected {
                                    egui::Color32::from_rgb(31, 106, 165)
                                } else {
                                    egui::Color32::from_rgb(50, 50, 50)
                                };

                                let button_widget = egui::Button::new(button_text).fill(background_color).min_size(egui::vec2(0.0, 45.0));
                                if !ui.add_sized([ui.available_width(), 45.0], button_widget).clicked() {
                                    continue;
                                }

                                if app.current_page == *page_enum {
                                    continue;
                                }

                                debug!("Navigating to page: {}", page_enum.tab_name());
                                app.current_page = *page_enum;
                                app.settings.runtime.show_ip_field = false;
                            }

                        });
                    });
            });
    }

    egui::Area::new("toggle_btn".into())
        .fixed_pos(egui::pos2(button_x, 2.5))
        .order(egui::Order::Middle)
        .show(ctx, |ui| {
            let arrow = if app.sidebar_open { "▶" } else { "◀" };
            let arrow_text = egui::RichText::new(arrow).size(20.0).strong();
            let button_widget = egui::Button::new(arrow_text)
                .fill(egui::Color32::from_rgb(31, 106, 165));

            if ui.add_sized([40.0, 40.0], button_widget).clicked() {
                app.sidebar_open = !app.sidebar_open;
            }
        });
}