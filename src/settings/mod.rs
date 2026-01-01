use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)] 
pub struct Settings {
    pub high_banner_quality: bool,
    pub expand_spirit_details: bool,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            high_banner_quality: false,
            expand_spirit_details: false,
        }
    }
}

pub fn show(ctx: &egui::Context, settings: &mut Settings) -> bool {
    let mut refresh_needed = false;

    egui::CentralPanel::default().show(ctx, |ui| {
        ui.heading("Settings");
        ui.add_space(20.0);

        ui.horizontal(|ui| {
            if toggle_ui(ui, &mut settings.high_banner_quality).changed() {
                refresh_needed = true;
            }
            ui.label("High Quality Banners");
        });
        
        ui.add_space(10.0);

        ui.horizontal(|ui| {
            toggle_ui(ui, &mut settings.expand_spirit_details);
            ui.label("Expand Spirit Details by Default");
        });
        
        ui.add_space(30.0);
    });

    refresh_needed
}

fn toggle_ui(ui: &mut egui::Ui, on: &mut bool) -> egui::Response {
    let desired_size = ui.spacing().interact_size.y * egui::vec2(2.0, 1.0);
    let (rect, mut response) = ui.allocate_exact_size(desired_size, egui::Sense::click());
    
    if response.clicked() {
        *on = !*on;
        response.mark_changed();
    }

    response.widget_info(|| egui::WidgetInfo::selected(egui::WidgetType::Checkbox, true, *on, ""));
    
    if ui.is_rect_visible(rect) {
        let how_on = ui.ctx().animate_bool(response.id, *on);
        let visuals = ui.style().interact_selectable(&response, *on);
        
        let rect = rect.expand(visuals.expansion);
        let radius = 0.5 * rect.height();
        ui.painter().rect(rect, radius, visuals.bg_fill, visuals.bg_stroke);
        
        let circle_x = egui::lerp((rect.left() + radius)..=(rect.right() - radius), how_on);
        let center = egui::pos2(circle_x, rect.center().y);
        ui.painter().circle(center, 0.75 * radius, visuals.fg_stroke.color, visuals.fg_stroke);
    }

    response
}