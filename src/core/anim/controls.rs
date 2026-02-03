use eframe::egui;

/// Handles all user input for the animation viewport (Zoom, Pan, etc.)
pub fn handle_viewport_input(
    ui: &egui::Ui,
    response: &egui::Response,
    pan_offset: &mut egui::Vec2,
    zoom_level: &mut f32,
    target_zoom_level: &mut f32,
    pending_initial_center: &mut bool,
) {
    // 1. Pan (Drag)
    // Logic: Dragging moves the pan offset. We divide by zoom_level to ensure
    // the mouse stays 1:1 with the object regardless of zoom.
    if response.dragged() {
        *pan_offset += response.drag_delta() / *zoom_level;
        
        // Cancel any pending auto-center if the user takes control
        *pending_initial_center = false;
    }

    // 2. Zoom (Scroll Wheel)
    // Logic: Modifies the TARGET zoom level for smooth interpolation.
    if response.hovered() {
        ui.input(|i| {
            // "raw_scroll_delta" gives the raw wheel ticks
            let scroll = i.raw_scroll_delta.y;
            if scroll != 0.0 {
                // Sensitivity: 0.006 per tick
                let zoom_factor = 1.0 + (scroll * 0.006);
                *target_zoom_level = (*target_zoom_level * zoom_factor).clamp(0.1, 10.0);
            }
        });
    }

    // 3. Zoom (Pinch / Touchpad)
    // Logic: Gestures usually require immediate feedback, so we snap the zoom level directly.
    ui.input(|i| {
        let delta = i.zoom_delta();
        if delta != 1.0 {
            *target_zoom_level *= delta;
            // Snap immediately for gestures to feel responsive
            *zoom_level = *target_zoom_level;
        }
    });
}