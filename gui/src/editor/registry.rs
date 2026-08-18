use core::domains::import::architecture;

use crate::app::Page;

use super::{attributes, Action, CatTarget, Context, EnemyTarget, FileTarget, Item};

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

    if context.page == Page::Cats
        && context.abilities
        && let Some(cat) = context.cat.as_ref()
    {
        cats(&mut items, cat);
    }

    if context.page == Page::Enemies
        && let Some(enemy) = context.enemy.as_ref()
    {
        enemies(&mut items, enemy);
    }

    items
}

pub(super) fn cat_plan(cat: &CatTarget, target_mod: Option<String>) -> attributes::Plan {
    attributes::cat(cat.form, cat.title(), &cat.source, target_mod)
}

pub(super) fn enemy_plan(enemy: &EnemyTarget, target_mod: Option<String>) -> attributes::Plan {
    attributes::enemy(enemy.row, enemy.title(), &enemy.source, target_mod)
}

fn enemies(items: &mut Vec<Item>, enemy: &EnemyTarget) {
    match enemy.active_mod.as_deref() {
        Some(active) => items.push(Item::new(
            format!("Modify \"{}\" in \"{active}\"", enemy.file),
            Action::ModifyAttributes(enemy_plan(enemy, Some(active.to_owned()))),
        )),
        None => items.push(Item::disabled(format!("Modify \"{}\" in Mod", enemy.file), NEEDS_MOD)),
    }

    if enemy.unlocked && enemy.active_mod.is_none() {
        items.push(
            Item::new(
                format!("Modify \"{}\" in game", enemy.file),
                Action::ModifyAttributes(enemy_plan(enemy, None)),
            )
            .confirming(),
        );
    }
}

fn cats(items: &mut Vec<Item>, cat: &CatTarget) {
    match cat.active_mod.as_deref() {
        Some(active) => items.push(Item::new(
            format!("Modify \"{}\" in \"{active}\"", cat.file),
            Action::ModifyAttributes(cat_plan(cat, Some(active.to_owned()))),
        )),
        None => items.push(Item::disabled(format!("Modify \"{}\" in Mod", cat.file), NEEDS_MOD)),
    }

    if cat.unlocked && cat.active_mod.is_none() {
        items.push(
            Item::new(
                format!("Modify \"{}\" in game", cat.file),
                Action::ModifyAttributes(cat_plan(cat, None)),
            )
            .confirming(),
        );
    }
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

    if file.unlocked && file.active_mod.is_none() {
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
