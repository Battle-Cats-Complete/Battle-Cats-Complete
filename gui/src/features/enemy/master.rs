use std::sync::Arc;

use eframe::egui;
use nyanko::graphics::rig::Unit;

use core::enemy::logic::context::EnemyRenderContext;
use core::enemy::logic::scanner::{self, EnemyEntry};
use core::enemy::logic::state::EnemyDetailTab;
use core::enemy::registry::Magnification;
use core::global::context::GlobalContext;
use core::settings::logic::Settings;

use crate::features::animation::viewer::AnimViewer;
use crate::features::statblock::builder::{generate_and_copy, generate_and_save};
use crate::global::assets::CustomAssets;
use crate::global::shared::DragGuard;
use crate::global::sheet::SpriteSheet;

use super::statblock::build_enemy_statblock;
use super::{abilities, details, header, stats, viewer};
use super::header::ExportAction;

pub fn show(
    ctx: &egui::Context,
    ui: &mut egui::Ui,
    enemy_entry: &EnemyEntry,
    current_tab: &mut EnemyDetailTab,
    mag_input: &mut String,
    magnification: &mut Magnification,
    img015_sheets: &mut Vec<SpriteSheet>,
    unit_sync: &mut Option<Arc<Unit>>,
    anim_viewer: &mut AnimViewer,
    settings: &mut Settings,
    detail_texture: &mut Option<egui::TextureHandle>,
    detail_key: &mut String,
    global_ctx: GlobalContext,
    assets: &CustomAssets,
    drag_guard: &mut DragGuard,
) {
    crate::global::img015::ensure_loaded(ctx, img015_sheets, settings);

    let export_action = header::render(
        ctx, ui, enemy_entry, current_tab, mag_input, magnification, detail_texture, detail_key,
    );

    let dynamic_entry = scanner::scan_single(enemy_entry.id, &settings.scanner_config());
    let stats = dynamic_entry.as_ref().map(|e| &e.stats).unwrap_or(&enemy_entry.stats);

    let enemy_ctx = EnemyRenderContext {
        global: global_ctx,
        stats,
        magnification: *magnification,
    };

    match export_action {
        ExportAction::Copy | ExportAction::Save => {
            let data = build_enemy_statblock(&enemy_ctx, enemy_entry);

            let priority_clone = settings.general.language_priority.clone();
            let mut cuts_clone = std::collections::HashMap::new();
            for sheet in img015_sheets.iter().rev() {
                cuts_clone.extend(sheet.core.cuts_map.clone());
            }

            if export_action == ExportAction::Copy {
                generate_and_copy(ctx.clone(), priority_clone, data, cuts_clone);
            } else {
                generate_and_save(ctx.clone(), priority_clone, data, cuts_clone);
            }
        },
        ExportAction::None => {}
    }

    ui.separator();
    ui.add_space(0.0);

    match current_tab {
        EnemyDetailTab::Abilities => {
            stats::render(ui, enemy_entry, *magnification);
            ui.spacing_mut().item_spacing.y = 7.0;
            ui.separator();
            egui::ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    abilities::render(
                        ui,
                        &enemy_ctx,
                        img015_sheets,
                        assets
                    );
                });
        },
        EnemyDetailTab::Details => {
            details::render(ui, &enemy_entry.description);
        },
        EnemyDetailTab::Animation => {
            viewer::show(ui, ctx, enemy_entry, anim_viewer, unit_sync, settings, drag_guard);
        }
    }
}