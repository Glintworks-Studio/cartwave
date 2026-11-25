//! The one shortcut table (UX.md F-30).
//!
//! Every keyboard action is defined once here: its id, its name, its keys on
//! Windows and on macOS, and the contexts where it applies. Key handling
//! (`lookup`), the native menus' shortcut text (`label`, `accelerator`) and
//! the Help ▸ Keyboard Shortcuts sheet all read this table, so a key can't
//! be changed in one place and stay stale in another. `tests::no_conflicts`
//! checks that no two actions share a key within any context.
//!
//! Contexts: an `App` action works wherever no text field is being edited and
//! no dialog is open. `Arrangement` and `PianoRoll` actions work while that
//! editor has keyboard focus. Edit commands that act on whichever editor has
//! focus (Delete, Duplicate, Select All, Cut/Copy/Paste) list both editors.
//! `Text` entries describe what a focused text field does with the key (the
//! field handles them itself; the app ignores every other shortcut while a
//! field is being edited, F-03). `Modal` entries apply while a dialog is open.

use vizia::prelude::{Code, Modifiers};

pub const MACOS: bool = cfg!(target_os = "macos");

/// A key with modifiers. `cmd` is Ctrl on Windows and ⌘ on macOS; `alt` is
/// Alt / ⌥ Option.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Chord {
    pub code: Code,
    pub cmd: bool,
    pub shift: bool,
    pub alt: bool,
}

const fn k(code: Code) -> Chord {
    Chord { code, cmd: false, shift: false, alt: false }
}
const fn cmd(code: Code) -> Chord {
    Chord { code, cmd: true, shift: false, alt: false }
}
const fn cmd_shift(code: Code) -> Chord {
    Chord { code, cmd: true, shift: true, alt: false }
}
const fn cmd_alt(code: Code) -> Chord {
    Chord { code, cmd: true, shift: false, alt: true }
}
const fn shift(code: Code) -> Chord {
    Chord { code, cmd: false, shift: true, alt: false }
}
const fn alt_shift(code: Code) -> Chord {
    Chord { code, cmd: false, shift: true, alt: true }
}

impl Chord {
    /// Whether a key press with these modifiers is this chord on this platform.
    pub fn matches(&self, code: Code, m: &Modifiers) -> bool {
        let command = if MACOS { m.logo() } else { m.ctrl() };
        // The modifier that isn't "command" must be up (Ctrl on macOS, the
        // Windows key elsewhere), so Ctrl+S on a Mac doesn't save.
        let other = if MACOS { m.ctrl() } else { m.logo() };
        self.code == code && self.cmd == command && self.shift == m.shift() && self.alt == m.alt() && !other
    }
}

/// Where an action applies (see the module docs).
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ctx {
    App,
    Arrangement,
    PianoRoll,
    Text,
    Modal,
}

/// Every keyboard action in this build.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Action {
    // File
    New,
    Open,
    Save,
    SaveAs,
    ExportAudio,
    Quit,
    // Edit
    Undo,
    Redo,
    Cut,
    Copy,
    Paste,
    Duplicate,
    Delete,
    SelectAll,
    Quantize,
    Settings,
    // Create
    InsertClip,
    // View / transport
    PlayStop,
    Rewind,
    ToggleBrowser,
    Follow,
    Escape,
    OpenClip,
    LoopSelection,
    // Piano roll
    TransposeUp,
    TransposeDown,
    OctaveUp,
    OctaveDown,
    NudgeLeft,
    NudgeRight,
    // Text fields (handled by the field)
    TextConfirm,
    TextCancel,
    TextNext,
    TextSelectAll,
    TextCopy,
    TextCut,
    TextPaste,
    // Dialogs
    CloseDialog,
    // Dev mode only (F-02)
    DevStats,
    DevStress,
    DevReloadStyles,
    DevFrameCap,
    DevFilter,
}

pub struct Shortcut {
    pub action: Action,
    /// Stable id (menu item ids, tests).
    pub id: &'static str,
    /// What the Help sheet and the menus call it.
    pub name: &'static str,
    /// Section of the Help sheet.
    pub group: &'static str,
    pub win: &'static [Chord],
    pub mac: &'static [Chord],
    pub ctx: &'static [Ctx],
    /// The operating system handles the key (Alt+F4); shown, never matched.
    pub system: bool,
    /// Only in dev mode (`CHIPDAW_DEV=1` or a debug build).
    pub dev: bool,
}

const APP: &[Ctx] = &[Ctx::App];
const EDITORS: &[Ctx] = &[Ctx::Arrangement, Ctx::PianoRoll];
const ARR: &[Ctx] = &[Ctx::Arrangement];
const PIANO: &[Ctx] = &[Ctx::PianoRoll];
const TEXT: &[Ctx] = &[Ctx::Text];
const MODAL: &[Ctx] = &[Ctx::Modal];

macro_rules! sc {
    ($a:ident, $id:literal, $name:literal, $group:literal, $win:expr, $mac:expr, $ctx:expr) => {
        Shortcut { action: Action::$a, id: $id, name: $name, group: $group, win: $win, mac: $mac, ctx: $ctx, system: false, dev: false }
    };
}

/// The table. Keys follow the reference where the UX audit verified them
/// (UX.md §1, decision 3: Quantize Cmd+U, Follow Ctrl+Shift+F / ⌥⇧F).
pub static TABLE: &[Shortcut] = &[
    sc!(New, "file.new", "New", "File", &[cmd(Code::KeyN)], &[cmd(Code::KeyN)], APP),
    sc!(Open, "file.open", "Open…", "File", &[cmd(Code::KeyO)], &[cmd(Code::KeyO)], APP),
    sc!(Save, "file.save", "Save", "File", &[cmd(Code::KeyS)], &[cmd(Code::KeyS)], APP),
    sc!(SaveAs, "file.save_as", "Save As…", "File", &[cmd_shift(Code::KeyS)], &[cmd_shift(Code::KeyS)], APP),
    sc!(ExportAudio, "file.export_audio", "Export Audio…", "File", &[cmd_shift(Code::KeyR)], &[cmd_shift(Code::KeyR)], APP),
    Shortcut {
        action: Action::Quit,
        id: "file.quit",
        name: if MACOS { "Quit ChipDAW" } else { "Exit" },
        group: "File",
        win: &[Chord { code: Code::F4, cmd: false, shift: false, alt: true }],
        mac: &[cmd(Code::KeyQ)],
        ctx: APP,
        // Windows closes the window on Alt+F4 itself (the close asks to save).
        system: !MACOS,
        dev: false,
    },
    sc!(Undo, "edit.undo", "Undo", "Edit", &[cmd(Code::KeyZ)], &[cmd(Code::KeyZ)], APP),
    sc!(Redo, "edit.redo", "Redo", "Edit", &[cmd(Code::KeyY), cmd_shift(Code::KeyZ)], &[cmd_shift(Code::KeyZ), cmd(Code::KeyY)], APP),
    sc!(Cut, "edit.cut", "Cut", "Edit", &[cmd(Code::KeyX)], &[cmd(Code::KeyX)], PIANO),
    sc!(Copy, "edit.copy", "Copy", "Edit", &[cmd(Code::KeyC)], &[cmd(Code::KeyC)], PIANO),
    sc!(Paste, "edit.paste", "Paste", "Edit", &[cmd(Code::KeyV)], &[cmd(Code::KeyV)], PIANO),
    sc!(Duplicate, "edit.duplicate", "Duplicate", "Edit", &[cmd(Code::KeyD)], &[cmd(Code::KeyD)], EDITORS),
    sc!(Delete, "edit.delete", "Delete", "Edit", &[k(Code::Delete), k(Code::Backspace)], &[k(Code::Backspace), k(Code::Delete)], EDITORS),
    sc!(SelectAll, "edit.select_all", "Select All", "Edit", &[cmd(Code::KeyA)], &[cmd(Code::KeyA)], EDITORS),
    sc!(Quantize, "edit.quantize", "Quantize", "Edit", &[cmd(Code::KeyU)], &[cmd(Code::KeyU)], PIANO),
    sc!(Settings, "edit.settings", "Settings…", "Edit", &[cmd(Code::Comma)], &[cmd(Code::Comma)], APP),
    sc!(InsertClip, "create.midi_clip", "Insert MIDI Clip", "Create", &[cmd_shift(Code::KeyM)], &[cmd_shift(Code::KeyM)], ARR),
    sc!(PlayStop, "transport.play", "Play / Stop", "Transport", &[k(Code::Space)], &[k(Code::Space)], APP),
    sc!(Rewind, "transport.rewind", "Back to the start", "Transport", &[k(Code::Home)], &[k(Code::Home)], APP),
    sc!(LoopSelection, "arrangement.loop_selection", "Loop Selection", "Arrangement", &[cmd(Code::KeyL)], &[cmd(Code::KeyL)], ARR),
    sc!(OpenClip, "arrangement.open_clip", "Open the clip in the piano roll", "Arrangement", &[k(Code::Enter), k(Code::NumpadEnter)], &[k(Code::Enter), k(Code::NumpadEnter)], ARR),
    sc!(ToggleBrowser, "view.browser", "Browser", "View", &[cmd_alt(Code::KeyB)], &[cmd_alt(Code::KeyB)], APP),
    sc!(Follow, "view.follow", "Follow", "View", &[cmd_shift(Code::KeyF)], &[alt_shift(Code::KeyF)], APP),
    sc!(Escape, "view.escape", "Close the menu or piano roll · clear the selection", "View", &[k(Code::Escape)], &[k(Code::Escape)], APP),
    sc!(TransposeUp, "piano.up", "Transpose up a semitone", "Piano roll", &[k(Code::ArrowUp)], &[k(Code::ArrowUp)], PIANO),
    sc!(TransposeDown, "piano.down", "Transpose down a semitone", "Piano roll", &[k(Code::ArrowDown)], &[k(Code::ArrowDown)], PIANO),
    sc!(OctaveUp, "piano.octave_up", "Transpose up an octave", "Piano roll", &[shift(Code::ArrowUp)], &[shift(Code::ArrowUp)], PIANO),
    sc!(OctaveDown, "piano.octave_down", "Transpose down an octave", "Piano roll", &[shift(Code::ArrowDown)], &[shift(Code::ArrowDown)], PIANO),
    sc!(NudgeLeft, "piano.nudge_left", "Move left by the grid", "Piano roll", &[k(Code::ArrowLeft)], &[k(Code::ArrowLeft)], PIANO),
    sc!(NudgeRight, "piano.nudge_right", "Move right by the grid", "Piano roll", &[k(Code::ArrowRight)], &[k(Code::ArrowRight)], PIANO),
    sc!(TextConfirm, "text.confirm", "Confirm", "Text fields", &[k(Code::Enter)], &[k(Code::Enter)], TEXT),
    sc!(TextCancel, "text.cancel", "Stop editing", "Text fields", &[k(Code::Escape)], &[k(Code::Escape)], TEXT),
    sc!(TextNext, "text.next", "Next field", "Text fields", &[k(Code::Tab)], &[k(Code::Tab)], TEXT),
    sc!(TextSelectAll, "text.select_all", "Select all text", "Text fields", &[cmd(Code::KeyA)], &[cmd(Code::KeyA)], TEXT),
    sc!(TextCopy, "text.copy", "Copy text", "Text fields", &[cmd(Code::KeyC)], &[cmd(Code::KeyC)], TEXT),
    sc!(TextCut, "text.cut", "Cut text", "Text fields", &[cmd(Code::KeyX)], &[cmd(Code::KeyX)], TEXT),
    sc!(TextPaste, "text.paste", "Paste text", "Text fields", &[cmd(Code::KeyV)], &[cmd(Code::KeyV)], TEXT),
    sc!(CloseDialog, "dialog.close", "Close the dialog", "Dialogs", &[k(Code::Escape)], &[k(Code::Escape)], MODAL),
    Shortcut { action: Action::DevStats, id: "dev.stats", name: "Frame-time overlay", group: "Dev mode", win: &[k(Code::F3)], mac: &[k(Code::F3)], ctx: APP, system: false, dev: true },
    Shortcut { action: Action::DevStress, id: "dev.stress", name: "200-clip stress set (replaces the song)", group: "Dev mode", win: &[k(Code::F4)], mac: &[k(Code::F4)], ctx: APP, system: false, dev: true },
    Shortcut { action: Action::DevReloadStyles, id: "dev.styles", name: "Reload the stylesheet", group: "Dev mode", win: &[k(Code::F5)], mac: &[k(Code::F5)], ctx: APP, system: false, dev: true },
    Shortcut { action: Action::DevFrameCap, id: "dev.frame_cap", name: "Cycle the frame-rate cap (60 / 120 / off)", group: "Dev mode", win: &[k(Code::F6)], mac: &[k(Code::F6)], ctx: APP, system: false, dev: true },
    Shortcut { action: Action::DevFilter, id: "dev.filter", name: "Cycle the output filter", group: "Dev mode", win: &[k(Code::F7)], mac: &[k(Code::F7)], ctx: APP, system: false, dev: true },
];

impl Shortcut {
    /// This platform's keys.
    pub fn keys(&self) -> &'static [Chord] {
        if MACOS {
            self.mac
        } else {
            self.win
        }
    }

    /// Whether the shortcut is live in this process (dev entries need dev mode).
    pub fn available(&self) -> bool {
        !self.dev || crate::dev::enabled()
    }
}

/// The table entry for an action.
pub fn get(a: Action) -> &'static Shortcut {
    TABLE.iter().find(|s| s.action == a).expect("every action is in the table")
}

/// Whether contexts `a` and `b` are active at the same time (`App` is active
/// in both editors).
fn overlaps(a: Ctx, b: Ctx) -> bool {
    use Ctx::*;
    a == b || matches!((a, b), (App, Arrangement) | (Arrangement, App) | (App, PianoRoll) | (PianoRoll, App))
}

/// The action a key press triggers in context `ctx` (`Arrangement` or
/// `PianoRoll` for the focused editor, `Text` while a field is edited,
/// `Modal` while a dialog is open), on this platform.
pub fn lookup(code: Code, m: &Modifiers, ctx: Ctx) -> Option<Action> {
    TABLE
        .iter()
        .filter(|s| !s.system && s.available() && s.ctx.iter().any(|c| overlaps(*c, ctx)))
        .find(|s| s.keys().iter().any(|c| c.matches(code, m)))
        .map(|s| s.action)
}

fn key_name(code: Code, mac: bool) -> String {
    let s = match code {
        Code::Space => "Space",
        Code::Enter => if mac { "↩" } else { "Enter" },
        Code::NumpadEnter => if mac { "⌤" } else { "Num Enter" },
        Code::Escape => "Esc",
        Code::Tab => if mac { "⇥" } else { "Tab" },
        Code::Delete => if mac { "⌦" } else { "Del" },
        Code::Backspace => if mac { "⌫" } else { "Backspace" },
        Code::Home => "Home",
        Code::Comma => ",",
        Code::ArrowUp => "↑",
        Code::ArrowDown => "↓",
        Code::ArrowLeft => "←",
        Code::ArrowRight => "→",
        other => {
            let d = format!("{other:?}");
            return d.strip_prefix("Key").or_else(|| d.strip_prefix("Digit")).unwrap_or(&d).to_string();
        }
    };
    s.to_string()
}

/// A chord as text, in the platform's convention: "Ctrl+Shift+S" on
/// Windows, "⇧⌘S" on macOS.
pub fn chord_text(c: &Chord, mac: bool) -> String {
    let key = key_name(c.code, mac);
    if mac {
        let mut s = String::new();
        if c.alt {
            s.push('⌥');
        }
        if c.shift {
            s.push('⇧');
        }
        if c.cmd {
            s.push('⌘');
        }
        s + &key
    } else {
        let mut parts: Vec<&str> = Vec::new();
        if c.cmd {
            parts.push("Ctrl");
        }
        if c.alt {
            parts.push("Alt");
        }
        if c.shift {
            parts.push("Shift");
        }
        parts.push(&key);
        parts.join("+")
    }
}

/// The first key of an action as text on this platform ("" if it has none).
pub fn label(a: Action) -> String {
    get(a).keys().first().map(|c| chord_text(c, MACOS)).unwrap_or_default()
}

/// Every key of an action, joined ("Ctrl+Y, Ctrl+Shift+Z").
pub fn all_labels(s: &Shortcut, mac: bool) -> String {
    let keys = if mac { s.mac } else { s.win };
    keys.iter().map(|c| chord_text(c, mac)).collect::<Vec<_>>().join(", ")
}

/// The first key as a muda accelerator string ("CmdOrCtrl+Shift+KeyS"), for
/// the macOS menu bar (on Windows the menus show text only; FEASIBILITY.md §1).
pub fn accelerator(a: Action) -> Option<String> {
    let c = get(a).keys().first()?;
    let mut parts: Vec<String> = Vec::new();
    if c.cmd {
        parts.push("CmdOrCtrl".into());
    }
    if c.alt {
        parts.push("Alt".into());
    }
    if c.shift {
        parts.push("Shift".into());
    }
    parts.push(format!("{:?}", c.code));
    Some(parts.join("+"))
}

/// Help ▸ Keyboard Shortcuts: (group, [(name, keys)]) in table order, for
/// this platform; dev entries only in dev mode.
pub fn sheet(mac: bool) -> Vec<(&'static str, Vec<(&'static str, String)>)> {
    let mut out: Vec<(&'static str, Vec<(&'static str, String)>)> = Vec::new();
    for s in TABLE.iter().filter(|s| s.available()) {
        let keys = all_labels(s, mac);
        if keys.is_empty() {
            continue;
        }
        match out.iter_mut().find(|(g, _)| *g == s.group) {
            Some((_, rows)) => rows.push((s.name, keys)),
            None => out.push((s.group, vec![(s.name, keys)])),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    const CTXS: [Ctx; 5] = [Ctx::App, Ctx::Arrangement, Ctx::PianoRoll, Ctx::Text, Ctx::Modal];

    /// No two actions share a key within any context, on either platform.
    #[test]
    fn no_conflicts() {
        for mac in [false, true] {
            for ctx in CTXS {
                let mut seen: Vec<(Chord, &str)> = Vec::new();
                for s in TABLE.iter().filter(|s| s.ctx.iter().any(|c| overlaps(*c, ctx))) {
                    let keys = if mac { s.mac } else { s.win };
                    for c in keys {
                        if let Some((_, other)) = seen.iter().find(|(k, _)| k == c) {
                            panic!("{} and {} share {} in {ctx:?} (mac: {mac})", other, s.id, chord_text(c, mac));
                        }
                        seen.push((*c, s.id));
                    }
                }
            }
        }
    }

    #[test]
    fn ids_unique_and_every_action_once() {
        for (i, a) in TABLE.iter().enumerate() {
            for b in &TABLE[i + 1..] {
                assert_ne!(a.id, b.id);
                assert_ne!(a.action, b.action, "{} listed twice", a.id);
            }
        }
    }

    #[test]
    fn decision_3_keys() {
        // UX.md decision 3: Quantize Cmd+U (Q retired), Follow Ctrl+Shift+F,
        // Cmd+I is no longer MIDI import (nothing uses it in this build).
        assert_eq!(chord_text(&get(Action::Quantize).win[0], false), "Ctrl+U");
        assert_eq!(chord_text(&get(Action::Follow).win[0], false), "Ctrl+Shift+F");
        assert_eq!(chord_text(&get(Action::Follow).mac[0], true), "⌥⇧F");
        for s in TABLE {
            for c in s.win.iter().chain(s.mac) {
                assert!(!(c.code == Code::KeyQ && !c.cmd), "Q is retired ({})", s.id);
                assert!(!(c.code == Code::KeyI && c.cmd), "Cmd+I is reserved for Insert Silence ({})", s.id);
            }
        }
    }

    #[test]
    fn labels_follow_the_platform() {
        assert_eq!(chord_text(&get(Action::SaveAs).win[0], false), "Ctrl+Shift+S");
        assert_eq!(chord_text(&get(Action::SaveAs).mac[0], true), "⇧⌘S");
        assert_eq!(chord_text(&get(Action::ToggleBrowser).win[0], false), "Ctrl+Alt+B");
        assert_eq!(chord_text(&get(Action::Settings).win[0], false), "Ctrl+,");
    }

    #[test]
    fn lookup_respects_context() {
        let ctrl = Modifiers::CTRL;
        let none = Modifiers::empty();
        if !MACOS {
            assert_eq!(lookup(Code::KeyZ, &ctrl, Ctx::Arrangement), Some(Action::Undo));
            assert_eq!(lookup(Code::KeyZ, &ctrl, Ctx::Text), None, "undo never reaches the document from a field");
            assert_eq!(lookup(Code::Space, &none, Ctx::Text), None, "space types a space");
            assert_eq!(lookup(Code::KeyU, &ctrl, Ctx::PianoRoll), Some(Action::Quantize));
            assert_eq!(lookup(Code::KeyU, &ctrl, Ctx::Arrangement), None);
            assert_eq!(lookup(Code::KeyS, &ctrl, Ctx::Modal), None);
            assert_eq!(lookup(Code::Escape, &none, Ctx::Modal), Some(Action::CloseDialog));
            // Alt+F4 is the system's.
            assert_eq!(lookup(Code::F4, &Modifiers::ALT, Ctx::Arrangement), None);
        }
    }

    #[cfg(feature = "menus")]
    #[test]
    fn accelerators_parse() {
        for s in TABLE.iter().filter(|s| !s.system && !s.dev) {
            if let Some(a) = accelerator(s.action) {
                assert!(a.parse::<muda::accelerator::Accelerator>().is_ok(), "{} -> {a}", s.id);
            }
        }
    }

    #[test]
    fn sheet_lists_every_entry() {
        let n: usize = sheet(false).iter().map(|(_, r)| r.len()).sum();
        let want = TABLE.iter().filter(|s| s.available() && !s.win.is_empty()).count();
        assert_eq!(n, want);
    }
}
