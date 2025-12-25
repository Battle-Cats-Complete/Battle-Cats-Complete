use eframe::egui;
use crate::{main_menu, import_data};

// To add a new page:
//   1. Add it to this Enum
//   2. Add it to the PAGES list below
//   3. Add it to the match statement in update()
#[derive(PartialEq, Clone, Copy)]
enum Page{
    MainMenu,
    ImportData,
}

const PAGES: &[(Page, &str)] = &[
    (Page::MainMenu, "Main Menu"),
    (Page::ImportData, "Import Data"),
];

pub struct BattleCatsApp {
    current_page: Page,
    sidebar_open: bool,
    // hold state for subpages so it persists between tabs
    import_state: import_data::ImportState,
}

impl Default for BattleCatsApp {
    fn default() -> Self {
        Self {
            current_page: Page::MainMenu,
            sidebar_open: false,
            import_state: import_data::ImportState::default()
        }
    }
}

impl eframe::App for BattleCatsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // --- 1. CUSTOMTKINTER THEME SETUP ---
        let mut style = (*ctx.style()).clone();
        
        // Rounding = 10.0 (Modern look)
        style.visuals.window_rounding = egui::Rounding::same(10.0);
        style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(10.0);
        style.visuals.widgets.inactive.rounding = egui::Rounding::same(10.0);
        style.visuals.widgets.hovered.rounding = egui::Rounding::same(10.0);
        style.visuals.widgets.active.rounding = egui::Rounding::same(10.0);

        // Spacing
        style.spacing.item_spacing = egui::vec2(10.0, 10.0);
        
        // Colors (Dark Backgrounds)
        style.visuals.window_fill = egui::Color32::from_rgb(33, 33, 33);
        style.visuals.panel_fill = egui::Color32::from_rgb(33, 33, 33);

        style.visuals.override_text_color = Some(egui::Color32::WHITE);
        
        ctx.set_style(style);


        // --- 2. RENDER PAGE ---
        match self.current_page {
            Page::MainMenu => main_menu::show(ctx),
            Page::ImportData => import_data::show(ctx, &mut self.import_state),
        }

        // --- 3. SIDEBAR ANIMATION ---
        let sidebar_inner_width = 150.0; 
        let sidebar_margin = 15.0;       
        let total_sidebar_width = sidebar_inner_width + (sidebar_margin * 2.0); // ~205px

        let screen_rect = ctx.screen_rect();
        
        let target_open = if self.sidebar_open { 1.0 } else { 0.0 };
        let open_factor = ctx.animate_value_with_time(egui::Id::new("sb_anim"), target_open, 0.35);

        if open_factor > 0.0 && open_factor < 1.0 {
            ctx.request_repaint();
        }

        // 1. Sidebar Position
        let sidebar_x = screen_rect.width() - (total_sidebar_width * open_factor);
        
        // 2. Button Position (LOCKED)
        // We set a fixed gap. The button will ALWAYS be this far from the sidebar.
        // This guarantees they move at the exact same speed.
        let button_gap = 10.0; // Distance between button and sidebar
        let button_size = 40.0;
        
        // Math: Sidebar Edge - Gap - Button Itself
        let button_x = sidebar_x - button_gap - button_size;

        // --- 4. RENDER SIDEBAR ---
        if open_factor > 0.0 {
            egui::Area::new("sidebar_area".into())
                .constrain(false)
                .fixed_pos(egui::pos2(sidebar_x, 0.0))
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(20, 20, 20))
                        .inner_margin(15.0)
                        .rounding(egui::Rounding {
                            nw: 10.0,
                            sw: 10.0,
                            ne: 0.0,
                            se: 0.0,
                        })
                        .show(ui, |ui| {
                            ui.set_min_size(egui::vec2(sidebar_inner_width, screen_rect.height()));

                            ui.vertical_centered_justified(|ui| {
                                for (page_enum, label) in PAGES {
                                    ui.add_space(5.0);
                                    let btn_text = egui::RichText::new(*label).size(16.0); 

                                    let is_selected = self.current_page == *page_enum;
                                    let bg_color = if is_selected {
                                        egui::Color32::from_rgb(31, 106, 165) 
                                    } else {
                                        egui::Color32::from_rgb(50, 50, 50)   
                                    };

                                    let btn = egui::Button::new(btn_text)
                                        .fill(bg_color)
                                        .min_size(egui::vec2(0.0, 45.0));

                                    if ui.add_sized([ui.available_width(), 45.0], btn).clicked() {
                                        self.current_page = *page_enum;
                                    }
                                }
                            });
                        });
                });
        }

        // --- 5. RENDER TOGGLE BUTTON ---
        egui::Area::new("toggle_btn".into())
            .fixed_pos(egui::pos2(button_x, 10.0))
            .order(egui::Order::Tooltip)
            .show(ctx, |ui| {
                let arrow = if self.sidebar_open { ">" } else { "<" };
                
                // Make the toggle button Blue too!
                let btn = egui::Button::new(egui::RichText::new(arrow).size(20.0).strong())
                    .fill(egui::Color32::from_rgb(31, 106, 165)); // Blue Accent

                if ui.add_sized([40.0, 40.0], btn).clicked() {
                    self.sidebar_open = !self.sidebar_open;
                }
            });
    }
}