use eframe::egui;
use crate::core::cat::scanner::CatEntry;
use crate::core::cat::stats::{self, CatRaw};
use crate::core::cat::abilities::{self, AbilityItem};
use crate::data::global::imgcut::SpriteSheet;
use crate::core::settings::Settings;
use crate::ui::components::shared::{render_fallback_icon, text_with_superscript};
use crate::data::global::img015;
use crate::data::cat::skillacquisition::TalentRaw;
use std::collections::HashMap;

pub fn render(
    ui: &mut egui::Ui, 
    s: &CatRaw, 
    cat: &CatEntry, 
    level: i32,
    sheet: &SpriteSheet, 
    multihit_tex: &Option<egui::TextureHandle>, 
    kamikaze_tex: &Option<egui::TextureHandle>,   
    boss_wave_tex: &Option<egui::TextureHandle>, 
    settings: &Settings, 
    talent_data: Option<&TalentRaw>,
    talent_levels: Option<&HashMap<u8, u8>>
) {
    let curve = cat.curve.as_ref();
    
    // Updated to unpack 6 groups (Traits are now grp_trait)
    let (grp_trait, grp_hl1, grp_hl2, grp_b1, grp_b2, grp_footer) = abilities::collect_ability_data(
        s, level, curve, multihit_tex, kamikaze_tex, boss_wave_tex, settings, false,
        talent_data,
        talent_levels
    );
    
    let mut previous_content = false;
    let main_border = egui::Color32::BLACK;

    // Render Traits (New Registry Group)
    if !grp_trait.is_empty() {
        render_icon_row(ui, &grp_trait, sheet, settings, main_border);
        previous_content = true;
    }

    if !grp_hl1.is_empty() { 
        if previous_content { ui.add_space(settings.ability_padding_y); }
        render_icon_row(ui, &grp_hl1, sheet, settings, main_border); 
        previous_content = true;
    }
    
    if !grp_hl2.is_empty() { 
        if previous_content { ui.add_space(settings.ability_padding_y); }
        render_icon_row(ui, &grp_hl2, sheet, settings, main_border); 
        previous_content = true;
    }

    let has_body = !grp_b1.is_empty() || !grp_b2.is_empty();
    if has_body {
       if previous_content { ui.add_space(settings.ability_padding_y); }
       
       render_list_view(ui, &grp_b1, sheet, multihit_tex, kamikaze_tex, boss_wave_tex, cat.id, level, curve, s, settings, main_border);
       render_list_view(ui, &grp_b2, sheet, multihit_tex, kamikaze_tex, boss_wave_tex, cat.id, level, curve, s, settings, main_border);
       previous_content = true;
    }

    if !grp_footer.is_empty() {
        if previous_content && !has_body { 
            ui.add_space(settings.ability_padding_y);
        }
        render_icon_row(ui, &grp_footer, sheet, settings, main_border); 
    }
}

pub fn render_icon_row(ui: &mut egui::Ui, items: &Vec<AbilityItem>, sheet: &SpriteSheet, settings: &Settings, border_color: egui::Color32) {
    ui.spacing_mut().item_spacing = egui::vec2(settings.ability_padding_x, settings.ability_padding_y);
    ui.horizontal_wrapped(|ui| {
        for item in items {
            let r = render_single_icon(ui, item, sheet, border_color);
            r.on_hover_ui(|ui| text_with_superscript(ui, &item.text));
        }
    });
}

fn render_single_icon(ui: &mut egui::Ui, item: &AbilityItem, sheet: &SpriteSheet, border: egui::Color32) -> egui::Response {
    let size = egui::vec2(stats::ICON_SIZE, stats::ICON_SIZE);
    let response = if let Some(tex_id) = item.custom_tex {
        ui.add(egui::Image::new(egui::load::SizedTexture::new(tex_id, size)))
    } else if let Some(cut) = sheet.cuts_map.get(&item.icon_id) {
        if let Some(tex) = &sheet.texture_handle {
             ui.add(egui::Image::new(egui::load::SizedTexture::new(tex.id(), size)).uv(cut.uv_coordinates))
        } else {
             ui.allocate_response(size, egui::Sense::hover())
        }
    } else {
        let alt = img015::img015_alt(item.icon_id);
        render_fallback_icon(ui, alt, border)
    };

    if let Some(border_id) = item.border_id {
        if let Some(b_cut) = sheet.cuts_map.get(&border_id) {
            if let Some(tex) = &sheet.texture_handle {
                ui.put(response.rect, egui::Image::new(egui::load::SizedTexture::new(tex.id(), size)).uv(b_cut.uv_coordinates));
            }
        }
    }

    response
}

pub fn render_list_view(
    ui: &mut egui::Ui, 
    items: &Vec<AbilityItem>, 
    sheet: &SpriteSheet,
    multihit_tex: &Option<egui::TextureHandle>,
    kamikaze_tex: &Option<egui::TextureHandle>,   
    boss_wave_tex: &Option<egui::TextureHandle>, 
    cat_id: u32,
    current_level: i32,
    curve: Option<&stats::CatLevelCurve>,
    s: &CatRaw,
    settings: &Settings, 
    border_color: egui::Color32,
) {
    for item in items {
        let is_conjure = item.icon_id == img015::ICON_CONJURE;
        let id = ui.make_persistent_id(format!("conjure_expand_{}", cat_id));
        
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0;
            render_single_icon(ui, item, sheet, border_color); 

            if !is_conjure {
                text_with_superscript(ui, &item.text);
            } else {
                render_conjure_toggle(ui, &item.text, id, settings);
            }
        }); 

        let expanded = ui.data(|d| d.get_temp::<bool>(id).unwrap_or(settings.expand_spirit_details));
        if is_conjure && expanded {
            ui.add_space(3.0);
            render_conjure_details(ui, s, current_level, curve, sheet, multihit_tex, kamikaze_tex, boss_wave_tex, settings);
        }
        
        ui.add_space(settings.ability_padding_y);
    }
}

fn render_conjure_toggle(ui: &mut egui::Ui, text: &str, id: egui::Id, settings: &Settings) {
    ui.with_layout(egui::Layout::left_to_right(egui::Align::Center), |ui| {
        ui.spacing_mut().item_spacing.x = 7.0;
        
        let mut expanded = ui.data(|d| d.get_temp::<bool>(id).unwrap_or(settings.expand_spirit_details));
        text_with_superscript(ui, text);
        
        let btn_text = egui::RichText::new("Details").size(11.0);
        let btn = if expanded {
            egui::Button::new(btn_text.color(egui::Color32::WHITE)).fill(egui::Color32::from_rgb(0, 100, 200))
        } else {
            egui::Button::new(btn_text)
        };

        if ui.add(btn).clicked() {
            expanded = !expanded;
            ui.data_mut(|d| d.insert_temp(id, expanded));
        }
    });
}

fn render_conjure_details(
    ui: &mut egui::Ui,
    parent_stats: &CatRaw,
    level: i32,
    curve: Option<&stats::CatLevelCurve>,
    sheet: &SpriteSheet,
    multihit_tex: &Option<egui::TextureHandle>,
    kamikaze_tex: &Option<egui::TextureHandle>,   
    boss_wave_tex: &Option<egui::TextureHandle>, 
    settings: &Settings
) {
    egui::Frame::none()
        .fill(egui::Color32::from_black_alpha(220)) 
        .rounding(egui::Rounding { nw: 0.0, ne: 0.0, sw: 8.0, se: 8.0 }) 
        .inner_margin(8.0)
        .show(ui, |ui| {
            let spirit_border = egui::Color32::WHITE;
            
            let conjure_stats_vec = match stats::load_from_id(parent_stats.conjure_unit_id) {
                Some(s) => s,
                None => {
                    ui.label(egui::RichText::new("Spirit data not found").weak());
                    return;
                }
            };

            let conjure_stats = match conjure_stats_vec.first() {
                Some(s) => s,
                None => return,
            };

            let dmg = curve.as_ref().map_or(
                conjure_stats.attack_1, 
                |curve| curve.calculate_stat(conjure_stats.attack_1, level)
            );
            
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 8.0;
                let icon = img015::ICON_AREA_ATTACK;
                
                let size = egui::vec2(stats::ICON_SIZE, stats::ICON_SIZE);
                
                if let Some(cut) = sheet.cuts_map.get(&icon) {
                    if let Some(tex) = &sheet.texture_handle {
                         ui.add(egui::Image::new(egui::load::SizedTexture::new(tex.id(), size)).uv(cut.uv_coordinates));
                    }
                } else {
                    let alt = img015::img015_alt(icon);
                    render_fallback_icon(ui, alt, spirit_border);
                }
                
                ui.label(format!("Damage: {}\nRange: {}", dmg, conjure_stats.standing_range));
            });
            
            ui.add_space(settings.ability_padding_y);

            // Added spirit_traits unpacking here as well
            let (spirit_traits, spirit_head_1, spirit_head_2, spirit_body_1, spirit_body_2, spirit_footer) = abilities::collect_ability_data(
                conjure_stats, level, curve, multihit_tex, kamikaze_tex, boss_wave_tex, settings, true,
                None, // Spirits don't have talents
                None  // No level map for spirit
            );
            
            if !spirit_traits.is_empty() { render_icon_row(ui, &spirit_traits, sheet, settings, spirit_border); }
            if !spirit_head_1.is_empty() { render_icon_row(ui, &spirit_head_1, sheet, settings, spirit_border); }
            if !spirit_head_2.is_empty() { render_icon_row(ui, &spirit_head_2, sheet, settings, spirit_border); }
            
            render_list_view(ui, &spirit_body_1, sheet, multihit_tex, kamikaze_tex, boss_wave_tex, 0, level, curve, conjure_stats, settings, spirit_border);
            render_list_view(ui, &spirit_body_2, sheet, multihit_tex, kamikaze_tex, boss_wave_tex, 0, level, curve, conjure_stats, settings, spirit_border);
            
            if !spirit_footer.is_empty() {
                render_icon_row(ui, &spirit_footer, sheet, settings, spirit_border);
            }
        });
}