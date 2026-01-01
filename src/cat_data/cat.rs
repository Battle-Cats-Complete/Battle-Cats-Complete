use eframe::egui;
use std::path::Path;
use image::imageops; 
use super::scanner::CatEntry;
use super::sprites::SpriteSheet;
use super::definitions; 
use super::stats::{self, CatRaw}; 

// --- GLOBAL PADDING CONFIGURATION ---
const ABILITY_PADDING_X: f32 = 3.0; // Spacing between icons horizontally
const ABILITY_PADDING_Y: f32 = 0.0; // Spacing between rows/groups vertically

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

    // START DYNAMIC SCROLL AREA
    egui::ScrollArea::vertical()
        .auto_shrink([false, false]) 
        .show(ui, |ui| {
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
                                grid_cell_custom(ui, false, |ui| render_frames(ui, s.cooldown));
                                grid_cell(ui, &format!("{}¢", s.eoc1_cost * 3 / 2), false); 
                                ui.end_row();
                        });
                    }
                });

                ui.add_space(5.0); 
                ui.separator(); 
                // Global Padding applied between stats and traits
                ui.add_space(ABILITY_PADDING_Y); 

                if let Some(s) = current_stats {
                    // Traits
                    ui.horizontal_wrapped(|ui| {
                        // Apply Global X Padding between icons
                        ui.spacing_mut().item_spacing = egui::vec2(ABILITY_PADDING_X, ABILITY_PADDING_Y);
                        
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

                    // Global Padding between Traits and Abilities
                    ui.add_space(ABILITY_PADDING_Y);

                    render_abilities(ui, s, sprite_sheet, multihit_texture, *current_level, cat.curve.as_ref());
                }
            });
        });
}

// -------------------------------------------------------------------------------------
// ABILITY RENDERING LOGIC
// -------------------------------------------------------------------------------------

struct AbilityItem {
    icon_id: usize,
    text: String,
    custom_tex: Option<egui::TextureId>, 
}

fn render_abilities(
    ui: &mut egui::Ui, 
    s: &CatRaw, 
    sheet: &SpriteSheet, 
    multihit_tex: &Option<egui::TextureHandle>, 
    level: i32,
    curve: Option<&stats::CatLevelCurve>,
) {
    // Collect Data
    let mut grp_headline_1 = Vec::new();
    let mut grp_headline_2 = Vec::new(); 
    let mut grp_headline_3 = Vec::new();

    let mut grp_body_1 = Vec::new();
    let mut grp_body_2 = Vec::new(); 
    let mut grp_body_3 = Vec::new(); 
    let mut grp_body_4 = Vec::new(); 

    let mut grp_footer = Vec::new();

    let mut push_ab = |vec: &mut Vec<AbilityItem>, cond: bool, icon: usize, txt: String| {
        if cond { vec.push(AbilityItem { icon_id: icon, text: txt, custom_tex: None }); }
    };

    // --- HEADLINE ---
    push_ab(&mut grp_headline_1, s.attack_only > 0, definitions::ICON_ATTACK_ONLY, "Only damages Target Traits".into());
    push_ab(&mut grp_headline_1, s.strong_against > 0, definitions::ICON_STRONG_AGAINST, "Deals 1.5×~1.8× Damage to and takes 0.5×~0.4× Damage from Target Traits".into());
    push_ab(&mut grp_headline_1, s.massive_damage > 0, definitions::ICON_MASSIVE_DAMAGE, "Deals 3×~4× Damage to Target Traits".into());
    push_ab(&mut grp_headline_1, s.insane_damage > 0, definitions::ICON_INSANE_DAMAGE, "Deals 5x~6x Damage to Target Traits".into());
    push_ab(&mut grp_headline_1, s.resist > 0, definitions::ICON_RESIST, "Takes 1/4×~1/5× Damage from Target Traits".into());
    push_ab(&mut grp_headline_1, s.insanely_tough > 0, definitions::ICON_INSANELY_TOUGH, "Takes 1/6×~1/7× Damage from Target Traits".into());

    push_ab(&mut grp_headline_2, s.metal > 0, definitions::ICON_METAL, "Damage taken is reduced to 1 for Non-Critical attacks".into());
    push_ab(&mut grp_headline_2, s.base_destroyer > 0, definitions::ICON_BASE_DESTROYER, "Deals 4× Damage to the Enemy Base".into());
    push_ab(&mut grp_headline_2, s.double_bounty > 0, definitions::ICON_DOUBLE_BOUNTY, "Receives 2× Cash from Enemies".into());
    push_ab(&mut grp_headline_2, s.zombie_killer > 0, definitions::ICON_ZOMBIE_KILLER, "Prevents Zombies from reviving".into());
    push_ab(&mut grp_headline_2, s.soulstrike > 0, definitions::ICON_SOULSTRIKE, "Will attack Zombie corpses".into());
    push_ab(&mut grp_headline_2, s.wave_block > 0, definitions::ICON_WAVE_BLOCK, "When hit with a Wave Attack, nullifies its Damage and prevents its advancement".into());
    push_ab(&mut grp_headline_2, s.counter_surge > 0, definitions::ICON_COUNTER_SURGE, "When hit with a Surge Attack, create a surge of equal level and range".into());

    push_ab(&mut grp_headline_3, s.colossus_slayer > 0, definitions::ICON_COLOSSUS_SLAYER, "Deal 1.6× Damage to and take 0.7× Damage from Colossus Enemies".into());
    push_ab(&mut grp_headline_3, s.behemoth_slayer > 0, definitions::ICON_BEHEMOTH_SLAYER, "Deal 2.5× Damage to and take 0.6× Damage from Behemoth Enemies".into());
    push_ab(&mut grp_headline_3, s.sage_slayer > 0, definitions::ICON_SAGE_SLAYER, "Deal 1.2× Damage to and take 0.5× Damage from Sage Enemies".into());
    push_ab(&mut grp_headline_3, s.eva_killer > 0, definitions::ICON_EVA_KILLER, "Deal 5× Damage to and take 0.2× Damage from Eva Angels".into());
    push_ab(&mut grp_headline_3, s.witch_killer > 0, definitions::ICON_WITCH_KILLER, "Deal 5× Damage to and take 0.1× Damage from Witches".into());

    // --- BODY ---
    if s.attack_2 > 0 {
        let a1 = curve.map_or(s.attack_1, |c| c.calculate_stat(s.attack_1, level));
        let a2 = curve.map_or(s.attack_2, |c| c.calculate_stat(s.attack_2, level));
        let a3 = curve.map_or(s.attack_3, |c| c.calculate_stat(s.attack_3, level));
        
        let ab1 = if s.attack_1_abilities > 0 { "True" } else { "False" };
        let ab2 = if s.attack_2_abilities > 0 { "True" } else { "False" };
        let ab3 = if s.attack_3 > 0 {
             if s.attack_3_abilities > 0 { "/ True" } else { "/ False" }
        } else { "" };
        let dmg_str = if s.attack_3 > 0 { format!("{}/{}/{}", a1, a2, a3) } else { format!("{}/{}", a1, a2) };

        let mh_desc = format!("Deals {} Damage | Abilities affect {} / {}{}", dmg_str, ab1, ab2, ab3);
        
        if let Some(tex) = multihit_tex {
            grp_body_1.push(AbilityItem { icon_id: 0, text: mh_desc, custom_tex: Some(tex.id()) });
        }
    }

    let mut is_omni = false;
    let mut has_ld = false;
    let check_hits = [
        (s.long_distance_1_anchor, s.long_distance_1_span),
        (s.long_distance_2_anchor, if s.long_distance_2_flag == 1 { s.long_distance_2_span } else { 0 }),
        (s.long_distance_3_anchor, if s.long_distance_3_flag == 1 { s.long_distance_3_span } else { 0 }),
    ];
    let mut range_strs = Vec::new();
    for (anchor, span) in check_hits {
        if span != 0 {
            // Calculate actual Start and End based on Anchor/Span
            let start = anchor;
            let end = anchor + span;
            
            // SORT to ensure we display Lowest~Highest
            let (min, max) = if start < end { (start, end) } else { (end, start) };

            if min <= 0 { is_omni = true; } else { has_ld = true; }
            range_strs.push(format!("{}~{}", min, max));
        }
    }
    let range_desc = format!("Attacks between ranges {}", range_strs.join(" / "));
    push_ab(&mut grp_body_1, is_omni, definitions::ICON_OMNI_STRIKE, range_desc.clone());
    push_ab(&mut grp_body_1, !is_omni && has_ld, definitions::ICON_LONG_DISTANCE, range_desc);

    push_ab(&mut grp_body_2, s.conjure_unit_id > 0, definitions::ICON_CONJURE, format!("Conjures Unit {}", s.conjure_unit_id));

    let wave_type = if s.mini_wave_flag > 0 { "Mini-Wave" } else { "Wave" };
    let wave_icon = if s.mini_wave_flag > 0 { definitions::ICON_MINI_WAVE } else { definitions::ICON_WAVE };
    let wave_range = 332.5 + ((s.wave_level - 1) as f32 * 200.0);
    push_ab(&mut grp_body_2, s.wave_chance > 0, wave_icon, format!("{}% Chance to create a Level {} {} reaching {} Range", s.wave_chance, s.wave_level, wave_type, wave_range));

    let surge_type = if s.mini_surge_flag > 0 { "Mini-Surge" } else { "Surge" };
    let surge_icon = if s.mini_surge_flag > 0 { definitions::ICON_MINI_SURGE } else { definitions::ICON_SURGE };
    let s_start = s.surge_spawn_anchor;
    let s_end = s.surge_spawn_anchor + s.surge_spawn_span;
    // Sort range for surge
    let (s_min, s_max) = if s_start < s_end { (s_start, s_end) } else { (s_end, s_start) };
    let s_pos = if s_min == s_max { format!("at {}", s_min) } else { format!("between {}~{}", s_min, s_max) };
    push_ab(&mut grp_body_2, s.surge_chance > 0, surge_icon, format!("{}% Chance to create a Level {} {} {} Range", s.surge_chance, s.surge_level, surge_type, s_pos));

    let e_start = s.explosion_spawn_anchor;
    let e_end = s.explosion_spawn_anchor + s.explosion_spawn_span;
    // Sort range for explosion
    let (e_min, e_max) = if e_start < e_end { (e_start, e_end) } else { (e_end, e_start) };
    let e_pos = if e_min == e_max { format!("at {}", e_min) } else { format!("between {}~{}", e_min, e_max) };
    push_ab(&mut grp_body_2, s.explosion_chance > 0, definitions::ICON_EXPLOSION, format!("{}% Chance to create an Explosion {} Range", s.explosion_chance, e_pos));

    let savage_mult = (s.savage_blow_boost as f32 + 100.0) / 100.0;
    push_ab(&mut grp_body_2, s.savage_blow_chance > 0, definitions::ICON_SAVAGE_BLOW, format!("{}% Chance to perform a Savage Blow dealing +{}% / {:.2}× Damage", s.savage_blow_chance, s.savage_blow_boost, savage_mult));

    push_ab(&mut grp_body_2, s.critical_chance > 0, definitions::ICON_CRITICAL_HIT, format!("{}% Chance to perform a Critical Hit dealing +100% / 2× Damage, doing full damage to Metal Enemies", s.critical_chance));

    let st_mult = (s.strengthen_boost as f32 + 100.0) / 100.0;
    push_ab(&mut grp_body_2, s.strengthen_threshold > 0, definitions::ICON_STRENGTHEN, format!("At {}% or less HP, Damage dealt increases by +{}% / {:.2}×", s.strengthen_threshold, s.strengthen_boost, st_mult));
    
    push_ab(&mut grp_body_2, s.survive > 0, definitions::ICON_SURVIVE, format!("{}% Chance to Survive a lethal strike", s.survive));

    push_ab(&mut grp_body_3, s.barrier_breaker_chance > 0, definitions::ICON_BARRIER_BREAKER, format!("{}% Chance to break enemy Barriers", s.barrier_breaker_chance));
    push_ab(&mut grp_body_3, s.shield_pierce_chance > 0, definitions::ICON_SHIELD_PEIRCER, format!("{}% Chance to pierce enemy Shields", s.shield_pierce_chance));
    push_ab(&mut grp_body_3, s.metal_killer_percent > 0, definitions::ICON_METAL_KILLER, format!("Deals {}% of a Metal Enemies current HP upon hit", s.metal_killer_percent));

    let f_to_s = |f: i32| format!("{:.2}s^{}f", f as f32 / 30.0, f);
    push_ab(&mut grp_body_4, s.dodge_chance > 0, definitions::ICON_DODGE, format!("{}% Chance to Dodge Target Traits for {}", s.dodge_chance, f_to_s(s.dodge_duration)));
    push_ab(&mut grp_body_4, s.weaken_chance > 0, definitions::ICON_WEAKEN, format!("{}% Chance to weaken Target Traits to {}% Attack Power for {}", s.weaken_chance, s.weaken_to, f_to_s(s.weaken_duration)));
    push_ab(&mut grp_body_4, s.freeze_chance > 0, definitions::ICON_FREEZE, format!("{}% Chance to Freeze Target Traits for {}", s.freeze_chance, f_to_s(s.freeze_duration)));
    push_ab(&mut grp_body_4, s.slow_chance > 0, definitions::ICON_SLOW, format!("{}% Chance to Slow Target Traits for {}", s.slow_chance, f_to_s(s.slow_duration)));
    push_ab(&mut grp_body_4, s.knockback_chance > 0, definitions::ICON_KNOCKBACK, format!("{}% Chance to Knockback Target Traits", s.knockback_chance));
    push_ab(&mut grp_body_4, s.curse_chance > 0, definitions::ICON_CURSE, format!("{}% Chance to Curse Target Traits for {}", s.curse_chance, f_to_s(s.curse_duration)));
    push_ab(&mut grp_body_4, s.warp_chance > 0, definitions::ICON_WARP, format!("{}% Chance to Warp Target Traits for {} range {}~{}", s.warp_chance, f_to_s(s.warp_duration), s.warp_distance_minimum, s.warp_distance_maximum));

    // --- FOOTER ---
    let immunities = [
        (s.wave_immune > 0, definitions::ICON_IMMUNE_WAVE, "Immune to Wave Attacks"),
        (s.surge_immune > 0, definitions::ICON_IMMUNE_SURGE, "Immune to Surge Attacks"),
        (s.explosion_immune > 0, definitions::ICON_IMMUNE_EXPLOSION, "Immune to Explosions"),
        (s.weaken_immune > 0, definitions::ICON_IMMUNE_WEAKEN, "Immune to Weaken"),
        (s.freeze_immune > 0, definitions::ICON_IMMUNE_FREEZE, "Immune to Freeze"),
        (s.slow_immune > 0, definitions::ICON_IMMUNE_SLOW, "Immune to Slow"),
        (s.knockback_immune > 0, definitions::ICON_IMMUNE_KNOCKBACK, "Immune to Knockback"),
        (s.curse_immune > 0, definitions::ICON_IMMUNE_CURSE, "Immune to Curse"),
        (s.toxic_immune > 0, definitions::ICON_IMMUNE_TOXIC, "Immune to Toxic"),
        (s.warp_immune > 0, definitions::ICON_IMMUNE_WARP, "Immune to Warp"),
    ];
    for (has, icon, txt) in immunities {
        push_ab(&mut grp_footer, has, icon, txt.into());
    }

    // --- RENDER ---
    
    // Helper for Icon-Only Rows (Headline/Footer)
    let render_icon_row = |ui: &mut egui::Ui, items: &Vec<AbilityItem>| {
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing = egui::vec2(ABILITY_PADDING_X, ABILITY_PADDING_Y);
            for item in items {
                let r = if let Some(tex_id) = item.custom_tex {
                    ui.add(egui::Image::new((tex_id, egui::vec2(stats::ICON_SIZE, stats::ICON_SIZE))))
                } else if let Some(sprite) = sheet.get_sprite_by_line(item.icon_id) {
                    ui.add(sprite.fit_to_exact_size(egui::vec2(stats::ICON_SIZE, stats::ICON_SIZE)))
                } else { continue; };
                r.on_hover_ui(|ui| { ui.label(&item.text); });
            }
        });
    };

    // Helper for List View (Body)
    let render_detailed_list = |ui: &mut egui::Ui, items: &Vec<AbilityItem>| {
        for item in items {
            ui.horizontal(|ui| {
                // Apply X Padding specifically between Icon and Text
                ui.spacing_mut().item_spacing.x = ABILITY_PADDING_X;
                
                 if let Some(tex_id) = item.custom_tex {
                    ui.add(egui::Image::new((tex_id, egui::vec2(stats::ICON_SIZE, stats::ICON_SIZE))));
                } else if let Some(sprite) = sheet.get_sprite_by_line(item.icon_id) {
                    ui.add(sprite.fit_to_exact_size(egui::vec2(stats::ICON_SIZE, stats::ICON_SIZE)));
                }
                
                // Use helper to render text with superscript parsing
                text_with_superscript(ui, &item.text);
            });
            // Apply Y Padding between list items
            ui.add_space(ABILITY_PADDING_Y);
        }
    };

    // Headline: Rows
    if !grp_headline_1.is_empty() { render_icon_row(ui, &grp_headline_1); }
    if !grp_headline_2.is_empty() { ui.add_space(ABILITY_PADDING_Y); render_icon_row(ui, &grp_headline_2); }
    if !grp_headline_3.is_empty() { ui.add_space(ABILITY_PADDING_Y); render_icon_row(ui, &grp_headline_3); }

    // Spacer between Headline and Body
    ui.add_space(ABILITY_PADDING_Y * 2.0);

    // Body: Detailed List
    render_detailed_list(ui, &grp_body_1);
    render_detailed_list(ui, &grp_body_2);
    render_detailed_list(ui, &grp_body_3);
    render_detailed_list(ui, &grp_body_4);

    // Spacer between Body and Footer
    ui.add_space(ABILITY_PADDING_Y);

    // Footer: Row
    if !grp_footer.is_empty() { render_icon_row(ui, &grp_footer); }
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

// Parses "Normal Text^Small Text" and renders accordingly
fn text_with_superscript(ui: &mut egui::Ui, text: &str) {
    if text.contains('^') {
        let parts: Vec<&str> = text.split('^').collect();
        if parts.len() >= 2 {
            let body_font = ui.style().text_styles.get(&egui::TextStyle::Body).cloned().unwrap_or(egui::FontId::proportional(14.0));
            let mut job = egui::text::LayoutJob::default();

            // Normal Part
            job.append(parts[0], 0.0, egui::TextFormat {
                font_id: body_font.clone(),
                color: ui.visuals().text_color(),
                ..Default::default()
            });

            // Superscript Part
            job.append(parts[1], 0.0, egui::TextFormat {
                font_id: egui::FontId::proportional(body_font.size * 0.70), // Smaller font
                color: ui.visuals().text_color(),
                valign: egui::Align::Min, // Raise text slightly (or Center if preferred)
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