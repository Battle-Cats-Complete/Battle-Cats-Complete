use eframe::egui;

// --- RAW BYTES (Embedded in Binary) ---
pub const MULTIHIT: &[u8] = include_bytes!("../assets/multihit.png");
pub const KAMIKAZE: &[u8] = include_bytes!("../assets/kamikaze.png");
pub const BOSS_WAVE: &[u8] = include_bytes!("../assets/boss_wave_immune.png");
pub const BASE: &[u8] = include_bytes!("../assets/base.png");
pub const STARRED_ALIEN: &[u8] = include_bytes!("../assets/starred_alien.png");
pub const BURROW: &[u8] = include_bytes!("../assets/burrow.png");
pub const REVIVE: &[u8] = include_bytes!("../assets/revive.png");
pub const UDI_F: &[u8] = include_bytes!("../assets/udi_f.png");

pub const ICON: &[u8] = include_bytes!("../assets/icon.ico");
pub const FONT_JP: &[u8] = include_bytes!("../assets/NotoSansJP-Regular.ttf");
pub const FONT_KR: &[u8] = include_bytes!("../assets/NotoSansKR-Regular.ttf");
pub const FONT_TC: &[u8] = include_bytes!("../assets/NotoSansTC-Regular.ttf");
pub const FONT_TH: &[u8] = include_bytes!("../assets/NotoSansThai-Regular.ttf");

// --- TEXTURE MANAGER ---

#[derive(Clone)]
pub struct CustomAssets {
    pub multihit: egui::TextureHandle,
    pub kamikaze: egui::TextureHandle,
    pub boss_wave: egui::TextureHandle,
    pub base: egui::TextureHandle,
    pub starred_alien: egui::TextureHandle,
    pub burrow: egui::TextureHandle,
    pub revive: egui::TextureHandle,
    #[allow(dead_code)] pub udi_f: egui::TextureHandle,
}

impl CustomAssets {
    pub fn new(ctx: &egui::Context) -> Self {
        let load = |name: &str, bytes: &[u8]| {
            let img = image::load_from_memory(bytes).expect("Failed to load embedded asset");
            let rgba = img.to_rgba8();
            let color_img = egui::ColorImage::from_rgba_unmultiplied(
                [rgba.width() as usize, rgba.height() as usize],
                rgba.as_flat_samples().as_slice(),
            );
            ctx.load_texture(name, color_img, egui::TextureOptions::LINEAR)
        };

        Self {
            multihit: load("multihit", MULTIHIT),
            kamikaze: load("kamikaze", KAMIKAZE),
            boss_wave: load("boss_wave", BOSS_WAVE),
            base: load("base", BASE),
            starred_alien: load("starred_alien", STARRED_ALIEN),
            burrow: load("burrow", BURROW),
            revive: load("revive", REVIVE),
            udi_f: load("udi_f", UDI_F),
        }
    }
}