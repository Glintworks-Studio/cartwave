//! User presets: instruments saved from a project ("Save as user preset"),
//! kept in `user_presets.json` in the app-data folder (next to the recovery
//! file) and listed in the browser after the factory presets.
//!
//! Browser and event indices cover both kinds: `0..PRESETS.len()` are the
//! factory presets, the rest are user presets in file order (`PresetView`).

use chip::presets::{lane_kind, PRESETS};
use engine::ParamId;
use serde::{Deserialize, Serialize};

use crate::instruments::LaneInstrument;

#[derive(Clone, Debug, PartialEq)]
pub struct UserPreset {
    pub name: String,
    /// The channel kinds it was made for (`chip::presets::K_*`): the
    /// macros mean different things on different channels.
    pub kinds: u8,
    pub inst: LaneInstrument,
}

#[derive(Serialize, Deserialize)]
struct FileEntry {
    kinds: u8,
    instrument: serde_json::Value,
}

#[derive(Serialize, Deserialize, Default)]
struct PresetFile {
    #[serde(default)]
    presets: Vec<FileEntry>,
}

const FILE: &str = "user_presets.json";
/// Most user presets kept.
pub const MAX_USER_PRESETS: usize = 256;

fn path() -> Option<std::path::PathBuf> {
    crate::file::data_dir().map(|d| d.join(FILE))
}

/// The saved user presets (empty if there are none, or the file is
/// unreadable: a damaged entry is skipped, never fatal).
pub fn load() -> Vec<UserPreset> {
    let Some(text) = path().and_then(|p| std::fs::read_to_string(p).ok()) else { return Vec::new() };
    let Ok(f) = serde_json::from_str::<PresetFile>(&text) else { return Vec::new() };
    f.presets
        .into_iter()
        .filter_map(|e| {
            let inst = crate::file::instrument_from_json(e.instrument).ok()?;
            Some(UserPreset { name: inst.name.clone(), kinds: e.kinds, inst })
        })
        .take(MAX_USER_PRESETS)
        .collect()
}

/// A name not used by a factory or user preset: "Cello" → "Cello 2".
fn unique_name(base: &str, user: &[UserPreset]) -> String {
    let base = base.trim().trim_end_matches('*').trim();
    let base = if base.is_empty() { "My Instrument" } else { base };
    let taken = |n: &str| PRESETS.iter().any(|p| p.name.eq_ignore_ascii_case(n)) || user.iter().any(|u| u.name.eq_ignore_ascii_case(n));
    if !taken(base) {
        return base.to_string();
    }
    (2..).map(|i| format!("{base} {i}")).find(|n| !taken(n)).unwrap_or_default()
}

/// Appends `inst` (made on `lane`) to the user presets file, atomically.
/// Returns the name it was saved under.
pub fn save(inst: &LaneInstrument, lane: usize) -> Result<String, String> {
    save_to(&path().ok_or("There is no app-data folder to save presets in.")?, inst, lane)
}

pub fn save_to(path: &std::path::Path, inst: &LaneInstrument, lane: usize) -> Result<String, String> {
    let mut user = std::fs::read_to_string(path)
        .ok()
        .and_then(|t| serde_json::from_str::<PresetFile>(&t).ok())
        .unwrap_or_default();
    if user.presets.len() >= MAX_USER_PRESETS {
        return Err(format!("You have {MAX_USER_PRESETS} user presets, the most there can be."));
    }
    let existing: Vec<UserPreset> = user
        .presets
        .iter()
        .filter_map(|e| crate::file::instrument_from_json(e.instrument.clone()).ok().map(|i| UserPreset { name: i.name.clone(), kinds: e.kinds, inst: i }))
        .collect();
    let mut copy = inst.clone();
    copy.name = unique_name(&inst.name, &existing);
    copy.edited = false;
    user.presets.push(FileEntry { kinds: lane_kind(lane), instrument: crate::file::instrument_json(&copy) });
    if let Some(dir) = path.parent() {
        std::fs::create_dir_all(dir).map_err(|e| format!("Can't create {}: {e}", dir.display()))?;
    }
    let text = serde_json::to_string_pretty(&user).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text).map_err(|e| format!("Can't save the preset: {e}"))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("Can't save the preset: {e}"))?;
    Ok(copy.name)
}

/// The user presets file (Show in Explorer / Finder).
pub fn file_path() -> Option<std::path::PathBuf> {
    path()
}

/// Renames user preset `i` (its index among the loaded user presets, B5).
/// The name is made unique; returns it.
pub fn rename(i: usize, name: &str) -> Result<String, String> {
    rename_at(&path().ok_or("There is no app-data folder.")?, i, name)
}

/// Deletes user preset `i` (B5).
pub fn delete(i: usize) -> Result<(), String> {
    delete_at(&path().ok_or("There is no app-data folder.")?, i)
}

/// Reads the file, applies `f` to the `i`-th readable entry's index in the
/// raw list (damaged entries are skipped by `load`, so indices are counted
/// the same way), and writes it back atomically.
fn edit_at<R>(path: &std::path::Path, i: usize, f: impl FnOnce(&mut PresetFile, usize) -> Result<R, String>) -> Result<R, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("Can't read your presets: {e}"))?;
    let mut file: PresetFile = serde_json::from_str(&text).map_err(|e| format!("Your presets file is damaged: {e}"))?;
    let raw = file
        .presets
        .iter()
        .enumerate()
        .filter(|(_, e)| crate::file::instrument_from_json(e.instrument.clone()).is_ok())
        .map(|(k, _)| k)
        .nth(i)
        .ok_or("That preset no longer exists.")?;
    let r = f(&mut file, raw)?;
    let text = serde_json::to_string_pretty(&file).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text).map_err(|e| format!("Can't save your presets: {e}"))?;
    std::fs::rename(&tmp, path).map_err(|e| format!("Can't save your presets: {e}"))?;
    Ok(r)
}

pub fn rename_at(path: &std::path::Path, i: usize, name: &str) -> Result<String, String> {
    edit_at(path, i, |file, raw| {
        let others: Vec<UserPreset> = file
            .presets
            .iter()
            .enumerate()
            .filter(|(k, _)| *k != raw)
            .filter_map(|(_, e)| crate::file::instrument_from_json(e.instrument.clone()).ok().map(|i| UserPreset { name: i.name.clone(), kinds: e.kinds, inst: i }))
            .collect();
        let mut inst = crate::file::instrument_from_json(file.presets[raw].instrument.clone())?;
        let wanted: String = name.trim().chars().take(MAX_NAME).collect();
        inst.name = unique_name(&wanted, &others);
        file.presets[raw].instrument = crate::file::instrument_json(&inst);
        Ok(inst.name)
    })
}

pub fn delete_at(path: &std::path::Path, i: usize) -> Result<(), String> {
    edit_at(path, i, |file, raw| {
        file.presets.remove(raw);
        Ok(())
    })
}

/// Longest preset or track name (characters).
pub const MAX_NAME: usize = 48;

/// A factory or user preset, by browser index.
pub struct PresetView {
    pub name: String,
    pub blurb: String,
    pub kinds: u8,
    pub inst: LaneInstrument,
    /// Channel controls a factory preset sets with it (user presets: none).
    pub params: &'static [(ParamId, i8)],
    pub user: bool,
}

impl PresetView {
    pub fn fits(&self, lane: usize) -> bool {
        self.kinds & lane_kind(lane) != 0
    }
}

pub fn view(p: usize, user: &[UserPreset]) -> Option<PresetView> {
    if let Some(f) = PRESETS.get(p) {
        return Some(PresetView {
            name: f.name.to_string(),
            blurb: f.blurb.to_string(),
            kinds: f.kinds,
            inst: LaneInstrument::from_preset(f),
            params: f.params,
            user: false,
        });
    }
    let u = user.get(p - PRESETS.len())?;
    Some(PresetView { name: u.name.clone(), blurb: "Your preset".into(), kinds: u.kinds, inst: u.inst.clone(), params: &[], user: true })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn save_and_load_round_trip_with_unique_names() {
        let dir = std::env::temp_dir().join(format!("chipdaw-presets-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let path = dir.join(FILE);
        let mut cello = LaneInstrument::named("Cello");
        cello.inst.vibrato.depth = 41;
        cello.edited = true;
        assert_eq!(save_to(&path, &cello, 5).unwrap(), "Cello 2", "factory names stay unique");
        assert_eq!(save_to(&path, &cello, 5).unwrap(), "Cello 3");
        let kit = LaneInstrument::named("Tight Kit");
        save_to(&path, &kit, 3).unwrap();
        let text = std::fs::read_to_string(&path).unwrap();
        let f: PresetFile = serde_json::from_str(&text).unwrap();
        let back: Vec<LaneInstrument> = f.presets.into_iter().map(|e| crate::file::instrument_from_json(e.instrument).unwrap()).collect();
        assert_eq!(back[0].inst, cello.inst);
        assert_eq!(back[2].kit, kit.kit, "kits keep their drums");
        // A damaged file is ignored, not fatal.
        std::fs::write(&path, "{ not json").unwrap();
        assert!(save_to(&path, &cello, 0).is_ok());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
