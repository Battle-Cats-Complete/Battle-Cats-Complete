use std::path::Path;

use core::domains::import::architecture;

use crate::app::{theme, Page};
use crate::common::feedback::LOCKED_NOTICE;

use super::{attributes, prose, Action, CatTarget, Context, EnemyTarget, Exception, FileTarget, IconTarget, Item, ProseTarget, Scope};

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

    if let Some(target) = context.prose.as_ref() {
        prose(&mut items, target);
    }

    items
}

fn replace(scope: &Scope<'_>, mount: &Mount<'_>) -> Item {
    let label = format!("Replace \"{}\" in \"{}\"", scope.name, mount.name);

    if mount.locked {
        return Item::disabled(label, LOCKED_NOTICE);
    }

    let Some(target_mod) = mount.target else {
        let Some(path) = scope.present else {
            return Item::disabled(label, format!("File does not exist in \"{}\"", mount.name));
        };

        let action = Action::ReplaceIcon { file: scope.name.to_owned(), target_mod: None, game: Some(path.to_path_buf()) };

        return Item::new(label, action).confirming();
    };

    let action =
        Action::ReplaceIcon { file: scope.name.to_owned(), target_mod: Some(target_mod.to_owned()), game: None };

    let item = Item::new(label, action);

    if scope.present.is_some() { item.confirming() } else { item }
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

fn option(label: impl Fn(&str) -> String, targets: Vec<Item>) -> Option<Item> {
    let mut targets = targets;

    match targets.len() {
        0 => None,
        1 => targets.pop().map(|only| {
            let named = label(&only.label);

            only.relabel(named)
        }),
        _ => Some(Item::list(label("..."), targets)),
    }
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

pub(super) fn prose_plans(target: &ProseTarget) -> Vec<prose::Plan> {
    let mount = mount(target.active_mod.as_deref(), target.unlocked);

    plans(target, &target.asset.scopes(mount.target.is_some()), mount.target)
}

fn plans(target: &ProseTarget, scopes: &[Scope<'_>], target_mod: Option<&str>) -> Vec<prose::Plan> {
    scopes
        .iter()
        .filter_map(|scope| {
            let source = scope.source.or(scope.present)?;

            Some(prose::plan(
                target.subject,
                target.row,
                [scope.name, target.label.as_str()].join(theme::HEADER_SEPARATOR),
                scope.name.to_owned(),
                source,
                target_mod.map(str::to_owned),
            ))
        })
        .collect()
}

fn prose(items: &mut Vec<Item>, target: &ProseTarget) {
    let mount = mount(target.active_mod.as_deref(), target.unlocked);
    let scopes = target.asset.scopes(mount.target.is_some());

    let edits = plans(target, &scopes, mount.target)
        .into_iter()
        .map(|plan| {
            let name = plan.file().to_owned();

            mount.item(name, Action::EditProse(plan), false)
        })
        .collect();

    items.extend(option(|name| format!("Edit \"{name}\" in \"{}\"", mount.name), edits));

    upkeep(items, &scopes, &mount);
}

fn upkeep(items: &mut Vec<Item>, scopes: &[Scope<'_>], mount: &Mount<'_>) {
    let syncs = scopes
        .iter()
        .filter_map(|scope| {
            sync(scope.name, scope.source, mount.target, scope.present.is_some())
                .map(|item| item.relabel(scope.name.to_owned()))
        })
        .collect();

    items.extend(option(|name| format!("Sync \"{name}\" with game in \"{}\"", mount.name), syncs));

    let removes =
        scopes.iter().map(|scope| remove(scope.name, mount, scope.present).relabel(scope.name.to_owned())).collect();

    items.extend(option(|name| format!("Remove \"{name}\" from \"{}\"", mount.name), removes));
}

fn icons(items: &mut Vec<Item>, icon: &IconTarget) {
    let mount = mount(icon.active_mod.as_deref(), icon.unlocked);

    entries(items, &icon.asset, &mount);
}

fn entries(items: &mut Vec<Item>, exception: &Exception, mount: &Mount<'_>) {
    let scopes = exception.scopes(mount.target.is_some());

    let replaces = scopes.iter().map(|scope| replace(scope, mount).relabel(scope.name.to_owned())).collect();

    items.extend(option(|name| format!("Replace \"{name}\" in \"{}\"", mount.name), replaces));

    upkeep(items, &scopes, mount);
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
