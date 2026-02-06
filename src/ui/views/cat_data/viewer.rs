use eframe::egui;
use std::path::{Path, PathBuf};

use crate::core::cat::scanner::CatEntry;
use crate::data::global::imgcut::SpriteSheet;
use crate::data::global::mamodel::Model;
use crate::ui::components::anim_viewer::AnimViewer;
use crate::core::settings::Settings;
use crate::paths::cat::{self, AnimType};

const IDX_WALK: usize = 0;
const IDX_IDLE: usize = 1;
const IDX_ATTACK: usize = 2;
const IDX_KB: usize = 3;
const IDX_SPIRIT: usize = 4;
const IDX_MODEL: usize = 99;

// --- LAYOUT CONSTANTS ---
const TILE_HEIGHT: f32 = 28.0; 
const GAP: f32 = 4.0;

// Column 1: Play/Orient
const ICON_W: f32 = 60.0;

// Column 2: Frame Controls (Calculated for seamless alignment)
const COL2_W: f32 = 148.0; 
const NAV_W: f32 = 30.0;
const INPUT_W: f32 = 80.0; 
const RANGE_INPUT_W: f32 = 60.0; 
const SEP_W: f32 = 20.0;

// Column 3: Export/Speed
const COL3_W: f32 = 80.0;

pub fn show(
    ui: &mut egui::Ui,
    ctx: &egui::Context,
    cat_entry: &CatEntry,
    current_form: usize,
    anim_viewer: &mut AnimViewer,
    model_data: &mut Option<Model>,
    anim_sheet: &mut SpriteSheet,
    settings: &Settings,
) {
    let root = Path::new(cat::DIR_CATS);
    let egg_ids = cat_entry.egg_ids;
    
    let form_char = match current_form { 0 => 'f', 1 => 'c', 2 => 's', _ => 'u' };
    let id_str = format!("{:03}", cat_entry.id);
    let form_viewer_id = format!("{}_{}", id_str, form_char);

    // Spirit Logic
    let conjure_id = if let Some(Some(stats)) = cat_entry.stats.get(current_form) {
        if stats.conjure_unit_id > 0 { Some(stats.conjure_unit_id as u32) } else { None }
    } else { None };

    let mut spirit_pack = None;
    let mut spirit_available = false;
    let spirit_sheet_id = if let Some(s_id) = conjure_id { format!("spirit_{}", s_id) } else { String::new() };

    if let Some(s_id) = conjure_id {
        let s_png = cat::anim(root, s_id, 0, (-1, -1), AnimType::Png);
        let s_cut = cat::anim(root, s_id, 0, (-1, -1), AnimType::Imgcut);
        let s_model = cat::anim(root, s_id, 0, (-1, -1), AnimType::Mamodel);
        let s_atk = cat::maanim(root, s_id, 0, (-1, -1), 2); 

        if s_png.exists() && s_cut.exists() && s_model.exists() && s_atk.exists() {
            spirit_pack = Some((s_png, s_cut, s_model, s_atk));
            spirit_available = true;
        }
    }

    // Animation List
    let mut available_anims = Vec::new();
    let anim_defs = [(IDX_WALK, "Walk"), (IDX_IDLE, "Idle"), (IDX_ATTACK, "Attack"), (IDX_KB, "Knockback")];
    for (idx, label) in anim_defs {
        let path = cat::maanim(root, cat_entry.id, current_form, egg_ids, idx);
        if path.exists() { available_anims.push((idx, label, path)); }
    }

    let std_png = cat::anim(root, cat_entry.id, current_form, egg_ids, AnimType::Png);
    let std_cut = cat::anim(root, cat_entry.id, current_form, egg_ids, AnimType::Imgcut);
    let std_model = cat::anim(root, cat_entry.id, current_form, egg_ids, AnimType::Mamodel);
    let base_assets_available = std_png.exists() && std_cut.exists() && std_model.exists();

    // 1. CALCULATE STATE
    let target_viewer_id = if anim_viewer.loaded_anim_index == IDX_SPIRIT {
        spirit_sheet_id.clone()
    } else {
        form_viewer_id.clone()
    };

    let is_stable = anim_viewer.loaded_id == target_viewer_id;
    let is_loading_new = !is_stable && (anim_viewer.staging_model.is_some() || anim_viewer.staging_sheet.is_some());
    let is_first_launch = anim_viewer.held_model.is_none() && model_data.is_none();
    let mut just_swapped = false;

    if is_stable {
        if let Some(m) = model_data {
            anim_viewer.held_model = Some(m.clone());
        }
        anim_viewer.held_sheet = Some((*anim_sheet).clone());
    }

    // A. Start Transition
    if !is_stable && !is_loading_new && !is_first_launch {
        let mut valid_idx = anim_viewer.loaded_anim_index;
        if valid_idx == IDX_SPIRIT && !spirit_available { valid_idx = IDX_WALK; }
        else if valid_idx == IDX_MODEL && !base_assets_available { valid_idx = IDX_WALK; }
        
        let (resolved_png, resolved_cut, resolved_model, _) = resolve_paths(valid_idx, &std_png, &std_cut, &std_model, &spirit_pack, &available_anims);
        
        if let (Some(png), Some(cut)) = (resolved_png, resolved_cut) {
            let mut new_sheet = SpriteSheet::default();
            new_sheet.load(ctx, png, cut, target_viewer_id.clone());
            anim_viewer.staging_sheet = Some(new_sheet);
        }

        if let Some(model_path) = resolved_model {
            if let Some(loaded_model) = Model::load(model_path) {
                anim_viewer.staging_model = Some(loaded_model);
            }
        }
    }

    // B. First Launch
    if is_first_launch {
        let mut valid_idx = anim_viewer.loaded_anim_index;
        if valid_idx == IDX_SPIRIT && !spirit_available { valid_idx = IDX_WALK; }
        else if valid_idx == IDX_MODEL && !base_assets_available { valid_idx = IDX_WALK; }
        
        let (resolved_png, resolved_cut, resolved_model, resolved_anim) = resolve_paths(valid_idx, &std_png, &std_cut, &std_model, &spirit_pack, &available_anims);

        if let (Some(png), Some(cut), Some(model_path)) = (resolved_png, resolved_cut, resolved_model) {
             anim_sheet.image_data = None; 
             anim_sheet.load(ctx, png, cut, target_viewer_id.clone());
             if let Some(loaded_model) = Model::load(model_path) {
                 anim_viewer.held_model = Some(loaded_model.clone());
                 anim_viewer.held_sheet = Some((*anim_sheet).clone());
                 *model_data = Some(loaded_model);
             }
        }
        
        if let Some(anim_path) = resolved_anim { 
            anim_viewer.load_anim(anim_path); 
        } else { 
            anim_viewer.current_anim = None; 
        }
        
        anim_viewer.loaded_id = target_viewer_id.clone();
        anim_viewer.pending_initial_center = true; 
    }

    // C. Completion
    if is_loading_new {
        if let Some(staging_sheet) = &mut anim_viewer.staging_sheet {
            staging_sheet.update(ctx);

            let texture_is_ready = staging_sheet.sheet_name == target_viewer_id 
                                && !staging_sheet.is_loading_active 
                                && staging_sheet.image_data.is_some();

            if texture_is_ready {
                if let (Some(new_model), Some(new_sheet)) = (anim_viewer.staging_model.take(), anim_viewer.staging_sheet.take()) {
                    anim_viewer.held_model = Some(new_model.clone());
                    anim_viewer.held_sheet = Some(new_sheet.clone());
                    *model_data = Some(new_model);
                    *anim_sheet = new_sheet; 
                    anim_viewer.loaded_id = target_viewer_id.clone();
                    
                    let mut valid_idx = anim_viewer.loaded_anim_index;
                    if valid_idx == IDX_SPIRIT && !spirit_available { valid_idx = IDX_WALK; }
                    else if valid_idx == IDX_MODEL && !base_assets_available { valid_idx = IDX_WALK; }
                    
                    let (_, _, _, resolved_anim) = resolve_paths(valid_idx, &std_png, &std_cut, &std_model, &spirit_pack, &available_anims);
                    
                    if let Some(anim_path) = resolved_anim { 
                        anim_viewer.load_anim(anim_path); 
                    } else { 
                        anim_viewer.current_anim = None; 
                    }
                    
                    anim_viewer.pending_initial_center = true;
                    just_swapped = true;
                    ctx.request_repaint();
                }
            }
        }
    } else {
        anim_sheet.update(ctx);
    }

    // =========================================================
    // 3. UI RENDER
    // =========================================================
    let mut clicked_index: Option<usize> = None;

    ui.vertical(|ui| {
        ui.add_space(3.0);
        
        // ROW 1: ANIMATION BUTTONS
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 5.0;
            let active_color = egui::Color32::from_rgb(31, 106, 165);
            let inactive_color = egui::Color32::from_gray(60);
            let btn_size = egui::vec2(60.0, 25.0);

            for (idx, label, _) in &available_anims {
                let is_active = anim_viewer.loaded_anim_index == *idx;
                if ui.add_sized(btn_size, egui::Button::new(egui::RichText::new(*label).color(egui::Color32::WHITE).size(13.0)).fill(if is_active { active_color } else { inactive_color })).clicked() { clicked_index = Some(*idx); }
            }
            if spirit_available {
                if ui.add_sized(btn_size, egui::Button::new(egui::RichText::new("Spirit").color(egui::Color32::WHITE).size(13.0)).fill(if anim_viewer.loaded_anim_index == IDX_SPIRIT { active_color } else { inactive_color })).clicked() { clicked_index = Some(IDX_SPIRIT); }
            }
            if base_assets_available {
                if ui.add_sized(btn_size, egui::Button::new(egui::RichText::new("Model").color(egui::Color32::WHITE).size(13.0)).fill(if anim_viewer.loaded_anim_index == IDX_MODEL { active_color } else { inactive_color })).clicked() { clicked_index = Some(IDX_MODEL); }
            }
        });

        if let Some(target_idx) = clicked_index {
            if !is_loading_new {
                anim_viewer.loaded_anim_index = target_idx;
                let intended_target_id = if target_idx == IDX_SPIRIT { spirit_sheet_id.clone() } else { form_viewer_id.clone() };
                
                if anim_viewer.loaded_id == intended_target_id {
                    let (_, _, _, t_anim) = resolve_paths(target_idx, &std_png, &std_cut, &std_model, &spirit_pack, &available_anims);
                    if let Some(a_path) = t_anim {
                        anim_viewer.load_anim(a_path);
                    } else if target_idx == IDX_MODEL {
                        anim_viewer.current_anim = None;
                    }
                }
            }
        }

        ui.add_space(5.0);
        
        // --- SNAPSHOT STATE (PRE-BORROW) ---
        let lcm_max = if let Some(anim) = &anim_viewer.current_anim {
            if anim_viewer.loaded_anim_index <= 1 { anim.calculate_true_loop() } else { anim.max_frame }
        } else { 0 };
        let max_frame = lcm_max;

        let cur_frame_val = anim_viewer.current_frame;
        let loop_range_0 = anim_viewer.loop_range.0;
        let loop_range_1 = anim_viewer.loop_range.1;
        let is_playing = anim_viewer.is_playing;
        let is_model_mode = anim_viewer.loaded_anim_index == IDX_MODEL;

        let cur_int = (cur_frame_val + 0.01).floor() as i32;
        let effective_max = loop_range_1.unwrap_or(max_frame);
        let display_max = if is_model_mode { 0 } else { effective_max };
        let display_cur = if cur_int > display_max { display_max } else { cur_int };

        // Tile Frame Style
        let tile_frame = egui::Frame::none()
            .fill(egui::Color32::from_gray(40))
            .rounding(4.0)
            .stroke(egui::Stroke::new(1.0, egui::Color32::from_gray(60)))
            .inner_margin(0.0);

        // --- CONTROLS AREA ---
        ui.horizontal(|ui| {
            ui.style_mut().spacing.item_spacing.x = GAP;

            // COLUMN 1: Play / Orient
            ui.vertical(|ui| {
                let play_icon = if is_playing { "⏸" } else { "▶" };
                if ui.add_sized(egui::vec2(ICON_W, TILE_HEIGHT), egui::Button::new(egui::RichText::new(play_icon).size(16.0))).clicked() {
                    anim_viewer.is_playing = !anim_viewer.is_playing;
                }
                
                ui.add_space(GAP);
                
                if ui.add_sized(egui::vec2(ICON_W, TILE_HEIGHT), egui::Button::new("Orient")).clicked() { 
                    anim_viewer.pan_offset = egui::Vec2::ZERO; 
                }
            });

            ui.add(egui::Separator::default().vertical());

            // COLUMN 2: Frame Controls / Info
            ui.vertical(|ui| {
                // ROW 1: Controls
                ui.allocate_ui(egui::vec2(COL2_W, TILE_HEIGHT), |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = GAP;
                        
                        if !is_playing {
                            // [◀]
                            let left_btn = ui.add_sized(egui::vec2(NAV_W, TILE_HEIGHT), egui::Button::new("◀").sense(egui::Sense::click().union(egui::Sense::drag())));
                            
                            // [F] (Single Frame Input - Fully Transparent & Centered)
                            tile_frame.show(ui, |ui| {
                                ui.set_width(INPUT_W);
                                ui.set_height(TILE_HEIGHT);
                                
                                ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                                    // Make text edit transparent
                                    ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                                    ui.style_mut().visuals.widgets.active.bg_fill = egui::Color32::TRANSPARENT;
                                    ui.style_mut().visuals.widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
                                    
                                    // Sync buffer if not editing
                                    let re = ui.add(egui::TextEdit::singleline(&mut anim_viewer.single_frame_str)
                                        .frame(false)
                                        .desired_width(INPUT_W)
                                        .vertical_align(egui::Align::Center)
                                        .horizontal_align(egui::Align::Center));
                                    
                                    // Update frame on change
                                    if re.changed() {
                                        if let Ok(val) = anim_viewer.single_frame_str.parse::<i32>() {
                                            anim_viewer.current_frame = val.clamp(0, max_frame) as f32;
                                        }
                                    }
                                    // Update buffer from logic if not focused
                                    if !re.has_focus() {
                                        anim_viewer.single_frame_str = format!("{}", cur_int);
                                    }
                                });
                            });

                            // [▶]
                            let right_btn = ui.add_sized(egui::vec2(NAV_W, TILE_HEIGHT), egui::Button::new("▶").sense(egui::Sense::click().union(egui::Sense::drag())));

                            // Hold Logic
                            if left_btn.is_pointer_button_down_on() { anim_viewer.hold_dir = -1; } 
                            else if right_btn.is_pointer_button_down_on() { anim_viewer.hold_dir = 1; } 
                            else { anim_viewer.hold_dir = 0; }
                            
                            if left_btn.clicked() {
                                let f = cur_frame_val - 1.0;
                                anim_viewer.current_frame = if f < 0.0 { max_frame as f32 } else { f };
                            }
                            if right_btn.clicked() {
                                let f = cur_frame_val + 1.0;
                                anim_viewer.current_frame = if f > max_frame as f32 { 0.0 } else { f };
                            }

                        } else {
                            // [F] ~ [F] (Ghost Text Mode)
                            
                            // Start Bound
                            tile_frame.show(ui, |ui| {
                                ui.set_width(60.0);
                                ui.set_height(TILE_HEIGHT);
                                
                                ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                                    ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                                    ui.style_mut().visuals.widgets.active.bg_fill = egui::Color32::TRANSPARENT;
                                    ui.style_mut().visuals.widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
                                    
                                    if loop_range_0.is_some() && anim_viewer.range_str_cache.0.is_empty() {
                                        anim_viewer.range_str_cache.0 = loop_range_0.unwrap().to_string();
                                    }
                                    
                                    let re = ui.add(egui::TextEdit::singleline(&mut anim_viewer.range_str_cache.0)
                                        .hint_text(egui::RichText::new("0").color(egui::Color32::GRAY))
                                        .frame(false)
                                        .desired_width(60.0)
                                        .vertical_align(egui::Align::Center)
                                        .horizontal_align(egui::Align::Center));
                                        
                                    if re.changed() {
                                        if anim_viewer.range_str_cache.0.is_empty() {
                                            anim_viewer.loop_range.0 = None;
                                        } else if let Ok(val) = anim_viewer.range_str_cache.0.parse::<i32>() {
                                            let clamped = val.clamp(0, max_frame);
                                            anim_viewer.loop_range.0 = Some(clamped);
                                            if cur_frame_val < clamped as f32 { anim_viewer.current_frame = clamped as f32; }
                                        }
                                    }
                                    if re.secondary_clicked() {
                                        anim_viewer.loop_range.0 = None;
                                        anim_viewer.range_str_cache.0.clear();
                                    }
                                });
                            });
                            
                            // Separator
                            tile_frame.show(ui, |ui| {
                                ui.set_width(20.0);
                                ui.set_height(TILE_HEIGHT);
                                ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                                    ui.label("~"); 
                                });
                            });
                            
                            // End Bound
                            tile_frame.show(ui, |ui| {
                                ui.set_width(60.0);
                                ui.set_height(TILE_HEIGHT);
                                ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                                    ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                                    ui.style_mut().visuals.widgets.active.bg_fill = egui::Color32::TRANSPARENT;
                                    ui.style_mut().visuals.widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
                                    
                                    if loop_range_1.is_some() && anim_viewer.range_str_cache.1.is_empty() {
                                        anim_viewer.range_str_cache.1 = loop_range_1.unwrap().to_string();
                                    }
                                    
                                    let hint = egui::RichText::new(format!("{}", max_frame)).color(egui::Color32::GRAY);
                                    let re = ui.add(egui::TextEdit::singleline(&mut anim_viewer.range_str_cache.1)
                                        .hint_text(hint)
                                        .frame(false)
                                        .desired_width(60.0)
                                        .vertical_align(egui::Align::Center)
                                        .horizontal_align(egui::Align::Center));
                                        
                                    if re.changed() {
                                        if anim_viewer.range_str_cache.1.is_empty() {
                                            anim_viewer.loop_range.1 = None;
                                        } else if let Ok(val) = anim_viewer.range_str_cache.1.parse::<i32>() {
                                            let start = loop_range_0.unwrap_or(0);
                                            let clamped = val.clamp(start, max_frame.max(1));
                                            anim_viewer.loop_range.1 = Some(clamped);
                                            if cur_frame_val > clamped as f32 { anim_viewer.current_frame = start as f32; }
                                        }
                                    }
                                    if re.secondary_clicked() {
                                        anim_viewer.loop_range.1 = None;
                                        anim_viewer.range_str_cache.1.clear();
                                    }
                                });
                            });
                        }
                    });
                });

                ui.add_space(GAP);

                // ROW 2: Info
                ui.allocate_ui(egui::vec2(COL2_W, TILE_HEIGHT), |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = GAP;
                        
                        tile_frame.show(ui, |ui| {
                            ui.set_width(60.0);
                            ui.set_height(TILE_HEIGHT);
                            ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                                ui.label(egui::RichText::new(format!("{}", display_cur)).color(egui::Color32::WHITE)); 
                            });
                        });
                        tile_frame.show(ui, |ui| {
                            ui.set_width(20.0);
                            ui.set_height(TILE_HEIGHT);
                            ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                                ui.label("/"); 
                            });
                        });
                        tile_frame.show(ui, |ui| {
                            ui.set_width(60.0);
                            ui.set_height(TILE_HEIGHT);
                            ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                                ui.label(egui::RichText::new(format!("{}", display_max)).color(egui::Color32::WHITE)); 
                            });
                        });
                    });
                });
            });

            ui.add(egui::Separator::default().vertical());

            // COLUMN 3: Export / Speed
            ui.vertical(|ui| {
                // Export Button
                let id = ui.make_persistent_id("export_popup");
                let btn_resp = ui.add_sized(egui::vec2(COL3_W, TILE_HEIGHT), egui::Button::new("Export"));
                if btn_resp.clicked() { ui.memory_mut(|mem| mem.open_popup(id)); }
                
                if ui.memory(|mem| mem.is_popup_open(id)) {
                    egui::popup_below_widget(ui, id, &btn_resp, egui::PopupCloseBehavior::CloseOnClickOutside, |ui: &mut egui::Ui| {
                        ui.label("Export\noptions\ncoming\nsoon!");
                        if ui.button("Close").clicked() { ui.memory_mut(|mem| mem.close_popup()); }
                    });
                }

                ui.add_space(GAP);

                // Speed Input Tile (Number Only - Transparent & Centered)
                tile_frame.show(ui, |ui| {
                    ui.set_width(COL3_W);
                    ui.set_height(TILE_HEIGHT);
                    ui.with_layout(egui::Layout::centered_and_justified(egui::Direction::LeftToRight), |ui| {
                        ui.style_mut().visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
                        ui.style_mut().visuals.widgets.active.bg_fill = egui::Color32::TRANSPARENT;
                        ui.style_mut().visuals.widgets.hovered.bg_fill = egui::Color32::TRANSPARENT;
                        
                        let re = ui.add(egui::TextEdit::singleline(&mut anim_viewer.speed_str)
                            .hint_text(egui::RichText::new("1.0").color(egui::Color32::GRAY))
                            .frame(false)
                            .desired_width(COL3_W)
                            .vertical_align(egui::Align::Center)
                            .horizontal_align(egui::Align::Center));
                            
                        if re.changed() {
                            if anim_viewer.speed_str.is_empty() {
                                anim_viewer.playback_speed = 1.0;
                            } else if let Ok(val) = anim_viewer.speed_str.parse::<f32>() {
                                anim_viewer.playback_speed = val.clamp(0.1, 10.0);
                            }
                        }
                        // Default ghosting when empty and lost focus
                        if !re.has_focus() && anim_viewer.speed_str.is_empty() {
                            anim_viewer.playback_speed = 1.0;
                        }
                    });
                });
            });
        });

        ui.add_space(5.0);

        // RENDER AREA
        let (rect, _response) = ui.allocate_exact_size(ui.available_size(), egui::Sense::hover());
        
        ui.put(rect, |ui: &mut egui::Ui| {
            if let (Some(model_to_draw), Some(sheet_to_draw)) = (anim_viewer.held_model.clone(), anim_viewer.held_sheet.clone()) {
                let allow_texture_update = !is_loading_new || just_swapped;
                anim_viewer.render(
                    ui, &sheet_to_draw, &model_to_draw,
                    settings.animation_interpolation, settings.animation_debug, settings.centering_behavior,
                    allow_texture_update
                );
            }
            ui.allocate_rect(rect, egui::Sense::hover())
        });

        let border_rect = rect.shrink(2.0);
        let border_color = egui::Color32::from_rgb(31, 106, 165); 
        ui.painter().rect_stroke(border_rect, egui::Rounding::same(5.0), egui::Stroke::new(4.0, border_color));
    });
}

fn resolve_paths<'a>(
    idx: usize,
    png_path_base: &'a PathBuf,
    cut_path_base: &'a PathBuf,
    model_path_base: &'a PathBuf,
    spirit_pack: &'a Option<(PathBuf, PathBuf, PathBuf, PathBuf)>,
    anims: &'a Vec<(usize, &str, PathBuf)>
) -> (Option<&'a PathBuf>, Option<&'a PathBuf>, Option<&'a PathBuf>, Option<&'a PathBuf>) {
    
    if idx == IDX_SPIRIT {
        if let Some((s_png, s_cut, s_model, s_anim)) = spirit_pack {
            return (Some(s_png), Some(s_cut), Some(s_model), Some(s_anim));
        }
    } else {
        let anim_path = anims.iter().find(|(i, _, _)| *i == idx).map(|(_, _, p)| p);
        return (Some(png_path_base), Some(cut_path_base), Some(model_path_base), anim_path);
    }
    (None, None, None, None)
}