//! Native menus (UX.md §1, FEASIBILITY.md §1).
//!
//! The structure is plain data (`spec`), built only from features that exist
//! in this build: items for later batches (split, consolidate, zoom keys,
//! create track, time commands, …) are added with their batches, never as
//! placeholders, because a native menu can't say why an item does nothing.
//! The shortcut text comes from the one shortcut table (`shortcuts.rs`).
//!
//! The muda part (`native`, feature `menus`) is standalone-only: a plugin
//! window has no menu bar, and every command also has an in-app home.
//! - Windows: the in-window menu bar. No accelerators are registered with
//!   muda (they double-fire, FEASIBILITY.md §1): items show the key as text
//!   and our own key path handles it.
//! - macOS: the system menu bar with the App and Window menus. Matching keys
//!   fire the menu item and never reach the window, so the accelerators are
//!   registered there (compiled here, tested in the Mac pass).

use crate::shortcuts::{self, Action};

/// What a menu item does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cmd {
    Action(Action),
    NewFromMidi,
    ImportAudio,
    ProjectSettings,
    OpenRecent(usize),
    ClearRecent,
    ClockBars,
    About,
    ShortcutSheet,
}

impl Cmd {
    pub fn id(self) -> String {
        match self {
            Cmd::Action(a) => shortcuts::get(a).id.to_string(),
            Cmd::NewFromMidi => "file.new_from_midi".into(),
            Cmd::ImportAudio => "file.import_audio".into(),
            Cmd::ProjectSettings => "file.project_settings".into(),
            Cmd::OpenRecent(i) => format!("file.recent.{i}"),
            Cmd::ClearRecent => "file.recent.clear".into(),
            Cmd::ClockBars => "view.clock_bars".into(),
            Cmd::About => "help.about".into(),
            Cmd::ShortcutSheet => "help.shortcuts".into(),
        }
    }

    pub fn from_id(id: &str) -> Option<Cmd> {
        if let Some(i) = id.strip_prefix("file.recent.").and_then(|n| n.parse().ok()) {
            return Some(Cmd::OpenRecent(i));
        }
        let fixed = [
            Cmd::NewFromMidi,
            Cmd::ImportAudio,
            Cmd::ProjectSettings,
            Cmd::ClearRecent,
            Cmd::ClockBars,
            Cmd::About,
            Cmd::ShortcutSheet,
        ];
        if let Some(c) = fixed.into_iter().find(|c| c.id() == id) {
            return Some(c);
        }
        shortcuts::TABLE.iter().find(|s| s.id == id).map(|s| Cmd::Action(s.action))
    }

    /// The item's text (without the shortcut).
    pub fn label(self) -> String {
        match self {
            Cmd::Action(Action::InsertClip) => "MIDI Clip".into(),
            Cmd::Action(Action::ToggleBrowser) => "Browser".into(),
            Cmd::Action(a) => shortcuts::get(a).name.into(),
            Cmd::NewFromMidi => "New from MIDI File…".into(),
            Cmd::ImportAudio => "Audio as DPCM Sample…".into(),
            Cmd::ProjectSettings => "Project Settings…".into(),
            Cmd::OpenRecent(i) => format!("Recent {i}"),
            Cmd::ClearRecent => "Clear Menu".into(),
            Cmd::ClockBars => "Clock Shows Bars".into(),
            Cmd::About => "About ChipDAW".into(),
            Cmd::ShortcutSheet => "Keyboard Shortcuts".into(),
        }
    }

    /// The shortcut text shown next to the item ("" if none).
    pub fn shortcut(self) -> String {
        match self {
            Cmd::Action(a) => shortcuts::label(a),
            _ => String::new(),
        }
    }
}

/// One entry of a menu.
#[derive(Clone, Debug, PartialEq)]
pub enum Entry {
    Item(Cmd),
    /// A checkable item.
    Check(Cmd),
    Separator,
    /// The Open Recent submenu (filled from settings).
    Recent,
    Sub(&'static str, Vec<Entry>),
    /// A predefined macOS item (About, Services, Hide, Quit, window items).
    MacPredefined(&'static str),
}

pub struct MenuSpec {
    pub title: &'static str,
    pub entries: Vec<Entry>,
}

/// The menu bar for a platform.
pub fn spec(mac: bool) -> Vec<MenuSpec> {
    use Entry::*;
    let a = |x: Action| Item(Cmd::Action(x));
    let mut file = vec![a(Action::New), a(Action::Open), Recent, Separator, a(Action::Save), a(Action::SaveAs), Separator];
    file.push(Item(Cmd::NewFromMidi));
    file.push(Sub("Import", vec![Item(Cmd::ImportAudio)]));
    file.push(a(Action::ExportAudio));
    file.push(Separator);
    file.push(Item(Cmd::ProjectSettings));
    if !mac {
        file.push(Separator);
        file.push(a(Action::Quit));
    }
    let mut edit = vec![
        a(Action::Undo),
        a(Action::Redo),
        Separator,
        a(Action::Cut),
        a(Action::Copy),
        a(Action::Paste),
        a(Action::Duplicate),
        a(Action::Delete),
        Separator,
        a(Action::CutTime),
        a(Action::CopyTime),
        a(Action::PasteTime),
        a(Action::DuplicateTime),
        a(Action::DeleteTime),
        a(Action::InsertSilence),
        Separator,
        a(Action::Split),
        a(Action::Consolidate),
        Separator,
        a(Action::SelectAll),
        a(Action::Rename),
        Separator,
        a(Action::Quantize),
    ];
    if !mac {
        edit.push(Separator);
        edit.push(a(Action::Settings));
    }
    let create = vec![a(Action::InsertClip)];
    let view = vec![
        Check(Cmd::Action(Action::ToggleBrowser)),
        Check(Cmd::Action(Action::Follow)),
        Separator,
        a(Action::ZoomIn),
        a(Action::ZoomOut),
        a(Action::ZoomToSelection),
        a(Action::ZoomBack),
        a(Action::ZoomToFit),
        Separator,
        Check(Cmd::Action(Action::DrawMode)),
        Separator,
        Check(Cmd::ClockBars),
    ];
    let mut help = vec![Item(Cmd::ShortcutSheet), a(Action::GettingStarted)];
    if !mac {
        help.push(Separator);
        help.push(Item(Cmd::About));
    }
    let mut out = Vec::new();
    if mac {
        out.push(MenuSpec {
            title: "ChipDAW",
            entries: vec![
                MacPredefined("about"),
                Separator,
                a(Action::Settings),
                Separator,
                MacPredefined("services"),
                Separator,
                MacPredefined("hide"),
                MacPredefined("hide_others"),
                MacPredefined("show_all"),
                Separator,
                a(Action::Quit),
            ],
        });
    }
    out.push(MenuSpec { title: if mac { "File" } else { "&File" }, entries: file });
    out.push(MenuSpec { title: if mac { "Edit" } else { "&Edit" }, entries: edit });
    out.push(MenuSpec { title: if mac { "Create" } else { "&Create" }, entries: create });
    out.push(MenuSpec { title: if mac { "View" } else { "&View" }, entries: view });
    if mac {
        out.push(MenuSpec {
            title: "Window",
            entries: vec![MacPredefined("minimize"), MacPredefined("zoom"), Separator, MacPredefined("bring_all_to_front")],
        });
    }
    out.push(MenuSpec { title: if mac { "Help" } else { "&Help" }, entries: help });
    out
}

/// Every command item in a spec (depth first).
pub fn commands(spec: &[MenuSpec]) -> Vec<Cmd> {
    fn walk(e: &[Entry], out: &mut Vec<Cmd>) {
        for x in e {
            match x {
                Entry::Item(c) | Entry::Check(c) => out.push(*c),
                Entry::Sub(_, v) => walk(v, out),
                _ => {}
            }
        }
    }
    let mut out = Vec::new();
    for m in spec {
        walk(&m.entries, &mut out);
    }
    out
}

/// What the menu bar shows that changes while the app runs.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct MenuState {
    pub can_undo: bool,
    pub can_redo: bool,
    /// The piano roll is open (Cut / Copy / Paste / Quantize act on notes).
    pub piano: bool,
    pub notes_selected: bool,
    pub can_paste: bool,
    /// Clips (or notes in the piano roll) are selected.
    pub selection: bool,
    pub time_selection: bool,
    pub browser: bool,
    pub follow: bool,
    pub clock_bars: bool,
    /// Open Recent, newest first (display text, path).
    pub recent: Vec<String>,
    /// Batch 3: something to paste in the active editor, a time range (time
    /// selection or clips), exactly one clip or a track to rename, a
    /// selection Split / Consolidate can act on, an earlier zoom, Draw Mode.
    pub can_paste_time: bool,
    pub range: bool,
    pub can_rename: bool,
    pub can_split: bool,
    pub can_consolidate: bool,
    pub can_zoom_back: bool,
    pub draw: bool,
}

impl MenuState {
    /// Whether an item is enabled.
    pub fn enabled(&self, c: Cmd) -> bool {
        match c {
            Cmd::Action(Action::Undo) => self.can_undo,
            Cmd::Action(Action::Redo) => self.can_redo,
            Cmd::Action(Action::Quantize) => self.piano && self.notes_selected,
            Cmd::Action(Action::Cut | Action::Copy) if self.piano => self.notes_selected,
            Cmd::Action(Action::Cut | Action::Copy | Action::Duplicate | Action::Delete) => self.selection || (!self.piano && self.time_selection),
            Cmd::Action(Action::Paste) => self.can_paste,
            Cmd::Action(Action::InsertClip | Action::InsertSilence) => !self.piano && self.time_selection,
            Cmd::Action(Action::CutTime | Action::CopyTime | Action::DuplicateTime | Action::DeleteTime) => !self.piano && self.range,
            Cmd::Action(Action::PasteTime) => !self.piano && self.can_paste_time,
            Cmd::Action(Action::Rename) => self.can_rename,
            Cmd::Action(Action::Split) => self.can_split,
            Cmd::Action(Action::Consolidate) => self.can_consolidate,
            Cmd::Action(Action::ZoomBack) => self.can_zoom_back,
            Cmd::Action(Action::ZoomToSelection) => self.range || (self.piano && self.notes_selected) || self.piano,
            Cmd::Action(Action::ZoomToFit) => !self.piano,
            Cmd::ClearRecent => !self.recent.is_empty(),
            _ => true,
        }
    }

    pub fn checked(&self, c: Cmd) -> bool {
        match c {
            Cmd::Action(Action::ToggleBrowser) => self.browser,
            Cmd::Action(Action::Follow) => self.follow,
            Cmd::Action(Action::DrawMode) => self.draw,
            Cmd::ClockBars => self.clock_bars,
            _ => false,
        }
    }
}

#[cfg(feature = "menus")]
pub use native::NativeMenus;

#[cfg(feature = "menus")]
mod native {
    use super::*;
    use muda::{CheckMenuItem, IsMenuItem, Menu, MenuItem, PredefinedMenuItem, Submenu};
    use std::collections::HashMap;

    /// The live menu bar and the items whose state changes.
    pub struct NativeMenus {
        menu: Menu,
        items: HashMap<String, (Cmd, MenuItem)>,
        checks: HashMap<String, (Cmd, CheckMenuItem)>,
        recent: Submenu,
        shown: MenuState,
    }

    fn text(c: Cmd, mac: bool) -> String {
        let sc = c.shortcut();
        // Windows: the key as text after a tab (muda right-aligns it); no
        // accelerator table. macOS shows the registered accelerator.
        if mac || sc.is_empty() {
            c.label()
        } else {
            format!("{}\t{sc}", c.label())
        }
    }

    fn accel(c: Cmd, mac: bool) -> Option<muda::accelerator::Accelerator> {
        match c {
            Cmd::Action(a) if mac => shortcuts::accelerator(a).and_then(|s| s.parse().ok()),
            _ => None,
        }
    }

    impl NativeMenus {
        pub fn build(state: &MenuState) -> Self {
            let mac = shortcuts::MACOS;
            let menu = Menu::new();
            let mut items = HashMap::new();
            let mut checks = HashMap::new();
            let recent = Submenu::new("Open Recent", true);
            fn fill(
                e: &[Entry],
                parent: &dyn Fn(&dyn IsMenuItem),
                items: &mut HashMap<String, (Cmd, MenuItem)>,
                checks: &mut HashMap<String, (Cmd, CheckMenuItem)>,
                recent: &Submenu,
                state: &MenuState,
                mac: bool,
            ) {
                for x in e {
                    match x {
                        Entry::Item(c) => {
                            let it = MenuItem::with_id(c.id(), text(*c, mac), state.enabled(*c), accel(*c, mac));
                            parent(&it);
                            items.insert(c.id(), (*c, it));
                        }
                        Entry::Check(c) => {
                            let it = CheckMenuItem::with_id(c.id(), text(*c, mac), true, state.checked(*c), accel(*c, mac));
                            parent(&it);
                            checks.insert(c.id(), (*c, it));
                        }
                        Entry::Separator => parent(&PredefinedMenuItem::separator()),
                        Entry::Recent => parent(recent),
                        Entry::Sub(title, v) => {
                            let sub = Submenu::new(*title, true);
                            fill(v, &|i| { let _ = sub.append(i); }, items, checks, recent, state, mac);
                            parent(&sub);
                        }
                        Entry::MacPredefined(which) => {
                            let p = match *which {
                                "about" => PredefinedMenuItem::about(Some("About ChipDAW"), None),
                                "services" => PredefinedMenuItem::services(None),
                                "hide" => PredefinedMenuItem::hide(None),
                                "hide_others" => PredefinedMenuItem::hide_others(None),
                                "show_all" => PredefinedMenuItem::show_all(None),
                                "minimize" => PredefinedMenuItem::minimize(None),
                                "zoom" => PredefinedMenuItem::maximize(Some("Zoom")),
                                _ => PredefinedMenuItem::bring_all_to_front(None),
                            };
                            parent(&p);
                        }
                    }
                }
            }
            for m in spec(mac) {
                let sub = Submenu::new(m.title, true);
                fill(&m.entries, &|i| { let _ = sub.append(i); }, &mut items, &mut checks, &recent, state, mac);
                let _ = menu.append(&sub);
            }
            let mut me = Self { menu, items, checks, recent, shown: MenuState::default() };
            me.update(state);
            me
        }

        /// Windows: attaches the bar to the main window's HWND.
        #[cfg(target_os = "windows")]
        pub fn attach_hwnd(&self, hwnd: isize) -> Result<(), String> {
            // SAFETY: `hwnd` is the live main window; the menu (owned by the
            // app model) outlives it.
            unsafe { self.menu.init_for_hwnd_with_theme(hwnd, muda::MenuTheme::Dark) }.map_err(|e| e.to_string())
        }

        /// macOS: the app's menu bar, replacing winit's default one. Call
        /// after the event loop has started (from a vizia event).
        #[cfg(target_os = "macos")]
        pub fn attach_nsapp(&self) {
            self.menu.init_for_nsapp();
        }

        /// Applies a changed state: enabled items, check marks, Open Recent.
        pub fn update(&mut self, state: &MenuState) {
            if *state == self.shown {
                return;
            }
            for (c, it) in self.items.values() {
                let on = state.enabled(*c);
                if on != self.shown.enabled(*c) || self.shown == MenuState::default() {
                    it.set_enabled(on);
                }
            }
            for (c, it) in self.checks.values() {
                it.set_checked(state.checked(*c));
            }
            if state.recent != self.shown.recent || self.recent.items().is_empty() {
                for _ in 0..self.recent.items().len() {
                    self.recent.remove_at(0);
                }
                if state.recent.is_empty() {
                    let _ = self.recent.append(&MenuItem::new("No recent projects", false, None));
                } else {
                    for (i, label) in state.recent.iter().enumerate() {
                        let _ = self.recent.append(&MenuItem::with_id(Cmd::OpenRecent(i).id(), label, true, None));
                    }
                }
                let _ = self.recent.append(&PredefinedMenuItem::separator());
                let _ = self.recent.append(&MenuItem::with_id(Cmd::ClearRecent.id(), "Clear Menu", !state.recent.is_empty(), None));
            }
            self.shown = state.clone();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// AT-06 [unit]: every menu item's shortcut text is the shortcut table's,
    /// and every item's id round-trips.
    #[test]
    fn menu_shortcuts_come_from_the_table() {
        for mac in [false, true] {
            for c in commands(&spec(mac)) {
                assert_eq!(Cmd::from_id(&c.id()), Some(c), "{}", c.id());
                if let Cmd::Action(a) = c {
                    let s = shortcuts::get(a);
                    let want = s.keys().first().map(|k| shortcuts::chord_text(k, shortcuts::MACOS)).unwrap_or_default();
                    assert_eq!(c.shortcut(), want, "{}", s.id);
                    assert!(!s.dev, "dev actions never appear in a menu: {}", s.id);
                }
            }
        }
    }

    /// Only features that exist: no item for a later batch (UX.md §7).
    #[test]
    fn no_placeholders() {
        let ids: Vec<String> = commands(&spec(false)).into_iter().map(|c| c.id()).collect();
        // Batch 4 (the model change) and later: tracks, time signatures,
        // automation, MIDI clip export.
        for later in ["track", "time_signature", "automation", "export_midi"] {
            assert!(!ids.iter().any(|i| i.contains(later)), "{later} belongs to a later batch");
        }
        // Batch 3's commands are all there (F-07, F-08, F-11, F-12, F-22, F-23, F-14).
        for b3 in ["edit.split", "edit.consolidate", "edit.rename", "edit.cut_time", "edit.copy_time", "edit.paste_time", "edit.duplicate_time", "edit.delete_time", "edit.insert_silence", "view.zoom_in", "view.zoom_out", "view.zoom_selection", "view.zoom_back", "view.zoom_fit", "view.draw_mode", "help.getting_started"] {
            assert!(ids.iter().any(|i| i == b3), "{b3} is missing from the menus");
        }
        // The Windows conventions: Exit under File, Settings under Edit, About under Help.
        let win = spec(false);
        assert!(win[0].entries.contains(&Entry::Item(Cmd::Action(Action::Quit))));
        assert!(win[1].entries.contains(&Entry::Item(Cmd::Action(Action::Settings))));
        assert!(win.last().unwrap().entries.contains(&Entry::Item(Cmd::About)));
        // macOS: the App menu first, a Window menu, no Exit in File.
        let mac = spec(true);
        assert_eq!(mac[0].title, "ChipDAW");
        assert!(mac.iter().any(|m| m.title == "Window"));
        assert!(!mac[1].entries.contains(&Entry::Item(Cmd::Action(Action::Quit))));
    }

    #[test]
    fn enabled_states() {
        let s = MenuState::default();
        assert!(!s.enabled(Cmd::Action(Action::Undo)));
        assert!(!s.enabled(Cmd::Action(Action::Paste)));
        assert!(s.enabled(Cmd::Action(Action::Save)));
        let s = MenuState { can_undo: true, piano: true, can_paste: true, ..Default::default() };
        assert!(s.enabled(Cmd::Action(Action::Undo)) && s.enabled(Cmd::Action(Action::Paste)));
    }
}
