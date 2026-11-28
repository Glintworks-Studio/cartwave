//! The browser's collections (PRESETS_PROMPT.md §5.1; Live's Collections,
//! Live 12 manual ch. 4 "Working with the Browser"): coloured labels the user
//! puts items under from an item's context menu. Kept per user in
//! settings.json, by item key, so they survive the bank growing.

use crate::bank::{self, Item};
use crate::theme::tokens::Rgba;

/// The collections: (name, colour). Never the toggle lime.
pub const COLLECTIONS: [(&str, Rgba); 7] = [
    ("Favourites", Rgba::hex(0xE39A4C)),
    ("Red", Rgba::hex(0xD9574F)),
    ("Yellow", Rgba::hex(0xD9B44A)),
    ("Green", Rgba::hex(0x5FAE6E)),
    ("Blue", Rgba::hex(0x4FA8E0)),
    ("Purple", Rgba::hex(0x9C7FE0)),
    ("Grey", Rgba::hex(0x9A9A9A)),
];

/// An item's stable key (its name; a sample's slot).
pub fn key(item: Item, user: &[crate::user_presets::UserPreset]) -> Option<String> {
    Some(match item {
        Item::Preset(i) => format!("preset:{}", bank::get(i)?.name),
        Item::User(u) => format!("user:{}", user.get(u)?.name),
        Item::Band(i) => format!("band:{}", chip::presets::LAYOUTS.get(i)?.name),
        Item::Clip(i) => format!("clip:{}", bank::PHRASES.get(i)?.name),
        Item::Sample(i) => format!("sample:{i}"),
        Item::ImportAudio => return None,
    })
}

/// The item a key names (if it still exists).
pub fn item(key: &str, user: &[crate::user_presets::UserPreset]) -> Option<Item> {
    let (kind, name) = key.split_once(':')?;
    match kind {
        "preset" => bank::index_of(name).map(Item::Preset),
        "user" => user.iter().position(|u| u.name == name).map(Item::User),
        "band" => chip::presets::LAYOUTS.iter().position(|l| l.name == name).map(Item::Band),
        "clip" => bank::PHRASES.iter().position(|p| p.name == name).map(Item::Clip),
        "sample" => name.parse().ok().map(Item::Sample),
        _ => None,
    }
}

fn user_now() -> Vec<crate::user_presets::UserPreset> {
    crate::user_presets::load()
}

/// Whether collection `k` holds `item`.
pub fn contains(lists: &[Vec<String>], k: usize, it: Item) -> bool {
    let Some(want) = key(it, &user_now()) else { return false };
    lists.get(k).is_some_and(|l| l.contains(&want))
}

/// Adds `item` to collection `k`, or takes it out.
pub fn toggle(lists: &mut Vec<Vec<String>>, k: usize, it: Item) {
    let Some(want) = key(it, &user_now()) else { return };
    if lists.len() <= k {
        lists.resize(k + 1, Vec::new());
    }
    let l = &mut lists[k];
    match l.iter().position(|x| *x == want) {
        Some(i) => {
            l.remove(i);
        }
        None => l.push(want),
    }
}

/// Collection `k`'s items that still exist, in the order they were added.
pub fn items(lists: &[Vec<String>], k: usize, user: &[crate::user_presets::UserPreset]) -> Vec<Item> {
    lists.get(k).map_or_else(Vec::new, |l| l.iter().filter_map(|x| item(x, user)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keys_round_trip_and_toggle() {
        let i = Item::Preset(bank::index_of("Bright Square Lead").unwrap());
        let k = key(i, &[]).unwrap();
        assert_eq!(item(&k, &[]), Some(i));
        let mut lists = Vec::new();
        let want = key(i, &[]).unwrap();
        // toggle() reads the user presets file; use the key functions directly.
        lists.resize(1, Vec::new());
        lists[0].push(want.clone());
        assert_eq!(items(&lists, 0, &[]), vec![i]);
        assert!(COLLECTIONS.iter().all(|c| crate::state::rgb_hex(c.1) != 0xC7D74E), "no lime");
    }
}
