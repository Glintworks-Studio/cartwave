//! The browser (left, Ctrl+Alt+B; PRESETS_PROMPT.md §5, VISION.md "The
//! browser"; the `workflow-chip-synth` board, locked for layout). It
//! **replaces** the old flat list.
//!
//! Live's structure (Live 12 manual, ch. 4 "Working with the Browser"):
//! a **sidebar** of COLLECTIONS (coloured labels, assigned from an item's
//! context menu), LIBRARY (Sounds by role, Drums, Instruments by voice,
//! Hardware: the authentic presets by profile with a lock, Clips, Samples)
//! and PLACES (User Library, Current Project), and a **content pane** of
//! folders with disclosure chevrons, the main way to browse. **Search**
//! looks across everything, tags included. Labels with no content yet
//! (Audio Effects, MIDI Effects, Imported Projects) aren't shown.
//!
//! Select an item to preview it (the header's Preview key), double-click or
//! drag it to load it: onto a track (or a clip), or onto empty space for a
//! new track. Loading from Hardware locks the track (`preset_ui`).
//!
//! The sidebar and the content pane are one custom-drawn view (no child per
//! row): it scrolls, hovers and hit-tests itself, so 200+ rows cost one draw.

use std::cell::{Cell, RefCell};
use std::collections::HashSet;

use chip::presets::lane_kind;
use vizia::prelude::*;
use vizia::vg;

use super::gfx::{self, Face};
use crate::bank::{self, Folder, Item};
use crate::theme::tokens::*;
use crate::HelpExt;
use crate::{AppEvent, Signals};

/// The browser's width (logical px): the board's 150 px sidebar and 270 px
/// content pane.
pub const BROWSER_W: f32 = 420.0;

/// The browser's width as built: `BROWSER_W`, or (dev mode only) the
/// `CARTWAVE_BROWSER_W` hook, which the earlier scenario suites set to the
/// old 240 so their arrangement coordinates hold (as `CARTWAVE_OVERVIEW=0`
/// does for theirs; RUNLOG P2-D16, PB-D12).
pub fn browser_w() -> f32 {
    crate::dev::var("CARTWAVE_BROWSER_W").and_then(|v| v.parse::<f32>().ok()).map_or(BROWSER_W, |w| w.clamp(160.0, 800.0))
}
const SIDEBAR_W: f32 = 150.0;
const ROW_H: f32 = 24.0;
const TITLE_H: f32 = 28.0;
const SIDE_TITLE_H: f32 = 28.0;
const INDENT: f32 = 16.0;
const PAD_X: f32 = 12.0;
const DRAG_START_PX: f32 = 4.0;
const WHEEL_PX: f32 = 48.0;

/// What is being dragged out of the browser, and where (physical window px).
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct PresetDrag {
    pub item: Item,
    pub x: f32,
    pub y: f32,
    /// The lane under the pointer, if any.
    pub target: Option<usize>,
    /// The clip under the pointer, if any: a preset dropped there becomes
    /// that clip's own instrument.
    pub clip: Option<u32>,
    /// Over empty space below the tracks: the drop makes a new track.
    pub empty: bool,
}

impl PresetDrag {
    /// The preset index (factory, or `bank::len()` + user) being dragged.
    pub fn preset(&self) -> Option<usize> {
        match self.item {
            Item::Preset(i) => Some(i),
            Item::User(u) => Some(bank::len() + u),
            _ => None,
        }
    }
}

thread_local! {
    /// Each track row's bounds as last drawn (physical px): drop targets
    /// (any number of tracks: the list grows as rows draw).
    static LANE_ROWS: RefCell<Vec<Option<BoundingBox>>> = const { RefCell::new(Vec::new()) };
    /// The arrangement's lanes area (physical px), for drops on empty space.
    static LANES_AREA: Cell<Option<(f32, f32, f32, f32)>> = const { Cell::new(None) };
}

/// Called by the track rows when they draw.
pub fn register_lane_row(lane: usize, b: BoundingBox) {
    LANE_ROWS.with(|r| {
        let mut r = r.borrow_mut();
        if r.len() <= lane {
            r.resize(lane + 1, None);
        }
        r[lane] = Some(b);
    });
}

/// Called by the arrangement when it draws: its lanes' box (physical px).
pub fn register_lanes_area(left: f32, top: f32, right: f32, bottom: f32) {
    LANES_AREA.with(|c| c.set(Some((left, top, right, bottom))));
}

/// The lane at a window position: over its track header or its lane in the
/// arrangement, wherever the headers are (polish 2 §5.4: they can be on the
/// right of the lanes).
pub fn lane_at(x: f32, y: f32) -> Option<usize> {
    let lanes = super::arrangement::arrangement_at();
    LANE_ROWS.with(|r| {
        r.borrow().iter().position(|b| {
            b.is_some_and(|b| {
                let over_header = x >= b.x && x < b.x + b.w;
                let over_lane = lanes.is_some_and(|(l, rt, _)| x >= l && x < rt);
                (over_header || over_lane) && y >= b.y && y < b.y + b.h
            })
        })
    })
}

/// Whether a window position is empty arrangement space under the last
/// track (a drop there makes a new track, §5.3).
pub fn over_empty_space(x: f32, y: f32, tracks: usize) -> bool {
    let Some((l, t, r, b)) = LANES_AREA.with(|c| c.get()) else { return false };
    let last = LANE_ROWS.with(|rows| rows.borrow().iter().take(tracks).flatten().map(|b| b.y + b.h).fold(t, f32::max));
    let headers = LANE_ROWS.with(|rows| rows.borrow().iter().take(tracks).flatten().next().map(|b| (b.x, b.x + b.w)));
    let in_x = (x >= l && x < r) || headers.is_some_and(|(h0, h1)| x >= h0 && x < h1);
    in_x && y >= last && y < b && lane_at(x, y).is_none()
}

/// A track row's bounds as last drawn (physical px).
pub fn lane_row(lane: usize) -> Option<BoundingBox> {
    LANE_ROWS.with(|r| r.borrow().get(lane).copied().flatten())
}

/// The vertical span of a lane's row (physical px), for the arrangement's dimming.
pub fn lane_span(lane: usize) -> Option<(f32, f32)> {
    LANE_ROWS.with(|r| r.borrow().get(lane).copied().flatten().map(|b| (b.y, b.y + b.h)))
}

/// While an item is dragged: how a lane looks. `Some((fits, target))`:
/// `fits` false dims it (it can't take the item as it is), `target` tints
/// the one it would land on; `None` = normal.
pub fn drag_state(s: &Signals, lane: usize) -> Option<(bool, bool)> {
    let d = s.drag_preset.get_untracked()?;
    let target = d.target == Some(lane);
    let fits = s.arrangement.with_untracked(|a| {
        let Some(l) = a.lanes.get(lane) else { return false };
        match d.item {
            Item::Preset(_) | Item::User(_) => {
                let Some(p) = d.preset().and_then(|p| s.user_presets.with_untracked(|u| crate::user_presets::view(p, u))) else { return false };
                // A studio track takes any preset (it switches voice); a
                // locked one a preset of its channel with no studio-only control.
                match l.lock {
                    None => true,
                    Some(c) => p.kinds & lane_kind(c) != 0 && !p.studio_only(),
                }
            }
            Item::Sample(_) => l.voice == chip::voice::VoiceKind::Dpcm,
            Item::Clip(_) | Item::Band(_) | Item::ImportAudio => true,
        }
    });
    Some((fits, target))
}

// ───────────────────────────── the sidebar's entries ─────────────────────────────

/// A sidebar entry (the content pane shows one at a time).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Section {
    Collection(usize),
    Sounds,
    Drums,
    Instruments,
    Hardware,
    Clips,
    Samples,
    User,
    Project,
}

impl Section {
    pub fn key(self) -> String {
        match self {
            Section::Collection(k) => format!("Collection {k}"),
            s => s.label().to_string(),
        }
    }

    pub fn from_key(k: &str) -> Section {
        if let Some(n) = k.strip_prefix("Collection ").and_then(|n| n.parse().ok()) {
            return Section::Collection(n);
        }
        [Section::Sounds, Section::Drums, Section::Instruments, Section::Hardware, Section::Clips, Section::Samples, Section::User, Section::Project].into_iter().find(|s| s.label() == k).unwrap_or(Section::Sounds)
    }

    pub fn label(self) -> &'static str {
        match self {
            Section::Collection(k) => crate::collections::COLLECTIONS.get(k).map_or("Collection", |c| c.0),
            Section::Sounds => "Sounds",
            Section::Drums => "Drums",
            Section::Instruments => "Instruments",
            Section::Hardware => "Hardware",
            Section::Clips => "Clips",
            Section::Samples => "Samples",
            Section::User => "User Library",
            Section::Project => "Current Project",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
enum Side {
    Title(&'static str),
    Entry(Section),
}

/// A row of the content pane.
#[derive(Clone, Debug, PartialEq)]
enum Row {
    /// A folder: its path in the section (the key of its open state).
    Folder { path: Vec<String>, depth: usize, open: bool },
    Item { item: Item, depth: usize },
}

impl Row {
    fn depth(&self) -> usize {
        match self {
            Row::Folder { depth, .. } | Row::Item { depth, .. } => *depth,
        }
    }
}

// ───────────────────────────── the view ─────────────────────────────

pub struct LibraryView {
    s: Signals,
    section: Section,
    side: Vec<Side>,
    rows: Vec<Row>,
    /// Open folders, by section and path ("Sounds/Leads/Square").
    open: HashSet<String>,
    scroll: f32,
    hover_side: Option<usize>,
    hover: Option<usize>,
    sel: Option<Item>,
    /// Pressed row and the press position (physical).
    press: Option<(usize, f32, f32)>,
    dragging: bool,
    /// A held sample audition (released on mouse up).
    sample_held: bool,
}

/// An item's label.
fn item_name(s: &Signals, it: Item) -> String {
    match it {
        Item::Preset(i) => bank::get(i).map_or(String::new(), |p| p.name.clone()),
        Item::User(u) => s.user_presets.with_untracked(|v| v.get(u).map_or(String::new(), |p| p.name.clone())),
        Item::Band(i) => chip::presets::LAYOUTS.get(i).map_or(String::new(), |l| l.name.to_string()),
        Item::Clip(i) => bank::PHRASES.get(i).map_or(String::new(), |p| p.name.to_string()),
        Item::Sample(i) => sample_name(s, i),
        Item::ImportAudio => {
            let free = s.arrangement.with_untracked(|a| crate::dpcm::free_bytes(&a.samples));
            format!("Import audio as DPCM…  ·  {:.1} KB free", free as f32 / 1024.0)
        }
    }
}

fn sample_name(s: &Signals, i: usize) -> String {
    let b = chip::samples::BUILTIN_COUNT;
    if i < b {
        return chip::samples::NAMES[i].to_string();
    }
    s.arrangement.with_untracked(|a| a.samples.get(i - b).cloned().flatten().map_or(format!("User {}", i - b + 1), |x| x.name))
}

/// An item's one-line help (the status bar).
fn item_help(s: &Signals, it: Item) -> String {
    let load = "double-click or drag to load (onto empty space: a new track)";
    match it {
        Item::Preset(i) => bank::get(i).map_or(String::new(), |p| {
            let lock = match p.profile {
                Some(pr) => format!(" · authentic ({}): loading it locks the track", pr.label()),
                None if p.studio_only() => " · studio only (Unison, Detune or Drift)".into(),
                None => String::new(),
            };
            format!("{}: {} · {}{lock} · {load}", p.name, p.blurb, bank::voice_name(p.voice))
        }),
        Item::User(u) => format!("{}: your preset · {load} · right-click to rename or delete", item_name(s, Item::User(u))),
        Item::Band(i) => chip::presets::LAYOUTS.get(i).map_or(String::new(), |l| format!("{}: {} · a whole band, locked to the hardware · double-click or drag to load", l.name, l.blurb)),
        Item::Clip(i) => bank::PHRASES.get(i).map_or(String::new(), |p| format!("{}: a {}-bar MIDI clip · double-click: on the selected track at the insert marker · drag onto a track, or onto empty space", p.name, p.bars)),
        Item::Sample(i) => format!("{}: a DPCM sample · click to hear it · double-click or drag onto a DPCM Sampler track (empty space: a new one)", sample_name(s, i)),
        Item::ImportAudio => "Import a WAV file as a DPCM sample (into a free user slot)".into(),
    }
}

impl LibraryView {
    pub fn new(cx: &mut Context, s: Signals) -> Handle<'_, Self> {
        let section = Section::from_key(&s.settings.with_untracked(|st| st.browser_section.clone()));
        let mut v = Self { s, section, side: Vec::new(), rows: Vec::new(), open: HashSet::new(), scroll: 0.0, hover_side: None, hover: None, sel: None, press: None, dragging: false, sample_held: false };
        v.refresh();
        v.build(cx, |_| {})
            .class("preset-list")
            .bind(s.browser_query, |h| {
                h.modify(|v: &mut LibraryView| {
                    v.scroll = 0.0;
                    v.refresh();
                })
                .needs_redraw();
            })
            .bind(s.user_presets, |h| {
                h.modify(|v: &mut LibraryView| v.refresh()).needs_redraw();
            })
            .bind(s.collections_rev, |h| {
                h.modify(|v: &mut LibraryView| v.refresh()).needs_redraw();
            })
            .bind(s.arrangement, |h| {
                // Samples and the Current Project follow the project.
                h.modify(|v: &mut LibraryView| {
                    if matches!(v.section, Section::Samples | Section::Project) {
                        v.refresh();
                    }
                })
                .needs_redraw();
            })
            .bind(s.browser_reveal, |h| {
                h.modify(|v: &mut LibraryView| v.reveal()).needs_redraw();
            })
            .bind(s.browser_sel, |mut h| h.needs_redraw())
    }

    fn query_words(&self) -> Vec<String> {
        self.s.browser_query.with_untracked(|q| q.split_whitespace().map(|w| w.to_lowercase()).collect())
    }

    /// The sidebar: COLLECTIONS (Favourites, and any that has items),
    /// LIBRARY, PLACES.
    fn side_entries(&self) -> Vec<Side> {
        let lists = self.s.settings.with_untracked(|st| st.collections.clone());
        let mut v = vec![Side::Title("COLLECTIONS")];
        for k in 0..crate::collections::COLLECTIONS.len() {
            if k == 0 || lists.get(k).is_some_and(|l| !l.is_empty()) || self.section == Section::Collection(k) {
                v.push(Side::Entry(Section::Collection(k)));
            }
        }
        v.push(Side::Title("LIBRARY"));
        for s in [Section::Sounds, Section::Drums, Section::Instruments, Section::Hardware, Section::Clips, Section::Samples] {
            v.push(Side::Entry(s));
        }
        v.push(Side::Title("PLACES"));
        v.push(Side::Entry(Section::User));
        v.push(Side::Entry(Section::Project));
        v
    }

    /// A section's folder (dynamic ones built from the project and files).
    fn folder_of(&self, sec: Section) -> Folder {
        let lib = bank::library();
        match sec {
            Section::Sounds => lib.sounds.clone(),
            Section::Drums => lib.drums.clone(),
            Section::Instruments => lib.instruments.clone(),
            Section::Hardware => lib.hardware.clone(),
            Section::Clips => lib.clips.clone(),
            Section::Samples => {
                let mut f = Folder { name: "Samples".into(), ..Default::default() };
                let b = chip::samples::BUILTIN_COUNT;
                f.items = (0..b).map(Item::Sample).collect();
                let filled: Vec<usize> = self.s.arrangement.with_untracked(|a| a.samples.iter().enumerate().filter(|(_, x)| x.is_some()).map(|(i, _)| b + i).collect());
                f.items.extend(filled.into_iter().map(Item::Sample));
                f.items.push(Item::ImportAudio);
                f
            }
            Section::User => Folder { name: "User Library".into(), items: (0..self.s.user_presets.with_untracked(|u| u.len())).map(Item::User).collect(), ..Default::default() },
            Section::Project => {
                // The presets the project's tracks play, in track order.
                let names: Vec<String> = self.s.arrangement.with_untracked(|a| (0..a.lanes.len()).filter_map(|l| a.instruments.lane(l).map(|i| i.name.clone())).collect());
                let mut items: Vec<Item> = Vec::new();
                for n in names {
                    let it = bank::index_of(&n).map(Item::Preset).or_else(|| self.s.user_presets.with_untracked(|u| u.iter().position(|p| p.name.eq_ignore_ascii_case(n.trim_end_matches('*')))).map(Item::User));
                    if let Some(it) = it.filter(|it| !items.contains(it)) {
                        items.push(it);
                    }
                }
                let b = chip::samples::BUILTIN_COUNT;
                let used: Vec<usize> = self.s.arrangement.with_untracked(|a| a.samples.iter().enumerate().filter(|(_, x)| x.is_some()).map(|(i, _)| b + i).collect());
                items.extend(used.into_iter().map(Item::Sample));
                Folder { name: "Current Project".into(), items, ..Default::default() }
            }
            Section::Collection(k) => {
                let lists = self.s.settings.with_untracked(|st| st.collections.clone());
                let items = self.s.user_presets.with_untracked(|u| crate::collections::items(&lists, k, u));
                Folder { name: Section::Collection(k).label().into(), items, ..Default::default() }
            }
        }
    }

    fn matches(&self, it: Item, words: &[String]) -> bool {
        match it {
            Item::Preset(i) => bank::get(i).is_some_and(|p| p.matches(words)),
            Item::ImportAudio => false,
            other => {
                let hay = format!("{} {}", item_name(&self.s, other), item_help(&self.s, other)).to_lowercase();
                words.iter().all(|w| hay.contains(w.as_str()))
            }
        }
    }

    /// The rows: the section's folders (open ones expanded), or with a
    /// query, every match across the library under its folders.
    fn refresh(&mut self) {
        self.side = self.side_entries();
        let words = self.query_words();
        let mut rows = Vec::new();
        if words.is_empty() {
            let f = self.folder_of(self.section);
            let key = self.section.key();
            push_folder(&mut rows, &f, &[key], 0, &|k| self.open.contains(k), None);
        } else {
            for sec in [Section::Sounds, Section::Drums, Section::Hardware, Section::Clips, Section::Samples, Section::User] {
                let f = self.folder_of(sec);
                let keep = |it: &Item| self.matches(*it, &words);
                let mut part = Vec::new();
                push_folder(&mut part, &f, &[sec.key()], 1, &|_| true, Some(&keep));
                if !part.is_empty() {
                    rows.push(Row::Folder { path: vec![sec.key()], depth: 0, open: true });
                    rows.extend(part);
                }
            }
        }
        self.rows = rows;
        self.hover = None;
        if crate::dev::enabled() {
            let items = self.rows.iter().filter(|r| matches!(r, Row::Item { .. })).count();
            eprintln!("dev: browser {} rows {} items {items} query {:?}", self.section.key(), self.rows.len(), words.join(" "));
        }
    }

    /// Show in Browser: the preset's section, its folders opened, selected.
    fn reveal(&mut self) {
        let Some(p) = self.s.browser_reveal.get_untracked() else { return };
        let it = if p >= bank::len() { Item::User(p - bank::len()) } else { Item::Preset(p) };
        if p >= bank::len() {
            self.section = Section::User;
        } else if let Some((path, _)) = bank::home_of(p) {
            self.section = Section::from_key(&path[0]);
            let mut key = String::new();
            for (k, part) in path.iter().enumerate() {
                key = if k == 0 { part.clone() } else { format!("{key}/{part}") };
                self.open.insert(key.clone());
            }
        }
        self.s.browser_query.set(String::new());
        self.sel = Some(it);
        self.refresh();
        if let Some(i) = self.rows.iter().position(|r| matches!(r, Row::Item { item, .. } if *item == it)) {
            self.scroll = (i as f32 * ROW_H - 3.0 * ROW_H).max(0.0);
        }
        self.s.browser_reveal.set(None);
    }

    fn content_h(&self) -> f32 {
        TITLE_H + self.rows.len() as f32 * ROW_H
    }

    fn local(cx: &EventContext) -> (f32, f32) {
        let b = cx.bounds();
        let s = cx.scale_factor();
        ((cx.mouse().cursor_x - b.x) / s, (cx.mouse().cursor_y - b.y) / s)
    }

    fn inside(cx: &EventContext) -> bool {
        let b = cx.bounds();
        let (x, y) = (cx.mouse().cursor_x, cx.mouse().cursor_y);
        x >= b.x && x <= b.x + b.w && y >= b.y && y <= b.y + b.h
    }

    fn side_at(&self, p: (f32, f32)) -> Option<usize> {
        if p.0 >= SIDEBAR_W {
            return None;
        }
        let mut y = 0.0;
        for (i, e) in self.side.iter().enumerate() {
            let h = if matches!(e, Side::Title(_)) { SIDE_TITLE_H } else { ROW_H };
            if p.1 >= y && p.1 < y + h {
                return matches!(e, Side::Entry(_)).then_some(i);
            }
            y += h;
        }
        None
    }

    fn row_at(&self, p: (f32, f32)) -> Option<usize> {
        if p.0 < SIDEBAR_W || p.1 < TITLE_H {
            return None;
        }
        let i = ((p.1 - TITLE_H + self.scroll) / ROW_H).floor();
        (i >= 0.0 && (i as usize) < self.rows.len()).then_some(i as usize)
    }

    fn toggle(&mut self, i: usize) {
        if let Some(Row::Folder { path, .. }) = self.rows.get(i) {
            if !self.query_words().is_empty() {
                return;
            }
            let key = path.join("/");
            let opened = !self.open.remove(&key);
            if opened {
                self.open.insert(key.clone());
            }
            if crate::dev::enabled() {
                eprintln!("dev: browser {} {key}", if opened { "open" } else { "close" });
            }
            self.refresh();
        }
    }

    /// Selects an item and previews it (the Preview key, while stopped).
    fn select(&mut self, cx: &mut EventContext, it: Item, brief: bool) {
        self.sel = Some(it);
        let preset = match it {
            Item::Preset(i) => Some(i),
            Item::User(u) => Some(bank::len() + u),
            _ => None,
        };
        cx.emit(AppEvent::BrowserSelect(preset));
        cx.emit(AppEvent::HoverHelp(item_help(&self.s, it)));
        match (it, preset) {
            (_, Some(p)) => cx.emit(if brief { AppEvent::AuditionBrief(p) } else { AppEvent::Audition(Some(p)) }),
            (Item::Sample(i), _) if !brief => {
                cx.emit(AppEvent::Preview { lane: chip::driver::LANE_DMC, note: Some(((36 + i) as u8, None)) });
                self.sample_held = true;
            }
            _ => {}
        }
    }

    /// Double-click / Enter: load the item (a clip on the selected track
    /// at the insert marker; a sample on the selected DPCM Sampler track,
    /// else the first one, else a new one).
    fn load(&self, cx: &mut EventContext, it: Item) {
        let sel = self.s.lane_sel.get_untracked();
        let (n, dpcm) = self.s.arrangement.with_untracked(|a| {
            let is = |l: usize| a.lanes.get(l).is_some_and(|t| t.voice == chip::voice::VoiceKind::Dpcm);
            (a.lanes.len(), if is(sel) { Some(sel) } else { (0..a.lanes.len()).find(|&l| is(l)) })
        });
        match it {
            Item::Preset(i) => cx.emit(AppEvent::AssignPreset { lane: None, preset: i }),
            Item::User(u) => cx.emit(AppEvent::AssignPreset { lane: None, preset: bank::len() + u }),
            Item::Band(i) => cx.emit(AppEvent::ApplyLayout(i)),
            Item::Clip(i) => cx.emit(AppEvent::InsertPhrase { phrase: i, lane: (sel < n).then_some(sel), tick: None }),
            Item::Sample(i) => cx.emit(AppEvent::AssignSample { slot: i, lane: dpcm }),
            Item::ImportAudio => cx.emit(AppEvent::ImportAudio),
        }
    }

    fn keep_visible(&mut self, i: usize, view_h: f32) {
        let top = i as f32 * ROW_H;
        let room = view_h - TITLE_H;
        if top < self.scroll {
            self.scroll = top;
        } else if top + ROW_H > self.scroll + room {
            self.scroll = top + ROW_H - room;
        }
    }

    /// The tick at a window x (snapped to the bar), for a clip drop.
    fn tick_at(&self, x: f32) -> Option<u32> {
        let (left, _, s) = super::arrangement::arrangement_at()?;
        let t = self.s.arrangement.with_untracked(|a| a.view.x_to_tick(((x - left) / s) as f64));
        let bar = crate::state::TICKS_PER_BAR as f64;
        Some(((t.max(0.0) / bar).floor() * bar) as u32)
    }
}

/// Lists a folder's rows below `path`: sub-folders (with their open state),
/// then items. `keep` (search): only items it keeps, folders that hold any,
/// all open.
fn push_folder(rows: &mut Vec<Row>, f: &Folder, path: &[String], depth: usize, open: &dyn Fn(&str) -> bool, keep: Option<&dyn Fn(&Item) -> bool>) {
    for sub in &f.folders {
        let mut p = path.to_vec();
        p.push(sub.name.clone());
        let key = p.join("/");
        if let Some(k) = keep {
            let mut inner = Vec::new();
            push_folder(&mut inner, sub, &p, depth + 1, open, Some(k));
            if !inner.is_empty() {
                rows.push(Row::Folder { path: p, depth, open: true });
                rows.extend(inner);
            }
        } else {
            let o = open(&key);
            rows.push(Row::Folder { path: p.clone(), depth, open: o });
            if o {
                push_folder(rows, sub, &p, depth + 1, open, None);
            }
        }
    }
    for it in &f.items {
        if keep.is_none_or(|k| k(it)) {
            rows.push(Row::Item { item: *it, depth });
        }
    }
}

impl View for LibraryView {
    fn element(&self) -> Option<&'static str> {
        Some("preset-list")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|we, meta| match we {
            WindowEvent::KeyDown(code, _) if self.s.rename.with_untracked(|r| r.is_none()) && !self.rows.is_empty() => {
                let cur = self.sel.and_then(|it| self.rows.iter().position(|r| matches!(r, Row::Item { item, .. } if *item == it))).or(self.hover);
                let view_h = cx.bounds().h / cx.scale_factor();
                match code {
                    Code::ArrowUp | Code::ArrowDown => {
                        let down = *code == Code::ArrowDown;
                        let n = self.rows.len();
                        let from = cur.map_or(if down { usize::MAX } else { n }, |c| c);
                        let next = if down { (from.wrapping_add(1)..n).find(|i| matches!(self.rows[*i], Row::Item { .. })) } else { (0..from.min(n)).rev().find(|i| matches!(self.rows[*i], Row::Item { .. })) };
                        if let Some(i) = next {
                            if let Row::Item { item, .. } = self.rows[i] {
                                self.select(cx, item, true);
                                self.keep_visible(i, view_h);
                            }
                        }
                    }
                    Code::Enter => {
                        if let Some(it) = self.sel {
                            self.load(cx, it);
                        }
                    }
                    _ => return,
                }
                cx.needs_redraw();
                meta.consume();
            }
            WindowEvent::MouseScroll(_, sy) => {
                let max = (self.content_h() - cx.bounds().h / cx.scale_factor()).max(0.0);
                self.scroll = (self.scroll - sy * WHEEL_PX).clamp(0.0, max);
                self.hover = self.row_at(Self::local(cx));
                cx.needs_redraw();
                meta.consume();
            }
            WindowEvent::MouseDown(MouseButton::Left) => {
                let p = Self::local(cx);
                cx.focus();
                if let Some(i) = self.side_at(p) {
                    if let Side::Entry(sec) = self.side[i] {
                        self.section = sec;
                        self.scroll = 0.0;
                        self.s.browser_query.set(String::new());
                        self.refresh();
                        cx.emit(AppEvent::BrowserSection(sec.key()));
                        cx.needs_redraw();
                    }
                } else if let Some(i) = self.row_at(p) {
                    match self.rows[i].clone() {
                        Row::Folder { .. } => self.toggle(i),
                        Row::Item { item, .. } => {
                            if item == Item::ImportAudio {
                                cx.emit(AppEvent::ImportAudio);
                            } else {
                                self.press = Some((i, cx.mouse().cursor_x, cx.mouse().cursor_y));
                                self.dragging = false;
                                cx.capture();
                                self.select(cx, item, false);
                            }
                        }
                    }
                    cx.needs_redraw();
                }
                meta.consume();
            }
            WindowEvent::MouseDoubleClick(MouseButton::Left) => {
                if let Some(i) = self.row_at(Self::local(cx)) {
                    if let Row::Item { item, .. } = self.rows[i] {
                        self.load(cx, item);
                    }
                }
                meta.consume();
            }
            WindowEvent::MouseDown(MouseButton::Right) => {
                if let Some(i) = self.row_at(Self::local(cx)) {
                    if let Row::Item { item, .. } = self.rows[i] {
                        if item != Item::ImportAudio {
                            self.sel = Some(item);
                            let s = cx.scale_factor();
                            cx.emit(AppEvent::ContextMenu(crate::edit::MenuSite::BrowserItem(item), cx.mouse().cursor_x / s, cx.mouse().cursor_y / s));
                            cx.needs_redraw();
                        }
                    }
                }
                meta.consume();
            }
            WindowEvent::MouseMove(x, y) => {
                if let Some((i, px, py)) = self.press {
                    let s = cx.scale_factor();
                    if !self.dragging && ((x - px).abs() > DRAG_START_PX * s || (y - py).abs() > DRAG_START_PX * s) {
                        self.dragging = true;
                        cx.emit(AppEvent::Audition(None));
                    }
                    if self.dragging {
                        if let Some(Row::Item { item, .. }) = self.rows.get(i) {
                            let out = !Self::inside(cx);
                            let target = lane_at(*x, *y).filter(|_| out);
                            let n = self.s.arrangement.with_untracked(|a| a.lanes.len());
                            let empty = out && target.is_none() && over_empty_space(*x, *y, n);
                            let is_preset = matches!(item, Item::Preset(_) | Item::User(_));
                            let clip = target
                                .filter(|_| is_preset)
                                .and_then(|_| self.s.arrangement.with_untracked(|a| super::arrangement::clip_at(a, *x, *y)));
                            self.s.drag_preset.set(Some(PresetDrag { item: *item, x: *x, y: *y, target, clip, empty }));
                        }
                    }
                } else {
                    let p = Self::local(cx);
                    let inside = Self::inside(cx);
                    let h = inside.then(|| self.row_at(p)).flatten();
                    let hs = inside.then(|| self.side_at(p)).flatten();
                    if h != self.hover || hs != self.hover_side {
                        self.hover = h;
                        self.hover_side = hs;
                        let help = match h.map(|i| self.rows[i].clone()) {
                            Some(Row::Item { item, .. }) => item_help(&self.s, item),
                            Some(Row::Folder { path, .. }) => format!("{}: click to open or close", path.last().cloned().unwrap_or_default()),
                            None => match hs.map(|i| self.side[i].clone()) {
                                Some(Side::Entry(Section::Hardware)) => "Hardware: authentic presets by profile and channel; loading one locks the track".into(),
                                Some(Side::Entry(Section::Collection(_))) => "A collection: right-click any item to add it".into(),
                                Some(Side::Entry(sec)) => sec.label().to_string(),
                                _ => String::new(),
                            },
                        };
                        cx.emit(AppEvent::HoverHelp(help));
                        cx.needs_redraw();
                    }
                }
            }
            WindowEvent::MouseUp(MouseButton::Left) if self.press.is_some() => {
                cx.release();
                if self.sample_held {
                    cx.emit(AppEvent::Preview { lane: chip::driver::LANE_DMC, note: None });
                    self.sample_held = false;
                }
                if self.dragging {
                    if let Some(d) = self.s.drag_preset.get_untracked() {
                        let tick = self.tick_at(d.x);
                        match (d.item, d.target, d.clip, d.empty) {
                            (Item::Preset(_) | Item::User(_), _, Some(clip), _) => cx.emit(AppEvent::AssignPresetToClip { clip, preset: d.preset().unwrap_or(0) }),
                            (Item::Preset(_) | Item::User(_), Some(lane), None, _) => cx.emit(AppEvent::AssignPreset { lane: Some(lane), preset: d.preset().unwrap_or(0) }),
                            (Item::Preset(_) | Item::User(_), None, _, true) => cx.emit(AppEvent::PresetNewTrack(d.preset().unwrap_or(0))),
                            (Item::Band(i), t, _, e) if t.is_some() || e => cx.emit(AppEvent::ApplyLayout(i)),
                            (Item::Clip(i), Some(lane), _, _) => cx.emit(AppEvent::InsertPhrase { phrase: i, lane: Some(lane), tick }),
                            (Item::Clip(i), None, _, true) => cx.emit(AppEvent::InsertPhrase { phrase: i, lane: None, tick }),
                            (Item::Sample(i), Some(lane), _, _) => cx.emit(AppEvent::AssignSample { slot: i, lane: Some(lane) }),
                            (Item::Sample(i), None, _, true) => cx.emit(AppEvent::AssignSample { slot: i, lane: None }),
                            _ => {}
                        }
                    }
                    self.s.drag_preset.set(None);
                } else {
                    cx.emit(AppEvent::Audition(None));
                }
                self.press = None;
                self.dragging = false;
                meta.consume();
            }
            WindowEvent::MouseLeave if self.hover.is_some() || self.hover_side.is_some() => {
                self.hover = None;
                self.hover_side = None;
                cx.needs_redraw();
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let b = cx.bounds();
        let s = cx.scale_factor();
        let rect: vg::Rect = b.into();
        canvas.save();
        canvas.clip_rect(rect, None, None);
        let ftitle = gfx::font(Face::SemiBold, 10.0 * s);
        let fside = gfx::font(Face::Medium, 12.0 * s);
        let ffold = gfx::font(Face::SemiBold, 12.0 * s);
        // ── The sidebar.
        let side = vg::Rect::from_xywh(rect.left, rect.top, SIDEBAR_W * s, rect.height());
        canvas.draw_rect(vg::Rect::from_xywh(side.right - s, rect.top, s, rect.height()), &gfx::fill(Rgba::hex(0x1A1A1A)));
        let mut y = rect.top;
        for (i, e) in self.side.iter().enumerate() {
            match e {
                Side::Title(t) => {
                    let base = gfx::css_baseline(&ftitle, y + 12.0 * s, 12.0 * s);
                    gfx::draw_text_tracked(canvas, t, (rect.left + PAD_X * s).round(), base.round(), &ftitle, 1.0 * s, DEVICE_INK_LO);
                    y += SIDE_TITLE_H * s;
                }
                Side::Entry(sec) => {
                    let r = vg::Rect::from_xywh(rect.left, y, SIDEBAR_W * s - s, ROW_H * s);
                    let selected = *sec == self.section && self.s.browser_query.with_untracked(|q| q.trim().is_empty());
                    if selected {
                        canvas.draw_rect(r, &gfx::fill(Rgba::hex(0x242424)));
                        canvas.draw_rect(vg::Rect::from_xywh(r.left, r.top, 2.0 * s, r.height()), &gfx::fill(Rgba::hex(0x8C8C8C)));
                    } else if self.hover_side == Some(i) {
                        canvas.draw_rect(r, &gfx::fill(Rgba::white(0.03)));
                    }
                    let mut x = r.left + PAD_X * s;
                    let base = gfx::baseline_for_center(&fside, r.center_y());
                    if let Section::Collection(k) = sec {
                        let sw = vg::Rect::from_xywh(x, r.center_y() - 4.0 * s, 8.0 * s, 8.0 * s);
                        canvas.draw_rrect(gfx::rrect(sw, 2.0 * s), &gfx::fill(crate::collections::COLLECTIONS[*k].1));
                        x += 16.0 * s;
                    }
                    if *sec == Section::Hardware {
                        super::device::lock_glyph(canvas, (x.round(), (base - 8.0 * s).round()), s, Rgba::hex(0x9A9A9A));
                        x += 12.0 * s;
                    }
                    gfx::draw_text(canvas, sec.label(), x.round(), base.round(), &fside, if selected { TEXT_HI } else { Rgba::hex(0xB0B0B0) });
                    y += ROW_H * s;
                }
            }
        }
        // ── The content pane.
        let pane = vg::Rect::from_ltrb(side.right, rect.top, rect.right, rect.bottom);
        let searching = !self.query_words().is_empty();
        let title = if searching { format!("RESULTS  ·  {}", self.rows.iter().filter(|r| matches!(r, Row::Item { .. })).count()) } else { self.section.label().to_uppercase() };
        let tbase = gfx::baseline_for_center(&ftitle, pane.top + TITLE_H * 0.5 * s);
        gfx::draw_text_tracked(canvas, &title, (pane.left + PAD_X * s).round(), tbase.round(), &ftitle, 1.0 * s, DEVICE_INK_LO);
        canvas.draw_rect(vg::Rect::from_xywh(pane.left, pane.top + TITLE_H * s - s, pane.width(), s), &gfx::fill(Rgba::hex(0x1A1A1A)));
        canvas.save();
        let list = vg::Rect::from_ltrb(pane.left, pane.top + TITLE_H * s, pane.right, pane.bottom);
        canvas.clip_rect(list, None, None);
        let sel = self.sel;
        for (i, row) in self.rows.iter().enumerate() {
            let top = list.top + (i as f32 * ROW_H - self.scroll) * s;
            if top + ROW_H * s < list.top || top > list.bottom {
                continue;
            }
            let r = vg::Rect::from_xywh(pane.left, top.round(), pane.width(), (ROW_H * s).round());
            let x = r.left + (PAD_X + row.depth() as f32 * INDENT) * s;
            match row {
                Row::Folder { path, open, .. } => {
                    if self.hover == Some(i) {
                        canvas.draw_rect(r, &gfx::fill(Rgba::white(0.03)));
                    }
                    let (d, ink) = if *open { ("M1 2L4 6L7 2", Rgba::hex(0x9A9A9A)) } else { ("M2 1L6 4L2 7", Rgba::hex(0x808080)) };
                    let mut st = gfx::stroke(ink, 1.4 * s);
                    st.set_stroke_cap(vg::PaintCap::Butt);
                    canvas.draw_path(&gfx::svg_path(d, (x.round(), (r.center_y() - 4.0 * s).round()), s), &st);
                    let name = path.last().map(String::as_str).unwrap_or("");
                    gfx::draw_text(canvas, name, (x + 16.0 * s).round(), gfx::baseline_for_center(&ffold, r.center_y()).round(), &ffold, Rgba::hex(0xD0D0D0));
                }
                Row::Item { item, .. } => {
                    let selected = sel == Some(*item);
                    if selected {
                        canvas.draw_rect(r, &gfx::fill(Rgba::hex(0xBDBDBD)));
                    } else if self.hover == Some(i) {
                        canvas.draw_rect(r, &gfx::fill(Rgba::white(0.05)));
                    }
                    let ink = if selected { Rgba::hex(0x111111) } else { Rgba::hex(0xB0B0B0) };
                    let mut tx = x + 16.0 * s;
                    if *item == Item::ImportAudio {
                        gfx::draw_text(canvas, "+", x.round(), gfx::baseline_for_center(&fside, r.center_y()).round(), &fside, ink);
                    } else {
                        canvas.draw_circle((x + 4.0 * s, r.center_y()), 2.0 * s, &gfx::fill(Rgba::hex(0x5A5A5A)));
                    }
                    let base = gfx::baseline_for_center(&fside, r.center_y());
                    if matches!(item, Item::Preset(p) if bank::get(*p).is_some_and(|p| p.authentic())) {
                        super::device::lock_glyph(canvas, (tx.round(), (base - 8.0 * s).round()), s, if selected { Rgba::hex(0x333333) } else { Rgba::hex(0x9A9A9A) });
                        tx += 12.0 * s;
                    }
                    gfx::draw_text_clipped(canvas, &item_name(&self.s, *item), tx.round(), base.round(), r.right - tx - 8.0 * s, &fside, ink);
                }
            }
        }
        canvas.restore();
        canvas.restore();
    }
}

/// The magnifier in the search field (the board's 12 px glyph).
pub struct SearchGlyph;

impl View for SearchGlyph {
    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let s = cx.scale_factor();
        let b = cx.bounds();
        let o = ((b.x).round(), (b.y + b.h * 0.5 - 6.0 * s).round());
        let ink = Rgba::hex(0x6A6A6A);
        canvas.draw_circle((o.0 + 5.0 * s, o.1 + 5.0 * s), 3.5 * s, &gfx::stroke(ink, 1.3 * s));
        canvas.draw_line((o.0 + 8.0 * s, o.1 + 8.0 * s), (o.0 + 11.0 * s, o.1 + 11.0 * s), &gfx::stroke(ink, 1.3 * s));
    }
}

/// The browser.
pub fn browser(cx: &mut Context, s: Signals) {
    VStack::new(cx, |cx| {
        HStack::new(cx, |cx| {
            Label::new(cx, "BROWSER").class("browser-title").help(format!("The browser ({} shows or hides it)", crate::shortcuts::label(crate::shortcuts::Action::ToggleBrowser)));
            Element::new(cx).width(Stretch(1.0)).height(Pixels(1.0));
            PreviewKey::new(cx, s).help_fn(move || {
                if s.settings.with_untracked(|st| st.preview_on_select) {
                    "Preview is on: selecting an item (click or the arrow keys) plays it while the song is stopped. Click to turn it off."
                } else {
                    "Preview is off: selecting an item is silent. Click to turn it on."
                }
                .into()
            });
        })
        .class("browser-head");
        HStack::new(cx, |cx| {
            SearchGlyph.build(cx, |_| {}).width(Pixels(12.0)).height(Stretch(1.0)).hoverable(false);
            Textbox::new(cx, s.browser_query)
                .placeholder("Search sounds, devices, clips")
                .on_edit(move |_cx, t| s.browser_query.set(t))
                // B1: Esc clears the search and leaves the field.
                .on_cancel(move |cx| {
                    s.browser_query.set(String::new());
                    cx.emit(TextEvent::EndEdit);
                })
                .width(Stretch(1.0));
        })
        .class("field")
        .class("browser-search");
        LibraryView::new(cx, s).height(Stretch(1.0)).width(Stretch(1.0));
    })
    .class("browser")
    .width(Pixels(browser_w()))
    .display(s.browser_open.map(|o| if *o { Display::Flex } else { Display::None }));
}

/// The item following the pointer while dragging.
pub struct DragGhost {
    s: Signals,
}

impl DragGhost {
    pub fn new(cx: &mut Context, s: Signals) -> Handle<'_, Self> {
        Self { s }
            .build(cx, |_| {})
            .position_type(PositionType::Absolute)
            .width(Stretch(1.0))
            .height(Stretch(1.0))
            .hoverable(false)
            .bind(s.drag_preset, |mut h| h.needs_redraw())
    }
}

impl View for DragGhost {
    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let Some(d) = self.s.drag_preset.get_untracked() else { return };
        let s = cx.scale_factor();
        let name = item_name(&self.s, d.item);
        let fits = d.target.and_then(|l| drag_state(&self.s, l)).map(|(f, _)| f);
        let f = gfx::font(Face::SemiBold, 12.0 * s);
        let track = |l: usize| self.s.arrangement.with_untracked(|a| a.lanes.get(l).map_or(String::new(), |t| t.name.clone()));
        let label = match (d.target, fits, d.empty) {
            (Some(_), Some(true), _) if d.clip.is_some() => format!("{name}  →  this clip"),
            (Some(l), Some(true), _) => format!("{name}  →  {}", track(l)),
            (Some(l), Some(false), _) => format!("{name}  →  {} (asks first)", track(l)),
            (None, _, true) => format!("{name}  →  a new track"),
            _ => name,
        };
        let w = gfx::text_width(&f, &label) + 32.0 * s;
        let r = vg::Rect::from_xywh(d.x + 12.0 * s, d.y + 8.0 * s, w, 28.0 * s);
        let rr = gfx::rrect(r, RADIUS_BOX * s);
        gfx::drop_shadow(canvas, &rr, 0.0, 6.0 * s, 16.0 * s, 0.0, Rgba::black(0.55));
        canvas.draw_rrect(rr, &gfx::vgrad(r, Rgba::hex(0x262626), Rgba::hex(0x161616)));
        let edge = match (fits, d.empty) {
            (Some(true), _) | (_, true) => NEUTRAL_HI,
            (Some(false), _) => RECORD,
            _ => STROKE,
        };
        gfx::ring(canvas, &rr, gfx::hairline(s), &gfx::fill(edge.with_alpha(0.7)));
        gfx::top_sheen(canvas, &rr, gfx::hairline(s), KEY_STROKE, KEY_SHEEN);
        canvas.draw_circle((r.left + 16.0 * s, r.center_y()), 2.5 * s, &gfx::fill(Rgba::hex(0x9A9A9A)));
        let base = gfx::baseline_for_center(&f, r.center_y());
        gfx::draw_text(canvas, &label, (r.left + 24.0 * s).round(), base.round(), &f, TEXT_HI);
    }
}

/// The browser header's Preview key (design pass §3.7): a headphone glyph
/// in a plain hairline key. On: auditions items on select; off: silent.
/// Neutral in both states (lime is for the listed on-states only).
pub struct PreviewKey {
    s: Signals,
    hover: bool,
}

/// The key's size (logical px): 24 × 24, centred on the title row.
const PREVIEW_W: f32 = 24.0;
const PREVIEW_H: f32 = 24.0;

impl PreviewKey {
    pub fn new(cx: &mut Context, s: Signals) -> Handle<'_, Self> {
        Self { s, hover: false }
            .build(cx, |_| {})
            .id("preview-key")
            .width(Pixels(PREVIEW_W))
            .height(Pixels(PREVIEW_H))
            .cursor(CursorIcon::Hand)
            .bind(s.settings, |mut h| h.needs_redraw())
    }
}

impl View for PreviewKey {
    fn element(&self) -> Option<&'static str> {
        Some("preview-key")
    }

    fn event(&mut self, cx: &mut EventContext, event: &mut Event) {
        event.map(|we, meta| match we {
            WindowEvent::MouseDown(MouseButton::Left) => {
                cx.emit(AppEvent::TogglePreviewOnSelect);
                meta.consume();
            }
            WindowEvent::MouseEnter => {
                self.hover = true;
                cx.needs_redraw();
            }
            WindowEvent::MouseLeave => {
                self.hover = false;
                cx.needs_redraw();
            }
            _ => {}
        });
    }

    fn draw(&self, cx: &mut DrawContext, canvas: &Canvas) {
        let s = cx.scale_factor();
        let r = gfx::snap(cx.bounds().into());
        let on = self.s.settings.with_untracked(|st| st.preview_on_select);
        let rr = gfx::rrect(r, RADIUS_KEY * s);
        // The board's key: lit from below, a #383838 hairline.
        canvas.draw_rrect(rr, &gfx::vgrad(r, Rgba::hex(0x161616), Rgba::hex(if self.hover { 0x4A4A4A } else { 0x424242 })));
        gfx::ring(canvas, &rr, gfx::hairline(s), &gfx::fill(Rgba::hex(0x383838)));
        let ink = if on { Rgba::hex(0x9A9A9A) } else { TEXT_LO };
        // A 12 × 12 headphone: the band (an arc) and two cups.
        let o = ((r.center_x() - 6.0 * s).round(), (r.center_y() - 6.0 * s).round());
        let band = gfx::svg_path("M2 7V6a4 4 0 0 1 8 0v1", o, s);
        let mut st = gfx::stroke(ink, 1.3 * s);
        st.set_stroke_cap(vg::PaintCap::Round);
        canvas.draw_path(&band, &st);
        for x in [1.5, 8.0] {
            let cup = vg::Rect::from_xywh(o.0 + x * s, o.1 + 7.0 * s, 2.5 * s, 3.5 * s);
            canvas.draw_rrect(gfx::rrect(cup, 1.0 * s), &gfx::fill(ink));
        }
        if !on {
            // Off: a slash through the glyph.
            let mut sl = gfx::stroke(ink, 1.3 * s);
            sl.set_stroke_cap(vg::PaintCap::Round);
            canvas.draw_line((o.0 + 0.5 * s, o.1 + 11.0 * s), (o.0 + 11.5 * s, o.1), &sl);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sections_round_trip_and_nothing_pretends() {
        for s in [Section::Sounds, Section::Drums, Section::Instruments, Section::Hardware, Section::Clips, Section::Samples, Section::User, Section::Project, Section::Collection(3)] {
            assert_eq!(Section::from_key(&s.key()), s);
        }
        // No section for content that doesn't exist yet.
        for name in ["Audio Effects", "MIDI Effects", "Imported Projects"] {
            assert_eq!(Section::from_key(name), Section::Sounds, "{name}");
        }
    }

    #[test]
    fn folders_list_and_search_opens_them() {
        let lib = bank::library();
        let mut rows = Vec::new();
        push_folder(&mut rows, &lib.sounds, &["Sounds".into()], 0, &|k| k == "Sounds/Leads", None);
        // Leads open, its sub-folders closed; the other roles closed.
        assert!(matches!(&rows[0], Row::Folder { path, open: true, .. } if path.last().unwrap() == "Leads"));
        assert!(rows.iter().filter(|r| matches!(r, Row::Folder { depth: 0, .. })).count() == 7);
        assert!(rows.iter().all(|r| !matches!(r, Row::Item { .. })), "items stay in their sub-folders");
        // A search keeps only matches, folders opened.
        let words = vec!["echo".to_string()];
        let keep = |it: &Item| matches!(it, Item::Preset(i) if bank::get(*i).unwrap().matches(&words));
        let mut found = Vec::new();
        push_folder(&mut found, &lib.sounds, &["Sounds".into()], 0, &|_| false, Some(&keep));
        let names: Vec<String> = found.iter().filter_map(|r| if let Row::Item { item: Item::Preset(i), .. } = r { Some(bank::get(*i).unwrap().name.clone()) } else { None }).collect();
        assert!(names.contains(&"Echo Square".to_string()) && names.iter().all(|n| bank::find(n).unwrap().matches(&words)));
        assert!(found.iter().all(|r| !matches!(r, Row::Folder { open: false, .. })));
    }
}
