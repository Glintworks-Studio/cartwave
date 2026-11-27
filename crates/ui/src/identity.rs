//! The product's names and identifiers, in one place (RENAME.md).

/// The app's name, as the window title, menus and About show it.
pub const APP_NAME: &str = "Cartwave";
/// The company: author and organisation.
pub const COMPANY: &str = "Glintworks";
/// The company's website (every product page lives under it).
pub const WEBSITE: &str = "glintworks.studio";
/// The Windows app id (AppUserModelID: taskbar grouping, pinning) and, for a
/// later macOS bundle, its `CFBundleIdentifier`.
pub const APP_ID: &str = "studio.glintworks.cartwave";

/// Gives the process its Windows app id. Call before the first window opens.
/// Elsewhere (and in a build without the `menus` feature's Windows calls)
/// it does nothing.
pub fn set_app_id() {
    #[cfg(all(windows, feature = "menus"))]
    {
        let wide: Vec<u16> = APP_ID.encode_utf16().chain(std::iter::once(0)).collect();
        // SAFETY: a NUL-terminated UTF-16 string that outlives the call.
        unsafe {
            windows_sys::Win32::UI::Shell::SetCurrentProcessExplicitAppUserModelID(wide.as_ptr());
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn identifiers() {
        assert_eq!(super::APP_ID, "studio.glintworks.cartwave");
        assert_eq!(super::APP_NAME, "Cartwave");
        assert_eq!(super::COMPANY, "Glintworks");
    }
}
