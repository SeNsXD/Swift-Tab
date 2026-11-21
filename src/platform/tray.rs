//! Tray icon (a small white pill, drawn at run time) and its menu.

use windows::core::{Result, HSTRING};
use windows::Win32::Foundation::{HWND, LPARAM, POINT, WPARAM};
use windows::Win32::Graphics::Gdi::{CreateBitmap, CreateDIBSection, DeleteObject, BITMAPINFO, BITMAPINFOHEADER, DIB_RGB_COLORS};
use windows::Win32::UI::Shell::{Shell_NotifyIconW, NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW};
use windows::Win32::UI::WindowsAndMessaging::*;

use super::windows::WM_TRAY;

pub const MENU_TIMER: u32 = 1;
pub const MENU_STOPWATCH: u32 = 2;
pub const MENU_AUTOSTART: u32 = 3;
pub const MENU_SETTINGS: u32 = 5;
pub const MENU_MATCH_OPEN: u32 = 6;
pub const MENU_MATCH_HIDE: u32 = 7;
pub const MENU_QUIT: u32 = 4;
pub const MENU_UPDATE: u32 = 8;
pub const MENU_BRIEF: u32 = 9;
pub const MENU_WELLBEING: u32 = 10;
pub const MENU_MUSIC: u32 = 11;
pub const MENU_QUESTION: u32 = 12;
pub const MENU_CLIPBOARD: u32 = 13;
pub const MENU_NOTES: u32 = 14;

const TRAY_ID: u32 = 1;

/// Straight-alpha BGRA pixels of a white capsule centred in a `size`×`size` square.
pub fn pill_icon_bgra(size: usize) -> Vec<u8> {
    let s = size as f32;
    let (w, h) = (s * 0.84, s * 0.44);
    let (x0, y0) = ((s - w) / 2.0, (s - h) / 2.0);
    let r = h / 2.0;
    let mut px = vec![0u8; size * size * 4];
    for y in 0..size {
        for x in 0..size {
            let (cx, cy) = (x as f32 + 0.5, y as f32 + 0.5);
            // Signed distance to the capsule: distance to the segment between the end centres, minus r.
            let sx = cx.clamp(x0 + r, x0 + w - r);
            let d = ((cx - sx).powi(2) + (cy - (y0 + r)).powi(2)).sqrt() - r;
            let coverage = (0.5 - d).clamp(0.0, 1.0);
            if coverage > 0.0 {
                let i = (y * size + x) * 4;
                px[i..i + 3].copy_from_slice(&[255, 255, 255]);
                px[i + 3] = (coverage * 255.0).round() as u8;
            }
        }
    }
    px
}

const TRAY_NOBG_64: &[u8] = include_bytes!("../../assets/tray-nobg-64.bgra");

/// Resamples the embedded 64×64 no-background Swift Tab ribbon logo to `size`×`size` BGRA.
pub fn tray_nobg_bgra(size: usize) -> Vec<u8> {
    let mut out = vec![0u8; size * size * 4];
    if TRAY_NOBG_64.len() < 64 * 64 * 4 || size == 0 {
        return pill_icon_bgra(size);
    }
    for y in 0..size {
        let sy = ((y * 64) + 32) / size;
        let sy = sy.min(63);
        for x in 0..size {
            let sx = ((x * 64) + 32) / size;
            let sx = sx.min(63);
            let src = (sy * 64 + sx) * 4;
            let dst = (y * size + x) * 4;
            out[dst..dst + 4].copy_from_slice(&TRAY_NOBG_64[src..src + 4]);
        }
    }
    out
}

fn pill_icon() -> Result<HICON> {
    let size = unsafe { GetSystemMetrics(SM_CXSMICON) }.max(16);
    // Resource ID 2 is assets/nowbar-nobg.ico (the no-background Swift Tab ribbon logo).
    if let Ok(handle) = unsafe {
        windows::Win32::System::LibraryLoader::GetModuleHandleW(None).and_then(|hinst| {
            LoadImageW(Some(hinst.into()), windows::core::PCWSTR(2 as _), IMAGE_ICON, size, size, LR_DEFAULTCOLOR | LR_SHARED)
        })
    } {
        if !handle.is_invalid() {
            return Ok(HICON(handle.0));
        }
    }
    let pixels = tray_nobg_bgra(size as usize);
    unsafe {
        let info = BITMAPINFO {
            bmiHeader: BITMAPINFOHEADER {
                biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                biWidth: size,
                biHeight: -size,
                biPlanes: 1,
                biBitCount: 32,
                ..Default::default()
            },
            ..Default::default()
        };
        let mut bits = std::ptr::null_mut();
        let color = CreateDIBSection(None, &info, DIB_RGB_COLORS, &mut bits, None, 0)?;
        std::ptr::copy_nonoverlapping(pixels.as_ptr(), bits as *mut u8, pixels.len());
        let mask = CreateBitmap(size, size, 1, 1, None);
        let icon = CreateIconIndirect(&ICONINFO { fIcon: true.into(), hbmMask: mask, hbmColor: color, ..Default::default() });
        let _ = DeleteObject(color.into());
        let _ = DeleteObject(mask.into());
        icon
    }
}

fn notify_data(owner: HWND) -> NOTIFYICONDATAW {
    NOTIFYICONDATAW { cbSize: std::mem::size_of::<NOTIFYICONDATAW>() as u32, hWnd: owner, uID: TRAY_ID, ..Default::default() }
}

pub fn create_tray(owner: HWND) -> Result<()> {
    let mut nid = notify_data(owner);
    nid.uFlags = NIF_ICON | NIF_MESSAGE | NIF_TIP;
    nid.uCallbackMessage = WM_TRAY;
    nid.hIcon = pill_icon()?;
    for (dst, src) in nid.szTip.iter_mut().zip("Swift Tab".encode_utf16()) {
        *dst = src;
    }
    unsafe { Shell_NotifyIconW(NIM_ADD, &nid).ok() }
}

pub fn remove_tray(owner: HWND) {
    unsafe {
        let _ = Shell_NotifyIconW(NIM_DELETE, &notify_data(owner));
    }
}

/// Shows the menu at the cursor and returns the chosen `MENU_*` id (0 if dismissed). A newer
/// release (`update`, its version) comes first.
pub fn show_menu(owner: HWND, autostart: bool, update: Option<&str>) -> u32 {
    enable_dark_menu(owner);
    unsafe {
        let Ok(menu) = CreatePopupMenu() else { return 0 };
        apply_dark_menu_style(menu);
        if let Some(version) = update {
            let _ = AppendMenuW(menu, MF_STRING, MENU_UPDATE as usize, &HSTRING::from(format!("Update: {version}")));
            let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
        }
        let _ = AppendMenuW(menu, MF_STRING, MENU_CLIPBOARD as usize, &HSTRING::from("Clipboard history"));
        let _ = AppendMenuW(menu, MF_STRING, MENU_NOTES as usize, &HSTRING::from("Sticky Notes"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
        let check = if autostart { MF_CHECKED } else { MF_UNCHECKED };
        let _ = AppendMenuW(menu, MF_STRING | check, MENU_AUTOSTART as usize, &HSTRING::from("Start with Windows"));
        let _ = AppendMenuW(menu, MF_STRING, MENU_SETTINGS as usize, &HSTRING::from("Settings…"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
        let _ = AppendMenuW(menu, MF_STRING, MENU_QUIT as usize, &HSTRING::from("Exit"));
        let mut p = POINT::default();
        let _ = GetCursorPos(&mut p);
        // Required so the menu closes when the user clicks elsewhere.
        let _ = SetForegroundWindow(owner);
        let flags = TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_BOTTOMALIGN | TPM_CENTERALIGN;
        let chosen = TrackPopupMenu(menu, flags, p.x, p.y, Some(0), owner, None).0 as u32;
        let _ = PostMessageW(Some(owner), WM_NULL, WPARAM(0), LPARAM(0));
        destroy_dark_menu(menu);
        chosen
    }
}

/// Right-click menu on the Clipboard or Shelf pill/card ("Clear All").
pub fn show_clear_menu(owner: HWND) -> bool {
    enable_dark_menu(owner);
    unsafe {
        let Ok(menu) = CreatePopupMenu() else { return false };
        apply_dark_menu_style(menu);
        let _ = AppendMenuW(menu, MF_STRING, 1, &HSTRING::from("Clear All"));
        let mut p = POINT::default();
        let _ = GetCursorPos(&mut p);
        let _ = SetForegroundWindow(owner);
        let flags = TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_BOTTOMALIGN | TPM_CENTERALIGN;
        let chosen = TrackPopupMenu(menu, flags, p.x, p.y, Some(0), owner, None).0 as u32;
        let _ = PostMessageW(Some(owner), WM_NULL, WPARAM(0), LPARAM(0));
        destroy_dark_menu(menu);
        chosen == 1
    }
}

/// Right-click menu on the Media pill ("Open · <App>").
pub fn show_media_menu(owner: HWND, app_name: &str) -> bool {
    enable_dark_menu(owner);
    unsafe {
        let Ok(menu) = CreatePopupMenu() else { return false };
        apply_dark_menu_style(menu);
        let label = format!("Open · {app_name}");
        let _ = AppendMenuW(menu, MF_STRING, 1, &HSTRING::from(label));
        let mut p = POINT::default();
        let _ = GetCursorPos(&mut p);
        let _ = SetForegroundWindow(owner);
        let flags = TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_BOTTOMALIGN | TPM_CENTERALIGN;
        let chosen = TrackPopupMenu(menu, flags, p.x, p.y, Some(0), owner, None).0 as u32;
        let _ = PostMessageW(Some(owner), WM_NULL, WPARAM(0), LPARAM(0));
        destroy_dark_menu(menu);
        chosen == 1
    }
}

/// Right-click menu on the Download pill ("Open download").
pub fn show_download_menu(owner: HWND) -> bool {
    enable_dark_menu(owner);
    unsafe {
        let Ok(menu) = CreatePopupMenu() else { return false };
        apply_dark_menu_style(menu);
        let _ = AppendMenuW(menu, MF_STRING, 1, &HSTRING::from("Open download folder"));
        let mut p = POINT::default();
        let _ = GetCursorPos(&mut p);
        let _ = SetForegroundWindow(owner);
        let flags = TPM_RETURNCMD | TPM_RIGHTBUTTON | TPM_BOTTOMALIGN | TPM_CENTERALIGN;
        let chosen = TrackPopupMenu(menu, flags, p.x, p.y, Some(0), owner, None).0 as u32;
        let _ = PostMessageW(Some(owner), WM_NULL, WPARAM(0), LPARAM(0));
        destroy_dark_menu(menu);
        chosen == 1
    }
}

pub const SHELF_OPEN: u32 = 1;
pub const SHELF_SHARE: u32 = 2;
pub const SHELF_REMOVE: u32 = 3;
pub const SHELF_EXPLORER: u32 = 4;
pub const SHELF_COPY_PATH: u32 = 5;
pub const SHELF_COPY_BASE64: u32 = 7;
pub const SHELF_PIN: u32 = 8;

pub const CLIPBOARD_PASTE: u32 = 1;
pub const CLIPBOARD_PASTE_PLAIN: u32 = 2;
pub const CLIPBOARD_OPEN: u32 = 3;
pub const CLIPBOARD_PIN: u32 = 4;
pub const CLIPBOARD_DELETE: u32 = 5;
pub const CLIPBOARD_CLEAR_ALL: u32 = 6;

pub const NOTE_OPEN: u32 = 1;
pub const NOTE_COPY: u32 = 2;
pub const NOTE_PIN: u32 = 3;
pub const NOTE_SHARE: u32 = 4;
pub const NOTE_DELETE: u32 = 5;
pub const NOTE_CLEAR_ALL: u32 = 6;

/// Initializes dark mode preferences globally in the process before windows are opened.
pub fn init_dark_mode() {
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    use windows::core::{w, PCSTR};

    unsafe {
        if let Ok(uxtheme) = LoadLibraryW(w!("uxtheme.dll")) {
            // Ordinal 135: SetPreferredAppMode(2 = ForceDark)
            if let Some(proc) = GetProcAddress(uxtheme, PCSTR(135 as _)) {
                let set_mode: unsafe extern "system" fn(i32) -> i32 = std::mem::transmute(proc);
                set_mode(2);
            }
            // Ordinal 136: FlushMenuThemes()
            if let Some(proc) = GetProcAddress(uxtheme, PCSTR(136 as _)) {
                let flush: unsafe extern "system" fn() = std::mem::transmute(proc);
                flush();
            }
        }
    }
}

/// Applies a dark background brush and theme settings to an HMENU so it matches Windows 11 dark mode.
pub fn apply_dark_menu_style(menu: HMENU) {
    use windows::Win32::UI::WindowsAndMessaging::{SetMenuInfo, MENUINFO, MIM_BACKGROUND, MIM_APPLYTOSUBMENUS};
    use windows::Win32::Graphics::Gdi::CreateSolidBrush;
    use windows::Win32::Foundation::COLORREF;

    unsafe {
        // Windows 11 dark context menu surface (#2C2C2C)
        let brush = CreateSolidBrush(COLORREF(0x002C_2C2C));
        let mi = MENUINFO {
            cbSize: std::mem::size_of::<MENUINFO>() as u32,
            fMask: MIM_BACKGROUND | MIM_APPLYTOSUBMENUS,
            hbrBack: brush,
            ..Default::default()
        };
        let _ = SetMenuInfo(menu, &mi);
    }
}

/// Cleans up any custom background brush allocated for the menu and destroys it.
pub fn destroy_dark_menu(menu: HMENU) {
    use windows::Win32::UI::WindowsAndMessaging::{DestroyMenu, GetMenuInfo, MENUINFO, MIM_BACKGROUND};
    use windows::Win32::Graphics::Gdi::DeleteObject;

    unsafe {
        let mut mi = MENUINFO {
            cbSize: std::mem::size_of::<MENUINFO>() as u32,
            fMask: MIM_BACKGROUND,
            ..Default::default()
        };
        if GetMenuInfo(menu, &mut mi).is_ok() && !mi.hbrBack.is_invalid() {
            let _ = DeleteObject(mi.hbrBack.into());
        }
        let _ = DestroyMenu(menu);
    }
}

/// Ensures Windows draws context menus with native dark mode styling and rounded corners.
pub fn enable_dark_menu(owner: HWND) {
    use windows::Win32::System::LibraryLoader::{GetProcAddress, LoadLibraryW};
    use windows::Win32::Graphics::Dwm::{DwmSetWindowAttribute, DWMWINDOWATTRIBUTE};
    use windows::core::{w, PCSTR};

    unsafe {
        let on: i32 = 1;
        let dark_mode = DWMWINDOWATTRIBUTE(crate::platform::os::build().map_or(20, crate::platform::os::dark_mode_attribute) as i32);
        let _ = DwmSetWindowAttribute(owner, dark_mode, &on as *const i32 as _, 4);

        if let Ok(uxtheme) = LoadLibraryW(w!("uxtheme.dll")) {
            // Ordinal 135: SetPreferredAppMode(2 = ForceDark)
            if let Some(proc) = GetProcAddress(uxtheme, PCSTR(135 as _)) {
                let set_mode: unsafe extern "system" fn(i32) -> i32 = std::mem::transmute(proc);
                set_mode(2);
            }
            // Ordinal 133: AllowDarkModeForWindow(hwnd, true)
            if let Some(proc) = GetProcAddress(uxtheme, PCSTR(133 as _)) {
                let allow_window: unsafe extern "system" fn(HWND, windows::core::BOOL) -> windows::core::BOOL = std::mem::transmute(proc);
                let _ = allow_window(owner, windows::core::BOOL(1));
            }
            // Ordinal 136: FlushMenuThemes()
            if let Some(proc) = GetProcAddress(uxtheme, PCSTR(136 as _)) {
                let flush: unsafe extern "system" fn() = std::mem::transmute(proc);
                flush();
            }
        }
    }
}

/// Right-click menu for one file on the shelf.
pub fn show_shelf_menu(owner: HWND, has_image: bool, is_pinned: bool) -> u32 {
    enable_dark_menu(owner);
    unsafe {
        let Ok(menu) = CreatePopupMenu() else { return 0 };
        apply_dark_menu_style(menu);
        let _ = AppendMenuW(menu, MF_STRING, SHELF_OPEN as usize, &HSTRING::from("Open"));
        let _ = AppendMenuW(menu, MF_STRING, SHELF_EXPLORER as usize, &HSTRING::from("Show in File Explorer"));
        let _ = AppendMenuW(menu, MF_STRING, SHELF_COPY_PATH as usize, &HSTRING::from("Copy File Path"));
        if has_image {
            let _ = AppendMenuW(menu, MF_STRING, SHELF_COPY_BASE64 as usize, &HSTRING::from("Copy Image as Base64"));
        }
        let pin_label = if is_pinned { "Unpin" } else { "Pin to Top" };
        let _ = AppendMenuW(menu, MF_STRING, SHELF_PIN as usize, &HSTRING::from(pin_label));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
        let _ = AppendMenuW(menu, MF_STRING, SHELF_SHARE as usize, &HSTRING::from("Share"));
        let _ = AppendMenuW(menu, MF_STRING, SHELF_REMOVE as usize, &HSTRING::from("Remove from Shelf"));
        let mut p = POINT::default();
        let _ = GetCursorPos(&mut p);
        let _ = SetForegroundWindow(owner);
        let flags = TPM_RETURNCMD | TPM_RIGHTBUTTON;
        let chosen = TrackPopupMenu(menu, flags, p.x, p.y, Some(0), owner, None).0 as u32;
        let _ = PostMessageW(Some(owner), WM_NULL, WPARAM(0), LPARAM(0));
        destroy_dark_menu(menu);
        chosen
    }
}

/// Right-click menu for a stored sticky note.
pub fn show_note_item_menu(owner: HWND, _has_image: bool, is_pinned: bool) -> u32 {
    enable_dark_menu(owner);
    unsafe {
        let Ok(menu) = CreatePopupMenu() else { return 0 };
        apply_dark_menu_style(menu);
        let _ = AppendMenuW(menu, MF_STRING, NOTE_OPEN as usize, &HSTRING::from("Open Note"));
        let _ = AppendMenuW(menu, MF_STRING, NOTE_COPY as usize, &HSTRING::from("Copy Note Text"));
        let pin_label = if is_pinned { "Unpin" } else { "Pin to Top" };
        let _ = AppendMenuW(menu, MF_STRING, NOTE_PIN as usize, &HSTRING::from(pin_label));
        let _ = AppendMenuW(menu, MF_STRING, NOTE_SHARE as usize, &HSTRING::from("Share"));
        let _ = AppendMenuW(menu, MF_STRING, NOTE_DELETE as usize, &HSTRING::from("Delete Note"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
        let _ = AppendMenuW(menu, MF_STRING, NOTE_CLEAR_ALL as usize, &HSTRING::from("Clear All Notes"));
        let mut p = POINT::default();
        let _ = GetCursorPos(&mut p);
        let _ = SetForegroundWindow(owner);
        let flags = TPM_RETURNCMD | TPM_RIGHTBUTTON;
        let chosen = TrackPopupMenu(menu, flags, p.x, p.y, Some(0), owner, None).0 as u32;
        let _ = PostMessageW(Some(owner), WM_NULL, WPARAM(0), LPARAM(0));
        destroy_dark_menu(menu);
        chosen
    }
}

/// Right-click menu for an item in the clipboard history.
pub fn show_clipboard_item_menu(owner: HWND, is_image: bool, has_html: bool, is_pinned: bool) -> u32 {
    enable_dark_menu(owner);
    unsafe {
        let Ok(menu) = CreatePopupMenu() else { return 0 };
        apply_dark_menu_style(menu);
        let _ = AppendMenuW(menu, MF_STRING, CLIPBOARD_PASTE as usize, &HSTRING::from("Copy to Clipboard"));
        if has_html {
            let _ = AppendMenuW(menu, MF_STRING, CLIPBOARD_PASTE_PLAIN as usize, &HSTRING::from("Copy Plain Text Only"));
        }
        if is_image {
            let _ = AppendMenuW(menu, MF_STRING, CLIPBOARD_OPEN as usize, &HSTRING::from("Open Image"));
        }
        let pin_label = if is_pinned { "Unpin" } else { "Pin to Top" };
        let _ = AppendMenuW(menu, MF_STRING, CLIPBOARD_PIN as usize, &HSTRING::from(pin_label));
        let _ = AppendMenuW(menu, MF_STRING, CLIPBOARD_DELETE as usize, &HSTRING::from("Delete from History"));
        let _ = AppendMenuW(menu, MF_SEPARATOR, 0, None);
        let _ = AppendMenuW(menu, MF_STRING, CLIPBOARD_CLEAR_ALL as usize, &HSTRING::from("Clear All History"));
        let mut p = POINT::default();
        let _ = GetCursorPos(&mut p);
        let _ = SetForegroundWindow(owner);
        let flags = TPM_RETURNCMD | TPM_RIGHTBUTTON;
        let chosen = TrackPopupMenu(menu, flags, p.x, p.y, Some(0), owner, None).0 as u32;
        let _ = PostMessageW(Some(owner), WM_NULL, WPARAM(0), LPARAM(0));
        destroy_dark_menu(menu);
        chosen
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn alpha(px: &[u8], size: usize, x: usize, y: usize) -> u8 {
        px[(y * size + x) * 4 + 3]
    }

    #[test]
    fn pill_icon_is_a_white_capsule() {
        let size = 32;
        let px = pill_icon_bgra(size);
        assert_eq!(px.len(), size * size * 4);
        assert_eq!(alpha(&px, size, 16, 16), 255);
        assert_eq!(&px[(16 * size + 16) * 4..][..3], &[255, 255, 255]);
        assert_eq!(alpha(&px, size, 0, 0), 0);
        assert_eq!(alpha(&px, size, 16, 4), 0, "capsule is shorter than the icon");
        let edge = alpha(&px, size, 2, 16);
        assert!(edge > 0 && edge < 255, "left end is anti-aliased: {edge}");
    }
}
