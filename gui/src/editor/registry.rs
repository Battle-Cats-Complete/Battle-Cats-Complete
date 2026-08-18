use std::path::Path;

use core::domains::import::architecture;

use crate::app::Page;
use crate::common::feedback::LOCKED_NOTICE;

use super::{attributes, Action, CatTarget, Context, EnemyTarget, FileTarget, IconTarget, Item};

struct Mount<'a> {
    name: &'a str,
    target: Option<&'a str>,
    locked: bool,
}

fn mount<'a>(active_mod: Option<&'a str>, unlocked: bool) -> Mount<'a> {
    match active_mod {
        Some(name) => Mount { name, target: Some(name), locked: false },
        None => Mount { name: architecture::GAME, target: None, locked: !unlocked },
    }
}

impl Mount<'_> {
    fn item(&self, label: String, action: Action, destructive: bool) -> Item {
        if self.locked {
            return Item::disabled(label, LOCKED_NOTICE);
        }

        let item = Item::new(label, action);

        if destructive || self.target.is_none() { item.confirming() } else { item }
    }
}

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
        && let Some(cat) = context.cat.as_ref()
    {
        cats(&mut items, cat);
    }

    if context.page == Page::Enemies
        && let Some(enemy) = context.enemy.as_ref()
    {
        enemies(&mut items, enemy);
    }

    if let Some(icon) = context.icon.as_ref() {
        icons(&mut items, icon);
    }

    items
}

fn remove(file: &str, mount: &Mount<'_>, present: Option<&Path>) -> Item {
    let label = format!("Remove \"{file}\" from \"{}\"", mount.name);

    if mount.locked {
        return Item::disabled(label, LOCKED_NOTICE);
    }

    let Some(path) = present else {
        return Item::disabled(label, format!("File does not exist in \"{}\"", mount.name));
    };

    Item::new(label, Action::DeleteFile { source: path.to_path_buf() }).confirming()
}

fn sync(file: &str, game: Option<&Path>, active_mod: Option<&str>, shadowed: bool) -> Option<Item> {
    let active = active_mod?;
    let game = game?;

    let item = Item::new(
        format!("Sync \"{file}\" with game in \"{active}\""),
        Action::SyncWithGame {
            file: file.to_owned(),
            target_mod: active.to_owned(),
            game: game.to_path_buf(),
        },
    );

    Some(if shadowed { item.confirming() } else { item })
}

fn icons(items: &mut Vec<Item>, icon: &IconTarget) {
    let mount = mount(icon.active_mod.as_deref(), icon.unlocked);

    items.push(mount.item(
        format!("Replace \"{}\" in \"{}\"", icon.file, mount.name),
        Action::ReplaceIcon {
            file: icon.file.clone(),
            target_mod: mount.target.map(str::to_owned),
            game: icon.game.clone(),
        },
        icon.mod_copy.is_some(),
    ));

    items.extend(sync(&icon.file, icon.game.as_deref(), icon.active_mod.as_deref(), icon.mod_copy.is_some()));
    items.push(remove(&icon.file, &mount, present(&mount, icon.mod_copy.as_deref(), icon.game.as_deref())));
}

fn present<'a>(mount: &Mount<'_>, mod_copy: Option<&'a Path>, game: Option<&'a Path>) -> Option<&'a Path> {
    if mount.target.is_some() { mod_copy } else { game }
}

pub(super) fn cat_plan(cat: &CatTarget, target_mod: Option<String>) -> attributes::Plan {
    attributes::cat(cat.form, cat.title(), &cat.source, target_mod)
}

pub(super) fn enemy_plan(enemy: &EnemyTarget, target_mod: Option<String>) -> attributes::Plan {
    attributes::enemy(enemy.row, enemy.title(), &enemy.source, target_mod)
}

fn cats(items: &mut Vec<Item>, cat: &CatTarget) {
    let mount = mount(cat.active_mod.as_deref(), cat.unlocked);

    items.push(mount.item(
        format!("Edit \"{}\" in \"{}\"", cat.file, mount.name),
        Action::ModifyAttributes(cat_plan(cat, mount.target.map(str::to_owned))),
        false,
    ));

    items.extend(sync(&cat.file, Some(&cat.source), cat.active_mod.as_deref(), cat.mod_copy.is_some()));
    items.push(remove(&cat.file, &mount, present(&mount, cat.mod_copy.as_deref(), Some(&cat.source))));
}

fn enemies(items: &mut Vec<Item>, enemy: &EnemyTarget) {
    let mount = mount(enemy.active_mod.as_deref(), enemy.unlocked);

    items.push(mount.item(
        format!("Edit \"{}\" in \"{}\"", enemy.file, mount.name),
        Action::ModifyAttributes(enemy_plan(enemy, mount.target.map(str::to_owned))),
        false,
    ));

    items.extend(sync(&enemy.file, Some(&enemy.source), enemy.active_mod.as_deref(), enemy.mod_copy.is_some()));
    items.push(remove(&enemy.file, &mount, present(&mount, enemy.mod_copy.as_deref(), Some(&enemy.source))));
}

fn files(items: &mut Vec<Item>, file: &FileTarget) {
    if file.folder {
        return;
    }

    if file.mount != architecture::GAME {
        items.extend(sync(&file.name, file.game.as_deref(), file.active_mod.as_deref(), true));
        items.push(delete(file));

        return;
    }

    let mount = mount(file.active_mod.as_deref(), file.unlocked);

    if let Some(active) = mount.target {
        items.push(mount.item(
            format!("Add \"{}\" to \"{active}\"", file.name),
            Action::AddFileToMod { source: file.source.clone(), target_mod: active.to_owned() },
            file.mod_copy.is_some(),
        ));

        return;
    }

    if mount.locked {
        items.push(Item::disabled(
            format!("Remove \"{}\" from \"{}\"", file.name, file.mount),
            LOCKED_NOTICE,
        ));

        return;
    }

    items.push(delete(file));
}

fn delete(file: &FileTarget) -> Item {
    Item::new(
        format!("Remove \"{}\" from \"{}\"", file.name, file.mount),
        Action::DeleteFile { source: file.source.clone() },
    )
    .confirming()
}
