use eframe::egui;
use super::{NAME_BOX_WIDTH, NAME_BOX_HEIGHT};

pub fn render_name_in_box(ui: &mut egui::Ui, name: &str) {
    let mut font_size = 22.0; 
    let text_color = ui.visuals().text_color();
    
    // 1. Reserve the fixed space immediately
    let (rect, _) = ui.allocate_exact_size(egui::vec2(NAME_BOX_WIDTH, NAME_BOX_HEIGHT), egui::Sense::hover());

    // 2. Iteratively reduce size until it fits
    while font_size > 8.0 { 
        let font_id = egui::FontId::proportional(font_size);
        let job = egui::text::LayoutJob::simple(
            name.to_owned(),
            font_id,
            text_color,
            NAME_BOX_WIDTH // Wrap at box width
        );
        
        let galley = ui.fonts(|f| f.layout_job(job));
        
        // **RESTORED LOGIC:**
        // Check line count (<= 2 lines), NOT strict pixel height.
        // This allows text to wrap naturally and maintain readability (e.g. 16pt)
        // even if the 2 lines are visually slightly taller than the 24px box.
        // The centering logic below ensures it looks correct.
        if galley.rows.len() <= 2 {
            let y_offset = (NAME_BOX_HEIGHT - galley.rect.height()) / 2.0;
            let pos = rect.min + egui::vec2(0.0, y_offset);
            ui.painter().galley(pos, galley, text_color);
            return;
        }
        
        font_size -= 1.0;
    }
    
    // Fallback: render at 8.0 if it still doesn't fit
    let font_id = egui::FontId::proportional(8.0);
    let job = egui::text::LayoutJob::simple(name.to_owned(), font_id, text_color, NAME_BOX_WIDTH);
    let galley = ui.fonts(|f| f.layout_job(job));
    let y_offset = (NAME_BOX_HEIGHT - galley.rect.height()) / 2.0;
    ui.painter().galley(rect.min + egui::vec2(0.0, y_offset), galley, text_color);
}