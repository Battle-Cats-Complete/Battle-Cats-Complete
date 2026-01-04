use eframe::egui;
use image::imageops; 
use super::scanner::CatEntry;
use super::sprites::SpriteSheet;
use crate::definitions; 
use super::stats::{self, CatRaw}; 
use super::abilities::{self, AbilityItem}; 

const ABILITY_PADDING_X: f32 = 3.0; 
const ABILITY_PADDING_Y: f32 = 0.0; 

pub fn show(
    ctx: &egui::Context, 
    ui: &mut egui::Ui, 
    cat: &CatEntry, 
    current_form: &mut usize,
    level_input: &mut String,   
    current_level: &mut i32,    
    texture_cache: &mut Option<egui::TextureHandle>,
    current_key: &mut String,
    sprite_sheet: &mut SpriteSheet,
    multihit_texture: &mut Option<egui::TextureHandle>,
    expand_spirit_details: bool, 
) {
    let base_dir = std::path::Path::new("game/assets");
    
    // 1. Sprite Sheet: Async (Fixes startup lag)
    let texture_path = base_dir.join("img015/img015_en.png");
    let cut_path = base_dir.join("img015/img015_en.imgcut");
    sprite_sheet.load(ctx, &texture_path, &cut_path);

    // 2. Multihit icon
    if multihit_texture.is_none() {
        const MULTIHIT_BYTES: &[u8] = include_bytes!("../../assets/multihit.png");
        if let Ok(img) = image::load_from_memory(MULTIHIT_BYTES) {
            let rgba = img.to_rgba8();
            *multihit_texture = Some(ctx.load_texture(
                "multihit_icon",
                egui::ColorImage::from_rgba_unmultiplied(
                    [rgba.width() as usize, rgba.height() as usize],
                    rgba.as_flat_samples().as_slice()
                ),
                egui::TextureOptions::LINEAR
            ));
        }
    }

    let current_stats = cat.stats.get(*current_form).and_then(|opt| opt.as_ref());

    ui.vertical(|ui| {
        ui.scope(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0; 
            ui.horizontal(|ui| {
                let form_labels = ["Normal", "Evolved", "True", "Ultra"];
                for (index, &exists) in cat.forms.iter().enumerate() {
                    if exists {
                        let label = form_labels[index];
                        let is_selected = *current_form == index;
                        let (fill, stroke, text) = if is_selected {
                            (egui::Color32::from_rgb(0, 100, 200), egui::Stroke::new(2.0, egui::Color32::WHITE), egui::Color32::WHITE)
                        } else {
                            (egui::Color32::from_gray(40), egui::Stroke::new(1.0, egui::Color32::from_gray(100)), egui::Color32::from_gray(200))
                        };
                        let btn = egui::Button::new(egui::RichText::new(label).color(text))
                            .fill(fill).stroke(stroke).rounding(egui::Rounding::ZERO).min_size(egui::vec2(60.0, 30.0));
                        if ui.add(btn).clicked() { *current_form = index; }
                    }
                }
            });
        });

        ui.separator(); 
        ui.add_space(5.0);

        ui.horizontal_top(|ui| {
            // 3. Unit Image: Synchronous (Fixes flickering)
            ui.horizontal_top(|ui| {
                let form_char = match *current_form { 0 => "f", 1 => "c", 2 => "s", _ => "u" };
                let expected = format!("game/cats/{:03}/{}/uni{:03}_{}00.png", cat.id, form_char, cat.id, form_char);

                if *current_key != expected {
                    *current_key = expected.clone(); 
                    *texture_cache = None; 
                    
                    let p = std::path::Path::new(&expected);
                    let f = std::path::Path::new("game/cats/uni.png");
                    let path_to_load = if p.exists() { Some(p) } else if f.exists() { Some(f) } else { None };

                    if let Some(path) = path_to_load {
                        // LOAD IMMEDIATELY (BLOCKING)
                        if let Ok(img) = image::open(path) {
                            let mut rgba = img.to_rgba8();
                            rgba = autocrop(rgba);
                            let size = [rgba.width() as usize, rgba.height() as usize];
                            let pixels = rgba.as_flat_samples();
                            *texture_cache = Some(ctx.load_texture("detail_icon", egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()), egui::TextureOptions::LINEAR));
                        }
                    }
                }

                if let Some(tex) = texture_cache { 
                    ui.image(&*tex); 
                } else { 
                    ui.allocate_space(egui::vec2(64.0, 64.0)); 
                }

                ui.add_space(10.0);

                ui.vertical(|ui| {
                    let form_num = *current_form + 1;
                    let raw_name = cat.names.get(*current_form).cloned().unwrap_or_default();
                    let disp_name = if raw_name.is_empty() { format!("{:03}-{}", cat.id, form_num) } else { raw_name };

                    ui.heading(disp_name);
                    ui.label(egui::RichText::new(format!("ID: {:03}-{}", cat.id, form_num)).color(egui::Color32::from_gray(100)).size(12.0));
                    
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        ui.label("Level:");
                        let response = ui.add(egui::TextEdit::singleline(level_input).desired_width(40.0));
                        if response.changed() {
                            let mut sum = 0;
                            let parts = level_input.split('+');
                            for part in parts {
                                if let Ok(val) = part.trim().parse::<i32>() { sum += val; }
                            }
                            if sum <= 0 { *current_level = 1; } 
                            else { *current_level = sum; }
                        }
                    });
                });
            }); 

            ui.add_space(30.0);

            // Stats grid
            if let Some(s) = current_stats {
                let hp = cat.curve.as_ref().map_or(s.hitpoints, |c| c.calculate_stat(s.hitpoints, *current_level));
                
                let atk_1 = cat.curve.as_ref().map_or(s.attack_1, |c| c.calculate_stat(s.attack_1, *current_level));
                let atk_2 = cat.curve.as_ref().map_or(s.attack_2, |c| c.calculate_stat(s.attack_2, *current_level));
                let atk_3 = cat.curve.as_ref().map_or(s.attack_3, |c| c.calculate_stat(s.attack_3, *current_level));
                let total_atk = atk_1 + atk_2 + atk_3;

                let total_atk_cycle = s.attack_cycle(cat.atk_anim_frames[*current_form]);
                
                let dps = if total_atk_cycle > 0 {
                    (total_atk as f32 * 30.0 / total_atk_cycle as f32) as i32
                } else { 0 };

                let atk_type = if s.area_attack == 0 { "Single" } else { "Area" };

                egui::Grid::new("stats_grid_right").min_col_width(60.0).spacing([4.0, 4.0]).show(ui, |ui| {
                        grid_cell(ui, "Atk", true);
                        grid_cell(ui, "Dps", true);
                        grid_cell(ui, "Range", true);
                        grid_cell(ui, "Atk Cycle", true);
                        grid_cell(ui, "Atk Type", true); 
                        ui.end_row();

                        grid_cell(ui, &format!("{}", total_atk), false);
                        grid_cell(ui, &format!("{}", dps), false);
                        grid_cell(ui, &format!("{}", s.standing_range), false);
                        grid_cell_custom(ui, false, |ui| render_frames(ui, total_atk_cycle)); 
                        grid_cell(ui, atk_type, false);
                        ui.end_row();

                        grid_cell(ui, "Hp", true);
                        grid_cell(ui, "Kb", true);
                        grid_cell(ui, "Speed", true);
                        grid_cell(ui, "Cooldown", true); 
                        grid_cell(ui, "Cost", true);     
                        ui.end_row();

                        grid_cell(ui, &format!("{}", hp), false);
                        grid_cell(ui, &format!("{}", s.knockbacks), false);
                        grid_cell(ui, &format!("{}", s.speed), false);
                        
                        grid_cell_custom(ui, false, |ui| render_frames(ui, s.effective_cooldown()));
                        
                        grid_cell(ui, &format!("{}¢", s.eoc1_cost * 3 / 2), false); 
                        ui.end_row();
                });
            }
        });

        ui.add_space(3.0); 
        ui.separator(); 
    });

    egui::ScrollArea::vertical()
        .auto_shrink([false, false]) 
        .show(ui, |ui| {
            if let Some(s) = current_stats {
                
                let has_any_trait = 
                    s.target_red > 0 || s.target_floating > 0 || s.target_black > 0 ||
                    s.target_metal > 0 || s.target_angel > 0 || s.target_alien > 0 ||
                    s.target_zombie > 0 || s.target_relic > 0 || s.target_aku > 0 ||
                    s.target_traitless > 0;

                if has_any_trait {
                    ui.add_space(ABILITY_PADDING_Y); 

                    ui.horizontal_wrapped(|ui| {
                        ui.spacing_mut().item_spacing = egui::vec2(ABILITY_PADDING_X, ABILITY_PADDING_Y);
                        
                        for &line_num in definitions::UI_TRAIT_ORDER {
                            let has_trait = match line_num {
                                definitions::ICON_TRAIT_RED => s.target_red > 0,
                                definitions::ICON_TRAIT_FLOATING => s.target_floating > 0,
                                definitions::ICON_TRAIT_BLACK => s.target_black > 0,
                                definitions::ICON_TRAIT_METAL => s.target_metal > 0,
                                definitions::ICON_TRAIT_ANGEL => s.target_angel > 0,
                                definitions::ICON_TRAIT_ALIEN => s.target_alien > 0,
                                definitions::ICON_TRAIT_ZOMBIE => s.target_zombie > 0,
                                definitions::ICON_TRAIT_RELIC => s.target_relic > 0,
                                definitions::ICON_TRAIT_AKU => s.target_aku > 0,
                                definitions::ICON_TRAIT_TRAITLESS => s.target_traitless > 0,
                                _ => false,
                            };
                            if has_trait {
                                if let Some(sprite) = sprite_sheet.get_sprite_by_line(line_num) {
                                    let r = ui.add(sprite.fit_to_exact_size(egui::vec2(stats::ICON_SIZE, stats::ICON_SIZE)));
                                    
                                    let tooltip_text = match line_num {
                                        definitions::ICON_TRAIT_RED => "Targets Red Enemies",
                                        definitions::ICON_TRAIT_FLOATING => "Targets Floating Enemies",
                                        definitions::ICON_TRAIT_BLACK => "Targets Black Enemies",
                                        definitions::ICON_TRAIT_METAL => "Targets Metal Enemies",
                                        definitions::ICON_TRAIT_ANGEL => "Targets Angel Enemies",
                                        definitions::ICON_TRAIT_ALIEN => "Targets Alien Enemies",
                                        definitions::ICON_TRAIT_ZOMBIE => "Targets Zombie Enemies",
                                        definitions::ICON_TRAIT_RELIC => "Targets Relic Enemies",
                                        definitions::ICON_TRAIT_AKU => "Targets Aku Enemies",
                                        definitions::ICON_TRAIT_TRAITLESS => "Targets Traitless Enemies",
                                        _ => "",
                                    };
                                    
                                    if !tooltip_text.is_empty() {
                                        r.on_hover_text(tooltip_text);
                                    }
                                }
                            }
                        }
                    });
                }

                ui.add_space(ABILITY_PADDING_Y);
                render_abilities(ui, s, sprite_sheet, multihit_texture, *current_level, cat.curve.as_ref(), cat.id, expand_spirit_details); 
                
                ui.add_space(20.0);
            }
        });
}

fn render_icon_row(ui: &mut egui::Ui, items: &Vec<AbilityItem>, sheet: &SpriteSheet) {
    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing = egui::vec2(ABILITY_PADDING_X, ABILITY_PADDING_Y);
        for item in items {
            let r = if let Some(tex_id) = item.custom_tex {
                ui.add(egui::Image::new((tex_id, egui::vec2(stats::ICON_SIZE, stats::ICON_SIZE))))
            } else if let Some(sprite) = sheet.get_sprite_by_line(item.icon_id) {
                ui.add(sprite.fit_to_exact_size(egui::vec2(stats::ICON_SIZE, stats::ICON_SIZE)))
            } else { continue; };
            
            r.on_hover_ui(|ui| { 
                text_with_superscript(ui, &item.text); 
            });
        }
    });
}

fn render_list_view(
    ui: &mut egui::Ui, 
    items: &Vec<AbilityItem>, 
    sheet: &SpriteSheet,
    multihit_tex: &Option<egui::TextureHandle>,
    cat_id: u32,
    current_level: i32,
    curve: Option<&stats::CatLevelCurve>,
    s: &CatRaw,
    expand_spirit_details: bool, 
) {
    for item in items {
        let is_conjure_item = item.icon_id == definitions::ICON_CONJURE;
        let mut expanded = false;
        let id = ui.make_persistent_id(format!("conjure_expand_{}", cat_id));

        ui.horizontal(|ui| {
            ui.spacing_mut().item_spacing.x = ABILITY_PADDING_X;
            
            let icon_size = egui::vec2(stats::ICON_SIZE, stats::ICON_SIZE);
            let (rect, _) = ui.allocate_exact_size(icon_size, egui::Sense::hover());
            
            if let Some(tex_id) = item.custom_tex {
                egui::Image::new((tex_id, icon_size)).paint_at(ui, rect);
            } else if let Some(sprite) = sheet.get_sprite_by_line(item.icon_id) {
                sprite.paint_at(ui, rect);
            }

            if is_conjure_item {
                expanded = ui.data(|d| d.get_temp::<bool>(id).unwrap_or(expand_spirit_details));
                text_with_superscript(ui, &item.text);
                
                ui.add_space(4.0);

                let btn_text = egui::RichText::new("Details").size(11.0);
                let btn = if expanded {
                    egui::Button::new(btn_text.color(egui::Color32::WHITE))
                        .fill(egui::Color32::from_rgb(0, 100, 200))
                } else {
                    egui::Button::new(btn_text)
                };

                if ui.add(btn).clicked() {
                    expanded = !expanded;
                    ui.data_mut(|d| d.insert_temp(id, expanded));
                }
            } else {
                text_with_superscript(ui, &item.text);
            }
        }); 

        if is_conjure_item && expanded {
            ui.add_space(5.0);
            
            egui::Frame::none()
                .fill(egui::Color32::from_black_alpha(220)) 
                .rounding(egui::Rounding { nw: 0.0, ne: 0.0, sw: 8.0, se: 8.0 }) 
                .inner_margin(8.0)
                .show(ui, |ui| {
                    
                    if let Some(conjure_stats) = stats::load_from_id(s.conjure_unit_id) {
                        
                        let dmg = curve.as_ref().map_or(
                            conjure_stats.attack_1, 
                            |c| c.calculate_stat(conjure_stats.attack_1, current_level)
                        );

                        let range_txt = format!("Damage: {}\nRange: {}", dmg, conjure_stats.standing_range);
                        ui.horizontal(|ui| {
                            ui.spacing_mut().item_spacing.x = ABILITY_PADDING_X;
                            
                            if let Some(sprite) = sheet.get_sprite_by_line(definitions::ICON_AREA_ATTACK) {
                                ui.add(sprite.fit_to_exact_size(egui::vec2(stats::ICON_SIZE, stats::ICON_SIZE)));
                            }
                            ui.label(range_txt);
                        });
                        ui.add_space(ABILITY_PADDING_Y);

                        let (c_hl1, c_hl2, c_b1, c_b2, c_ft) = abilities::collect_ability_data(&conjure_stats, current_level, curve, multihit_tex, true);
                        
                        if !c_hl1.is_empty() { render_icon_row(ui, &c_hl1, sheet); }
                        if !c_hl2.is_empty() { ui.add_space(ABILITY_PADDING_Y); render_icon_row(ui, &c_hl2, sheet); }
                        if !c_hl1.is_empty() || !c_hl2.is_empty() { ui.add_space(ABILITY_PADDING_Y * 2.0); }
                        
                        render_list_view(ui, &c_b1, sheet, multihit_tex, 0, current_level, curve, &conjure_stats, expand_spirit_details);
                        render_list_view(ui, &c_b2, sheet, multihit_tex, 0, current_level, curve, &conjure_stats, expand_spirit_details);
                        
                        if !c_ft.is_empty() {
                            ui.add_space(ABILITY_PADDING_Y);
                            render_icon_row(ui, &c_ft, sheet);
                        }
                    } else {
                        ui.label(egui::RichText::new("Spirit data not found").weak());
                    }
                });
        }
        ui.add_space(ABILITY_PADDING_Y);
    }
}

fn render_abilities(
    ui: &mut egui::Ui, 
    s: &CatRaw, 
    sheet: &SpriteSheet, 
    multihit_tex: &Option<egui::TextureHandle>, 
    level: i32,
    curve: Option<&stats::CatLevelCurve>,
    cat_id: u32,
    expand_spirit_details: bool, 
) {
    let (grp_hl1, grp_hl2, grp_b1, grp_b2, grp_footer) = abilities::collect_ability_data(s, level, curve, multihit_tex, false);

    if !grp_hl1.is_empty() { render_icon_row(ui, &grp_hl1, sheet); }
    if !grp_hl2.is_empty() { ui.add_space(ABILITY_PADDING_Y); render_icon_row(ui, &grp_hl2, sheet); }

    if !grp_hl1.is_empty() || !grp_hl2.is_empty() {
        ui.add_space(ABILITY_PADDING_Y * 2.0);
    }

    render_list_view(ui, &grp_b1, sheet, multihit_tex, cat_id, level, curve, s, expand_spirit_details);
    render_list_view(ui, &grp_b2, sheet, multihit_tex, cat_id, level, curve, s, expand_spirit_details);

    if !grp_footer.is_empty() {
        ui.add_space(ABILITY_PADDING_Y);
        render_icon_row(ui, &grp_footer, sheet); 
    }
}

fn grid_cell(ui: &mut egui::Ui, text: &str, is_header: bool) {
    grid_cell_custom(ui, is_header, |ui| {
        let rt = if is_header { egui::RichText::new(text).strong() } else { egui::RichText::new(text) };
        ui.label(rt);
    });
}

fn grid_cell_custom<F>(ui: &mut egui::Ui, is_header: bool, add_contents: F) where F: FnOnce(&mut egui::Ui) {
    let bg = if is_header { egui::Color32::from_gray(20) } else { egui::Color32::from_gray(60) };
    egui::Frame::none().fill(bg).rounding(4.0).inner_margin(1.5).show(ui, |ui| {
        ui.set_min_width(60.0);
        ui.vertical_centered(|ui| add_contents(ui));
    });
}

fn render_frames(ui: &mut egui::Ui, frames: i32) {
    let seconds = frames as f32 / 30.0;
    let body_font = ui.style().text_styles.get(&egui::TextStyle::Body).cloned().unwrap_or(egui::FontId::proportional(14.0));
    let mut job = egui::text::LayoutJob::default();
    
    job.append(&format!("{:.2}s", seconds), 0.0, egui::TextFormat {
        font_id: body_font.clone(),
        color: ui.visuals().text_color(),
        ..Default::default()
    });

    job.append(&format!(" {}f", frames), 0.0, egui::TextFormat {
        font_id: egui::FontId::proportional(body_font.size * 0.65), 
        color: egui::Color32::from_gray(200),
        valign: egui::Align::Center, 
        ..Default::default()
    });
    ui.label(job);
}

fn text_with_superscript(ui: &mut egui::Ui, text: &str) {
    if text.contains('^') {
        let parts: Vec<&str> = text.split('^').collect();
        if parts.len() >= 2 {
            let body_font = ui.style().text_styles.get(&egui::TextStyle::Body).cloned().unwrap_or(egui::FontId::proportional(14.0));
            let mut job = egui::text::LayoutJob::default();
            job.wrap.max_width = ui.spacing().tooltip_width;

            job.append(parts[0], 0.0, egui::TextFormat {
                font_id: body_font.clone(),
                color: ui.visuals().text_color(),
                ..Default::default()
            });

            job.append(parts[1], 0.0, egui::TextFormat {
                font_id: egui::FontId::proportional(body_font.size * 0.70), 
                color: ui.visuals().text_color(),
                valign: egui::Align::Min, 
                ..Default::default()
            });
            ui.label(job);
            return;
        }
    }
    ui.label(text);
}

fn autocrop(img: image::RgbaImage) -> image::RgbaImage {
    let (width, height) = img.dimensions();
    let (mut min_x, mut min_y, mut max_x, mut max_y) = (width, height, 0, 0);
    let mut found = false;
    for (x, y, pixel) in img.enumerate_pixels() {
        if pixel[3] > 0 { 
            min_x = min_x.min(x); min_y = min_y.min(y);
            max_x = max_x.max(x); max_y = max_y.max(y);
            found = true;
        }
    }
    if !found { return img; }
    imageops::crop_imm(&img, min_x, min_y, max_x - min_x + 1, max_y - min_y + 1).to_image()
}