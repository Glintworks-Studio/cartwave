//! Environment variables. Every name is `CARTWAVE_*`; the pre-rename
//! `CHIPDAW_*` spelling of the same name stays accepted as an alias (RENAME.md),
//! so old scripts and shells keep working. The new name wins when both are set.
//! Using an alias prints a one-line note in dev mode (a debug build, or
//! `CARTWAVE_DEV=1`).

use std::ffi::OsString;
use std::sync::{Mutex, OnceLock};

/// The prefix of every current name.
pub const PREFIX: &str = "CARTWAVE_";
/// The pre-rename prefix, accepted as an alias.
pub const LEGACY_PREFIX: &str = "CHIPDAW_";

/// The alias of a `CARTWAVE_*` name (`CARTWAVE_DEV` → `CHIPDAW_DEV`).
pub fn legacy_name(name: &str) -> Option<String> {
    name.strip_prefix(PREFIX).map(|rest| format!("{LEGACY_PREFIX}{rest}"))
}

/// Dev mode: a debug build, or `CARTWAVE_DEV=1` (alias `CHIPDAW_DEV=1`).
/// Decided once per process.
pub fn dev_mode() -> bool {
    static NOTED: std::sync::Once = std::sync::Once::new();
    let on = dev_flag();
    // Note `CHIPDAW_DEV` itself, once dev mode is decided.
    NOTED.call_once(|| {
        let _ = var_os("CARTWAVE_DEV");
    });
    on
}

/// The dev-mode flag, read without any alias note (the note asks for it).
fn dev_flag() -> bool {
    static ON: OnceLock<bool> = OnceLock::new();
    *ON.get_or_init(|| cfg!(debug_assertions) || lookup("CARTWAVE_DEV").is_some_and(|(v, _)| v == "1"))
}

/// `name`'s value, else its alias's, and the alias's name when that was used.
fn lookup(name: &str) -> Option<(OsString, Option<String>)> {
    if let Some(v) = std::env::var_os(name) {
        return Some((v, None));
    }
    let old = legacy_name(name)?;
    std::env::var_os(&old).map(|v| (v, Some(old)))
}

/// `name`'s value, else its `CHIPDAW_*` alias's (with a dev-mode note).
pub fn var_os(name: &str) -> Option<OsString> {
    let (v, alias) = lookup(name)?;
    if let Some(old) = alias {
        note_alias(name, &old);
    }
    Some(v)
}

/// Like [`var_os`], as a `String` (`None` if it isn't Unicode).
pub fn var(name: &str) -> Option<String> {
    var_os(name).and_then(|v| v.into_string().ok())
}

/// Prints "old name used" once per name, in dev mode only.
fn note_alias(new: &str, old: &str) {
    static SEEN: Mutex<Vec<String>> = Mutex::new(Vec::new());
    // The flag is read after the lock is released, without `dev_mode`'s own note.
    let first = match SEEN.lock() {
        Ok(mut seen) if !seen.iter().any(|s| s == old) => {
            seen.push(old.to_string());
            true
        }
        _ => false,
    };
    if first && dev_flag() {
        eprintln!("dev: {old} is the old name of {new}; it still works, but use {new}");
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_names() {
        assert_eq!(legacy_name("CARTWAVE_DEV").as_deref(), Some("CHIPDAW_DEV"));
        assert_eq!(legacy_name("CARTWAVE_PILOT_HZ").as_deref(), Some("CHIPDAW_PILOT_HZ"));
        assert_eq!(legacy_name("PATH"), None);
    }

    // One test owns these two names: tests run in parallel threads.
    #[test]
    fn alias_and_precedence() {
        let (new, old) = ("CARTWAVE_ENV_TEST_ALIAS", "CHIPDAW_ENV_TEST_ALIAS");
        std::env::remove_var(new);
        std::env::remove_var(old);
        assert_eq!(var(new), None);
        std::env::set_var(old, "old");
        assert_eq!(var(new).as_deref(), Some("old"), "the old name is an alias");
        std::env::set_var(new, "new");
        assert_eq!(var(new).as_deref(), Some("new"), "the new name wins");
        std::env::remove_var(old);
        assert_eq!(var(new).as_deref(), Some("new"));
        std::env::remove_var(new);
        assert_eq!(var(new), None);
    }
}
