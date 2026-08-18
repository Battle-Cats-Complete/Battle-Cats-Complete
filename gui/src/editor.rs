mod menu;
mod registry;
mod attributes;
mod prose;
mod target;
mod watch;

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use iced::{Element, Point, Size, Task};
use tracing::{info, trace, warn};

use core::domains::cat::files as cat_files;
use core::domains::cat::waiter as cat_waiter;
use core::domains::enemy::files as enemy_files;
use core::domains::enemy::scanner::EnemyEntry;
use core::domains::import::architecture;
use core::domains::mods;

use crate::app::{theme, BattleCatsApp, Page};
use crate::domains::cat::DetailTab;
use crate::domains::enemy::DetailTab as EnemyTab;
use crate::common::feedback::Slot;

pub(crate) use target::{suppress, target};
pub(crate) use watch::watch;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Target {
    FileRow(usize),
    CatAttributes,
    EnemyAttributes,
    CatIcon,
    EnemyIcon,
    CatExplanation,
    EnemyName,
    EnemyDescription,
}

pub(crate) struct Context {
    enabled: bool,
    page: Page,
    file: Option<FileTarget>,
    cat: Option<CatTarget>,
    enemy: Option<EnemyTarget>,
    icon: Option<IconTarget>,
    prose: Option<ProseTarget>,
}

struct AssetFile {
    name: String,
    game: Option<PathBuf>,
    mod_copy: Option<PathBuf>,
}

fn asset_files(app: &BattleCatsApp, base: &str) -> Vec<AssetFile> {
    app.vault
        .vfs
        .variants(base)
        .into_iter()
        .map(|name| {
            let game = app.vault.vfs.rooted_in(architecture::GAME, &name);
            let mod_copy = mod_copy(app, &name);

            AssetFile { name, game, mod_copy }
        })
        .collect()
}

struct Exception {
    internal: String,
    vanilla: Option<PathBuf>,
    mod_copy: Option<PathBuf>,
    variants: Vec<AssetFile>,
}

struct Scope<'a> {
    name: &'a str,
    source: Option<&'a Path>,
    present: Option<&'a Path>,
}

fn exception(app: &BattleCatsApp, resolved: &Path) -> Option<Exception> {
    let visible = resolved.file_name()?.to_string_lossy().into_owned();
    let internal = app.vault.vfs.base_name(&visible).unwrap_or_else(|| visible.clone());
    let variants = asset_files(app, &internal);

    let vanilla = app
        .vault
        .vfs
        .rooted_in(architecture::GAME, &visible)
        .or_else(|| variants.iter().find_map(|file| file.game.clone()));

    Some(Exception { mod_copy: mod_copy(app, &internal), variants, vanilla, internal })
}

impl Exception {
    fn scopes(&self, in_mod: bool) -> Vec<Scope<'_>> {
        if in_mod {
            return vec![Scope { name: &self.internal, source: self.vanilla.as_deref(), present: self.mod_copy.as_deref() }];
        }

        self.variants
            .iter()
            .map(|file| Scope { name: &file.name, source: file.game.as_deref(), present: file.game.as_deref() })
            .collect()
    }
}

enum Asset {
    Variants(Vec<AssetFile>),
    Exception(Exception),
}

impl Asset {
    fn scopes(&self, in_mod: bool) -> Vec<Scope<'_>> {
        match self {
            Asset::Exception(exception) => exception.scopes(in_mod),
            Asset::Variants(files) => files
                .iter()
                .map(|file| Scope {
                    name: &file.name,
                    source: file.game.as_deref(),
                    present: if in_mod { file.mod_copy.as_deref() } else { file.game.as_deref() },
                })
                .collect(),
        }
    }
}

struct ProseTarget {
    subject: prose::Subject,
    asset: Asset,
    label: String,
    row: usize,
    unlocked: bool,
    active_mod: Option<String>,
}

struct IconTarget {
    asset: Exception,
    unlocked: bool,
    active_mod: Option<String>,
}

const ENEMY_HEADER_ROWS: usize = 2;

struct EnemyTarget {
    file: String,
    mod_copy: Option<PathBuf>,
    name: Option<String>,
    source: PathBuf,
    row: usize,
    unlocked: bool,
    active_mod: Option<String>,
}

impl EnemyTarget {
    fn title(&self) -> String {
        match self.name.as_deref() {
            Some(name) if !name.is_empty() => [name, self.file.as_str()].join(theme::HEADER_SEPARATOR),
            _ => self.file.clone(),
        }
    }
}

const FORM_LABELS: [&str; 4] = ["Normal", "Evolved", "True", "Ultra"];

struct CatTarget {
    file: String,
    mod_copy: Option<PathBuf>,
    name: Option<String>,
    source: PathBuf,
    form: usize,
    unlocked: bool,
    active_mod: Option<String>,
}

impl CatTarget {
    fn title(&self) -> String {
        let form = FORM_LABELS.get(self.form).copied().unwrap_or("Unknown");

        match self.name.as_deref() {
            Some(name) => [name, form, self.file.as_str()].join(theme::HEADER_SEPARATOR),
            None => [self.file.as_str(), form].join(theme::HEADER_SEPARATOR),
        }
    }
}

struct FileTarget {
    source: PathBuf,
    game: Option<PathBuf>,
    name: String,
    mount: String,
    folder: bool,
    unlocked: bool,
    active_mod: Option<String>,
    mod_copy: Option<PathBuf>,
}

#[derive(Clone, Debug)]
pub enum Message {
    Opened(Point, Option<Target>),
    Dismissed,
    Invoked(usize),
    InvokedChild(usize, usize),
    Hovered(Option<usize>),
    ConfirmExpired,
    Attributes(attributes::Subject, attributes::Message),
    Prose(prose::Subject, prose::Message),
}

struct Item {
    label: String,
    hint: Option<String>,
    action: Option<Action>,
    children: Vec<Item>,
    confirm: bool,
}

impl Item {
    fn new(label: impl Into<String>, action: Action) -> Self {
        Self { label: label.into(), hint: None, action: Some(action), children: Vec::new(), confirm: false }
    }

    fn disabled(label: impl Into<String>, hint: impl Into<String>) -> Self {
        Self { label: label.into(), hint: Some(hint.into()), action: None, children: Vec::new(), confirm: false }
    }

    fn list(label: impl Into<String>, children: Vec<Item>) -> Self {
        Self { label: label.into(), hint: None, action: None, children, confirm: false }
    }

    fn opens(&self) -> bool {
        !self.children.is_empty()
    }

    fn live(&self) -> bool {
        self.action.is_some() || self.children.iter().any(|child| child.action.is_some())
    }

    fn relabel(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self
    }

    fn confirming(mut self) -> Self {
        self.confirm = true;
        self
    }
}

enum Action {
    AddFileToMod { source: PathBuf, target_mod: String },
    DeleteFile { source: PathBuf },
    ModifyAttributes(attributes::Plan),
    EditProse(prose::Plan),
    ReplaceIcon { file: String, target_mod: Option<String>, game: Option<PathBuf> },
    SyncWithGame { file: String, target_mod: String, game: PathBuf },
}

const ARM_STRIDE: usize = 1024;

fn arm_slot(index: usize, child: Option<usize>) -> usize {
    child.map_or(index, |child| ARM_STRIDE + index * ARM_STRIDE + child)
}

fn pick(items: &[Item], index: usize, child: Option<usize>) -> Option<&Item> {
    let item = items.get(index)?;

    match child {
        Some(child) => item.children.get(child),
        None => Some(item),
    }
}

#[derive(Default)]
pub(crate) struct State {
    open: Option<Open>,
    confirm: Slot<usize>,
    cats: attributes::State,
    enemies: attributes::State,
    prose: [prose::State; prose::COUNT],
}

pub(crate) struct Snapshot {
    page: Page,
    plan: Option<attributes::Plan>,
    prose: [Vec<prose::Plan>; prose::COUNT],
}

struct Open {
    at: Point,
    items: Vec<Item>,
    hovered: Option<usize>,
}

impl State {
    pub(crate) fn open(&mut self, at: Point, context: &Context) {
        let items = registry::items(context);

        if items.is_empty() {
            trace!(
                page = ?context.page,
                targeted = context.file.is_some(),
                nightly = context.enabled,
                "Right click produced no context menu actions"
            );
        }

        self.confirm.expire();
        self.open = (!items.is_empty()).then_some(Open { at, items, hovered: None });
    }

    pub(crate) fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Invoked(index) => self.invoke(index, None),
            Message::InvokedChild(parent, index) => self.invoke(parent, Some(index)),
            Message::Hovered(index) => {
                if let Some(open) = self.open.as_mut() {
                    open.hovered = index;
                }

                Task::none()
            }
            Message::Dismissed => {
                self.open = None;
                self.confirm.expire();
                Task::none()
            }
            Message::ConfirmExpired => {
                self.confirm.expire();
                Task::none()
            }
            Message::Prose(subject, msg) => self
                .prose_mut(subject)
                .update(msg)
                .map(move |inner| Message::Prose(subject, inner)),
            Message::Attributes(subject, msg) => self
                .subject_mut(subject)
                .update(msg)
                .map(move |inner| Message::Attributes(subject, inner)),
            Message::Opened(..) => Task::none(),
        }
    }

    pub(crate) fn popup_view(&self, app: &BattleCatsApp, window: Size) -> Option<Element<'_, Message>> {
        for subject in prose::SUBJECTS {
            if subject.page() != app.current_page || !prose_tab(app, subject) {
                continue;
            }

            if let Some(view) = self.prose[subject.slot()].view(window) {
                return Some(view.map(move |inner| Message::Prose(subject, inner)));
            }
        }

        let (state, subject) = match app.current_page {
            Page::Cats if app.cat_state.selected_tab == DetailTab::Abilities => {
                (&self.cats, attributes::Subject::Cat)
            }
            Page::Enemies if app.enemy_state.selected_tab == EnemyTab::Abilities => {
                (&self.enemies, attributes::Subject::Enemy)
            }
            _ => return None,
        };

        state
            .view(window)
            .map(|view| view.map(move |inner| Message::Attributes(subject, inner)))
    }

    pub(crate) fn sync(&mut self, snapshot: Snapshot) {
        for subject in prose::SUBJECTS {
            if subject.page() != snapshot.page {
                continue;
            }

            self.prose[subject.slot()].sync(&snapshot.prose[subject.slot()]);
        }

        match snapshot.page {
            Page::Cats => self.cats.sync(snapshot.plan),
            Page::Enemies => self.enemies.sync(snapshot.plan),
            _ => {}
        }
    }

    fn subject_mut(&mut self, subject: attributes::Subject) -> &mut attributes::State {
        match subject {
            attributes::Subject::Cat => &mut self.cats,
            attributes::Subject::Enemy => &mut self.enemies,
        }
    }

    fn prose_mut(&mut self, subject: prose::Subject) -> &mut prose::State {
        &mut self.prose[subject.slot()]
    }

    fn perform(&mut self, action: &Action) {
        match action {
            Action::AddFileToMod { source, target_mod } => match mods::adopt(target_mod, source) {
                Ok(path) => info!(path = %path.display(), "Added a file to a mod"),
                Err(err) => warn!(source = %source.display(), "Failed to add the file to the mod: {}", err),
            },
            Action::DeleteFile { source } => match fs::remove_file(source) {
                Ok(()) => info!(path = %source.display(), "Deleted a mod file"),
                Err(err) => warn!(path = %source.display(), "Failed to delete the file: {}", err),
            },
            Action::ModifyAttributes(plan) => self.subject_mut(plan.subject()).begin(plan.clone()),
            Action::EditProse(plan) => {
                let subject = plan.subject();

                for other in prose::SUBJECTS {
                    if other != subject && other.page() == subject.page() {
                        self.prose_mut(other).close();
                    }
                }

                self.prose_mut(subject).begin(plan.clone());
            }
            Action::ReplaceIcon { file, target_mod, game } => {
                replace_icon(file, target_mod.as_deref(), game.as_deref());
            }
            Action::SyncWithGame { file, target_mod, game } => match mods::place(target_mod, game, file) {
                Ok(path) => info!(path = %path.display(), "Synced a mod file with game"),
                Err(err) => warn!(file, "Failed to sync the file with game: {}", err),
            },
        }
    }

    fn invoke(&mut self, index: usize, child: Option<usize>) -> Task<Message> {
        let Some((actionable, confirms)) = self
            .open
            .as_ref()
            .and_then(|open| pick(&open.items, index, child))
            .map(|item| (item.action.is_some(), item.confirm))
        else {
            return Task::none();
        };

        if !actionable {
            return Task::none();
        }

        let slot = arm_slot(index, child);

        if confirms && !self.confirm.armed_for(&slot) {
            return self.confirm.set(slot, Message::ConfirmExpired);
        }

        self.confirm.expire();

        let Some(open) = self.open.take() else {
            return Task::none();
        };

        if let Some(action) = pick(&open.items, index, child).and_then(|item| item.action.as_ref()) {
            self.perform(action);
        }

        Task::none()
    }
}

pub(crate) fn context(app: &BattleCatsApp, target: Option<Target>) -> Context {
    Context {
        enabled: app.settings.general.enable_nightly,
        page: app.current_page,
        file: file_target(app, target),
        cat: matches!(target, Some(Target::CatAttributes)).then(|| cat_subject(app)).flatten(),
        enemy: matches!(target, Some(Target::EnemyAttributes)).then(|| enemy_subject(app)).flatten(),
        icon: icon_target(app, target),
        prose: prose_subject(target).and_then(|subject| prose_target(app, subject)),
    }
}

fn prose_tab(app: &BattleCatsApp, subject: prose::Subject) -> bool {
    match subject {
        prose::Subject::EnemyDescription => app.enemy_state.selected_tab == EnemyTab::Details,
        prose::Subject::Explanation | prose::Subject::EnemyName => true,
    }
}

fn prose_subject(target: Option<Target>) -> Option<prose::Subject> {
    match target? {
        Target::CatExplanation => Some(prose::Subject::Explanation),
        Target::EnemyName => Some(prose::Subject::EnemyName),
        Target::EnemyDescription => Some(prose::Subject::EnemyDescription),
        _ => None,
    }
}

fn prose_target(app: &BattleCatsApp, subject: prose::Subject) -> Option<ProseTarget> {
    match subject {
        prose::Subject::Explanation => explanation_target(app),
        prose::Subject::EnemyName => enemy_name_target(app),
        prose::Subject::EnemyDescription => enemy_description_target(app),
    }
}

fn mod_copy(app: &BattleCatsApp, file: &str) -> Option<PathBuf> {
    mods::find(app.mods_state.active_mod().as_deref()?, Path::new(file))
}

fn enemy_name_target(app: &BattleCatsApp) -> Option<ProseTarget> {
    let id = app.app_state.enemy.selected_enemy?;
    let resolved = app.vault.vfs.find(enemy_files::NAMES)?;

    Some(ProseTarget {
        subject: prose::Subject::EnemyName,
        asset: Asset::Exception(exception(app, &resolved)?),
        label: enemy_label(app, id),
        row: id as usize,
        unlocked: app.settings.files.unlock_game_mount,
        active_mod: app.mods_state.active_mod(),
    })
}

fn enemy_description_target(app: &BattleCatsApp) -> Option<ProseTarget> {
    let id = app.app_state.enemy.selected_enemy?;
    let files = asset_files(app, enemy_files::PICTURE_BOOK);

    if files.is_empty() {
        return None;
    }

    Some(ProseTarget {
        subject: prose::Subject::EnemyDescription,
        asset: Asset::Variants(files),
        label: enemy_label(app, id),
        row: id as usize,
        unlocked: app.settings.files.unlock_game_mount,
        active_mod: app.mods_state.active_mod(),
    })
}

fn enemy_label(app: &BattleCatsApp, id: u32) -> String {
    app.enemy_state
        .data
        .enemies
        .iter()
        .find(|enemy| enemy.id == id)
        .map_or_else(|| format!("{id:03}-E"), EnemyEntry::display_name)
}

fn explanation_target(app: &BattleCatsApp) -> Option<ProseTarget> {
    let id = app.app_state.cat.selected_cat?;
    let form = app.app_state.cat.selected_form;

    let resolved = cat_waiter::unitexplanation_source(&app.vault.vfs, id, form)?;
    let file = resolved.file_name()?.to_string_lossy().into_owned();

    let files = asset_files(app, &cat_files::explanation_file(id));

    if files.is_empty() {
        return None;
    }

    let name = app
        .cat_state
        .data
        .cats
        .iter()
        .find(|cat| cat.id == id)
        .and_then(|cat| cat.names.get(form).cloned().flatten());

    let form_label = FORM_LABELS.get(form).copied().unwrap_or("Unknown");

    let label = match name.as_deref() {
        Some(name) if !name.is_empty() => [name, form_label, file.as_str()].join(theme::HEADER_SEPARATOR),
        _ => [file.as_str(), form_label].join(theme::HEADER_SEPARATOR),
    };

    Some(ProseTarget {
        subject: prose::Subject::Explanation,
        asset: Asset::Variants(files),
        label,
        row: form,
        unlocked: app.settings.files.unlock_game_mount,
        active_mod: app.mods_state.active_mod(),
    })
}

fn icon_target(app: &BattleCatsApp, target: Option<Target>) -> Option<IconTarget> {
    let resolved = match target? {
        Target::CatIcon => {
            let id = app.app_state.cat.selected_cat?;
            let cat = app.cat_state.data.cats.iter().find(|cat| cat.id == id)?;

            cat.deploy_icon_paths[app.app_state.cat.selected_form].as_ref()?
        }
        Target::EnemyIcon => {
            let id = app.app_state.enemy.selected_enemy?;
            let enemy = app.enemy_state.data.enemies.iter().find(|enemy| enemy.id == id)?;

            enemy.icon_path.as_ref()?
        }
        _ => return None,
    };

    Some(IconTarget {
        asset: exception(app, resolved)?,
        unlocked: app.settings.files.unlock_game_mount,
        active_mod: app.mods_state.active_mod(),
    })
}

fn replace_icon(file: &str, target_mod: Option<&str>, game: Option<&Path>) {
    let Some(source) = rfd::FileDialog::new().add_filter("PNG Image", &["png"]).pick_file() else {
        return;
    };

    let placed = match target_mod {
        Some(name) => mods::place(name, &source, file),
        None => match game {
            Some(path) => fs::copy(&source, path).map(|_| path.to_path_buf()),
            None => Err(io::Error::new(io::ErrorKind::NotFound, "the vanilla icon is missing")),
        },
    };

    match placed {
        Ok(path) => info!(path = %path.display(), "Replaced an icon"),
        Err(err) => warn!(file, "Failed to replace the icon: {}", err),
    }
}

fn cat_subject(app: &BattleCatsApp) -> Option<CatTarget> {
    if app.current_page != Page::Cats {
        return None;
    }

    let id = app.app_state.cat.selected_cat?;
    let file = cat_files::stats_file(id);
    let source = app.vault.vfs.rooted_in(architecture::GAME, &file)?;
    let form = app.app_state.cat.selected_form;

    let mod_copy = mod_copy(app, &file);

    let name = app
        .cat_state
        .data
        .cats
        .iter()
        .find(|cat| cat.id == id)
        .and_then(|cat| cat.names.get(form).cloned().flatten());

    Some(CatTarget {
        file,
        mod_copy,
        name,
        source,
        form,
        unlocked: app.settings.files.unlock_game_mount,
        active_mod: app.mods_state.active_mod(),
    })
}

fn enemy_subject(app: &BattleCatsApp) -> Option<EnemyTarget> {
    if app.current_page != Page::Enemies {
        return None;
    }

    let id = app.app_state.enemy.selected_enemy?;
    let file = enemy_files::STATS.to_owned();
    let source = app.vault.vfs.rooted_in(architecture::GAME, &file)?;

    let mod_copy = mod_copy(app, &file);

    let name = app
        .enemy_state
        .data
        .enemies
        .iter()
        .find(|enemy| enemy.id == id)
        .map(|enemy| enemy.name.clone());

    Some(EnemyTarget {
        file,
        mod_copy,
        name,
        source,
        row: ENEMY_HEADER_ROWS + id as usize,
        unlocked: app.settings.files.unlock_game_mount,
        active_mod: app.mods_state.active_mod(),
    })
}

pub(crate) fn snapshot(app: &BattleCatsApp) -> Snapshot {
    Snapshot {
        page: app.current_page,
        plan: current_plan(app),
        prose: prose::SUBJECTS.map(|subject| {
            if subject.page() != app.current_page {
                return Vec::new();
            }

            prose_target(app, subject).map(|target| registry::prose_plans(&target)).unwrap_or_default()
        }),
    }
}

fn current_plan(app: &BattleCatsApp) -> Option<attributes::Plan> {
    if let Some(cat) = cat_subject(app) {
        let active = cat.active_mod.clone();

        return Some(registry::cat_plan(&cat, active));
    }

    let enemy = enemy_subject(app)?;
    let active = enemy.active_mod.clone();

    Some(registry::enemy_plan(&enemy, active))
}

fn file_target(app: &BattleCatsApp, target: Option<Target>) -> Option<FileTarget> {
    let Some(Target::FileRow(index)) = target else {
        return None;
    };

    let mount = app.files_state.mount()?;
    let (folder, relative) = app.files_state.entry_at(&app.vault.vfs, index)?;
    let source = app.vault.vfs.root(mount)?.join(&relative);
    let name = relative.file_name()?.to_string_lossy().into_owned();
    let active_mod = app.mods_state.active_mod();

    let mod_copy = active_mod.as_deref().and_then(|active| mods::find(active, Path::new(&name)));

    let game = app.vault.vfs.rooted_in(architecture::GAME, &name);

    Some(FileTarget {
        source,
        game,
        name,
        mount: mount.to_owned(),
        folder,
        unlocked: app.settings.files.unlock_game_mount,
        active_mod,
        mod_copy,
    })
}
