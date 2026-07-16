use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use eframe::egui;
use nyanko::graphics::rig::{Animation, Unit};

use core::modules::animation::export::encoding::{self, EncoderMessage};
use core::modules::animation::export::process::calculate_export_time;
use core::modules::animation::export::state::ExporterState;
use core::modules::animation::logic::canvas::GlowRenderer;

pub fn process_frame(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    state: &mut ExporterState,
    unit: Arc<Unit>,
    animation: Option<Arc<Animation>>,
    renderer_reference: Arc<Mutex<Option<GlowRenderer>>>,
    current_time: f32,
) {
    if state.tx.is_none() {
        return;
    }

    if let Some(abort_signal) = &state.abort
        && abort_signal.load(Ordering::Relaxed) {
            state.tx = None;
            state.abort = None;
            return;
        }

    let frame_count = (state.frame_end - state.frame_start).abs() + 1;

    if state.current_progress >= frame_count {
        if let Some(sender) = state.tx.take() {
            let _ = sender.send(EncoderMessage::Finish);
        }
        return;
    }

    let frame_time = calculate_export_time(state, animation.as_deref(), current_time);
    let frame_delay_milliseconds = 1000.0 / state.fps as f32;

    let snap_x = (state.region_x * state.zoom).round() / state.zoom;
    let snap_y = (state.region_y * state.zoom).round() / state.zoom;

    let pan_x = -snap_x - (state.region_w / (2.0 * state.zoom));
    let pan_y = -snap_y - (state.region_h / (2.0 * state.zoom));
    let background_color = if state.background { [80, 80, 80, 255] } else { [0, 0, 0, 0] };

    let renderer_arc = renderer_reference.clone();
    let unit_arc = unit.clone();
    let animation_arc = animation.clone();

    let Some(sender) = state.tx.as_ref().cloned() else {
        return;
    };

    let width = state.region_w;
    let height = state.region_h;
    let zoom = state.zoom;

    ui.painter().add(egui::PaintCallback {
        rect,
        callback: Arc::new(eframe::egui_glow::CallbackFn::new(move |_, painter| {
            let Ok(mut lock) = renderer_arc.lock() else { return; };
            let Some(renderer) = lock.as_mut() else { return; };

            let render_result = encoding::render_frame(
                renderer,
                painter.gl(),
                width as u32,
                height as u32,
                &unit_arc,
                animation_arc.as_deref(),
                frame_time,
                pan_x,
                pan_y,
                zoom,
                background_color
            );

            match render_result {
                Ok(raw_pixels) => {
                    let _ = sender.send(EncoderMessage::Frame(
                        raw_pixels,
                        width as u32,
                        height as u32,
                        frame_delay_milliseconds as u32
                    ));
                }
                Err(error_message) => {
                    eprintln!("Export rendering failed: {}", error_message);
                }
            }
        })),
    });

    state.current_progress += 1;
}