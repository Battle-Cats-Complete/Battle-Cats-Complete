use eframe::egui;
use crate::core::settings::Settings;
use crate::core::addons::adb::adbdownload::AdbManager;
use crate::core::addons::avifenc::avifencdownload::AvifManager;
use crate::core::addons::ffmpeg::ffmpegdownload::FfmpegManager;
use crate::core::addons::toolpaths::AddonStatus;
use std::sync::Mutex;

static ADB_MANAGER: Mutex<Option<AdbManager>> = Mutex::new(None);
static AVIF_MANAGER: Mutex<Option<AvifManager>> = Mutex::new(None);
static FFMPEG_MANAGER: Mutex<Option<FfmpegManager>> = Mutex::new(None);

pub fn show(ui: &mut egui::Ui, _settings: &mut Settings) -> bool {
    let mut adb_lock = ADB_MANAGER.lock().unwrap();
    let adb_manager = adb_lock.get_or_insert_with(AdbManager::default);
    adb_manager.update();

    let mut avif_lock = AVIF_MANAGER.lock().unwrap();
    let avif_manager = avif_lock.get_or_insert_with(AvifManager::default);
    avif_manager.update();

    let mut ffmpeg_lock = FFMPEG_MANAGER.lock().unwrap();
    let ffmpeg_manager = ffmpeg_lock.get_or_insert_with(FfmpegManager::default);
    ffmpeg_manager.update();

    egui::ScrollArea::vertical()
        .id_salt("addons_scroll")
        .auto_shrink([false, true])
        .show(ui, |ui| {
            
            ui.heading("Android Bridge");
            ui.add_space(5.0);
            ui.label("Required for Game Data Import");
            ui.add_space(8.0);
            let adb_status = adb_manager.status.clone();
            render_addon_controls(ui, &adb_status, "ADB", || adb_manager.install(), "adb_delete_confirm");

            ui.add_space(20.0);

            ui.heading("AVIFENC");
            ui.add_space(5.0);
            ui.label("Required to encode AVIF files for Animation Export");
            ui.add_space(8.0);
            let avif_status = avif_manager.status.clone();
            render_addon_controls(ui, &avif_status, "AVIFENC", || avif_manager.install(), "avif_delete_confirm");

            ui.add_space(20.0);

            ui.heading("FFMPEG");
            ui.add_space(5.0);
            ui.label("Significantly increases encoding speed for Animation Export");
            ui.add_space(8.0);
            let ffmpeg_status = ffmpeg_manager.status.clone();
            render_addon_controls(ui, &ffmpeg_status, "FFMPEG", || ffmpeg_manager.install(), "ffmpeg_delete_confirm");
        });

    handle_delete_modal(ui, "adb_delete_confirm", "Android Bridge", || adb_manager.uninstall());
    handle_delete_modal(ui, "avif_delete_confirm", "AVIFENC", || avif_manager.uninstall());
    handle_delete_modal(ui, "ffmpeg_delete_confirm", "FFMPEG", || ffmpeg_manager.uninstall());

    false
}

fn render_addon_controls(ui: &mut egui::Ui, status: &AddonStatus, name: &str, on_download: impl FnOnce(), confirm_id: &str) {
    match status {
        AddonStatus::Installed => {
            let btn = egui::Button::new(format!("Delete {}", name)).fill(egui::Color32::from_rgb(180, 50, 50));
            if ui.add_sized([140.0, 30.0], btn).clicked() {
                ui.ctx().data_mut(|d| d.insert_temp(egui::Id::new(confirm_id), true));
            }
        },
        AddonStatus::NotInstalled | AddonStatus::Error(_) => {
            let btn = egui::Button::new(format!("Download {}", name)).fill(egui::Color32::from_rgb(40, 160, 40));
            if ui.add_sized([140.0, 30.0], btn).clicked() {
                on_download();
            }
            if let AddonStatus::Error(e) = status {
                ui.add_space(5.0);
                ui.label(egui::RichText::new(format!("Error: {}", e)).color(egui::Color32::RED));
            }
        },
        AddonStatus::Downloading(prog, msg) => {
            ui.add(egui::ProgressBar::new(*prog).text(msg).desired_width(140.0));
        }
    }
}

fn handle_delete_modal(ui: &mut egui::Ui, id: &str, display_name: &str, on_yes: impl FnOnce()) {
    let confirm_id = egui::Id::new(id);
    let mut show_confirm = ui.ctx().data(|d| d.get_temp(confirm_id).unwrap_or(false));

    if show_confirm {
        egui::Window::new("Delete Add-On")
            .collapsible(false).resizable(false).anchor(egui::Align2::CENTER_CENTER, egui::vec2(0.0, 0.0))
            .show(ui.ctx(), |ui| {
                ui.label(format!("Are you sure you want to delete {}?", display_name));
                ui.add_space(10.0);
                ui.horizontal(|ui| {
                    if ui.button("Yes, Delete").clicked() {
                        on_yes();
                        show_confirm = false;
                    }
                    if ui.button("Cancel").clicked() {
                        show_confirm = false;
                    }
                });
            });
        ui.ctx().data_mut(|d| d.insert_temp(confirm_id, show_confirm));
    }
}