use std::collections::HashSet;

use eframe::egui;
use nyanko::enemy::abilities::{Identity, REGISTRY};

use core::modules::enemy::filter::{evaluation::get_identity_name, ATTACK_TYPE_IDENTITIES, EnemyFilterState, MatchMode};
use core::modules::enemy::game::registry::{get_display_def, AbilityIcon, DisplayGroup};
use core::modules::settings::logic::state::Settings;

use crate::common::assets::CustomAssets;
use crate::common::shared::DragGuard;
use crate::common::sheet::SpriteSheet;

pub(crate) const WINDOW_WIDTH: f32 = 420.0;
pub(crate) const WINDOW_HEIGHT: f32 = 400.0;
pub(crate) const TILDE_SPACING: f32 = 5.0;

pub(crate) fn show_popup(
    ctx: &egui::Context,
    state: &mut EnemyFilterState,
    sheets: &mut Vec<SpriteSheet>,
    assets: &CustomAssets,
    settings: &Settings,
    drag_guard: &mut DragGuard,
) {
    if !state.is_open { return; }

    crate::common::img015::ensure_loaded(ctx, sheets, settings);

    let window_id = egui::Id::new("Enemy Filter");
    let (allow_drag, fixed_pos) = drag_guard.assign_bounds(ctx, window_id);

    let mut clear_filters = false;
    let mut is_open_local = state.is_open;

    let mut window = egui::Window::new("Advanced Enemy Filter")
        .id(window_id)
        .open(&mut is_open_local)
        .collapsible(false)
        .resizable(true)
        .constrain(false)
        .movable(allow_drag)
        .default_pos(ctx.screen_rect().center() - egui::vec2(WINDOW_WIDTH / 2.0, WINDOW_HEIGHT / 2.0))
        .default_size([WINDOW_WIDTH, WINDOW_HEIGHT])
        .min_width(380.0)
        .min_height(400.0);

    if let Some(pos) = fixed_pos { window = window.current_pos(pos); }

    window.show(ctx, |ui| {
        let max_rect = ui.max_rect();

        egui::ScrollArea::vertical()
            .auto_shrink([false, false])
            .show(ui, |ui| {

                ui.heading("Attributes");
                ui.add_space(5.0);

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    ui.label(egui::RichText::new("Mode:").strong());

                    egui::ComboBox::from_id_salt("cb_match_mode")
                        .selected_text(if state.match_mode == MatchMode::And { "And" } else { "Or" })
                        .width(55.0)
                        .show_ui(ui, |ui| {
                            ui.selectable_value(&mut state.match_mode, MatchMode::And, "And");
                            ui.selectable_value(&mut state.match_mode, MatchMode::Or, "Or");
                        });
                });
                ui.add_space(15.0);

                ui.heading("Stats");
                ui.add_space(5.0);

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0;
                    ui.label(egui::RichText::new("Target Magnification:").strong());
                    ui.add_sized(
                        egui::vec2(60.0, 20.0),
                        egui::TextEdit::singleline(&mut state.mag_input)
                            .hint_text(egui::RichText::new("100").color(egui::Color32::from_gray(100)))
                    );
                    ui.label("%");
                });
                ui.add_space(8.0);

                let stat_keys = ["Attack", "Dps", "Range", "Atk Cycle (f)", "Hitpoints", "Knockbacks", "Speed", "Cash Drop"];

                egui::Grid::new("stat_filter_grid")
                    .spacing([16.0, 6.0])
                    .show(ui, |ui| {
                        for (i, &stat) in stat_keys.iter().enumerate() {
                            ui.label(format!("{}:", stat));

                            let range = state.stat_ranges.entry(stat).or_default();

                            ui.horizontal(|ui| {
                                ui.spacing_mut().item_spacing.x = TILDE_SPACING;

                                let hint = egui::RichText::new("Any").color(egui::Color32::from_gray(100));

                                ui.add_sized(
                                    egui::vec2(45.0, 20.0),
                                    egui::TextEdit::singleline(&mut range.min).hint_text(hint.clone())
                                );

                                ui.label("~");

                                ui.add_sized(
                                    egui::vec2(45.0, 20.0),
                                    egui::TextEdit::singleline(&mut range.max).hint_text(hint)
                                );
                            });

                            if (i + 1) % 2 == 0 {
                                ui.end_row();
                            }
                        }
                    });
                ui.add_space(15.0);

                let mut rendered_identities = HashSet::new();

                ui.heading("Trait Type");
                ui.add_space(5.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                    for def in REGISTRY.iter() {
                        let display_def = get_display_def(def.identity);
                        if display_def.group == DisplayGroup::Type {
                            render_filter_icon(ui, def.identity, &mut state.active_identities, sheets, assets);
                            rendered_identities.insert(def.identity);
                        }
                    }
                });

                ui.add_space(8.0);
                render_display_group(ui, state, &mut rendered_identities, DisplayGroup::Headline1, false, true, sheets, assets);
                ui.add_space(15.0);

                ui.heading("Attack Type");
                ui.add_space(5.0);
                ui.horizontal_wrapped(|ui| {
                    ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                    for &identity in ATTACK_TYPE_IDENTITIES {
                        render_filter_icon(ui, identity, &mut state.active_identities, sheets, assets);
                        rendered_identities.insert(identity);
                    }
                });
                ui.add_space(15.0);

                ui.heading("Abilities");
                ui.add_space(5.0);

                render_display_group(ui, state, &mut rendered_identities, DisplayGroup::Headline2, false, true, sheets, assets);
                render_display_group(ui, state, &mut rendered_identities, DisplayGroup::Body1, true, true, sheets, assets);
                render_display_group(ui, state, &mut rendered_identities, DisplayGroup::Body2, true, true, sheets, assets);
                render_display_group(ui, state, &mut rendered_identities, DisplayGroup::Footer, false, true, sheets, assets);
                ui.add_space(50.0);
            });

        let btn_size = egui::vec2(160.0, 34.0);
        let btn_rect = egui::Rect::from_center_size(
            max_rect.center_bottom() - egui::vec2(0.0, btn_size.y / 2.0 + 12.0),
            btn_size
        );

        let clear_btn = egui::Button::new(
            egui::RichText::new("Clear Filter").color(egui::Color32::WHITE).strong().size(15.0)
        )
            .fill(egui::Color32::from_rgb(210, 50, 50))
            .rounding(6.0);

        if ui.put(btn_rect, clear_btn).on_hover_cursor(egui::CursorIcon::PointingHand).clicked() {
            clear_filters = true;
        }
    });

    state.is_open = is_open_local;

    if clear_filters {
        *state = EnemyFilterState { is_open: state.is_open, ..Default::default() };
    }
}

fn render_display_group(
    ui: &mut egui::Ui,
    state: &mut EnemyFilterState,
    rendered_identities: &mut HashSet<Identity>,
    target_group: DisplayGroup,
    is_vertical: bool,
    draw_labels: bool,
    sheets: &[SpriteSheet],
    assets: &CustomAssets,
) {
    let mut identities_in_group = Vec::new();

    for def in REGISTRY.iter() {
        let display_def = get_display_def(def.identity);
        if display_def.group != target_group { continue; }
        if display_def.group == DisplayGroup::Type { continue; }
        if ATTACK_TYPE_IDENTITIES.contains(&def.identity) { continue; }
        if identities_in_group.contains(&def.identity) { continue; }

        identities_in_group.push(def.identity);
        rendered_identities.insert(def.identity);
    }

    if identities_in_group.is_empty() { return; }

    if is_vertical {
        ui.vertical(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(0.0, 4.0);
            for identity in identities_in_group {
                render_filter_icon_row(ui, state, identity, draw_labels, sheets, assets);
            }
        });
    } else {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
            for identity in identities_in_group {
                render_filter_icon(ui, identity, &mut state.active_identities, sheets, assets);
            }
        });
    }
    ui.add_space(8.0);
}

fn render_filter_icon_row(
    ui: &mut egui::Ui,
    state: &mut EnemyFilterState,
    identity: Identity,
    draw_labels: bool,
    sheets: &[SpriteSheet],
    assets: &CustomAssets,
) {
    let is_active = state.active_identities.contains(&identity);
    let name = get_identity_name(identity);

    let pure_def = REGISTRY.iter().find(|d| d.identity == identity);
    let schema = pure_def.map(|d| d.schema).unwrap_or(&[]);
    let has_adv = !schema.is_empty();

    let bg_fill = if is_active && has_adv { egui::Color32::from_black_alpha(150) } else { egui::Color32::TRANSPARENT };
    let margin = if is_active && has_adv { egui::Margin::symmetric(8.0, 8.0) } else { egui::Margin::same(0.0) };

    egui::Frame::none()
        .fill(bg_fill)
        .rounding(6.0)
        .inner_margin(margin)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    render_filter_icon(ui, identity, &mut state.active_identities, sheets, assets);

                    if draw_labels {
                        ui.add_space(10.0);
                        let color = if is_active { egui::Color32::WHITE } else { egui::Color32::from_gray(120) };
                        if ui.add(egui::Label::new(egui::RichText::new(&name).color(color)).sense(egui::Sense::click())).clicked() {
                            if is_active { state.active_identities.remove(&identity); }
                            else { state.active_identities.insert(identity); }
                        }
                    }
                });

                if is_active && has_adv {
                    ui.add_space(4.0);
                    egui::Grid::new(format!("adv_grid_{}", name))
                        .spacing([8.0, 6.0])
                        .show(ui, |ui| {
                            for &(attr, _unit) in schema {
                                ui.label(format!("{}:", attr));

                                let range = state.adv_ranges
                                    .entry(identity)
                                    .or_default()
                                    .entry(attr)
                                    .or_default();

                                ui.horizontal(|ui| {
                                    ui.spacing_mut().item_spacing.x = TILDE_SPACING;

                                    let hint = egui::RichText::new("Any").color(egui::Color32::from_gray(100));

                                    ui.add_sized(
                                        egui::vec2(45.0, 20.0),
                                        egui::TextEdit::singleline(&mut range.min).hint_text(hint.clone())
                                    );

                                    ui.label("~");

                                    ui.add_sized(
                                        egui::vec2(45.0, 20.0),
                                        egui::TextEdit::singleline(&mut range.max).hint_text(hint)
                                    );
                                });
                                ui.end_row();
                            }
                        });
                }
            });
        });
}

fn render_filter_icon(
    ui: &mut egui::Ui,
    identity: Identity,
    active_identities: &mut HashSet<Identity>,
    sheets: &[SpriteSheet],
    assets: &CustomAssets,
) {
    let is_active = active_identities.contains(&identity);
    let tint = if is_active { egui::Color32::WHITE } else { egui::Color32::from_gray(80) };

    let display_def = get_display_def(identity);

    match display_def.icon {
        AbilityIcon::Custom(custom_icon) => {
            if let Some(tex) = assets.get_icon_texture(custom_icon) {
                let img = egui::Image::new(tex).fit_to_exact_size(egui::vec2(32.0, 32.0)).tint(tint);
                let response = ui.add(egui::ImageButton::new(img).frame(false));
                if response.clicked() {
                    if is_active { active_identities.remove(&identity); }
                    else { active_identities.insert(identity); }
                }
                response.on_hover_text(get_identity_name(identity));
                return;
            }
        },
        AbilityIcon::Standard(icon_id) => {
            for sheet in sheets {
                let Some(cut) = sheet.core.cuts_map.get(&icon_id) else { continue; };
                let Some(tex) = &sheet.texture_handle else { continue; };

                let img = egui::Image::new(egui::load::SizedTexture::new(tex.id(), egui::vec2(32.0, 32.0)))
                    .uv(egui::Rect::from_min_max(egui::pos2(cut.uv_coordinates.min.x, cut.uv_coordinates.min.y), egui::pos2(cut.uv_coordinates.max.x, cut.uv_coordinates.max.y)))
                    .tint(tint);

                let response = ui.add(egui::ImageButton::new(img).frame(false));
                if response.clicked() {
                    if is_active { active_identities.remove(&identity); }
                    else { active_identities.insert(identity); }
                }
                response.on_hover_text(get_identity_name(identity));
                return;
            }
        },
        AbilityIcon::None => {}
    }

    let (rect, response) = ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::click());
    if ui.is_rect_visible(rect) {
        ui.painter().rect_filled(rect, 4.0, egui::Color32::from_black_alpha(100));
        let text_color = if is_active { egui::Color32::WHITE } else { egui::Color32::from_gray(100) };
        ui.painter().text(rect.center(), egui::Align2::CENTER_CENTER, "?", egui::FontId::proportional(20.0), text_color);
    }
    if response.clicked() {
        if is_active { active_identities.remove(&identity); }
        else { active_identities.insert(identity); }
    }
    response.on_hover_text(get_identity_name(identity));
}