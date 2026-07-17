use eframe::egui;

use core::common::assets::*;
use core::common::game::CustomIcon;

#[derive(Clone)]
pub(crate) struct CustomAssets {
    pub multihit: egui::TextureHandle,
    pub kamikaze: egui::TextureHandle,
    pub boss_wave: egui::TextureHandle,
    pub dojo: egui::TextureHandle,
    pub starred_alien: egui::TextureHandle,
    pub burrow: egui::TextureHandle,
    pub revive: egui::TextureHandle,
    pub stop: egui::TextureHandle,
    pub death_timer: egui::TextureHandle,
    pub god: egui::TextureHandle,
    pub unknown: egui::TextureHandle,
    #[allow(dead_code)] pub udi_f: egui::TextureHandle,
}

impl CustomAssets {
    pub(crate) fn new(ctx: &egui::Context) -> Self {
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
            dojo: load("dojo", DOJO),
            starred_alien: load("starred_alien", STARRED_ALIEN),
            burrow: load("burrow", BURROW),
            revive: load("revive", REVIVE),
            stop: load("stop", STOP),
            death_timer: load("death_timer", DEATH_TIMER),
            god: load("god", GOD),
            unknown: load("unknown", UNKNOWN),
            udi_f: load("udi_f", UDI_F),
        }
    }

    pub(crate) fn get_icon_texture(&self, icon: CustomIcon) -> Option<&egui::TextureHandle> {
        match icon {
            CustomIcon::Multihit => Some(&self.multihit),
            CustomIcon::Kamikaze => Some(&self.kamikaze),
            CustomIcon::BossWave => Some(&self.boss_wave),
            CustomIcon::Dojo => Some(&self.dojo),
            CustomIcon::StarredAlien => Some(&self.starred_alien),
            CustomIcon::Burrow => Some(&self.burrow),
            CustomIcon::Revive => Some(&self.revive),
            CustomIcon::Stop => Some(&self.stop),
            CustomIcon::DeathTimer => Some(&self.death_timer),
            CustomIcon::God => Some(&self.god),
            CustomIcon::Unknown => Some(&self.unknown),
            CustomIcon::None => None,
        }
    }
}