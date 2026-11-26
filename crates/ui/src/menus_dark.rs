//! Dark dropdown menus on Windows (Settings > Display > Dark menus;
//! docs/design/DARK-MENUS.md, BATCH3.md §5 option (c)).
//!
//! Documented Win32 only: a window subclass (`SetWindowSubclass`) on the
//! main window, installed after muda's, sees `WM_INITMENUPOPUP` for every
//! dropdown just before it shows and flags each item `MFT_OWNERDRAW` (or
//! clears the flag while the setting is off), with `SetMenuInfo
//! (MIM_BACKGROUND)` for the popup's own background. The rows are then
//! ours: `WM_MEASUREITEM`, `WM_DRAWITEM`, and `WM_MENUCHAR` for the letter
//! keys the system no longer matches on owner-drawn items.
//!
//! Nothing is cached about an item but where it is: `dwItemData` holds a
//! key into a (menu, position) table, and every measure and draw re-reads
//! the text, check and enabled state with `GetMenuItemInfoW`, so muda's
//! own updates (`set_enabled`, `set_checked`, `set_text`, the rebuilt Open
//! Recent submenu) always show. Items muda adds later are flagged the next
//! time their popup opens.
//!
//! The menu bar itself is muda's (`MenuTheme::Dark`), unchanged.

use super::dark_layout as lay;
use std::cell::RefCell;
use std::sync::atomic::{AtomicBool, Ordering};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, SIZE, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    CreateFontIndirectW, CreatePen, CreateSolidBrush, DeleteObject, DrawTextW, ExcludeClipRect, FillRect, GetDC, GetStockObject, GetTextExtentPoint32W, GetTextMetricsW, Polyline, ReleaseDC,
    RoundRect, SelectObject, SetBkMode, SetTextColor, DT_CALCRECT, DT_HIDEPREFIX, DT_LEFT, DT_NOPREFIX, DT_RIGHT, DT_SINGLELINE, DT_VCENTER, HBRUSH, HDC, HFONT, HGDIOBJ, NULL_PEN, PS_SOLID,
    TEXTMETRICW, TRANSPARENT,
};
use windows_sys::Win32::UI::Controls::{DRAWITEMSTRUCT, MEASUREITEMSTRUCT, ODS_CHECKED, ODS_DISABLED, ODS_GRAYED, ODS_NOACCEL, ODS_SELECTED, ODT_MENU};
use windows_sys::Win32::UI::HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi, SystemParametersInfoForDpi};
use windows_sys::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    GetMenuItemCount, GetMenuItemInfoW, InsertMenuItemW, RemoveMenu, SetMenuInfo, SetMenuItemInfoW, SystemParametersInfoW, HMENU, MENUINFO, MENUITEMINFOW, MFS_CHECKED, MFS_DISABLED, MFS_HILITE, MFT_OWNERDRAW,
    MFT_SEPARATOR, MF_BYPOSITION, MIIM_BITMAP, MIIM_CHECKMARKS, MIIM_DATA, MIIM_FTYPE, MIIM_ID, MIIM_STATE, MIIM_STRING, MIIM_SUBMENU, MIM_BACKGROUND, MNC_EXECUTE, MNC_SELECT, NONCLIENTMETRICSW, SM_CXMENUCHECK, SPI_GETNONCLIENTMETRICS,
    WM_DRAWITEM, WM_INITMENUPOPUP, WM_MEASUREITEM, WM_MENUCHAR,
};

/// Our subclass id (any value distinct from muda's).
const SUBCLASS_ID: usize = 0xC41D_DA4C;
/// `dwItemData` values we set carry this tag in the high bits, so an item
/// someone else owner-draws is never mistaken for ours.
const TAG: usize = 0xDA4C_0000;
const TAG_MASK: usize = 0xFFFF_0000;

static ENABLED: AtomicBool = AtomicBool::new(false);

pub fn set_enabled(on: bool) {
    ENABLED.store(on, Ordering::Relaxed);
}

/// GDI objects, made once per thread (the UI thread) and kept for the
/// app's life; the font is remade when the DPI changes.
struct Gdi {
    bg: HBRUSH,
    wash: HBRUSH,
    wash_disabled: HBRUSH,
    separator: HBRUSH,
    font: Option<(u32, HFONT)>,
}

thread_local! {
    /// Where each owner-drawn item is: index + 1 (low 16 bits of the item
    /// data) -> (menu, position).
    static ITEMS: RefCell<Vec<(usize, u32)>> = const { RefCell::new(Vec::new()) };
    static GDI: RefCell<Option<Gdi>> = const { RefCell::new(None) };
}

fn with_gdi<R>(f: impl FnOnce(&mut Gdi) -> R) -> R {
    GDI.with(|g| {
        let mut g = g.borrow_mut();
        // SAFETY: plain GDI object creation.
        let gdi = g.get_or_insert_with(|| unsafe {
            Gdi {
                bg: CreateSolidBrush(lay::BG),
                wash: CreateSolidBrush(lay::WASH),
                wash_disabled: CreateSolidBrush(lay::WASH_DISABLED),
                separator: CreateSolidBrush(lay::SEPARATOR),
                font: None,
            }
        });
        f(gdi)
    })
}

/// The system menu font (NONCLIENTMETRICS.lfMenuFont) at `dpi`.
fn menu_font(gdi: &mut Gdi, dpi: u32) -> HFONT {
    if let Some((d, f)) = gdi.font {
        if d == dpi {
            return f;
        }
        // SAFETY: our own font, no longer selected anywhere (each draw
        // restores the previous font).
        unsafe { DeleteObject(f as HGDIOBJ) };
    }
    // SAFETY: a correctly sized NONCLIENTMETRICSW; the ...ForDpi variant
    // (Windows 10 1607+) gives the font for this monitor; the plain call
    // (96-DPI metrics scaled by the system) is the fallback.
    let font = unsafe {
        let mut ncm: NONCLIENTMETRICSW = std::mem::zeroed();
        ncm.cbSize = std::mem::size_of::<NONCLIENTMETRICSW>() as u32;
        let ok = SystemParametersInfoForDpi(SPI_GETNONCLIENTMETRICS, ncm.cbSize, &mut ncm as *mut _ as *mut _, 0, dpi) != 0
            || SystemParametersInfoW(SPI_GETNONCLIENTMETRICS, ncm.cbSize, &mut ncm as *mut _ as *mut _, 0) != 0;
        if ok {
            CreateFontIndirectW(&ncm.lfMenuFont)
        } else {
            std::ptr::null_mut()
        }
    };
    gdi.font = Some((dpi, font));
    font
}

/// One item as it is now.
struct Item {
    separator: bool,
    checked: bool,
    disabled: bool,
    hilite: bool,
    submenu: bool,
    data: usize,
    text: Vec<u16>,
}

/// Reads an item by position (text without the trailing NUL).
unsafe fn read_item(menu: HMENU, pos: u32) -> Option<Item> {
    let mut mii: MENUITEMINFOW = std::mem::zeroed();
    mii.cbSize = std::mem::size_of::<MENUITEMINFOW>() as u32;
    mii.fMask = MIIM_FTYPE | MIIM_STATE | MIIM_SUBMENU | MIIM_DATA | MIIM_STRING;
    if GetMenuItemInfoW(menu, pos, 1, &mut mii) == 0 {
        return None;
    }
    let mut item = Item {
        separator: mii.fType & MFT_SEPARATOR != 0,
        checked: mii.fState & MFS_CHECKED != 0,
        disabled: mii.fState & MFS_DISABLED != 0,
        hilite: mii.fState & MFS_HILITE != 0,
        submenu: !mii.hSubMenu.is_null(),
        data: mii.dwItemData,
        text: Vec::new(),
    };
    // The text in a second call of its own (a string read rewrites other
    // fields of the struct, e.g. hSubMenu comes back null).
    if !item.separator && mii.cch > 0 {
        let mut text = vec![0u16; mii.cch as usize + 1];
        let mut s: MENUITEMINFOW = std::mem::zeroed();
        s.cbSize = std::mem::size_of::<MENUITEMINFOW>() as u32;
        s.fMask = MIIM_STRING;
        s.dwTypeData = text.as_mut_ptr();
        s.cch = mii.cch + 1;
        if GetMenuItemInfoW(menu, pos, 1, &mut s) == 0 {
            return None;
        }
        text.truncate(s.cch as usize);
        item.text = text;
    }
    Some(item)
}

/// The table key for (menu, position), adding it if new.
fn key_for(menu: HMENU, pos: u32) -> usize {
    ITEMS.with(|t| {
        let mut t = t.borrow_mut();
        let k = (menu as usize, pos);
        let i = t.iter().position(|e| *e == k).unwrap_or_else(|| {
            t.push(k);
            t.len() - 1
        });
        TAG | ((i + 1) & !TAG_MASK)
    })
}

/// (menu, position) for an item-data value, if it is ours.
fn lookup(data: usize) -> Option<(HMENU, u32)> {
    if data & TAG_MASK != TAG {
        return None;
    }
    let i = (data & !TAG_MASK).checked_sub(1)?;
    ITEMS.with(|t| t.borrow().get(i).map(|&(m, p)| (m as HMENU, p)))
}

/// Turns an owner-drawn item back into a system-drawn one. Clearing
/// MFT_OWNERDRAW in place (even with the string set again) leaves the row
/// blank and at the owner-drawn size (seen on Windows 11 build 26200), so the
/// item is removed and inserted again at the same position, with the same
/// id, submenu, state and text: muda finds its items by id, so it doesn't
/// notice.
unsafe fn unflag(menu: HMENU, pos: u32) {
    let mut mii: MENUITEMINFOW = std::mem::zeroed();
    mii.cbSize = std::mem::size_of::<MENUITEMINFOW>() as u32;
    mii.fMask = MIIM_FTYPE | MIIM_ID | MIIM_STATE | MIIM_SUBMENU | MIIM_CHECKMARKS | MIIM_BITMAP;
    if GetMenuItemInfoW(menu, pos, 1, &mut mii) == 0 {
        return;
    }
    let mut text = read_item(menu, pos).map(|i| i.text).unwrap_or_default();
    text.push(0);
    mii.fType &= !MFT_OWNERDRAW;
    mii.fMask |= MIIM_DATA;
    mii.dwItemData = 0;
    if mii.fType & MFT_SEPARATOR == 0 {
        mii.fMask |= MIIM_STRING;
        mii.dwTypeData = text.as_mut_ptr();
    }
    // RemoveMenu (not DeleteMenu) keeps a submenu alive for the re-insert.
    if RemoveMenu(menu, pos, MF_BYPOSITION) != 0 {
        InsertMenuItemW(menu, pos, 1, &mii);
    }
}

/// Flags (dark on) or unflags (off) every item of a popup that is about to
/// open, and sets or clears its background brush.
unsafe fn prepare(menu: HMENU, dark: bool) {
    let n = GetMenuItemCount(menu);
    for pos in 0..n.max(0) as u32 {
        let mut mii: MENUITEMINFOW = std::mem::zeroed();
        mii.cbSize = std::mem::size_of::<MENUITEMINFOW>() as u32;
        mii.fMask = MIIM_FTYPE | MIIM_DATA;
        if GetMenuItemInfoW(menu, pos, 1, &mut mii) == 0 {
            continue;
        }
        let ours = lookup(mii.dwItemData).is_some();
        if dark {
            // Always rewritten: setting the type also drops the item's
            // cached size, so a DPI change re-measures.
            mii.fType |= MFT_OWNERDRAW;
            mii.dwItemData = key_for(menu, pos);
            mii.fMask = MIIM_FTYPE | MIIM_DATA;
            SetMenuItemInfoW(menu, pos, 1, &mii);
        } else if ours && mii.fType & MFT_OWNERDRAW != 0 {
            unflag(menu, pos);
        }
    }
    let mut mi: MENUINFO = std::mem::zeroed();
    mi.cbSize = std::mem::size_of::<MENUINFO>() as u32;
    mi.fMask = MIM_BACKGROUND;
    mi.hbrBack = if dark { with_gdi(|g| g.bg) } else { std::ptr::null_mut() };
    SetMenuInfo(menu, &mi);
}

fn dpi_of(hwnd: HWND) -> u32 {
    // SAFETY: any HWND; 0 for an invalid one.
    match unsafe { GetDpiForWindow(hwnd) } {
        0 => 96,
        d => d,
    }
}

/// Text extent with the font selected in `hdc` ('&' prefixes as drawn).
unsafe fn text_width(hdc: HDC, s: &[u16], prefix: bool) -> i32 {
    if s.is_empty() {
        return 0;
    }
    if !prefix {
        let mut sz = SIZE { cx: 0, cy: 0 };
        GetTextExtentPoint32W(hdc, s.as_ptr(), s.len() as i32, &mut sz);
        return sz.cx;
    }
    let mut r = RECT { left: 0, top: 0, right: 0, bottom: 0 };
    DrawTextW(hdc, s.as_ptr(), s.len() as i32, &mut r, DT_CALCRECT | DT_SINGLELINE | DT_LEFT);
    r.right - r.left
}

fn split(text: &[u16]) -> (&[u16], &[u16]) {
    match text.iter().position(|&c| c == '\t' as u16) {
        Some(i) => (&text[..i], &text[i + 1..]),
        None => (text, &[]),
    }
}

unsafe fn measure(hwnd: HWND, mis: &mut MEASUREITEMSTRUCT) -> bool {
    let Some((menu, pos)) = lookup(mis.itemData) else { return false };
    let Some(item) = read_item(menu, pos) else { return false };
    if item.data != mis.itemData {
        return false;
    }
    let dpi = dpi_of(hwnd);
    let allowance = GetSystemMetricsForDpi(SM_CXMENUCHECK, dpi) - 1;
    let (w, h) = if item.separator {
        lay::item_size(true, 0, 0, 0, allowance, dpi)
    } else {
        let hdc = GetDC(hwnd);
        let font = with_gdi(|g| menu_font(g, dpi));
        let old = SelectObject(hdc, font as HGDIOBJ);
        let mut tm: TEXTMETRICW = std::mem::zeroed();
        GetTextMetricsW(hdc, &mut tm);
        let (label, accel) = split(&item.text);
        let size = lay::item_size(false, text_width(hdc, label, true), text_width(hdc, accel, false), tm.tmHeight, allowance, dpi);
        SelectObject(hdc, old);
        ReleaseDC(hwnd, hdc);
        size
    };
    mis.itemWidth = w;
    mis.itemHeight = h;
    true
}

unsafe fn stroke(hdc: HDC, pts: [(i32, i32); 3], color: u32, dpi: u32) {
    let pen = CreatePen(PS_SOLID, lay::stroke(dpi), color);
    let old = SelectObject(hdc, pen as HGDIOBJ);
    let p = pts.map(|(x, y)| POINT { x, y });
    Polyline(hdc, p.as_ptr(), p.len() as i32);
    SelectObject(hdc, old);
    DeleteObject(pen as HGDIOBJ);
}

unsafe fn draw(hwnd: HWND, dis: &DRAWITEMSTRUCT) -> bool {
    let Some((menu, pos)) = lookup(dis.itemData) else { return false };
    let Some(item) = read_item(menu, pos) else { return false };
    if item.data != dis.itemData {
        return false;
    }
    let dpi = dpi_of(hwnd);
    let hdc = dis.hDC;
    let rc = dis.rcItem;
    let (bg, wash, wash_off, sep, font) = with_gdi(|g| (g.bg, g.wash, g.wash_disabled, g.separator, menu_font(g, dpi)));
    FillRect(hdc, &rc, bg);
    if item.separator {
        let inset = lay::px(lay::WASH_INSET, dpi);
        let y = rc.top + (rc.bottom - rc.top) / 2;
        let line = RECT { left: rc.left + inset, top: y, right: rc.right - inset, bottom: y + lay::px(1, dpi).max(1) };
        FillRect(hdc, &line, sep);
        return true;
    }
    let disabled = item.disabled || dis.itemState & (ODS_GRAYED | ODS_DISABLED) != 0;
    let checked = item.checked || dis.itemState & ODS_CHECKED != 0;
    if dis.itemState & ODS_SELECTED != 0 {
        let inset = lay::px(lay::WASH_INSET, dpi);
        let r = lay::px(lay::WASH_RADIUS, dpi) * 2;
        let old_b = SelectObject(hdc, if disabled { wash_off } else { wash } as HGDIOBJ);
        let old_p = SelectObject(hdc, GetStockObject(NULL_PEN));
        // RoundRect with a null pen fills one pixel short: +1 on the far sides.
        RoundRect(hdc, rc.left + inset, rc.top, rc.right - inset + 1, rc.bottom + 1, r, r);
        SelectObject(hdc, old_p);
        SelectObject(hdc, old_b);
    }
    let color = if disabled { lay::TEXT_DISABLED } else { lay::TEXT };
    let old_f = SelectObject(hdc, font as HGDIOBJ);
    SetBkMode(hdc, TRANSPARENT as i32);
    let old_c = SetTextColor(hdc, color);
    let (label, accel) = split(&item.text);
    let mut text_rc = RECT { left: rc.left + lay::px(lay::GUTTER, dpi), top: rc.top, right: rc.right - lay::px(lay::ARROW_COL, dpi), bottom: rc.bottom };
    let hide = if dis.itemState & ODS_NOACCEL != 0 { DT_HIDEPREFIX } else { 0 };
    DrawTextW(hdc, label.as_ptr(), label.len() as i32, &mut text_rc, DT_SINGLELINE | DT_VCENTER | DT_LEFT | hide);
    if !accel.is_empty() {
        DrawTextW(hdc, accel.as_ptr(), accel.len() as i32, &mut text_rc, DT_SINGLELINE | DT_VCENTER | DT_RIGHT | DT_NOPREFIX);
    }
    SetTextColor(hdc, old_c);
    SelectObject(hdc, old_f);
    let h = rc.bottom - rc.top;
    if checked {
        stroke(hdc, lay::check_points(rc.left, rc.top, h, dpi), color, dpi);
    }
    if item.submenu {
        stroke(hdc, lay::arrow_points(rc.right, rc.top, h, dpi), color, dpi);
        // The system draws its own (dark) arrow after WM_DRAWITEM in the
        // same DC, just right of rcItem; clipping our column and the
        // space past it out keeps ours.
        let col = lay::px(lay::ARROW_COL, dpi);
        ExcludeClipRect(hdc, rc.right - col, rc.top, rc.right + 2 * col, rc.bottom);
    }
    true
}

/// WM_MENUCHAR for a popup whose items are ours: the system no longer
/// matches letters on owner-drawn items, so we do (UX: same as light).
unsafe fn menu_char(menu: HMENU, ch: u16) -> Option<LRESULT> {
    let n = GetMenuItemCount(menu).max(0) as u32;
    let mut keys = Vec::with_capacity(n as usize);
    let mut hilite = None;
    let mut any_ours = false;
    for pos in 0..n {
        let Some(item) = read_item(menu, pos) else {
            keys.push(None);
            continue;
        };
        any_ours |= lookup(item.data).is_some();
        if item.hilite {
            hilite = Some(pos as usize);
        }
        if item.separator {
            keys.push(None);
            continue;
        }
        let (label, _) = split(&item.text);
        keys.push(lay::mnemonic(&String::from_utf16_lossy(label)));
    }
    if !any_ours {
        return None;
    }
    let key = char::from_u32(ch as u32)?;
    lay::menu_char(&keys, key, hilite).map(|(row, unique)| {
        let code = if unique { MNC_EXECUTE } else { MNC_SELECT };
        ((code << 16) | row as u32) as LRESULT
    })
}

unsafe extern "system" fn subclass_proc(hwnd: HWND, msg: u32, wparam: WPARAM, lparam: LPARAM, _id: usize, _data: usize) -> LRESULT {
    match msg {
        // lParam's high word is set for the window (system) menu: left alone.
        WM_INITMENUPOPUP if (lparam >> 16) & 0xFFFF == 0 => {
            prepare(wparam as HMENU, ENABLED.load(Ordering::Relaxed));
        }
        WM_MEASUREITEM => {
            let mis = &mut *(lparam as *mut MEASUREITEMSTRUCT);
            if mis.CtlType == ODT_MENU && measure(hwnd, mis) {
                return 1;
            }
        }
        WM_DRAWITEM => {
            let dis = &*(lparam as *const DRAWITEMSTRUCT);
            if dis.CtlType == ODT_MENU && draw(hwnd, dis) {
                return 1;
            }
        }
        WM_MENUCHAR => {
            if let Some(r) = menu_char(lparam as HMENU, (wparam & 0xFFFF) as u16) {
                return r;
            }
        }
        _ => {}
    }
    DefSubclassProc(hwnd, msg, wparam, lparam)
}

/// Hooks the main window. Safe to call while the setting is off: popups are
/// then left (or put back) as the system draws them.
///
/// # Safety
/// `hwnd` must be a live window owned by this (the UI) thread.
pub unsafe fn install(hwnd: isize) -> bool {
    SetWindowSubclass(hwnd as HWND, Some(subclass_proc), SUBCLASS_ID, 0) != 0
}
