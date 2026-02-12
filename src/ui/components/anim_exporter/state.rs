use std::sync::mpsc::Sender;
use crate::core::anim::export::{ExportFormat, QualityLevel, EncoderMessage};
use crate::core::utils::DragGuard;

pub struct ExporterState {
    // Input
    pub frame_start: i32,
    pub frame_end: i32,
    pub max_frame: i32,
    pub frame_start_str: String,
    pub frame_end_str: String,

    // Showcase Inputs
    pub showcase_mode: bool,
    pub showcase_walk_str: String,
    pub showcase_idle_str: String,
    pub showcase_kb_str: String,
    
    // Showcase Data (Parsed/Calculated)
    pub showcase_walk_len: i32,
    pub showcase_idle_len: i32,
    pub showcase_attack_len: i32, // Detected automatically
    pub showcase_kb_len: i32,

    pub fps: i32,
    pub zoom: f32,
    
    // Camera / Crop
    pub region_x: f32,
    pub region_y: f32,
    pub region_w: f32,
    pub region_h: f32,
    
    // Output
    pub file_name: String,
    pub name_prefix: String,
    pub format: ExportFormat,
    pub quality: QualityLevel,
    pub interpolation: bool,
    
    // Processing State
    pub is_processing: bool,
    pub current_progress: i32,
    pub tx: Option<Sender<EncoderMessage>>,
    
    // UI Helpers
    pub drag_guard: DragGuard,
    pub anim_name: String,
    pub completion_time: Option<f64>, 
}

impl Default for ExporterState {
    fn default() -> Self {
        Self {
            frame_start: 0,
            frame_end: 0,
            max_frame: 100,
            frame_start_str: String::new(),
            frame_end_str: String::new(),

            showcase_mode: false,
            showcase_walk_str: String::new(),
            showcase_idle_str: String::new(),
            showcase_kb_str: String::new(),
            
            showcase_walk_len: 90,
            showcase_idle_len: 90,
            showcase_attack_len: 0, 
            showcase_kb_len: 90,

            fps: 30,
            zoom: 1.0,
            
            region_x: -150.0,
            region_y: -150.0,
            region_w: 300.0,
            region_h: 300.0,
            
            file_name: String::new(),
            name_prefix: String::new(),
            format: ExportFormat::Gif,
            quality: QualityLevel::High,
            interpolation: false,
            
            is_processing: false,
            current_progress: 0,
            tx: None,
            
            drag_guard: DragGuard::default(), 
            anim_name: String::new(),
            completion_time: None, 
        }
    }
}