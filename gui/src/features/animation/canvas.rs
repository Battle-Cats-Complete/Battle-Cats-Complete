use std::sync::{Arc, Mutex};

use eframe::egui;
use nyanko::graphics::engine::resolve_frame;
use nyanko::graphics::rig::{Animation, Unit};

use core::animation::logic::canvas::GlowRenderer;

pub fn paint(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    renderer_reference: Arc<Mutex<Option<GlowRenderer>>>,
    unit: Arc<Unit>,
    animation: Option<Arc<Animation>>,
    current_frame: f32,
    pan: egui::Vec2,
    zoom: f32,
) {
    let paint_callback = egui::PaintCallback {
        rect,
        callback: Arc::new(eframe::egui_glow::CallbackFn::new(move |info, painter| {
            let Ok(mut renderer_lock) = renderer_reference.lock() else {
                return;
            };

            if renderer_lock.is_none() {
                // Safely handle the Result without unwrapping/panicking
                let Ok(new_renderer) = GlowRenderer::new(painter.gl()) else {
                    return;
                };
                *renderer_lock = Some(new_renderer);
            }

            let Some(renderer) = renderer_lock.as_mut() else {
                return;
            };

            let viewport_width = info.viewport.width();
            let viewport_height = info.viewport.height();

            // 1. Get the pure world geometry from the library
            let frame_geometry = resolve_frame(
                &unit,
                animation.as_deref(), // Converts Option<Arc<Animation>> to Option<&Animation>
                current_frame
            );

            // 2. Delegate hardware rendering to the core's canvas
            let _ = renderer.draw_frame(
                painter.gl(),
                &frame_geometry,
                &unit.sheet,
                viewport_width,
                viewport_height,
                pan.x,
                pan.y,
                zoom
            );
        })),
    };

    ui.painter().add(paint_callback);
}