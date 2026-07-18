use eframe::egui;
use nyanko::common::data::img015;

use core::modules::cat::game::abilities;
use core::modules::cat::game::CatRenderContext;
use core::modules::cat::scanner::CatEntry;
use core::modules::cat::game::registry::AbilityIcon;
use core::common::game::{
    AbilityItem, CustomIcon, ABILITY_X, ABILITY_Y, TRAIT_Y,
};

use crate::common::CustomAssets;
use crate::common::shared::{render_fallback_icon, text_with_superscript, ICON_SIZE};
use crate::common::SpriteSheet;

pub(crate) fn render(
    ui: &mut egui::Ui, 
    ctx: &CatRenderContext,
    cat: &CatEntry, 
    sheets: &[SpriteSheet],
    assets: &CustomAssets,
    settings: &core::modules::settings::Settings
) {
    ui.spacing_mut().item_spacing.y = 0.0;
    
    let (grp_trait, grp_hl1, grp_hl2, grp_b1, grp_b2, grp_footer) = abilities::collect_ability_data(ctx);
    
    let mut previous_content = false;
    let mut last_was_trait = false;
    let main_border = egui::Color32::BLACK;

    if !grp_trait.is_empty() {
        render_icon_row(ui, &grp_trait, sheets, main_border, assets);
        previous_content = true;
        last_was_trait = true;
    }

    if !grp_hl1.is_empty() { 
        if previous_content { ui.add_space(if last_was_trait { TRAIT_Y } else { ABILITY_Y }); last_was_trait = false; }
        render_icon_row(ui, &grp_hl1, sheets, main_border, assets);
        previous_content = true;
    }
    
    if !grp_hl2.is_empty() { 
        if previous_content { ui.add_space(if last_was_trait { TRAIT_Y } else { ABILITY_Y }); last_was_trait = false; }
        render_icon_row(ui, &grp_hl2, sheets, main_border, assets);
        previous_content = true;
    }

    let has_body = !grp_b1.is_empty() || !grp_b2.is_empty();
    if has_body {
       if previous_content { ui.add_space(if last_was_trait { TRAIT_Y } else { ABILITY_Y }); last_was_trait = false; }
       
       render_list_view(ui, &grp_b1, sheets, cat.id, ctx, main_border, assets, settings);
       
       if !grp_b1.is_empty() && !grp_b2.is_empty() { ui.add_space(ABILITY_Y); }

       render_list_view(ui, &grp_b2, sheets, cat.id, ctx, main_border, assets, settings);
       previous_content = true;
    }

    if !grp_footer.is_empty() {
        if previous_content { ui.add_space(if last_was_trait { TRAIT_Y } else { ABILITY_Y }); }
        render_icon_row(ui, &grp_footer, sheets, main_border, assets);
    }
}

pub(crate) fn render_icon_row(
    ui: &mut egui::Ui,
    items: &Vec<AbilityItem>,
    sheets: &[SpriteSheet],
    border_color: egui::Color32,
    assets: &crate::common::CustomAssets,
) {
    ui.scope(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(ABILITY_X, ABILITY_Y);
        ui.horizontal_wrapped(|ui| {
            for item in items {
                let r = render_single_icon(ui, item, sheets, border_color, assets);
                r.on_hover_ui(|ui| text_with_superscript(ui, &item.text));
            }
        });
    });
}

fn render_single_icon(
    ui: &mut egui::Ui,
    item: &AbilityItem,
    sheets: &[SpriteSheet],
    border: egui::Color32,
    assets: &crate::common::CustomAssets,
) -> egui::Response {
    let size = egui::vec2(ICON_SIZE, ICON_SIZE);

    if let Some(tex) = assets.get_icon_texture(item.custom_icon) {
        return ui.add(egui::Image::new(egui::load::SizedTexture::new(tex.id(), size)));
    }

    if let Some(icon_id) = item.icon_id {
        for sheet in sheets {
            if let Some(cut) = sheet.core.cuts_map.get(&icon_id) {
                if let Some(tex) = &sheet.texture_handle {
                     let response = ui.add(egui::Image::new(egui::load::SizedTexture::new(tex.id(), size)).uv(egui::Rect::from_min_max(egui::pos2(cut.uv_coordinates.min.x, cut.uv_coordinates.min.y), egui::pos2(cut.uv_coordinates.max.x, cut.uv_coordinates.max.y))));
                     if let Some(border_id) = item.border_id
                         && let Some(b_cut) = sheet.core.cuts_map.get(&border_id) {
                             ui.put(response.rect, egui::Image::new(egui::load::SizedTexture::new(tex.id(), size)).uv(egui::Rect::from_min_max(egui::pos2(b_cut.uv_coordinates.min.x, b_cut.uv_coordinates.min.y), egui::pos2(b_cut.uv_coordinates.max.x, b_cut.uv_coordinates.max.y))));
                         }
                     return response;
                } else if sheet.core.is_loading_active {
                     return ui.allocate_response(size, egui::Sense::hover());
                }
            }
        }
    }

    let icon_enum = if item.custom_icon != CustomIcon::None {
        AbilityIcon::Custom(item.custom_icon)
    } else {
        AbilityIcon::Standard(item.icon_id.unwrap_or(9999)) 
    };

    let alt = core::modules::cat::game::registry::get_fallback_by_icon(icon_enum);
    render_fallback_icon(ui, alt, border)
}

pub(crate) fn render_list_view(
    ui: &mut egui::Ui,
    items: &Vec<AbilityItem>,
    sheets: &[SpriteSheet],
    cat_id: u32,
    ctx: &CatRenderContext,
    border_color: egui::Color32,
    assets: &crate::common::CustomAssets,
    settings: &core::modules::settings::Settings
) {
    for (i, item) in items.iter().enumerate() {
        let is_conjure = item.icon_id == Some(img015::ICON_CONJURE) && item.custom_icon == CustomIcon::None;
        let id = egui::Id::new(format!("conjure_expand_{}", cat_id));
        
        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = 8.0; 
            render_single_icon(ui, item, sheets, border_color, assets);

            if !is_conjure {
                text_with_superscript(ui, &item.text);
            } else {
                crate::modules::cat::conjure::render_conjure_toggle(ui, &item.text, id, settings);
            }
        }); 

        let expanded = ui.data(|d| d.get_temp::<bool>(id).unwrap_or(settings.cat_data.expand_spirit_details));
        if is_conjure && expanded {
            ui.add_space(ABILITY_Y);
            crate::modules::cat::conjure::render_conjure_details(ui, ctx, sheets, assets, settings);
        }
        
        if i < items.len() - 1 {
            ui.add_space(ABILITY_Y);
        }
    }
}