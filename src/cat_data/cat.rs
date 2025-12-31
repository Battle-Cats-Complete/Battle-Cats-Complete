use eframe::egui;
use std::path::Path;
use image::imageops; 
use super::scanner::CatEntry;
use super::sprites::SpriteSheet;
use super::definitions; 
use super::stats::{self, CatRaw}; 

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
) {
    let base_dir = Path::new("game/assets");
    let tex_en = base_dir.join("img015_en.png");
    let tex_ja = base_dir.join("img015_ja.png");
    let tex_raw = base_dir.join("img015.png");
    
    let texture_path = if tex_en.exists() { tex_en } 
        else if tex_ja.exists() { tex_ja } 
        else { tex_raw };

    let cut_path = base_dir.join("img015.imgcut");
    sprite_sheet.load(ctx, &texture_path, &cut_path);

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
            // Cat identity
            ui.horizontal_top(|ui| {
                let form_char = match *current_form { 0 => "f", 1 => "c", 2 => "s", _ => "u" };
                let expected = format!("game/cats/{:03}/{}/uni{:03}_{}00.png", cat.id, form_char, cat.id, form_char);

                if *current_key != expected {
                    *current_key = expected.clone(); 
                    *texture_cache = None; 
                    let p = Path::new(&expected);
                    let f = Path::new("game/cats/uni.png");
                    let load = if p.exists() { Some(p) } else if f.exists() { Some(f) } else { None };

                    if let Some(path) = load {
                        if let Ok(img) = image::open(path) {
                            let mut rgba = img.to_rgba8();
                            rgba = autocrop(rgba);
                            let size = [rgba.width() as usize, rgba.height() as usize];
                            let pixels = rgba.as_flat_samples();
                            *texture_cache = Some(ctx.load_texture("detail_icon", egui::ColorImage::from_rgba_unmultiplied(size, pixels.as_slice()), egui::TextureOptions::LINEAR));
                        }
                    }
                }

                if let Some(tex) = texture_cache { ui.image(&*tex); } 
                else { ui.allocate_space(egui::vec2(64.0, 64.0)); }

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
                
                // Damage
                let atk_1 = cat.curve.as_ref().map_or(s.attack_1, |c| c.calculate_stat(s.attack_1, *current_level));
                let atk_2 = cat.curve.as_ref().map_or(s.attack_2, |c| c.calculate_stat(s.attack_2, *current_level));
                let atk_3 = cat.curve.as_ref().map_or(s.attack_3, |c| c.calculate_stat(s.attack_3, *current_level));
                
                let total_atk = atk_1 + atk_2 + atk_3;

                let total_atk_cycle = s.attack_cycle(cat.atk_anim_frames[*current_form]);
                
                let dps = if total_atk_cycle > 0 {
                    (total_atk as f32 * 30.0 / total_atk_cycle as f32) as i32
                } else {
                    0
                };

                let atk_type = if s.area_attack == 0 { "Single" } else { "Area" };

                egui::Grid::new("stats_grid_right")
                    .min_col_width(60.0) 
                    .spacing([4.0, 4.0]) 
                    .show(ui, |ui| {
                        // Headers
                        grid_cell(ui, "Atk", true);
                        grid_cell(ui, "Dps", true);
                        grid_cell(ui, "Range", true);
                        grid_cell(ui, "Atk Cycle", true);
                        grid_cell(ui, "Atk Type", true); 
                        ui.end_row();

                        // Values 
                        grid_cell(ui, &format!("{}", total_atk), false);
                        grid_cell(ui, &format!("{}", dps), false);
                        grid_cell(ui, &format!("{}", s.standing_range), false);
                        grid_cell_custom(ui, false, |ui| render_frames(ui, total_atk_cycle)); 
                        grid_cell(ui, atk_type, false);
                        ui.end_row();

                        // Headers
                        grid_cell(ui, "Hp", true);
                        grid_cell(ui, "Kb", true);
                        grid_cell(ui, "Speed", true);
                        grid_cell(ui, "Cooldown", true); 
                        grid_cell(ui, "Cost", true);     
                        ui.end_row();

                        // Values
                        grid_cell(ui, &format!("{}", hp), false);
                        grid_cell(ui, &format!("{}", s.knockbacks), false);
                        grid_cell(ui, &format!("{}", s.speed), false);
                        grid_cell_custom(ui, false, |ui| render_frames(ui, s.cooldown));
                        grid_cell(ui, &format!("{}¢", s.eoc1_cost * 3 / 2), false); 
                        ui.end_row();
                    });
            }
        });

        ui.add_space(3.0); 
        ui.separator(); 
        ui.add_space(3.0);

        if let Some(s) = current_stats {
            // Traits
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 4.0;
                for &line_num in definitions::UI_TRAIT_ORDER {
                    let has_trait = match line_num {
                        definitions::ICON_TRAIT_RED       => s.target_red > 0,
                        definitions::ICON_TRAIT_FLOATING  => s.target_floating > 0,
                        definitions::ICON_TRAIT_BLACK     => s.target_black > 0,
                        definitions::ICON_TRAIT_METAL     => s.target_metal > 0,
                        definitions::ICON_TRAIT_ANGEL     => s.target_angel > 0,
                        definitions::ICON_TRAIT_ALIEN     => s.target_alien > 0,
                        definitions::ICON_TRAIT_ZOMBIE    => s.target_zombie > 0,
                        definitions::ICON_TRAIT_RELIC     => s.target_relic > 0,
                        definitions::ICON_TRAIT_AKU       => s.target_aku > 0,
                        definitions::ICON_TRAIT_TRAITLESS => s.target_traitless > 0,
                        _ => false,
                    };
                    if has_trait {
                        if let Some(sprite) = sprite_sheet.get_sprite_by_line(line_num) {
                            ui.add(sprite.fit_to_exact_size(egui::vec2(stats::ICON_SIZE, stats::ICON_SIZE)));
                        }
                    }
                }
            });

            ui.add_space(5.0);

            render_abilities(ui, s, sprite_sheet, multihit_texture);
        }
    });
}

// Abilities
fn render_abilities(ui: &mut egui::Ui, s: &CatRaw, sprite_sheet: &SpriteSheet, multihit_tex: &Option<egui::TextureHandle>) {
    let mut is_omni = false;
    let mut has_ld = false;

    // Check all 3 hits for LD/Omni
    let check_hits = [
        (s.long_distance_1_anchor, s.long_distance_1_span),
        (s.long_distance_2_anchor, if s.long_distance_2_flag == 1 { s.long_distance_2_span } else { 0 }),
        (s.long_distance_3_anchor, if s.long_distance_3_flag == 1 { s.long_distance_3_span } else { 0 }),
    ];

    for (anchor, span) in check_hits {
        if span != 0 {
            let start_range = std::cmp::min(anchor, anchor + span);
            if start_range <= 0 { is_omni = true; }
            else { has_ld = true; }
        }
    }

    let show_omni = is_omni;
    let show_ld = !is_omni && has_ld;
    
    // Check Multi-Hit (If 2nd attack has damage > 0)
    let is_multihit = s.attack_2 > 0;

    // 1. General Abilities
    let abilities_main = [
        (s.strong_against > 0, definitions::ICON_STRONG_AGAINST),
        (s.resist > 0, definitions::ICON_RESIST),
        (s.insanely_tough > 0, definitions::ICON_INSANELY_TOUGH),
        (s.massive_damage > 0, definitions::ICON_MASSIVE_DAMAGE),
        (s.insane_damage > 0, definitions::ICON_INSANE_DAMAGE),
        (s.attack_only > 0, definitions::ICON_ATTACK_ONLY),
        (s.weaken_chance > 0, definitions::ICON_WEAKEN),
        (s.freeze_chance > 0, definitions::ICON_FREEZE),
        (s.slow_chance > 0, definitions::ICON_SLOW),
        (s.knockback_chance > 0, definitions::ICON_KNOCKBACK),
        (s.strengthen_threshold > 0, definitions::ICON_STRENGTHEN),
        (s.survive > 0, definitions::ICON_SURVIVE),
        (s.base_destroyer > 0, definitions::ICON_BASE_DESTROYER),
        (s.critical_chance > 0, definitions::ICON_CRITICAL_HIT),
        (s.double_bounty > 0, definitions::ICON_DOUBLE_BOUNTY),
        (s.wave_chance > 0 && s.mini_wave_flag == 0, definitions::ICON_WAVE),
        (s.wave_chance > 0 && s.mini_wave_flag > 0, definitions::ICON_MINI_WAVE),
        (s.metal > 0, definitions::ICON_METAL),
        (s.savage_blow_chance > 0, definitions::ICON_SAVAGE_BLOW),
        (s.surge_chance > 0 && s.mini_surge_flag == 0, definitions::ICON_SURGE),
        (s.surge_chance > 0 && s.mini_surge_flag > 0, definitions::ICON_MINI_SURGE),
        (s.zombie_killer > 0, definitions::ICON_ZOMBIE_KILLER),
        (s.barrier_breaker_chance > 0, definitions::ICON_BARRIER_BREAKER),
        (s.shield_pierce_chance > 0, definitions::ICON_SHIELD_PEIRCER), 
        (s.soulstrike > 0, definitions::ICON_SOULSTRIKE),
        (s.conjure_unit_id > 0, definitions::ICON_CONJURE),
        (s.metal_killer_percent > 0, definitions::ICON_METAL_KILLER),
        (s.explosion_chance > 0, definitions::ICON_EXPLOSION),
        (s.curse_chance > 0, definitions::ICON_CURSE),
        (s.dodge_chance > 0, definitions::ICON_DODGE),
        (s.warp_chance > 0, definitions::ICON_WARP),
        (s.eva_killer > 0, definitions::ICON_EVA_KILLER),
        (s.witch_killer > 0, definitions::ICON_WITCH_KILLER),
        (s.colossus_slayer > 0, definitions::ICON_COLOSSUS_SLAYER),
        (s.behemoth_slayer > 0, definitions::ICON_BEHEMOTH_SLAYER),
        (s.sage_slayer > 0, definitions::ICON_SAGE_SLAYER),
        (s.curse_immune > 0, definitions::ICON_IMMUNE_CURSE),
        (s.wave_immune > 0, definitions::ICON_IMMUNE_WAVE),
        (s.weaken_immune > 0, definitions::ICON_IMMUNE_WEAKEN),
        (s.freeze_immune > 0, definitions::ICON_IMMUNE_FREEZE),
        (s.slow_immune > 0, definitions::ICON_IMMUNE_SLOW),
        (s.knockback_immune > 0, definitions::ICON_IMMUNE_KNOCKBACK),
        (s.toxic_immune > 0, definitions::ICON_IMMUNE_TOXIC),
        (s.surge_immune > 0, definitions::ICON_IMMUNE_SURGE),
        (s.warp_immune > 0, definitions::ICON_IMMUNE_WARP),
        (s.explosion_immune > 0, definitions::ICON_IMMUNE_EXPLOSION),
        (s.wave_block > 0, definitions::ICON_WAVE_BLOCK),
        (s.counter_surge > 0, definitions::ICON_COUNTER_SURGE),
    ];

    // 2. Range Abilities (Omni/LD)
    let abilities_range = [
        (show_omni, definitions::ICON_OMNI_STRIKE),
        (show_ld, definitions::ICON_LONG_DISTANCE),
    ];

    ui.horizontal_wrapped(|ui| {
        ui.spacing_mut().item_spacing.x = 4.0;
        ui.spacing_mut().item_spacing.y = 4.0;
        
        // Render Main Abilities
        for (has, icon) in abilities_main {
            if has {
                if let Some(sprite) = sprite_sheet.get_sprite_by_line(icon) {
                    ui.add(sprite.fit_to_exact_size(egui::vec2(stats::ICON_SIZE, stats::ICON_SIZE)));
                } 
            }
        }

        // Render Multi-Hit
        if is_multihit {
            if let Some(tex) = multihit_tex {
                ui.add(egui::Image::new(tex).fit_to_exact_size(egui::vec2(stats::ICON_SIZE, stats::ICON_SIZE)));
            }
        }

        for (has, icon) in abilities_range {
            if has {
                if let Some(sprite) = sprite_sheet.get_sprite_by_line(icon) {
                    ui.add(sprite.fit_to_exact_size(egui::vec2(stats::ICON_SIZE, stats::ICON_SIZE)));
                } 
            }
        }
    });
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