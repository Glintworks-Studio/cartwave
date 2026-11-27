//! Profile warnings (BATCH4A_PROMPT.md §5.5; VISION.md principles 5 and 6,
//! "Warn, don't surprise" and "Ask only when it matters"): a question with up
//! to three answers (Unlock track · Fit to hardware · Cancel) and a "Don't
//! ask again" box.
//!
//! The dialog style is the app's existing conflict dialog: the native message
//! box (`AppState::ask`). On Windows it is a task dialog, whose verification
//! box is the "Don't ask again" check (the binaries carry the common-controls
//! v6 manifest it needs, `crates/app/build.rs`). Elsewhere (macOS: not tested
//! in this batch) it falls back to the three-button message box without the
//! box.

/// What a warning offers.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Choice {
    Unlock,
    Fit,
    /// MIDI import (Batch 4b §6): Fit to hardware with chords as fast
    /// arpeggios.
    FitArp,
    Cancel,
}

impl Choice {
    pub fn key(self) -> &'static str {
        match self {
            Choice::Unlock => "unlock",
            Choice::Fit => "fit",
            Choice::FitArp => "fitarp",
            Choice::Cancel => "cancel",
        }
    }
}

/// The kinds of warning, each with its own "Don't ask again" (Settings >
/// Warnings lists and resets them).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WarnKind {
    /// A chord or overlapping notes on a locked track.
    Chord,
    /// A note outside the channel's real range.
    Range,
    /// A layout (authentic preset band) for another expansion chip.
    Expansion,
    /// DPCM notes on rows with no sample (the hardware plays nothing).
    Dpcm,
    /// A profile that lacks a channel in use.
    Profile,
    /// A dropped or imported MIDI file whose parts break the profile
    /// (Batch 4b §6: the "Unlock" answer is "Import as studio tracks").
    MidiImport,
}

impl WarnKind {
    pub fn id(self) -> &'static str {
        match self {
            WarnKind::Chord => "locked-chord",
            WarnKind::Range => "locked-range",
            WarnKind::Expansion => "expansion-chip",
            WarnKind::Dpcm => "dpcm-sample",
            WarnKind::Profile => "profile-channels",
            WarnKind::MidiImport => "midi-import",
        }
    }

    /// The Settings > Warnings row.
    pub fn label(self) -> &'static str {
        match self {
            WarnKind::Chord => "A chord or overlapping notes on a locked track",
            WarnKind::Range => "A note outside a locked channel's range",
            WarnKind::Expansion => "A layout for another expansion chip",
            WarnKind::Dpcm => "DPCM notes with no sample on a locked track",
            WarnKind::Profile => "A hardware profile without a channel in use",
            WarnKind::MidiImport => "MIDI parts that don't fit the hardware profile",
        }
    }

    pub const ALL: [WarnKind; 6] = [WarnKind::Chord, WarnKind::Range, WarnKind::Expansion, WarnKind::Dpcm, WarnKind::Profile, WarnKind::MidiImport];
}

/// A silenced warning's stored form: `"{id}={choice}"` (the answer to give
/// without asking).
pub fn silenced_entry(kind: WarnKind, c: Choice) -> String {
    format!("{}={}", kind.id(), c.key())
}

/// The remembered answer for `kind`, if the user ticked "Don't ask again".
pub fn remembered(silenced: &[String], kind: WarnKind) -> Option<Choice> {
    let want = format!("{}=", kind.id());
    silenced.iter().find_map(|s| match s.strip_prefix(&want)? {
        "unlock" => Some(Choice::Unlock),
        "fit" => Some(Choice::Fit),
        "fitarp" => Some(Choice::FitArp),
        _ => None,
    })
}

/// Settings > Warnings: a stored entry as text ("… (always Fit to hardware)").
pub fn describe(entry: &str) -> String {
    let (id, c) = entry.split_once('=').unwrap_or((entry, ""));
    let label = WarnKind::ALL.iter().find(|k| k.id() == id).map_or(id, |k| k.label());
    match c {
        "unlock" if id == WarnKind::MidiImport.id() => format!("{label}: always Import as studio tracks"),
        "unlock" => format!("{label}: always Unlock track"),
        "fit" => format!("{label}: always Fit to hardware"),
        "fitarp" => format!("{label}: always Fit to hardware, chords as arpeggios"),
        _ => label.to_string(),
    }
}

/// One question.
pub struct Ask<'a> {
    pub title: &'a str,
    pub text: &'a str,
    /// The Unlock answer's label (None: not offered, it wouldn't help).
    pub unlock: Option<&'a str>,
    /// The Fit answer's label (None: no fit applies).
    pub fit: Option<&'a str>,
    /// A second Fit answer (MIDI import: chords as arpeggios).
    pub fit_arp: Option<&'a str>,
    /// The answer Enter gives (the recommended one); None: Fit when offered,
    /// else Cancel (never the answer that changes a track's kind).
    pub default: Option<Choice>,
}

/// Shows the question; returns the answer and whether "Don't ask again" was
/// ticked. `hwnd`: the main window (Windows), for a modal dialog.
pub fn show(q: &Ask, hwnd: Option<isize>) -> (Choice, bool) {
    if crate::dev::enabled() {
        eprintln!("dev: warning {} | {}", q.title, q.text.replace('\n', " / "));
    }
    // Scripted UI tests: `CHIPDAW_WARN_ANSWER=fit|unlock|cancel[,dont]` answers
    // without a dialog (the harness still sees the dev line above).
    if let Some(a) = crate::dev::var("CHIPDAW_WARN_ANSWER") {
        let c = match a.split(',').next().unwrap_or("") {
            "unlock" if q.unlock.is_some() => Choice::Unlock,
            "fit" if q.fit.is_some() => Choice::Fit,
            "fitarp" if q.fit_arp.is_some() => Choice::FitArp,
            _ => Choice::Cancel,
        };
        return (c, a.contains("dont"));
    }
    platform::show(q, hwnd)
}

#[cfg(all(windows, feature = "menus"))]
mod platform {
    use super::{Ask, Choice};
    use windows_sys::Win32::UI::Controls::{TaskDialogIndirect, TASKDIALOGCONFIG, TASKDIALOG_BUTTON, TDF_ALLOW_DIALOG_CANCELLATION, TDF_POSITION_RELATIVE_TO_WINDOW, TD_WARNING_ICON};

    fn wide(s: &str) -> Vec<u16> {
        s.encode_utf16().chain(std::iter::once(0)).collect()
    }

    const ID_UNLOCK: i32 = 101;
    const ID_FIT: i32 = 102;
    const ID_FIT_ARP: i32 = 103;
    const ID_CANCEL: i32 = 2; // IDCANCEL: Esc and the close box answer Cancel.

    pub fn show(q: &Ask, hwnd: Option<isize>) -> (Choice, bool) {
        let title = wide("ChipDAW");
        let main = wide(q.title);
        let content = wide(q.text);
        let verify = wide("Don't ask again");
        let labels: Vec<(i32, Vec<u16>)> = [q.unlock.map(|l| (ID_UNLOCK, wide(l))), q.fit.map(|l| (ID_FIT, wide(l))), q.fit_arp.map(|l| (ID_FIT_ARP, wide(l))), Some((ID_CANCEL, wide("Cancel")))].into_iter().flatten().collect();
        let buttons: Vec<TASKDIALOG_BUTTON> = labels.iter().map(|(id, t)| TASKDIALOG_BUTTON { nButtonID: *id, pszButtonText: t.as_ptr() }).collect();
        let mut cfg = TASKDIALOGCONFIG { cbSize: std::mem::size_of::<TASKDIALOGCONFIG>() as u32, ..Default::default() };
        cfg.hwndParent = hwnd.map_or(std::ptr::null_mut(), |h| h as _);
        cfg.dwFlags = TDF_ALLOW_DIALOG_CANCELLATION | TDF_POSITION_RELATIVE_TO_WINDOW;
        cfg.pszWindowTitle = title.as_ptr();
        cfg.Anonymous1.pszMainIcon = TD_WARNING_ICON;
        cfg.pszMainInstruction = main.as_ptr();
        cfg.pszContent = content.as_ptr();
        cfg.cButtons = buttons.len() as u32;
        cfg.pButtons = buttons.as_ptr();
        // The default (Enter) is Fit when offered, else Cancel: never the
        // answer that changes the track's kind.
        cfg.nDefaultButton = match q.default {
            Some(Choice::Unlock) => ID_UNLOCK,
            Some(Choice::Fit) => ID_FIT,
            Some(Choice::FitArp) => ID_FIT_ARP,
            Some(Choice::Cancel) => ID_CANCEL,
            None if q.fit.is_some() => ID_FIT,
            None => ID_CANCEL,
        };
        cfg.pszVerificationText = verify.as_ptr();
        let (mut button, mut checked) = (0i32, 0);
        // SAFETY: every pointer in `cfg` points into the locals above, which
        // outlive the (modal, synchronous) call.
        let hr = unsafe { TaskDialogIndirect(&cfg, &mut button, std::ptr::null_mut(), &mut checked) };
        if hr < 0 {
            return (Choice::Cancel, false);
        }
        let c = match button {
            ID_UNLOCK => Choice::Unlock,
            ID_FIT => Choice::Fit,
            ID_FIT_ARP => Choice::FitArp,
            _ => Choice::Cancel,
        };
        // "Don't ask again" with Cancel remembers nothing.
        (c, checked != 0 && c != Choice::Cancel)
    }
}

#[cfg(not(all(windows, feature = "menus")))]
mod platform {
    use super::{Ask, Choice};

    pub fn show(q: &Ask, _hwnd: Option<isize>) -> (Choice, bool) {
        use rfd::{MessageButtons, MessageDialogResult as R};
        let text = format!("{}\n\n{}", q.title, q.text);
        let buttons = match (q.unlock, q.fit) {
            (Some(u), Some(f)) => MessageButtons::YesNoCancelCustom(u.into(), f.into(), "Cancel".into()),
            (Some(x), None) | (None, Some(x)) => MessageButtons::OkCancelCustom(x.into(), "Cancel".into()),
            (None, None) => MessageButtons::Ok,
        };
        let r = rfd::MessageDialog::new().set_level(rfd::MessageLevel::Warning).set_title("ChipDAW").set_description(text).set_buttons(buttons).show();
        let c = match r {
            R::Custom(c) if Some(c.as_str()) == q.unlock => Choice::Unlock,
            R::Custom(c) if Some(c.as_str()) == q.fit => Choice::Fit,
            _ => Choice::Cancel,
        };
        (c, false)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn remembered_answers_round_trip_and_describe() {
        let s = vec![silenced_entry(WarnKind::Chord, Choice::Fit), "old-id".to_string()];
        assert_eq!(remembered(&s, WarnKind::Chord), Some(Choice::Fit));
        assert_eq!(remembered(&s, WarnKind::Range), None);
        assert!(describe(&s[0]).contains("always Fit to hardware"));
        assert_eq!(describe("old-id"), "old-id");
    }
}
