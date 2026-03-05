use eframe::egui;
use std::collections::{HashSet, HashMap};
use crate::global::imgcut::SpriteSheet;
use crate::core::utils::{DragGuard, UI_TRAIT_ORDER};
use crate::features::cat::registry::{ABILITY_REGISTRY, DisplayGroup};
use crate::features::cat::logic::stats::CatRaw;
use crate::features::cat::logic::scanner::CatEntry;
use crate::features::cat::data::skillacquisition::TalentGroupRaw;
use crate::global::img015;
use crate::features::settings::logic::Settings;

pub const ATTACK_TYPE_ICONS: &[usize] = &[
    img015::ICON_SINGLE_ATTACK,
    img015::ICON_AREA_ATTACK,
    img015::ICON_OMNI_STRIKE,
    img015::ICON_LONG_DISTANCE,
    img015::ICON_MULTIHIT,
];

pub const WINDOW_WIDTH: f32 = 500.0;
pub const WINDOW_HEIGHT: f32 = 580.0;
pub const TILDE_SPACING: f32 = 5.0; 
pub const BTN_SIZE_RARITY: [f32; 2] = [77.0, 24.0];
pub const BTN_SIZE_FORM: [f32; 2] = [118.0, 24.0];

#[derive(Clone, Copy, PartialEq, Default)]
pub enum TalentFilterMode {
    #[default]
    Ignore,
    Consider,
    Only,
}

impl TalentFilterMode {
    pub fn label(&self) -> &'static str {
        match self {
            TalentFilterMode::Ignore => "Ignore",
            TalentFilterMode::Consider => "Consider",
            TalentFilterMode::Only => "Only",
        }
    }
}

#[derive(Clone, Copy, PartialEq, Default)]
pub enum MatchMode {
    #[default]
    And,
    Or,
}

#[derive(Clone, PartialEq, Default)]
pub struct RangeInput {
    pub min: String,
    pub max: String,
}

#[derive(Clone, PartialEq)]
pub struct CatFilterState {
    pub is_open: bool,
    pub active_icons: HashSet<usize>,
    pub rarities: [bool; 6], 
    pub forms: [bool; 4],    
    pub match_mode: MatchMode,
    pub talent_mode: TalentFilterMode,
    pub ultra_talent_mode: TalentFilterMode,
    pub adv_ranges: HashMap<usize, HashMap<&'static str, RangeInput>>,
}

impl Default for CatFilterState {
    fn default() -> Self {
        Self {
            is_open: false,
            active_icons: HashSet::new(),
            rarities: [false; 6],
            forms: [false; 4],
            match_mode: MatchMode::And,
            talent_mode: TalentFilterMode::Ignore,
            ultra_talent_mode: TalentFilterMode::Ignore,
            adv_ranges: HashMap::new(),
        }
    }
}

impl CatFilterState {
    pub fn is_active(&self) -> bool {
        !self.active_icons.is_empty()
            || self.rarities.iter().any(|&r| r)
            || self.forms.iter().any(|&f| f)
            || self.talent_mode == TalentFilterMode::Only
            || self.ultra_talent_mode == TalentFilterMode::Only
    }
}

pub fn show_popup(
    ctx: &egui::Context,
    state: &mut CatFilterState,
    sheet: &mut SpriteSheet,
    multihit_tex: &Option<egui::TextureHandle>,
    kamikaze_tex: &Option<egui::TextureHandle>,
    boss_wave_tex: &Option<egui::TextureHandle>,
    settings: &Settings,
    drag_guard: &mut DragGuard,
) {
    if !state.is_open { return; }
    
    img015::ensure_loaded(ctx, sheet, settings);

    let window_id = egui::Id::new("Cat Filter");
    let (allow_drag, fixed_pos) = drag_guard.assign_bounds(ctx, window_id);
    
    let mut clear_filters = false;
    let mut is_open_local = state.is_open;
    
    let mut window = egui::Window::new("Cat Filter")
        .id(window_id)
        .open(&mut is_open_local)
        .collapsible(false)
        .resizable(false)
        .constrain(false)
        .movable(allow_drag)
        .default_pos(ctx.screen_rect().center() - egui::vec2(WINDOW_WIDTH / 2.0, WINDOW_HEIGHT / 2.0))
        .fixed_size([WINDOW_WIDTH, WINDOW_HEIGHT]);
        
    if let Some(pos) = fixed_pos { window = window.current_pos(pos); }
    
    window.show(ctx, |ui| {
        let max_rect = ui.max_rect(); 
        
        egui::ScrollArea::vertical().show(ui, |ui| {
            
            ui.set_min_width(WINDOW_WIDTH - 20.0);
            
            ui.heading("Attributes");
            ui.add_space(5.0);
            
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                filter_button(ui, &mut state.rarities[0], "Normal", BTN_SIZE_RARITY);
                filter_button(ui, &mut state.rarities[1], "Special", BTN_SIZE_RARITY);
                filter_button(ui, &mut state.rarities[2], "Rare", BTN_SIZE_RARITY);
                filter_button(ui, &mut state.rarities[3], "Super Rare", BTN_SIZE_RARITY);
                filter_button(ui, &mut state.rarities[4], "Uber Rare", BTN_SIZE_RARITY);
                filter_button(ui, &mut state.rarities[5], "Legend Rare", BTN_SIZE_RARITY);
            });
            ui.add_space(4.0);

            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                filter_button(ui, &mut state.forms[0], "Normal Form", BTN_SIZE_FORM);
                filter_button(ui, &mut state.forms[1], "Evolved Form", BTN_SIZE_FORM);
                filter_button(ui, &mut state.forms[2], "True Form", BTN_SIZE_FORM);
                filter_button(ui, &mut state.forms[3], "Ultra Form", BTN_SIZE_FORM);
            });
            ui.add_space(8.0);

            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(16.0, 4.0);
                
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

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0; 
                    ui.label(egui::RichText::new("Talents:").strong());
                    
                    ui.scope(|ui| {
                        if state.talent_mode == TalentFilterMode::Only {
                            let active_blue = egui::Color32::from_rgb(31, 106, 165);
                            let visuals = ui.visuals_mut();
                            visuals.widgets.inactive.bg_fill = active_blue;
                            visuals.widgets.hovered.bg_fill = active_blue;
                            visuals.widgets.active.bg_fill = active_blue;
                            visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
                            visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
                        }
                        
                        egui::ComboBox::from_id_salt("cb_talent_mode")
                            .selected_text(state.talent_mode.label())
                            .width(85.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut state.talent_mode, TalentFilterMode::Ignore, "Ignore");
                                ui.selectable_value(&mut state.talent_mode, TalentFilterMode::Consider, "Consider");
                                ui.selectable_value(&mut state.talent_mode, TalentFilterMode::Only, "Only");
                            });
                    });
                });

                ui.horizontal(|ui| {
                    ui.spacing_mut().item_spacing.x = 6.0; 
                    ui.label(egui::RichText::new("Ultra Talents:").strong());
                    
                    ui.scope(|ui| {
                        if state.ultra_talent_mode == TalentFilterMode::Only {
                            let active_blue = egui::Color32::from_rgb(31, 106, 165);
                            let visuals = ui.visuals_mut();
                            visuals.widgets.inactive.bg_fill = active_blue;
                            visuals.widgets.hovered.bg_fill = active_blue;
                            visuals.widgets.active.bg_fill = active_blue;
                            visuals.widgets.inactive.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
                            visuals.widgets.hovered.fg_stroke = egui::Stroke::new(1.0, egui::Color32::WHITE);
                        }
                        
                        egui::ComboBox::from_id_salt("cb_ultra_talent_mode")
                            .selected_text(state.ultra_talent_mode.label())
                            .width(85.0)
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut state.ultra_talent_mode, TalentFilterMode::Ignore, "Ignore");
                                ui.selectable_value(&mut state.ultra_talent_mode, TalentFilterMode::Consider, "Consider");
                                ui.selectable_value(&mut state.ultra_talent_mode, TalentFilterMode::Only, "Only");
                            });
                    });
                });
            });
            ui.add_space(15.0);

            ui.heading("Target Traits");
            ui.add_space(5.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                for &icon_id in UI_TRAIT_ORDER {
                    render_filter_icon(ui, icon_id, &mut state.active_icons, sheet, multihit_tex, kamikaze_tex, boss_wave_tex);
                }
            });
            ui.add_space(15.0);

            ui.heading("Attack Type");
            ui.add_space(5.0);
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                for &icon_id in ATTACK_TYPE_ICONS {
                    render_filter_icon(ui, icon_id, &mut state.active_icons, sheet, multihit_tex, kamikaze_tex, boss_wave_tex);
                }
            });
            ui.add_space(15.0);

            ui.heading("Abilities");
            ui.add_space(5.0);

            let mut rendered_icons = HashSet::new();

            render_display_group(ui, state, &mut rendered_icons, DisplayGroup::Headline1, false, true, sheet, multihit_tex, kamikaze_tex, boss_wave_tex);
            render_display_group(ui, state, &mut rendered_icons, DisplayGroup::Headline2, false, true, sheet, multihit_tex, kamikaze_tex, boss_wave_tex);
            render_display_group(ui, state, &mut rendered_icons, DisplayGroup::Body1, true, true, sheet, multihit_tex, kamikaze_tex, boss_wave_tex); 
            render_display_group(ui, state, &mut rendered_icons, DisplayGroup::Body2, true, true, sheet, multihit_tex, kamikaze_tex, boss_wave_tex); 
            render_display_group(ui, state, &mut rendered_icons, DisplayGroup::Footer, false, true, sheet, multihit_tex, kamikaze_tex, boss_wave_tex);

            let check_talents = state.talent_mode != TalentFilterMode::Ignore || state.ultra_talent_mode != TalentFilterMode::Ignore;
            if check_talents {
                let mut talent_icons = Vec::new();
                for def in ABILITY_REGISTRY.iter() {
                    if !rendered_icons.contains(&def.icon_id) && !UI_TRAIT_ORDER.contains(&def.icon_id) && !ATTACK_TYPE_ICONS.contains(&def.icon_id) {
                        if !talent_icons.contains(&def.icon_id) {
                            talent_icons.push(def.icon_id);
                        }
                    }
                }

                if !talent_icons.is_empty() {
                    ui.add_space(2.0);
                    ui.heading("Talents");
                    ui.add_space(5.0);
                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                        for icon_id in talent_icons {
                            render_filter_icon(ui, icon_id, &mut state.active_icons, sheet, multihit_tex, kamikaze_tex, boss_wave_tex);
                        }
                    });
                }
            }
            
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
        *state = CatFilterState { is_open: state.is_open, ..Default::default() };
    }
}

fn render_display_group(
    ui: &mut egui::Ui,
    state: &mut CatFilterState,
    rendered_icons: &mut HashSet<usize>,
    target_group: DisplayGroup,
    is_vertical: bool,
    draw_labels: bool,
    sheet: &SpriteSheet,
    multihit_tex: &Option<egui::TextureHandle>,
    kamikaze_tex: &Option<egui::TextureHandle>,
    boss_wave_tex: &Option<egui::TextureHandle>,
) {
    let mut icons_in_group = Vec::new();
    
    for def in ABILITY_REGISTRY.iter() {
        if def.group == target_group && !UI_TRAIT_ORDER.contains(&def.icon_id) && !ATTACK_TYPE_ICONS.contains(&def.icon_id) {
            if !icons_in_group.contains(&def.icon_id) {
                icons_in_group.push(def.icon_id);
                rendered_icons.insert(def.icon_id);
            }
        }
    }
    
    if target_group == DisplayGroup::Headline2 {
        if !icons_in_group.contains(&img015::ICON_CONJURE) { 
            icons_in_group.insert(0, img015::ICON_CONJURE); 
            rendered_icons.insert(img015::ICON_CONJURE);
        }
        if !icons_in_group.contains(&img015::ICON_KAMIKAZE) { 
            icons_in_group.push(img015::ICON_KAMIKAZE); 
            rendered_icons.insert(img015::ICON_KAMIKAZE);
        }
    }
    
    if !icons_in_group.is_empty() {
        if is_vertical {
            ui.vertical(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, 4.0);
                for icon_id in icons_in_group {
                    render_filter_icon_row(ui, state, icon_id, draw_labels, sheet, multihit_tex, kamikaze_tex, boss_wave_tex);
                }
            });
        } else {
            ui.horizontal_wrapped(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(4.0, 4.0);
                for icon_id in icons_in_group {
                    render_filter_icon(ui, icon_id, &mut state.active_icons, sheet, multihit_tex, kamikaze_tex, boss_wave_tex);
                }
            });
        }
        ui.add_space(8.0); 
    }
}

fn filter_button(ui: &mut egui::Ui, active: &mut bool, label: &str, size: [f32; 2]) -> egui::Response {
    let mut btn = egui::Button::new(label);
    if *active {
        btn = btn.fill(egui::Color32::from_rgb(31, 106, 165));
    }
    let response = ui.add_sized(size, btn);
    if response.clicked() {
        *active = !*active;
    }
    response
}

fn get_adv_attributes(name: &str) -> Option<&'static [&'static str]> {
    match name {
        "Metal Killer" => Some(&["Hitpoints (%)"]),
        "Wave Attack" => Some(&["Chance", "Level"]),
        "Mini-Wave" => Some(&["Chance", "Level"]),
        "Surge Attack" => Some(&["Chance", "Level", "Min-Range", "Max-Range"]),
        "Mini-Surge" => Some(&["Chance", "Level", "Min-Range", "Max-Range"]),
        "Explosion" => Some(&["Chance", "Min-Range", "Max-Range"]),
        "Savage Blow" => Some(&["Chance", "Boost (%)"]),
        "Critical Hit" => Some(&["Chance"]),
        "Strengthen" => Some(&["Hitpoints (%)", "Boost (%)"]),
        "Survive" => Some(&["Chance"]),
        "Barrier Breaker" => Some(&["Chance"]),
        "Shield Piercer" => Some(&["Chance"]),
        "Dodge" => Some(&["Chance", "Duration (f)"]),
        "Weaken" => Some(&["Chance", "Reduced-To", "Duration (f)"]),
        "Freeze" => Some(&["Chance", "Duration (f)"]),
        "Slow" => Some(&["Chance", "Duration (f)"]),
        "Knockback" => Some(&["Chance"]),
        "Curse" => Some(&["Chance", "Duration (f)"]),
        "Warp" => Some(&["Chance", "Duration (f)", "Min-Distance", "Max-Distance"]),
        _ => None,
    }
}

fn render_filter_icon_row(
    ui: &mut egui::Ui, 
    state: &mut CatFilterState,
    icon_id: usize, 
    draw_labels: bool,
    sheet: &SpriteSheet,
    multihit_tex: &Option<egui::TextureHandle>,
    kamikaze_tex: &Option<egui::TextureHandle>,
    boss_wave_tex: &Option<egui::TextureHandle>,
) {
    let is_active = state.active_icons.contains(&icon_id);
    let name = get_icon_name(icon_id);
    let has_adv = get_adv_attributes(&name).is_some();

    let bg_fill = if is_active && has_adv { egui::Color32::from_black_alpha(150) } else { egui::Color32::TRANSPARENT };
    let margin = if is_active && has_adv { egui::Margin::symmetric(8.0, 8.0) } else { egui::Margin::same(0.0) };

    egui::Frame::none()
        .fill(bg_fill)
        .rounding(6.0)
        .inner_margin(margin)
        .show(ui, |ui| {
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    render_filter_icon(ui, icon_id, &mut state.active_icons, sheet, multihit_tex, kamikaze_tex, boss_wave_tex);
                    
                    if draw_labels {
                        ui.add_space(10.0); 
                        let color = if is_active { egui::Color32::WHITE } else { egui::Color32::from_gray(120) };
                        if ui.add(egui::Label::new(egui::RichText::new(&name).color(color)).sense(egui::Sense::click())).clicked() {
                            if is_active { state.active_icons.remove(&icon_id); } 
                            else { state.active_icons.insert(icon_id); }
                        }
                    }
                });

                if is_active && has_adv {
                    if let Some(attrs) = get_adv_attributes(&name) {
                        ui.add_space(4.0);
                        ui.horizontal(|ui| {
                            ui.add_space(3.0); 
                            
                            egui::Grid::new(format!("adv_grid_{}", icon_id))
                                .spacing([8.0, 6.0]) 
                                .show(ui, |ui| {
                                    for &attr in attrs {
                                        ui.label(format!("{}:", attr));
                                        
                                        let range = state.adv_ranges
                                            .entry(icon_id)
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
                        });
                    }
                }
            });
        });
}

fn render_filter_icon(
    ui: &mut egui::Ui, 
    icon_id: usize, 
    active_icons: &mut HashSet<usize>,
    sheet: &SpriteSheet,
    multihit_tex: &Option<egui::TextureHandle>,
    kamikaze_tex: &Option<egui::TextureHandle>,
    boss_wave_tex: &Option<egui::TextureHandle>,
) {
    let is_active = active_icons.contains(&icon_id);
    let tint = if is_active { egui::Color32::WHITE } else { egui::Color32::from_gray(80) };
    
    let mut drawn = false;
    let lower_name = get_icon_name(icon_id).to_lowercase();

    let custom_tex = if icon_id == img015::ICON_MULTIHIT {
        multihit_tex.as_ref()
    } else if icon_id == img015::ICON_KAMIKAZE {
        kamikaze_tex.as_ref()
    } else if lower_name.contains("boss") && lower_name.contains("wave") {
        boss_wave_tex.as_ref() 
    } else {
        None
    };

    if let Some(tex) = custom_tex {
        let img = egui::Image::new(tex).fit_to_exact_size(egui::vec2(32.0, 32.0)).tint(tint);
        let response = ui.add(egui::ImageButton::new(img).frame(false));
        if response.clicked() {
            if is_active { active_icons.remove(&icon_id); } 
            else { active_icons.insert(icon_id); }
        }
        response.on_hover_text(get_icon_name(icon_id));
        drawn = true;
    } 
    else if let Some(cut) = sheet.cuts_map.get(&icon_id) {
        if let Some(tex) = &sheet.texture_handle {
            let img = egui::Image::new(egui::load::SizedTexture::new(tex.id(), egui::vec2(32.0, 32.0)))
                .uv(cut.uv_coordinates)
                .tint(tint);
                
            let response = ui.add(egui::ImageButton::new(img).frame(false));
            
            if response.clicked() {
                if is_active { active_icons.remove(&icon_id); } 
                else { active_icons.insert(icon_id); }
            }
            response.on_hover_text(get_icon_name(icon_id));
            drawn = true;
        }
    }
    
    if !drawn {
        let (rect, response) = ui.allocate_exact_size(egui::vec2(32.0, 32.0), egui::Sense::click());
        if ui.is_rect_visible(rect) {
            ui.painter().rect_filled(rect, 4.0, egui::Color32::from_black_alpha(100));
            let text_color = if is_active { egui::Color32::WHITE } else { egui::Color32::from_gray(100) };
            let galley = ui.painter().layout_no_wrap("?".to_string(), egui::FontId::proportional(20.0), text_color);
            let text_pos = rect.center() - galley.rect.size() / 2.0;
            ui.painter().galley(text_pos, galley, text_color);
        }
        if response.clicked() {
            if is_active { active_icons.remove(&icon_id); } 
            else { active_icons.insert(icon_id); }
        }
        response.on_hover_text(get_icon_name(icon_id));
    }
}

fn get_icon_name(icon_id: usize) -> String {
    use crate::global::img015::*;
    match icon_id {
        ICON_TRAIT_RED => "Red",
        ICON_TRAIT_FLOATING => "Floating",
        ICON_TRAIT_BLACK => "Black",
        ICON_TRAIT_METAL => "Metal",
        ICON_TRAIT_ANGEL => "Angel",
        ICON_TRAIT_ALIEN => "Alien",
        ICON_TRAIT_ZOMBIE => "Zombie",
        ICON_TRAIT_RELIC => "Relic",
        ICON_TRAIT_AKU => "Aku",
        ICON_TRAIT_TRAITLESS => "Traitless",
        ICON_SINGLE_ATTACK => "Single Attack",
        ICON_AREA_ATTACK => "Area Attack",
        ICON_OMNI_STRIKE => "Omni Strike",
        ICON_LONG_DISTANCE => "Long Distance",
        ICON_MULTIHIT => "Multi-Hit",
        ICON_CONJURE => "Conjure / Spirit",
        ICON_KAMIKAZE => "Kamikaze",
        _ => ABILITY_REGISTRY.iter().find(|d| d.icon_id == icon_id).map(|d| d.name).unwrap_or("Unknown")
    }.to_string()
}

fn get_ability_value(s: &CatRaw, ability_name: &str, attr: &str) -> i32 {
    match (ability_name, attr) {
        ("Metal Killer", "Hitpoints (%)") => s.metal_killer_percent,
        ("Wave Attack", "Chance") => s.wave_chance,
        ("Wave Attack", "Level") => s.wave_level,
        ("Mini-Wave", "Chance") => s.wave_chance, 
        ("Mini-Wave", "Level") => s.wave_level,
        ("Surge Attack", "Chance") => s.surge_chance,
        ("Surge Attack", "Level") => s.surge_level,
        ("Surge Attack", "Min-Range") => s.surge_spawn_anchor,
        ("Surge Attack", "Max-Range") => s.surge_spawn_anchor + s.surge_spawn_span,
        ("Mini-Surge", "Chance") => s.surge_chance, 
        ("Mini-Surge", "Level") => s.surge_level,
        ("Mini-Surge", "Min-Range") => s.surge_spawn_anchor,
        ("Mini-Surge", "Max-Range") => s.surge_spawn_anchor + s.surge_spawn_span,
        ("Explosion", "Chance") => s.explosion_chance,
        ("Explosion", "Min-Range") => s.explosion_spawn_anchor,
        ("Explosion", "Max-Range") => s.explosion_spawn_anchor + s.explosion_spawn_span,
        ("Savage Blow", "Chance") => s.savage_blow_chance,
        ("Savage Blow", "Boost (%)") => s.savage_blow_boost,
        ("Critical Hit", "Chance") => s.critical_chance,
        ("Strengthen", "Hitpoints (%)") => s.strengthen_threshold,
        ("Strengthen", "Boost (%)") => s.strengthen_boost,
        ("Survive", "Chance") => s.survive,
        ("Barrier Breaker", "Chance") => s.barrier_breaker_chance,
        ("Shield Piercer", "Chance") => s.shield_pierce_chance,
        ("Dodge", "Chance") => s.dodge_chance,
        ("Dodge", "Duration (f)") => s.dodge_duration,
        ("Weaken", "Chance") => s.weaken_chance,
        ("Weaken", "Reduced-To") => s.weaken_to,
        ("Weaken", "Duration (f)") => s.weaken_duration,
        ("Freeze", "Chance") => s.freeze_chance,
        ("Freeze", "Duration (f)") => s.freeze_duration,
        ("Slow", "Chance") => s.slow_chance,
        ("Slow", "Duration (f)") => s.slow_duration,
        ("Knockback", "Chance") => s.knockback_chance,
        ("Curse", "Chance") => s.curse_chance,
        ("Curse", "Duration (f)") => s.curse_duration,
        ("Warp", "Chance") => s.warp_chance,
        ("Warp", "Duration (f)") => s.warp_duration,
        ("Warp", "Min-Distance") => s.warp_distance_minimum,
        ("Warp", "Max-Distance") => s.warp_distance_maximum,
        _ => 0,
    }
}

fn get_talent_modifier(g: &TalentGroupRaw, attr: &str) -> i32 {
    match attr {
        "Chance" => g.max_1 as i32,
        "Duration (f)" => if g.max_2 > 0 { g.max_2 as i32 } else { g.max_1 as i32 },
        "Level" => g.max_2 as i32,
        "Hitpoints (%)" => g.max_1 as i32,
        "Boost (%)" => g.max_2 as i32,
        "Reduced-To" => g.max_2 as i32,
        "Min-Distance" | "Min-Range" => g.max_3 as i32,
        "Max-Distance" | "Max-Range" => g.max_4 as i32,
        _ => 0,
    }
}

pub fn entity_passes_filter(cat: &CatEntry, filter: &CatFilterState) -> bool {
    let any_rarity_selected = filter.rarities.iter().any(|&r| r);
    if any_rarity_selected {
        let r_idx = cat.unit_buy.rarity as usize;
        if r_idx >= filter.rarities.len() || !filter.rarities[r_idx] {
            return false; 
        }
    }

    let any_form_selected = filter.forms.iter().any(|&f| f);
    let mut forms_to_check = Vec::new();
    
    for i in 0..4 {
        if cat.forms[i] {
            if !any_form_selected || filter.forms[i] {
                forms_to_check.push(i);
            }
        }
    }
    
    if forms_to_check.is_empty() { return false; } 

    let req_normal = filter.talent_mode == TalentFilterMode::Only;
    let req_ultra = filter.ultra_talent_mode == TalentFilterMode::Only;

    if filter.active_icons.is_empty() {
        if !req_normal && !req_ultra {
            return true;
        }

        for &form_idx in &forms_to_check {
            let mut has_any_normal = false;
            let mut has_any_ultra = false;

            if form_idx >= 2 {
                if let Some(t_data) = cat.talent_data.as_ref() {
                    for g in &t_data.groups {
                        if g.limit == 1 {
                            has_any_ultra = true;
                        } else {
                            has_any_normal = true;
                        }
                    }
                }
            }

            let passed = if req_normal && req_ultra {
                has_any_normal || has_any_ultra
            } else if req_normal {
                has_any_normal
            } else if req_ultra {
                has_any_ultra
            } else {
                true
            };

            if passed {
                return true;
            }
        }
        return false;
    }

    for form_idx in forms_to_check {
        if let Some(Some(stats)) = cat.stats.get(form_idx) {
            
            let mut form_passes = filter.match_mode == MatchMode::And;
            
            for &icon_id in &filter.active_icons {
                let name = get_icon_name(icon_id);
                let has_inherent = has_trait_or_ability(stats, icon_id);
                
                let mut normal_talents = Vec::new();
                let mut ultra_talents = Vec::new();

                if form_idx >= 2 {
                    if let Some(t_data) = cat.talent_data.as_ref() {
                        for g in &t_data.groups {
                            let matches_icon = ABILITY_REGISTRY.iter()
                                .any(|d| d.icon_id == icon_id && (g.ability_id == d.talent_id || g.name_id as u8 == d.talent_id));

                            if matches_icon {
                                if g.limit == 1 {
                                    ultra_talents.push(g);
                                } else {
                                    normal_talents.push(g);
                                }
                            }
                        }
                    }
                }

                let has_normal = !normal_talents.is_empty();
                let has_ultra = !ultra_talents.is_empty();

                let valid_inherent = filter.talent_mode != TalentFilterMode::Only && filter.ultra_talent_mode != TalentFilterMode::Only && has_inherent;
                let valid_normal = filter.talent_mode != TalentFilterMode::Ignore && has_normal;
                let valid_ultra = filter.ultra_talent_mode != TalentFilterMode::Ignore && has_ultra;

                let mut icon_passed = false;

                if valid_inherent || valid_normal || valid_ultra {
                    if let Some(adv_map) = filter.adv_ranges.get(&icon_id) {
                        
                        let mut test_builds = Vec::new();
                        if valid_inherent { test_builds.push(0); } 
                        if valid_normal { test_builds.push(1); }   
                        if valid_ultra { test_builds.push(2); }    

                        let mut any_build_passed = false;

                        for build in test_builds {
                            let mut build_passed_all_attrs = true;
                            
                            for (attr, range) in adv_map {
                                let mut val = if has_inherent { get_ability_value(stats, &name, attr) } else { 0 };
                                
                                if build >= 1 {
                                    for g in &normal_talents { val += get_talent_modifier(g, attr); }
                                }
                                if build >= 2 {
                                    for g in &ultra_talents { val += get_talent_modifier(g, attr); }
                                }

                                if let Some(min) = range.min.parse::<i32>().ok() {
                                    if val < min {
                                        build_passed_all_attrs = false;
                                        break;
                                    }
                                }
                                
                                if let Some(max) = range.max.parse::<i32>().ok() {
                                    if val > max {
                                        build_passed_all_attrs = false;
                                        break;
                                    }
                                }
                            }

                            if build_passed_all_attrs {
                                any_build_passed = true;
                                break;
                            }
                        }

                        if any_build_passed {
                            icon_passed = true;
                        }
                    } else {
                        icon_passed = true;
                    }
                }

                if filter.match_mode == MatchMode::And {
                    if !icon_passed {
                        form_passes = false;
                        break;
                    }
                } 
                else {
                    if icon_passed {
                        form_passes = true;
                        break;
                    }
                }
            }

            if form_passes { return true; } 
        }
    }
    
    false
}

fn has_trait_or_ability(s: &CatRaw, icon_id: usize) -> bool {
    use crate::global::img015::*;
    if UI_TRAIT_ORDER.contains(&icon_id) {
        match icon_id {
            ICON_TRAIT_RED => s.target_red != 0,
            ICON_TRAIT_FLOATING => s.target_floating != 0,
            ICON_TRAIT_BLACK => s.target_black != 0,
            ICON_TRAIT_METAL => s.target_metal != 0,
            ICON_TRAIT_ANGEL => s.target_angel != 0,
            ICON_TRAIT_ALIEN => s.target_alien != 0,
            ICON_TRAIT_ZOMBIE => s.target_zombie != 0,
            ICON_TRAIT_RELIC => s.target_relic != 0,
            ICON_TRAIT_AKU => s.target_aku != 0,
            ICON_TRAIT_TRAITLESS => s.target_traitless != 0,
            _ => false,
        }
    } else if icon_id == img015::ICON_CONJURE {
        s.conjure_unit_id > 0
    } else if icon_id == img015::ICON_KAMIKAZE {
        s.kamikaze != 0
    } else if ATTACK_TYPE_ICONS.contains(&icon_id) {
        let ranges = [
            (s.long_distance_1_anchor, s.long_distance_1_span),
            (s.long_distance_2_anchor, s.long_distance_2_span),
            (s.long_distance_3_anchor, s.long_distance_3_span),
        ];
        
        let mut has_range = false;
        let mut is_omni = false;
        
        for &(anchor, span) in &ranges {
            if anchor != 0 || span != 0 {
                has_range = true;
                let min = anchor.min(anchor + span);
                if min <= 0 {
                    is_omni = true;
                }
            }
        }

        match icon_id {
            ICON_SINGLE_ATTACK => s.area_attack == 0,
            ICON_AREA_ATTACK => s.area_attack == 1,
            ICON_LONG_DISTANCE => has_range && !is_omni, 
            ICON_OMNI_STRIKE => has_range && is_omni, 
            ICON_MULTIHIT => s.attack_2 > 0,
            _ => false,
        }
    } else {
        ABILITY_REGISTRY.iter().find(|d| d.icon_id == icon_id).map_or(false, |def| (def.getter)(s) > 0)
    }
}