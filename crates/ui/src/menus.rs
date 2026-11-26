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

/// Layout of the dark (owner-drawn) dropdown rows, in physical pixels for a
/// DPI. Platform-free so it is unit-tested everywhere; drawn by `dark`.
/// Distances are in 96-DPI pixels on the 4 px grid.
pub mod dark_layout {
    /// Row height of an item (at least; a taller menu font grows it).
    pub const ROW_H: i32 = 24;
    /// Space above and below the text inside a row.
    pub const TEXT_PAD_V: i32 = 4;
    /// Height of a separator row (a 1 px line in its middle).
    pub const SEP_H: i32 = 8;
    /// The left column holding the check mark.
    pub const GUTTER: i32 = 32;
    /// Minimum gap between an item's text and its shortcut.
    pub const ACCEL_GAP: i32 = 32;
    /// The right column holding the submenu arrow (also the right padding).
    pub const ARROW_COL: i32 = 24;
    /// The hover wash is inset this much from the popup's sides...
    pub const WASH_INSET: i32 = 4;
    /// ...with this corner radius.
    pub const WASH_RADIUS: i32 = 4;
    /// The check mark and the arrow are drawn in a square this big...
    pub const GLYPH: i32 = 16;
    /// ...whose left edge is this far into the gutter (centred: (32-16)/2).
    pub const GLYPH_X: i32 = 8;

    /// 0x00BBGGRR, as GDI wants it.
    pub const fn rgb(r: u8, g: u8, b: u8) -> u32 {
        r as u32 | (g as u32) << 8 | (b as u32) << 16
    }
    /// Colours: a dark neutral background, light text, a neutral light-grey
    /// wash for the hovered row (white at ~12 % over the background; ~6 % on
    /// a disabled row), separators. No accent colour.
    pub const BG: u32 = rgb(0x1A, 0x1A, 0x1A);
    pub const TEXT: u32 = rgb(0xE6, 0xE6, 0xE6);
    pub const TEXT_DISABLED: u32 = rgb(0x80, 0x80, 0x80);
    pub const WASH: u32 = rgb(0x35, 0x35, 0x35);
    pub const WASH_DISABLED: u32 = rgb(0x27, 0x27, 0x27);
    pub const SEPARATOR: u32 = rgb(0x32, 0x32, 0x32);

    /// A 96-DPI distance at `dpi` (rounded; 0 DPI counts as 96).
    pub fn px(v: i32, dpi: u32) -> i32 {
        let dpi = if dpi == 0 { 96 } else { dpi as i32 };
        (v * dpi + 48) / 96
    }

    /// The stroke width of the check mark and arrow (1.5 px at 96 DPI, at
    /// least 1).
    pub fn stroke(dpi: u32) -> i32 {
        (px(3, dpi) / 2).max(1)
    }

    /// Item text and shortcut as muda stores them: "Label\tShortcut".
    pub fn split_text(s: &str) -> (&str, &str) {
        match s.split_once('\t') {
            Some((a, b)) => (a, b),
            None => (s, ""),
        }
    }

    /// The size WM_MEASUREITEM reports for a row. `text_w` / `accel_w` /
    /// `text_h` are the measured text extents; `check_allowance` is what
    /// the system adds to an owner-drawn item's width on its own (the menu
    /// check-mark width minus one, SM_CXMENUCHECK - 1), taken back off so
    /// the popup isn't wider than drawn.
    pub fn item_size(separator: bool, text_w: i32, accel_w: i32, text_h: i32, check_allowance: i32, dpi: u32) -> (u32, u32) {
        if separator {
            return (0, px(SEP_H, dpi) as u32);
        }
        let h = px(ROW_H, dpi).max(text_h + 2 * px(TEXT_PAD_V, dpi));
        let mut w = px(GUTTER, dpi) + text_w + px(ARROW_COL, dpi);
        if accel_w > 0 {
            w += px(ACCEL_GAP, dpi) + accel_w;
        }
        ((w - check_allowance.max(0)).max(0) as u32, h as u32)
    }

    /// The check mark's polyline in a row (`left`, `top`, height `h`).
    pub fn check_points(left: i32, top: i32, h: i32, dpi: u32) -> [(i32, i32); 3] {
        let g = px(GLYPH, dpi);
        let x0 = left + px(GLYPH_X, dpi);
        let y0 = top + (h - g) / 2;
        // In a 16-unit box: (3.5, 8.5) -> (6.5, 11.5) -> (12.5, 4.5).
        let p = |x: i32, y: i32| (x0 + px(x, dpi) / 2, y0 + px(y, dpi) / 2);
        [p(7, 17), p(13, 23), p(25, 9)]
    }

    /// The submenu arrow (a chevron) at a row's right end.
    pub fn arrow_points(right: i32, top: i32, h: i32, dpi: u32) -> [(i32, i32); 3] {
        let g = px(GLYPH, dpi);
        let x0 = right - px(ARROW_COL, dpi) + (px(ARROW_COL, dpi) - g) / 2;
        let y0 = top + (h - g) / 2;
        let p = |x: i32, y: i32| (x0 + px(x, dpi), y0 + px(y, dpi));
        [p(6, 4), p(10, 8), p(6, 12)]
    }

    /// The key a row answers to while its menu is open: the letter after a
    /// single '&', else the first character (what the system does for a
    /// string item). Lower-cased.
    pub fn mnemonic(label: &str) -> Option<char> {
        let mut it = label.chars().peekable();
        while let Some(c) = it.next() {
            if c == '&' {
                match it.next() {
                    Some('&') => continue,
                    Some(m) => return m.to_lowercase().next(),
                    None => break,
                }
            }
        }
        label.chars().find(|c| !c.is_whitespace() && *c != '&').and_then(|c| c.to_lowercase().next())
    }

    /// WM_MENUCHAR: which row a key picks among `keys` (each row's mnemonic,
    /// `None` for separators and rows that can't be picked), starting after
    /// the highlighted row. `Some((row, unique))`: unique rows execute,
    /// shared ones only move the highlight.
    pub fn menu_char(keys: &[Option<char>], key: char, hilite: Option<usize>) -> Option<(usize, bool)> {
        let key = key.to_lowercase().next()?;
        let hits: Vec<usize> = (0..keys.len()).filter(|&i| keys[i] == Some(key)).collect();
        match hits.len() {
            0 => None,
            1 => Some((hits[0], true)),
            _ => {
                let next = hits.iter().copied().find(|&i| hilite.is_some_and(|h| i > h)).unwrap_or(hits[0]);
                Some((next, false))
            }
        }
    }
}

#[cfg(all(windows, feature = "menus"))]
#[path = "menus_dark.rs"]
mod dark;

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

        /// Windows: attaches the bar to the main window's HWND, and the
        /// owner-draw hook for the dark dropdowns (`dark`; it does nothing
        /// while Dark menus is off).
        #[cfg(target_os = "windows")]
        pub fn attach_hwnd(&self, hwnd: isize) -> Result<(), String> {
            // SAFETY: `hwnd` is the live main window; the menu (owned by the
            // app model) outlives it.
            unsafe { self.menu.init_for_hwnd_with_theme(hwnd, muda::MenuTheme::Dark) }.map_err(|e| e.to_string())?;
            // SAFETY: as above; installed after muda's subclass, so it sees
            // the menu messages first and passes on everything it doesn't own.
            if !unsafe { super::dark::install(hwnd) } {
                eprintln!("dark menus: SetWindowSubclass failed; dropdowns stay light");
            }
            Ok(())
        }

        /// Settings > Display > Dark menus. Windows: applies the next time a
        /// dropdown opens (each popup is re-flagged as it opens). Elsewhere
        /// nothing (macOS draws its own menus).
        pub fn set_dark(&self, on: bool) {
            #[cfg(target_os = "windows")]
            super::dark::set_enabled(on);
            #[cfg(not(target_os = "windows"))]
            let _ = on;
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

    /// The dark rows' measuring: DPI scaling, the shortcut column, the
    /// system's check allowance, separators, a tall font.
    #[test]
    fn dark_row_layout() {
        use dark_layout::*;
        assert_eq!((px(24, 96), px(24, 144), px(24, 192), px(24, 0)), (24, 36, 48, 24));
        // 96 DPI, 60 px text, 40 px shortcut, 16 px font, allowance 12.
        let (w, h) = item_size(false, 60, 40, 16, 12, 96);
        assert_eq!(w as i32, GUTTER + 60 + ACCEL_GAP + 40 + ARROW_COL - 12);
        assert_eq!(h as i32, ROW_H);
        // No shortcut: no gap.
        assert_eq!(item_size(false, 60, 0, 16, 0, 96).0 as i32, GUTTER + 60 + ARROW_COL);
        // 150 %: every fixed distance scales, the text is measured as is.
        let (w, h) = item_size(false, 90, 60, 24, 0, 144);
        assert_eq!((w, h), ((48 + 90 + 48 + 60 + 36) as u32, 36));
        // A tall menu font grows the row.
        assert_eq!(item_size(false, 10, 0, 30, 0, 96).1 as i32, 30 + 2 * TEXT_PAD_V);
        // Separators: their own height, no width of their own.
        assert_eq!(item_size(true, 99, 99, 99, 12, 120), (0, 10));
        // Every 96-DPI distance is on the 4 px grid.
        for d in [ROW_H, TEXT_PAD_V, SEP_H, GUTTER, ACCEL_GAP, ARROW_COL, WASH_INSET, WASH_RADIUS, GLYPH, GLYPH_X] {
            assert_eq!(d % 4, 0, "{d}");
        }
        assert_eq!(split_text("Save\tCtrl+S"), ("Save", "Ctrl+S"));
        assert_eq!(split_text("Import"), ("Import", ""));
        assert_eq!((stroke(96), stroke(144), stroke(192)), (1, 2, 3));
    }

    /// The check mark sits in the gutter and the arrow in the right column,
    /// both vertically inside the row, at any DPI.
    #[test]
    fn dark_glyphs_stay_in_their_columns() {
        use dark_layout::*;
        for dpi in [96, 120, 144, 192] {
            let (top, h) = (100, px(ROW_H, dpi));
            for (x, y) in check_points(0, top, h, dpi) {
                assert!((0..px(GUTTER, dpi)).contains(&x), "{dpi}: {x}");
                assert!((top..top + h).contains(&y), "{dpi}: {y}");
            }
            let right = 400;
            for (x, y) in arrow_points(right, top, h, dpi) {
                assert!((right - px(ARROW_COL, dpi)..right).contains(&x), "{dpi}: {x}");
                assert!((top..top + h).contains(&y), "{dpi}: {y}");
            }
        }
        // The colours are neutral greys (no accent): r == g == b.
        for c in [BG, TEXT, TEXT_DISABLED, WASH, WASH_DISABLED, SEPARATOR] {
            assert!(c & 0xFF == (c >> 8) & 0xFF && c & 0xFF == (c >> 16) & 0xFF, "{c:06x}");
        }
        assert_eq!(rgb(0x1A, 0x2B, 0x3C), 0x3C2B1A);
    }

    /// Letter keys in an open dark dropdown pick rows as the system does.
    #[test]
    fn dark_menu_letters() {
        use dark_layout::*;
        assert_eq!(mnemonic("&Save"), Some('s'));
        assert_eq!(mnemonic("Save &As…"), Some('a'));
        assert_eq!(mnemonic("Fish && Chips"), Some('f'));
        assert_eq!(mnemonic("Undo"), Some('u'));
        assert_eq!(mnemonic(""), None);
        let keys = [Some('n'), Some('o'), None, Some('s'), Some('s'), Some('e')];
        assert_eq!(menu_char(&keys, 'N', None), Some((0, true)));
        assert_eq!(menu_char(&keys, 'e', Some(0)), Some((5, true)));
        // Shared letters cycle through the matches without executing.
        assert_eq!(menu_char(&keys, 's', None), Some((3, false)));
        assert_eq!(menu_char(&keys, 's', Some(3)), Some((4, false)));
        assert_eq!(menu_char(&keys, 's', Some(4)), Some((3, false)));
        assert_eq!(menu_char(&keys, 'z', None), None);
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
