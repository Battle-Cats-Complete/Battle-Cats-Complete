use eframe::egui;
use crate::{main_menu, import_data, cat_data};

#[derive(PartialEq, Clone, Copy)]
enum Page{
    MainMenu,
    ImportData,
    CatData,
}

const PAGES: &[(Page, &str)] = &[
    (Page::MainMenu, "Main Menu"),
    (Page::ImportData, "Import Data"),
    (Page::CatData, "Cat Data"),
];

pub struct BattleCatsApp {
    current_page: Page,
    sidebar_open: bool,
    import_state: import_data::ImportState,
    cat_list_state: cat_data::CatListState,
}

impl Default for BattleCatsApp {
    fn default() -> Self {
        Self {
            current_page: Page::MainMenu,
            sidebar_open: false,
            import_state: import_data::ImportState::default(),
            cat_list_state: cat_data::CatListState::default()
        }
    }
}

impl eframe::App for BattleCatsApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        
        self.cat_list_state.update_data();
        if self.cat_list_state.scan_receiver.is_some() {
            ctx.request_repaint();
        }

        let import_finished = self.import_state.update(ctx);
        if import_finished {
            self.cat_list_state.refresh();
        }

        let mut style = (*ctx.style()).clone();
        style.visuals.window_rounding = egui::Rounding::same(10.0);
        style.visuals.widgets.noninteractive.rounding = egui::Rounding::same(10.0);
        style.visuals.widgets.inactive.rounding = egui::Rounding::same(10.0);
        style.visuals.widgets.hovered.rounding = egui::Rounding::same(10.0);
        style.visuals.widgets.active.rounding = egui::Rounding::same(10.0);
        style.spacing.item_spacing = egui::vec2(10.0, 10.0);
        style.visuals.window_fill = egui::Color32::from_rgb(33, 33, 33);
        style.visuals.panel_fill = egui::Color32::from_rgb(33, 33, 33);
        style.visuals.override_text_color = Some(egui::Color32::WHITE);
        ctx.set_style(style);


        match self.current_page {
            Page::MainMenu => main_menu::show(ctx),
            Page::ImportData => {
                import_data::show(ctx, &mut self.import_state);
            },
            Page::CatData => cat_data::show(ctx, &mut self.cat_list_state),
        }

        let sidebar_inner_width = 150.0; 
        let sidebar_margin = 15.0;       
        let total_sidebar_width = sidebar_inner_width + (sidebar_margin * 2.0);
        let screen_rect = ctx.screen_rect();
        
        let target_open = if self.sidebar_open { 1.0 } else { 0.0 };
        let open_factor = ctx.animate_value_with_time(egui::Id::new("sb_anim"), target_open, 0.35);

        if open_factor > 0.0 && open_factor < 1.0 {
            ctx.request_repaint();
        }

        let sidebar_x = screen_rect.width() - (total_sidebar_width * open_factor);
        let button_gap = 10.0;
        let button_size = 40.0;
        let button_x = sidebar_x - button_gap - button_size;

        if open_factor > 0.0 {
            egui::Area::new("sidebar_area".into())
                .constrain(false)
                .fixed_pos(egui::pos2(sidebar_x, 0.0))
                .order(egui::Order::Foreground)
                .show(ctx, |ui| {
                    egui::Frame::none()
                        .fill(egui::Color32::from_rgb(20, 20, 20))
                        .inner_margin(15.0)
                        .rounding(egui::Rounding { nw: 10.0, sw: 10.0, ne: 0.0, se: 0.0 })
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

                                    let btn = egui::Button::new(btn_text).fill(bg_color).min_size(egui::vec2(0.0, 45.0));
                                    if ui.add_sized([ui.available_width(), 45.0], btn).clicked() {
                                        self.current_page = *page_enum;
                                    }
                                }
                            });
                        });
                });
        }

        egui::Area::new("toggle_btn".into())
            .fixed_pos(egui::pos2(button_x, 2.5))
            .order(egui::Order::Tooltip)
            .show(ctx, |ui| {
                let arrow = if self.sidebar_open { ">" } else { "<" };
                let btn = egui::Button::new(egui::RichText::new(arrow).size(20.0).strong())
                    .fill(egui::Color32::from_rgb(31, 106, 165));

                if ui.add_sized([40.0, 40.0], btn).clicked() {
                    self.sidebar_open = !self.sidebar_open;
                }
            });
    }
}