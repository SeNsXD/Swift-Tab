//! A window that keeps Windows' frame (rounded corners, shadow, open and minimise animations) but
//! draws its own title bar: the settings window's and the setup's.

use std::cell::Cell;
use std::ffi::c_void;
use std::path::PathBuf;
use std::sync::Once;

use windows::core::{w, HSTRING, PCWSTR};
use windows::Win32::Foundation::{HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Dwm::{
    DwmSetWindowAttribute, DWMWA_BORDER_COLOR, DWMWA_CAPTION_COLOR, DWMWA_TEXT_COLOR, DWMWA_WINDOW_CORNER_PREFERENCE, DWMWCP_ROUND,
    DWMWINDOWATTRIBUTE,
};
use windows::Win32::System::Registry::{RegSetKeyValueW, HKEY_CURRENT_USER, REG_SZ};
use windows::Win32::System::Threading::GetCurrentThreadId;
use windows::Win32::UI::HiDpi::{GetDpiForWindow, GetSystemMetricsForDpi};
use windows::Win32::UI::Shell::{DefSubclassProc, SetWindowSubclass};
use windows::Win32::UI::WindowsAndMessaging::{
    CallNextHookEx, DefWindowProcW, GetClassNameW, IsZoomed, SetWindowPos, SetWindowsHookExW, UnhookWindowsHookEx, CBT_CREATEWNDW,
    HCBT_CREATEWND, HHOOK, NCCALCSIZE_PARAMS, SM_CXPADDEDBORDER, SM_CYFRAME, SWP_FRAMECHANGED, SWP_NOMOVE, SWP_NOSIZE,
    SWP_NOZORDER, WH_CBT, WM_NCCALCSIZE, WS_CHILD,
};

const NOBG_ICO: &[u8] = include_bytes!("../../assets/nowbar-nobg.ico");

#[link(name = "shell32")]
extern "system" {
    fn SHChangeNotify(w_event_id: i32, u_flags: u32, dw_item1: *const c_void, dw_item2: *const c_void);
}

#[link(name = "user32")]
extern "system" {
    fn SetClassLongPtrW(hwnd: HWND, n_index: i32, dw_new_long: isize) -> usize;
}

const GCLP_HICON: i32 = -14;
const GCLP_HICONSM: i32 = -34;
const SHCNE_ASSOCCHANGED: i32 = 0x0800_0000;
const SHCNF_IDLIST: u32 = 0x0000;

fn cached_nobg_ico_path() -> Option<PathBuf> {
    let base = std::env::var_os("LOCALAPPDATA")
        .map(PathBuf::from)
        .unwrap_or_else(std::env::temp_dir);
    let dir = base.join("SwiftTab");
    let _ = std::fs::create_dir_all(&dir);
    let ico = dir.join("swift-tab-icon-v2.ico");
    let needs_write = std::fs::metadata(&ico).map_or(true, |m| m.len() != NOBG_ICO.len() as u64);
    if needs_write {
        let _ = std::fs::write(&ico, NOBG_ICO);
    }
    ico.exists().then_some(ico)
}

fn set_mui_cache_string(value_name: &str, text: &str) {
    let key = w!("Software\\Classes\\Local Settings\\Software\\Microsoft\\Windows\\Shell\\MuiCache");
    let name = HSTRING::from(value_name);
    let data: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let _ = RegSetKeyValueW(
            HKEY_CURRENT_USER,
            key,
            &name,
            REG_SZ.0,
            Some(data.as_ptr() as *const _),
            (data.len() * 2) as u32,
        );
    }
}

/// Registers the clean Swift Tab taskbar icon file,
/// updates Windows Explorer's MuiCache so `swifttab-settings.exe` / `swifttab.exe` show as
/// "Swift Tab" instead of the raw `.exe` filename, and notifies the Shell.
pub fn register_app_identity() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let _ = cached_nobg_ico_path();
        if let Ok(exe) = std::env::current_exe() {
            let mut paths = vec![exe.clone()];
            if let Some(parent) = exe.parent() {
                paths.push(parent.join("swifttab.exe"));
                paths.push(parent.join("swifttab-settings.exe"));
                paths.push(parent.join("swifttab-setup.exe"));
            }
            for p in paths {
                let s = p.display().to_string();
                set_mui_cache_string(&format!("{s}.FriendlyAppName"), "Swift Tab");
                set_mui_cache_string(&format!("{s}.ApplicationCompany"), "Swift Tab");
            }
        }
        unsafe {
            SHChangeNotify(SHCNE_ASSOCCHANGED, SHCNF_IDLIST, std::ptr::null(), std::ptr::null());
        }
    });
}

pub fn apply_window_icon_and_identity(hwnd: HWND) {
    register_app_identity();
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SendMessageW, ICON_BIG, ICON_SMALL, SM_CXICON, SM_CXSMICON, WM_SETICON};
        let sm = GetSystemMetrics(SM_CXSMICON).max(16);
        let lg = GetSystemMetrics(SM_CXICON).max(48);
        if let Some(h_sm) = load_window_icon(sm) {
            let _ = SendMessageW(hwnd, WM_SETICON, Some(WPARAM(ICON_SMALL as usize)), Some(LPARAM(h_sm)));
            let _ = SetClassLongPtrW(hwnd, GCLP_HICONSM, h_sm);
        }
        if let Some(h_lg) = load_window_icon(lg) {
            let _ = SendMessageW(hwnd, WM_SETICON, Some(WPARAM(ICON_BIG as usize)), Some(LPARAM(h_lg)));
            let _ = SetClassLongPtrW(hwnd, GCLP_HICON, h_lg);
        }
    }
}

/// The window icon at `size` px. Checks embedded PE resources first (2, then 1),
/// then memory icon, then cached file.
unsafe fn load_window_icon(size: i32) -> Option<isize> {
    use windows::Win32::UI::WindowsAndMessaging::{LoadImageW, IMAGE_ICON, LR_DEFAULTCOLOR, LR_LOADFROMFILE, LR_SHARED};
    if let Ok(hinst) = windows::Win32::System::LibraryLoader::GetModuleHandleW(None) {
        for id in [2usize, 1] {
            if let Ok(h) = LoadImageW(Some(hinst.into()), PCWSTR(id as _), IMAGE_ICON, size, size, LR_DEFAULTCOLOR | LR_SHARED) {
                if !h.is_invalid() {
                    return Some(h.0 as isize);
                }
            }
        }
    }
    if let Some(h) = icon_from_memory(NOBG_ICO, size) {
        return Some(h);
    }
    let path = HSTRING::from(cached_nobg_ico_path()?.display().to_string());
    match LoadImageW(None, PCWSTR(path.as_ptr()), IMAGE_ICON, size, size, LR_DEFAULTCOLOR | LR_LOADFROMFILE) {
        Ok(h) if !h.is_invalid() => Some(h.0 as isize),
        _ => None,
    }
}

/// An HICON at `size` px from the bytes of an .ico file: the smallest image that is at least
/// `size` (or the largest there is), handed to `CreateIconFromResourceEx` (PNG images included).
unsafe fn icon_from_memory(ico: &[u8], size: i32) -> Option<isize> {
    use windows::Win32::UI::WindowsAndMessaging::{CreateIconFromResourceEx, LR_DEFAULTCOLOR};
    let u16_at = |o: usize| ico.get(o..o + 2).map(|b| u16::from_le_bytes([b[0], b[1]]) as usize);
    let u32_at = |o: usize| ico.get(o..o + 4).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]) as usize);
    if u16_at(2)? != 1 {
        return None;
    }
    let count = u16_at(4)?;
    let mut best: Option<(usize, usize, usize)> = None; // (width, offset, length)
    for i in 0..count {
        let e = 6 + i * 16;
        let w = match *ico.get(e)? {
            0 => 256,
            n => n as usize,
        };
        let len = u32_at(e + 8)?;
        let off = u32_at(e + 12)?;
        let better = match best {
            None => true,
            Some((bw, _, _)) => {
                let want = size.max(1) as usize;
                if bw >= want { w >= want && w < bw } else { w > bw }
            }
        };
        if better {
            best = Some((w, off, len));
        }
    }
    let (_, off, len) = best?;
    let bits = ico.get(off..off.checked_add(len)?)?;
    let h = CreateIconFromResourceEx(bits, true, 0x0003_0000, size, size, LR_DEFAULTCOLOR).ok()?;
    (!h.is_invalid()).then_some(h.0 as isize)
}

/// Takes over a window made without the hook (it existed already): its title bar goes now.
pub fn adopt(hwnd: HWND) {
    unsafe {
        let _ = SetWindowSubclass(hwnd, Some(frame_proc), 1, 0);
        let _ = SetWindowPos(hwnd, None, 0, 0, 0, 0, SWP_FRAMECHANGED | SWP_NOMOVE | SWP_NOSIZE | SWP_NOZORDER);
    }
    apply_window_icon_and_identity(hwnd);
}

pub fn style_frame(hwnd: HWND, dark: bool) {
    // COLORREF is 0x00BBGGRR: Windows 11 Fluent Mica base (#202020 dark, #F3F3F3 light) and primary text.
    let (bg, text): (u32, u32) = if dark { (0x0020_2020, 0x00FF_FFFF) } else { (0x00F3_F3F3, 0x001A_1A1A) };
    let on: i32 = dark as i32;
    // DWMWA_SYSTEMBACKDROP_TYPE (38): DWMSBT_MAINWINDOW (2 = Mica) when transparency is active, else DWMSBT_NONE (1).
    let mica: i32 = if crate::platform::os::solid_fallback() { 1 } else { 2 };
    // 19 before Windows 10 20H1. The colours, corners and Mica backdrop are Windows 11's: they fail on 10.
    let dark_mode = DWMWINDOWATTRIBUTE(crate::platform::os::build().map_or(20, crate::platform::os::dark_mode_attribute) as i32);
    let system_backdrop = DWMWINDOWATTRIBUTE(38);
    unsafe {
        let _ = DwmSetWindowAttribute(hwnd, dark_mode, &on as *const i32 as _, 4);
        let _ = DwmSetWindowAttribute(hwnd, system_backdrop, &mica as *const i32 as _, 4);
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_CAPTION_COLOR, &bg as *const u32 as _, 4);
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_BORDER_COLOR, &bg as *const u32 as _, 4);
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_TEXT_COLOR, &text as *const u32 as _, 4);
        let _ = DwmSetWindowAttribute(hwnd, DWMWA_WINDOW_CORNER_PREFERENCE, &DWMWCP_ROUND as *const _ as _, 4);
    }
}

/// Windows keeps the whole frame (rounded corners, the shadow, the side and bottom resize
/// borders, the minimise and restore animations) and gives up only its title bar: the client
/// area reaches the window's top, where the window draws its own (as Windows Terminal does).
unsafe extern "system" fn frame_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM, _: usize, _: usize) -> LRESULT {
    if msg == WM_NCCALCSIZE {
        // wParam TRUE: NCCALCSIZE_PARAMS, whose first rect is the window's; FALSE (while the window
        // is created): that rect alone. Either way it becomes the client rect.
        let rect = if wp.0 != 0 { unsafe { &raw mut (*(lp.0 as *mut NCCALCSIZE_PARAMS)).rgrc[0] } } else { lp.0 as *mut RECT };
        let top = unsafe { (*rect).top };
        let result = unsafe { DefWindowProcW(hwnd, msg, wp, lp) };
        unsafe { (*rect).top = top };
        // Maximised, the window overhangs the screen by its frame: keep that much off the top.
        if unsafe { IsZoomed(hwnd) }.as_bool() {
            let dpi = unsafe { GetDpiForWindow(hwnd) };
            unsafe { (*rect).top += GetSystemMetricsForDpi(SM_CYFRAME, dpi) + GetSystemMetricsForDpi(SM_CXPADDEDBORDER, dpi) };
        }
        return result;
    }
    // Prevent background erasing to completely eliminate window redraw flicker
    if msg == windows::Win32::UI::WindowsAndMessaging::WM_ERASEBKGND {
        return LRESULT(1);
    }
    unsafe { DefSubclassProc(hwnd, msg, wp, lp) }
}

thread_local! {
    /// The hook that catches the window as winit creates it, and the window's dark mode.
    static CREATE_HOOK: Cell<(HHOOK, bool)> = const { Cell::new((HHOOK(std::ptr::null_mut()), false)) };
}

/// Takes over the title bar from the window's first message: a CBT hook on this thread subclasses
/// the window as winit creates it, before it has a size. It is shown with its frame, so Windows'
/// open animation (a fade and a slight zoom) plays on the window as it stays, not on a frameless
/// one whose edges and layout change some 40 ms in. `adopt` still tells winit and Slint.
pub fn frame_from_creation(dark: bool) {
    if let Ok(hook) = unsafe { SetWindowsHookExW(WH_CBT, Some(on_create), None, GetCurrentThreadId()) } {
        CREATE_HOOK.with(|h| h.set((hook, dark)));
    }
}

unsafe extern "system" fn on_create(code: i32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    if code == HCBT_CREATEWND as i32 {
        let hwnd = HWND(wp.0 as *mut _);
        let style = unsafe { (*(*(lp.0 as *const CBT_CREATEWNDW)).lpcs).style } as u32;
        let mut class = [0u16; 32];
        let len = unsafe { GetClassNameW(hwnd, &mut class) }.max(0) as usize;
        // winit's window class (its event target and glutin's dummy window have their own).
        if style & WS_CHILD.0 == 0 {
            let class_str = String::from_utf16_lossy(&class[..len]);
            if class_str == "Window Class" || class_str.contains("Window") || class_str.contains("winit") {
                let (hook, dark) = CREATE_HOOK.with(|h| h.replace((HHOOK::default(), false)));
                unsafe {
                    let _ = SetWindowSubclass(hwnd, Some(frame_proc), 1, 0);
                    let _ = UnhookWindowsHookEx(hook);
                }
                style_frame(hwnd, dark);
                apply_window_icon_and_identity(hwnd);
            }
        }
    }
    unsafe { CallNextHookEx(None, code, wp, lp) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_load_icon() {
        unsafe {
            let h = load_window_icon(32);
            assert!(h.is_some(), "icon must be loaded!");
        }
    }
}
