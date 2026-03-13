use std::fs;
use std::path::Path;
use eframe::egui;
use crate::features::settings::logic::state::{GameDataSettings, RuntimeState};
use crate::features::settings::logic::delete::FolderDeleter;
use crate::global::ui::shared::DragGuard;
use super::tabs::toggle_ui;

const COL_PREFIX_WIDTH: f32 = 120.0;
const COL_SUFFIX_WIDTH: f32 = 120.0;
const COL_EXT_WIDTH: f32 = 80.0;
const COL_LOGIC_WIDTH: f32 = 90.0;
const COL_LANG_WIDTH: f32 = 100.0;
const COL_ACTION_WIDTH: f32 = 60.0;
const WINDOW_MAX_HEIGHT: f32 = 600.0;
// -----------------------------



#[derive(Clone, PartialEq, Default)]
enum NameLogic {
    #[default]
    Contains,
    Only,
}

#[derive(Clone, PartialEq, Default)]
enum LangLogic {
    #[default]
    Append,
    Only,
}

#[derive(Clone)]
struct ExceptionRule {
    prefix: String,
    suffix: String,
    extension: String,
    name_logic: NameLogic,
    languages: std::collections::BTreeMap<String, bool>, 
    lang_logic: LangLogic,
}

impl Default for ExceptionRule {
    fn default() -> Self {
        let mut languages = std::collections::BTreeMap::new();
        // Combined GLOBAL_CODES and REGION_CODES (de-duplicated)
        let all_langs = ["de", "en", "es", "fr", "it", "jp", "kr", "th", "tw"];
        for lang in all_langs {
            languages.insert(lang.to_string(), false);
        }
        Self {
            prefix: String::new(),
            suffix: String::new(),
            extension: String::new(),
            name_logic: NameLogic::Contains,
            languages,
            lang_logic: LangLogic::Append,
        }
    }
}

#[derive(Clone, Default)]
struct ManageExceptionsState {
    is_open: bool,
    rules: Vec<ExceptionRule>,
}

// --- EXISTING MODAL LOGIC ---

#[derive(Clone, Default)]
struct FolderDeleteState {
    is_open: bool,
    size_str: Option<String>,
}

fn get_folder_size(path: &Path) -> u64 {
    let mut size = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            if let Ok(metadata) = entry.metadata() {
                if metadata.is_dir() {
                    size += get_folder_size(&entry.path());
                } else {
                    size += metadata.len();
                }
            }
        }
    }
    size
}

fn format_size(size: u64) -> String {
    let kb = 1024.0;
    let mb = kb * 1024.0;
    let gb = mb * 1024.0;
    let size_f = size as f64;

    if size_f >= gb {
        format!("{:.2} GB", size_f / gb)
    } else if size_f >= mb {
        format!("{:.2} MB", size_f / mb)
    } else if size_f >= kb {
        format!("{:.2} KB", size_f / kb)
    } else {
        format!("{} B", size)
    }
}

fn show_folder_delete_modal(
    ctx: &egui::Context,
    drag_guard: &mut DragGuard,
    id_str: &str,
    content: &str,
) -> bool {
    let state_id = egui::Id::new(id_str);
    let mut state = ctx.data(|d| d.get_temp::<FolderDeleteState>(state_id)).unwrap_or_default();
    let mut yes_clicked = false;

    if state.is_open {
        let window_id = egui::Id::new(format!("{}_window", id_str));
        let (allow_drag, fixed_pos) = drag_guard.assign_bounds(ctx, window_id);
        let mut should_close = false;

        let mut window = egui::Window::new("Confirm Deletion")
            .id(window_id)
            .collapsible(false)
            .resizable(false)
            .constrain(false)
            .movable(allow_drag) 
            .default_pos(ctx.screen_rect().center() - egui::vec2(150.0, 50.0));
            
        if let Some(pos) = fixed_pos { window = window.current_pos(pos); }
            
        window.show(ctx, |ui| {
            ui.set_min_width(280.0);
            ui.vertical_centered(|ui| {
                ui.add_space(5.0);
                ui.label(content); 
                
                if let Some(size) = &state.size_str {
                    ui.add_space(5.0);
                    ui.label(egui::RichText::new(format!("\"raw\" folder size: {}", size)).color(ui.visuals().weak_text_color()));
                }

                ui.add_space(15.0);
                
                ui.horizontal(|ui| {
                    let total_width = 130.0;
                    let x_offset = (ui.available_width() - total_width) / 2.0;
                    ui.add_space(x_offset);

                    if ui.add_sized([60.0, 30.0], egui::Button::new("Yes")).clicked() {
                        yes_clicked = true;
                        should_close = true;
                    }
                    
                    ui.add_space(10.0);

                    if ui.add_sized([60.0, 30.0], egui::Button::new("No")).clicked() {
                        should_close = true;
                    }
                });
                ui.add_space(5.0);
            });
        });
        
        if should_close {
            state.is_open = false;
        }
        
        ctx.data_mut(|d| d.insert_temp(state_id, state));
    }
    
    yes_clicked
}

// --- NEW EXCEPTIONS MODAL LOGIC ---

// --- NEW EXCEPTIONS MODAL LOGIC ---

fn show_manage_exceptions_modal(ctx: &egui::Context, drag_guard: &mut DragGuard) {
    let state_id = egui::Id::new("manage_exceptions_state");
    let mut state = ctx.data(|d| d.get_temp::<ManageExceptionsState>(state_id)).unwrap_or_else(|| {
        ManageExceptionsState { 
            is_open: false, 
            rules: vec![ExceptionRule::default()] 
        }
    });

    let mut is_open = state.is_open;

    if is_open {
        let window_id = egui::Id::new("manage_exceptions_window");
        let (allow_drag, fixed_pos) = drag_guard.assign_bounds(ctx, window_id);

        let mut window = egui::Window::new("Manage Exceptions")
            .id(window_id)
            .open(&mut is_open) 
            .collapsible(false)
            .resizable(false) // Shrink-wraps to the grid
            .constrain(false)
            .movable(allow_drag)
            .default_pos(ctx.screen_rect().center() - egui::vec2(375.0, 200.0));

        if let Some(pos) = fixed_pos { window = window.current_pos(pos); }

        window.show(ctx, |ui| {
            ui.add_space(10.0);

            // Centered Toolbar
            let btn_h = 24.0;
            let btn_w = 120.0;
            let default_color = egui::Color32::from_rgb(31, 106, 165);

            ui.vertical_centered(|ui| {
                ui.horizontal(|ui| {
                    let spacing = ui.spacing().item_spacing.x;
                    let total_width = (btn_w * 3.0) + (spacing * 2.0); 
                    let x_offset = (ui.available_width() - total_width) / 2.0;
                    ui.add_space(x_offset.max(0.0));

                    let add_btn = egui::Button::new(egui::RichText::new("Add Rule").size(12.0).strong().color(egui::Color32::WHITE))
                        .fill(default_color)
                        .rounding(4.0);
                    if ui.add_sized([btn_w, btn_h], add_btn).clicked() {
                        state.rules.push(ExceptionRule::default());
                    }

                    let reset_btn = egui::Button::new(egui::RichText::new("Reset to Default").size(12.0).strong().color(egui::Color32::WHITE))
                        .fill(default_color)
                        .rounding(4.0);
                    if ui.add_sized([btn_w, btn_h], reset_btn).clicked() {
                        state.rules.clear();
                        state.rules.push(ExceptionRule::default());
                    }

                    let export_btn = egui::Button::new(egui::RichText::new("Export List").size(12.0).strong().color(egui::Color32::WHITE))
                        .fill(default_color)
                        .rounding(4.0);
                    if ui.add_sized([btn_w, btn_h], export_btn).clicked() {
                        // Placeholder for RFD file save dialogue
                    }
                });
            });

            ui.add_space(15.0);
            ui.separator();
            ui.add_space(5.0);

            // Table Header & Contents
            egui::ScrollArea::vertical()
                .max_height(WINDOW_MAX_HEIGHT)
                .auto_shrink([true, true]) // Forces the void space at the bottom to collapse
                .show(ui, |ui| {
                    egui::Grid::new("exceptions_grid")
                        .striped(true)
                        .spacing(egui::vec2(15.0, 10.0))
                        .show(ui, |ui| {
                            
                            // Headers with forced minimum widths so they never truncate
                            ui.vertical_centered(|ui| { ui.set_min_width(COL_PREFIX_WIDTH); ui.label(egui::RichText::new("Prefix").strong()); });
                            ui.vertical_centered(|ui| { ui.set_min_width(COL_SUFFIX_WIDTH); ui.label(egui::RichText::new("Suffix").strong()); });
                            ui.vertical_centered(|ui| { ui.set_min_width(COL_EXT_WIDTH); ui.label(egui::RichText::new("Extension").strong()); });
                            ui.vertical_centered(|ui| { ui.set_min_width(COL_LOGIC_WIDTH); ui.label(egui::RichText::new("Name Logic").strong()); });
                            ui.vertical_centered(|ui| { ui.set_min_width(COL_LANG_WIDTH); ui.label(egui::RichText::new("Languages").strong()); });
                            ui.vertical_centered(|ui| { ui.set_min_width(COL_LOGIC_WIDTH); ui.label(egui::RichText::new("Lang Logic").strong()); });
                            ui.vertical_centered(|ui| { ui.set_min_width(COL_ACTION_WIDTH); ui.label(egui::RichText::new("Actions").strong()); });
                            ui.end_row();

                            let mut row_to_delete = None;
                            
                            for (i, rule) in state.rules.iter_mut().enumerate() {
                                // Strings using our new constants
                                ui.add(egui::TextEdit::singleline(&mut rule.prefix).desired_width(COL_PREFIX_WIDTH));
                                ui.add(egui::TextEdit::singleline(&mut rule.suffix).desired_width(COL_SUFFIX_WIDTH));
                                ui.add(egui::TextEdit::singleline(&mut rule.extension).desired_width(COL_EXT_WIDTH));

                                ui.vertical_centered(|ui| {
                                    egui::ComboBox::from_id_salt(format!("name_logic_{}", i))
                                        .selected_text(match rule.name_logic {
                                            NameLogic::Contains => "Contains",
                                            NameLogic::Only => "Only",
                                        })
                                        .width(COL_LOGIC_WIDTH)
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(&mut rule.name_logic, NameLogic::Contains, "Contains");
                                            ui.selectable_value(&mut rule.name_logic, NameLogic::Only, "Only");
                                        });
                                });

                                ui.vertical_centered(|ui| {
                                    let active_count = rule.languages.values().filter(|&&v| v).count();
                                    
                                    ui.menu_button(format!("Manage ({})", active_count), |ui| {
                                        // Use a grid with a unique ID for this specific row's popup
                                        egui::Grid::new(format!("lang_popup_grid_{}", i))
                                            .num_columns(2)
                                            .spacing(egui::vec2(0.0, 5.0)) // X-spacing locks the gap, Y-spacing breathes
                                            .show(ui, |ui| {
                                                for (lang, enabled) in rule.languages.iter_mut() {
                                                    ui.label(lang.to_uppercase());
                                                    
                                                    // The toggle sits cleanly in the second column
                                                    toggle_ui(ui, enabled); 
                                                    
                                                    ui.end_row(); // Move to the next line in the grid
                                                }
                                            });
                                    });
                                });

                                ui.vertical_centered(|ui| {
                                    egui::ComboBox::from_id_salt(format!("lang_logic_{}", i))
                                        .selected_text(match rule.lang_logic {
                                            LangLogic::Append => "Append",
                                            LangLogic::Only => "Only",
                                        })
                                        .width(COL_LOGIC_WIDTH)
                                        .show_ui(ui, |ui| {
                                            ui.selectable_value(&mut rule.lang_logic, LangLogic::Append, "Append");
                                            ui.selectable_value(&mut rule.lang_logic, LangLogic::Only, "Only");
                                        });
                                });

                                ui.vertical_centered(|ui| {
                                    if ui.button("🗑").on_hover_text("Delete Rule").clicked() {
                                        row_to_delete = Some(i);
                                    }
                                });
                                
                                ui.end_row();
                            }

                            if let Some(idx) = row_to_delete {
                                state.rules.remove(idx);
                            }
                        });
                });
        });

        state.is_open = is_open;
        ctx.data_mut(|d| d.insert_temp(state_id, state));
    }
}

// --- MAIN SHOW FUNCTION ---

pub fn show(ui: &mut egui::Ui, settings: &mut GameDataSettings, runtime: &mut RuntimeState, drag_guard: &mut DragGuard) -> bool {
    let mut refresh_needed = false;
    let ctx = ui.ctx().clone();

    // Pull our background deleters from Egui's temporary memory
    let mut game_deleter = ctx.data_mut(|d| d.get_temp::<FolderDeleter>(egui::Id::new("game_deleter")).unwrap_or_default());
    let mut raw_deleter = ctx.data_mut(|d| d.get_temp::<FolderDeleter>(egui::Id::new("raw_deleter")).unwrap_or_default());

    game_deleter.update();
    raw_deleter.update();

    if game_deleter.is_active() || raw_deleter.is_active() {
        ctx.request_repaint();
    }

    let game_exists = Path::new("game").exists();
    let raw_exists = Path::new("game/raw").exists();

    egui::ScrollArea::vertical()
        .id_salt("game_data_scroll")
        .auto_shrink([false, true])
        .show(ui, |ui| {

            ui.heading("Disk");
            ui.add_space(5.0);

            // GAME FOLDER BUTTON LOGIC
            if game_deleter.is_deleting() {
                let btn = egui::Button::new("Deleting \"game\" Folder...")
                    .fill(egui::Color32::from_rgb(200, 180, 50)); 
                ui.add_sized([180.0, 30.0], btn);
            } else if game_deleter.is_done() {
                let btn = egui::Button::new("Deleted \"game\" Folder!")
                    .fill(egui::Color32::from_rgb(40, 160, 40)); 
                ui.add_sized([180.0, 30.0], btn);
            } else if game_exists {
                let btn = egui::Button::new("Delete \"game\" Folder")
                    .fill(egui::Color32::from_rgb(180, 50, 50)); 
                if ui.add_sized([180.0, 30.0], btn).clicked() {
                    let state_id = egui::Id::new("delete_game_modal");
                    ctx.data_mut(|d| d.insert_temp(state_id, FolderDeleteState { is_open: true, size_str: None }));
                }
            } else {
                let btn = egui::Button::new("No \"game\" Folder")
                    .fill(egui::Color32::from_rgb(60, 60, 60)); 
                ui.add_sized([180.0, 30.0], btn);
            }

            ui.add_space(5.0);

            // RAW FOLDER BUTTON LOGIC
            if raw_deleter.is_deleting() {
                let btn = egui::Button::new("Deleting \"raw\" Folder...")
                    .fill(egui::Color32::from_rgb(200, 180, 50)); 
                ui.add_sized([180.0, 30.0], btn);
            } else if raw_deleter.is_done() {
                let btn = egui::Button::new("Deleted \"raw\" Folder!")
                    .fill(egui::Color32::from_rgb(40, 160, 40)); 
                ui.add_sized([180.0, 30.0], btn);
            } else if raw_exists {
                let btn = egui::Button::new("Delete \"raw\" Folder")
                    .fill(egui::Color32::from_rgb(180, 50, 50)); 
                if ui.add_sized([180.0, 30.0], btn).clicked() {
                    let size = get_folder_size(Path::new("game/raw"));
                    let state_id = egui::Id::new("delete_raw_modal");
                    ctx.data_mut(|d| d.insert_temp(state_id, FolderDeleteState { 
                        is_open: true, 
                        size_str: Some(format_size(size)) 
                    }));
                }
            } else {
                let btn = egui::Button::new("No \"raw\" Folder")
                    .fill(egui::Color32::from_rgb(60, 60, 60)); 
                ui.add_sized([180.0, 30.0], btn);
            }

            ui.add_space(20.0);
            ui.heading("Android");
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                let tooltip = "Attempt to connect to this IP Address Wirelessly if not automatically found when using Android import method\nMake sure you have \"Wireless USB Debugging\" enabled in your devices developer settings\nRequires ABD OEM Drivers Add-On to function";
                
                ui.label("Fallback IP Address").on_hover_text(tooltip);
                ui.spacing_mut().item_spacing.x = 4.0; 

                ui.allocate_ui(egui::vec2(100.0, 20.0), |ui| {
                    ui.centered_and_justified(|ui| {
                        if runtime.show_ip_field {
                            let hint = egui::RichText::new("192.168.X.X").color(egui::Color32::GRAY);
                            ui.add(egui::TextEdit::singleline(&mut settings.manual_ip)
                                .hint_text(hint)
                                .vertical_align(egui::Align::Center))
                                .on_hover_text(tooltip); 
                        } else {
                            if ui.button("Click to Reveal").on_hover_text(tooltip).clicked() {
                                runtime.show_ip_field = true;
                            }
                        }
                    });
                });

                ui.add_space(2.0);

                if ui.button("👁").on_hover_text("Toggle Visibility").clicked() {
                    runtime.show_ip_field = !runtime.show_ip_field;
                }
            });
            
            ui.add_space(5.0);

            ui.horizontal(|ui| {
                let label_response = ui.label("App Folder Persistence");
                let tooltip_text = "Skip the deletion of the \"game/app\" directory after android import";
                label_response.on_hover_text(tooltip_text);

                let toggle_response = toggle_ui(ui, &mut settings.app_folder_persistence).on_hover_text(tooltip_text);
                if toggle_response.changed() { refresh_needed = true; }
            });

            // --- NEW IMPORT SECTION ---
            ui.add_space(20.0);
            ui.heading("Import");
            ui.add_space(5.0);

            let import_btn = egui::Button::new("Manage Exceptions")
                .fill(egui::Color32::from_rgb(40, 90, 160)); // Nice actionable blue
            
            if ui.add_sized([180.0, 30.0], import_btn).clicked() {
                let state_id = egui::Id::new("manage_exceptions_state");
                let mut state = ctx.data(|d| d.get_temp::<ManageExceptionsState>(state_id)).unwrap_or_default();
                state.is_open = true;
                ctx.data_mut(|d| d.insert_temp(state_id, state));
            }
            // --------------------------

            ui.add_space(20.0);
            ui.heading("Export");
            ui.add_space(10.0);

            ui.horizontal(|ui| {
                let label_response = ui.label("Enable Ultra Compression");
                let tooltip_text = "Allows compression levels up to 21\nWARNING: Levels above 15 require significant RAM and time";
                label_response.on_hover_text(tooltip_text);

                let toggle_response = toggle_ui(ui, &mut settings.enable_ultra_compression).on_hover_text(tooltip_text);

                if toggle_response.changed() {
                    refresh_needed = true;
                    if !settings.enable_ultra_compression && settings.last_compression_level > 15 {
                        settings.last_compression_level = 15;
                    }
                }
            });
    });

    // Check if the user confirmed deletion in the modals
    if show_folder_delete_modal(&ctx, drag_guard, "delete_game_modal", "Are you sure you want to delete the \"game\" folder?\nMost app function will be lost.") {
        game_deleter.start("game");
    }

    if show_folder_delete_modal(&ctx, drag_guard, "delete_raw_modal", "Are you sure you want to delete the \"raw\" folder?\nYou may need to import again if an app update requires new game assets.") {
        raw_deleter.start("game/raw");
    }

    // Call our new exception modal
    show_manage_exceptions_modal(&ctx, drag_guard);

    // Save the thread trackers back into memory for the next frame
    ctx.data_mut(|d| {
        d.insert_temp(egui::Id::new("game_deleter"), game_deleter);
        d.insert_temp(egui::Id::new("raw_deleter"), raw_deleter);
    });

    refresh_needed
}