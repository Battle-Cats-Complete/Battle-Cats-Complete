use core::domains::import::architecture;

use crate::app::Page;

use super::{Action, Context, FileTarget, Item};

const NEEDS_MOD: &str = "This action requires a mod to be enabled";

pub(super) fn items(context: &Context) -> Vec<Item> {
    let mut items = Vec::new();

    if !context.enabled {
        return items;
    }

    if context.page == Page::Files
        && let Some(file) = context.file.as_ref()
    {
        files(&mut items, file);
    }

    items
}

fn files(items: &mut Vec<Item>, file: &FileTarget) {
    if file.folder {
        return;
    }

    if file.mount != architecture::GAME {
        items.push(delete(file));

        return;
    }

    items.push(adopt(file));

    if file.unlocked {
        items.push(delete(file));
    }
}

fn adopt(file: &FileTarget) -> Item {
    let Some(active) = file.active_mod.as_deref() else {
        return Item::disabled("Add File to Mod", NEEDS_MOD);
    };

    let item = Item::new(
        format!("Add \"{}\" to \"{active}\"", file.name),
        Action::AddFileToMod { source: file.source.clone(), target_mod: active.to_owned() },
    );

    if file.in_active_mod { item.confirming() } else { item }
}

fn delete(file: &FileTarget) -> Item {
    Item::new(
        format!("Delete \"{}\"", file.name),
        Action::DeleteFile { source: file.source.clone() },
    )
    .confirming()
}
