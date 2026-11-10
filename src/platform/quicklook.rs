//! Floating Quick Look file preview (like macOS Quick Look / PowerToys Peek).
//! Pressing Spacebar or selecting Preview from the context menu opens a centered borderless preview.
//! Pressing Spacebar, Escape, or clicking outside closes it instantly.

use std::cell::RefCell;
use std::os::windows::ffi::OsStrExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};

use windows::core::{w, PCWSTR};
use windows::Win32::Foundation::{COLORREF, HWND, LPARAM, LRESULT, RECT, WPARAM};
use windows::Win32::Graphics::Gdi::{
    BeginPaint, BitBlt, CreateCompatibleBitmap, CreateCompatibleDC, CreateFontW, CreateSolidBrush, DeleteDC, DeleteObject,
    DrawTextW, EndPaint, FillRect, SelectObject, SetBkMode, SetTextColor, StretchDIBits, BITMAPINFO, BITMAPINFOHEADER, BI_RGB,
    DIB_RGB_COLORS, DT_END_ELLIPSIS, DT_LEFT, DT_NOPREFIX, DT_SINGLELINE, DT_WORDBREAK, FONT_CHARSET, FONT_CLIP_PRECISION,
    FONT_OUTPUT_PRECISION, FONT_QUALITY, FW_BOLD, FW_NORMAL, PAINTSTRUCT, SRCCOPY, TRANSPARENT,
};
use windows::Win32::System::LibraryLoader::GetModuleHandleW;
use windows::Win32::UI::Input::KeyboardAndMouse::{VK_ESCAPE, VK_RETURN, VK_SPACE};
use windows::Win32::UI::Shell::{SHGetFileInfoW, SHFILEINFOW, SHGFI_ICON, SHGFI_LARGEICON, SHGFI_TYPENAME};
use windows::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DestroyWindow, DrawIconEx, GetSystemMetrics, RegisterClassW, SetWindowPos, ShowWindow,
    DI_NORMAL, HWND_TOPMOST, SM_CXSCREEN, SM_CYSCREEN, SWP_NOMOVE, SWP_NOSIZE, SWP_SHOWWINDOW, SW_SHOW, WM_DESTROY,
    WM_ERASEBKGND, WM_KEYDOWN, WM_KILLFOCUS, WM_LBUTTONDOWN, WM_PAINT, WNDCLASSW, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_POPUP,
    WS_VISIBLE,
};

pub const QUICKLOOK_CLASS: PCWSTR = w!("SwiftTabQuickLook");

static REGISTERED: AtomicBool = AtomicBool::new(false);

thread_local! {
    static CURRENT_HWND: RefCell<Option<HWND>> = const { RefCell::new(None) };
    static CURRENT_PATH: RefCell<Option<PathBuf>> = const { RefCell::new(None) };
    static CACHED_PREVIEW: RefCell<Option<PreviewData>> = const { RefCell::new(None) };
}

struct PreviewData {
    title: String,
    size_text: String,
    type_name: String,
    icon: Option<windows::Win32::UI::WindowsAndMessaging::HICON>,
    image_crest: Option<crate::render::crest::Crest>,
    text_content: Option<String>,
}

fn register_class() {
    if REGISTERED.swap(true, Ordering::SeqCst) {
        return;
    }
    unsafe {
        let instance = GetModuleHandleW(None).unwrap_or_default();
        let wc = WNDCLASSW {
            style: windows::Win32::UI::WindowsAndMessaging::CS_HREDRAW | windows::Win32::UI::WindowsAndMessaging::CS_VREDRAW,
            lpfnWndProc: Some(quicklook_proc),
            hInstance: instance.into(),
            hCursor: windows::Win32::UI::WindowsAndMessaging::LoadCursorW(None, windows::Win32::UI::WindowsAndMessaging::IDC_ARROW).unwrap_or_default(),
            lpszClassName: QUICKLOOK_CLASS,
            ..Default::default()
        };
        let _ = RegisterClassW(&wc);
    }
}

pub fn is_open() -> bool {
    CURRENT_HWND.with(|h| h.borrow().is_some())
}

pub fn current_path() -> Option<PathBuf> {
    CURRENT_PATH.with(|p| p.borrow().clone())
}

pub fn close() {
    CURRENT_HWND.with(|h| {
        if let Some(hwnd) = h.borrow_mut().take() {
            unsafe {
                let _ = DestroyWindow(hwnd);
            }
        }
    });
    CURRENT_PATH.with(|p| *p.borrow_mut() = None);
    CACHED_PREVIEW.with(|c| *c.borrow_mut() = None);
}

pub fn toggle(path: &Path) {
    if is_open() {
        if let Some(cur) = current_path() {
            if cur == path {
                close();
                return;
            }
        }
        close();
    }
    show(path);
}

pub fn show(path: &Path) {
    if !path.exists() {
        return;
    }
    register_class();

    // Prepare preview data
    let title = path.file_name().map_or_else(|| "File Preview".to_string(), |n| n.to_string_lossy().into_owned());
    let meta = std::fs::metadata(path).ok();
    let byte_len = meta.as_ref().map_or(0, |m| m.len());
    let size_text = if byte_len < 1024 {
        format!("{byte_len} B")
    } else if byte_len < 1024 * 1024 {
        format!("{:.1} KB", byte_len as f64 / 1024.0)
    } else if byte_len < 1024 * 1024 * 1024 {
        format!("{:.1} MB", byte_len as f64 / (1024.0 * 1024.0))
    } else {
        format!("{:.2} GB", byte_len as f64 / (1024.0 * 1024.0 * 1024.0))
    };

    let mut shfi = SHFILEINFOW::default();
    let mut type_name = String::new();
    let mut icon = None;
    unsafe {
        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
        if SHGetFileInfoW(PCWSTR(wide.as_ptr()), Default::default(), Some(&mut shfi), std::mem::size_of::<SHFILEINFOW>() as u32, SHGFI_ICON | SHGFI_LARGEICON | SHGFI_TYPENAME) != 0 {
            if !shfi.hIcon.is_invalid() {
                icon = Some(shfi.hIcon);
            }
            let len = shfi.szTypeName.iter().position(|&c| c == 0).unwrap_or(shfi.szTypeName.len());
            type_name = String::from_utf16_lossy(&shfi.szTypeName[..len]);
        }
    }

    let ext = path.extension().and_then(|e| e.to_str()).map(|e| e.to_lowercase()).unwrap_or_default();
    let is_img = matches!(ext.as_str(), "png" | "jpg" | "jpeg" | "webp" | "bmp" | "gif" | "ico" | "avif" | "heic");
    let is_txt = matches!(ext.as_str(), "txt" | "md" | "rs" | "json" | "toml" | "yaml" | "yml" | "js" | "ts" | "py" | "c" | "cpp" | "h" | "html" | "css" | "log" | "sh" | "bat" | "ps1");

    let image_crest = if is_img {
        crate::render::crest::load_cover(path, 800, 600)
    } else {
        None
    };

    let text_content = if is_txt && !is_img {
        if let Ok(bytes) = std::fs::read(path) {
            let max_bytes = bytes.len().min(32 * 1024);
            let text = String::from_utf8_lossy(&bytes[..max_bytes]).into_owned();
            Some(text)
        } else {
            None
        }
    } else {
        None
    };

    let preview = PreviewData {
        title,
        size_text,
        type_name,
        icon,
        image_crest,
        text_content,
    };

    let (mut win_w, mut win_h) = (720, 500);
    if let Some(c) = &preview.image_crest {
        if c.size.0 > 0 && c.size.1 > 0 {
            let aspect = c.size.0 as f32 / c.size.1 as f32;
            if aspect > 1.0 {
                win_w = (c.size.0.min(840) as i32).max(480);
                win_h = ((win_w as f32 / aspect) as i32).clamp(360, 640) + 54;
            } else {
                win_h = (c.size.1.min(620) as i32).max(380) + 54;
                win_w = ((win_h as f32 * aspect) as i32).clamp(420, 800);
            }
        }
    } else if !is_txt {
        win_w = 480;
        win_h = 320;
    }

    unsafe {
        let (screen_w, screen_h) = (GetSystemMetrics(SM_CXSCREEN), GetSystemMetrics(SM_CYSCREEN));
        let x = (screen_w - win_w) / 2;
        let y = (screen_h - win_h) / 2 - 30;

        let instance = GetModuleHandleW(None).unwrap_or_default();
        let ex = WS_EX_TOOLWINDOW | WS_EX_TOPMOST;
        let hwnd = CreateWindowExW(
            ex,
            QUICKLOOK_CLASS,
            w!("Quick Look"),
            WS_POPUP | WS_VISIBLE,
            x,
            y,
            win_w,
            win_h,
            None,
            None,
            Some(instance.into()),
            None,
        );

        if let Ok(hwnd) = hwnd {
            // Apply Windows 11 DWM rounded corners and dark theme
            let corners: u32 = 2; // DWMWCP_ROUND
            let dark: u32 = 1;
            let _ = windows::Win32::Graphics::Dwm::DwmSetWindowAttribute(hwnd, windows::Win32::Graphics::Dwm::DWMWA_WINDOW_CORNER_PREFERENCE, &corners as *const _ as _, 4);
            let _ = windows::Win32::Graphics::Dwm::DwmSetWindowAttribute(hwnd, windows::Win32::Graphics::Dwm::DWMWA_USE_IMMERSIVE_DARK_MODE, &dark as *const _ as _, 4);

            CURRENT_HWND.with(|h| *h.borrow_mut() = Some(hwnd));
            CURRENT_PATH.with(|p| *p.borrow_mut() = Some(path.to_path_buf()));
            CACHED_PREVIEW.with(|c| *c.borrow_mut() = Some(preview));

            let _ = SetWindowPos(hwnd, Some(HWND_TOPMOST), 0, 0, 0, 0, SWP_NOMOVE | SWP_NOSIZE | SWP_SHOWWINDOW);
            let _ = ShowWindow(hwnd, SW_SHOW);
        }
    }
}

unsafe fn create_app_font(size: i32, weight: i32, family: PCWSTR) -> windows::Win32::Graphics::Gdi::HFONT {
    CreateFontW(
        size,
        0,
        0,
        0,
        weight,
        0,
        0,
        0,
        FONT_CHARSET(0),
        FONT_OUTPUT_PRECISION(0),
        FONT_CLIP_PRECISION(0),
        FONT_QUALITY(0),
        0,
        family,
    )
}

extern "system" fn quicklook_proc(hwnd: HWND, msg: u32, wp: WPARAM, lp: LPARAM) -> LRESULT {
    match msg {
        WM_KEYDOWN => {
            let vk = wp.0 as u16;
            if vk == VK_SPACE.0 || vk == VK_ESCAPE.0 {
                close();
                return LRESULT(0);
            }
            if vk == VK_RETURN.0 {
                if let Some(path) = current_path() {
                    unsafe {
                        let wide: Vec<u16> = path.as_os_str().encode_wide().chain(std::iter::once(0)).collect();
                        windows::Win32::UI::Shell::ShellExecuteW(None, w!("open"), PCWSTR(wide.as_ptr()), None, None, windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL);
                    }
                }
                close();
                return LRESULT(0);
            }
            LRESULT(0)
        }
        WM_LBUTTONDOWN => {
            let x = (lp.0 & 0xFFFF) as i16 as i32;
            let y = ((lp.0 >> 16) & 0xFFFF) as i16 as i32;
            // Check top right close button (within 44px of top right)
            let mut rect = RECT::default();
            unsafe { let _ = windows::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &mut rect); };
            if x >= rect.right - 44 && y <= 44 {
                close();
                return LRESULT(0);
            }
            // Drag window when clicking top bar
            if y <= 48 {
                unsafe {
                    let _ = windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture();
                    windows::Win32::UI::WindowsAndMessaging::SendMessageW(hwnd, windows::Win32::UI::WindowsAndMessaging::WM_NCLBUTTONDOWN, Some(WPARAM(2)), Some(LPARAM(0)));
                }
                return LRESULT(0);
            }
            LRESULT(0)
        }
        WM_KILLFOCUS => {
            close();
            LRESULT(0)
        }
        WM_PAINT => {
            unsafe {
                let mut ps = PAINTSTRUCT::default();
                let hdc = BeginPaint(hwnd, &mut ps);
                if !hdc.is_invalid() {
                    let mut rect = RECT::default();
                    let _ = windows::Win32::UI::WindowsAndMessaging::GetClientRect(hwnd, &mut rect);
                    let (w, h) = (rect.right - rect.left, rect.bottom - rect.top);

                    // Double buffer
                    let mem_dc = CreateCompatibleDC(Some(hdc));
                    let mem_bmp = CreateCompatibleBitmap(hdc, w, h);
                    let old_bmp = SelectObject(mem_dc, mem_bmp.into());

                    // Background: Dark charcoal #1C1C1E
                    let bg_brush = CreateSolidBrush(COLORREF(0x001E1C1C));
                    FillRect(mem_dc, &rect, bg_brush);
                    let _ = DeleteObject(bg_brush.into());

                    // Top header bar: #2C2C2E
                    let top_rect = RECT { left: 0, top: 0, right: w, bottom: 48 };
                    let header_brush = CreateSolidBrush(COLORREF(0x002E2C2C));
                    FillRect(mem_dc, &top_rect, header_brush);
                    let _ = DeleteObject(header_brush.into());

                    // Divider line
                    let div_rect = RECT { left: 0, top: 48, right: w, bottom: 49 };
                    let div_brush = CreateSolidBrush(COLORREF(0x003A3838));
                    FillRect(mem_dc, &div_rect, div_brush);
                    let _ = DeleteObject(div_brush.into());

                    CACHED_PREVIEW.with(|c| {
                        if let Some(preview) = c.borrow().as_ref() {
                            // Draw icon
                            if let Some(hicon) = preview.icon {
                                let _ = DrawIconEx(mem_dc, 12, 8, hicon, 32, 32, 0, None, DI_NORMAL);
                            }

                            // Draw title & size
                            SetBkMode(mem_dc, TRANSPARENT);
                            let font = create_app_font(17, FW_BOLD.0 as i32, w!("Segoe UI Variable Text"));
                            let old_font = SelectObject(mem_dc, font.into());
                            SetTextColor(mem_dc, COLORREF(0x00FFFFFF));

                            let mut title_rect = RECT { left: 52, top: 8, right: w - 90, bottom: 28 };
                            let mut wide_title: Vec<u16> = preview.title.encode_utf16().collect();
                            DrawTextW(mem_dc, &mut wide_title, &mut title_rect, DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX);

                            let small_font = create_app_font(13, FW_NORMAL.0 as i32, w!("Segoe UI Variable Text"));
                            SelectObject(mem_dc, small_font.into());
                            SetTextColor(mem_dc, COLORREF(0x00A0A0A0));
                            let sub_text = format!("{} • {}", preview.type_name, preview.size_text);
                            let mut wide_sub: Vec<u16> = sub_text.encode_utf16().collect();
                            let mut sub_rect = RECT { left: 52, top: 27, right: w - 90, bottom: 44 };
                            DrawTextW(mem_dc, &mut wide_sub, &mut sub_rect, DT_LEFT | DT_SINGLELINE | DT_END_ELLIPSIS | DT_NOPREFIX);

                            // Close button "×"
                            SelectObject(mem_dc, font.into());
                            SetTextColor(mem_dc, COLORREF(0x00888888));
                            let mut x_rect = RECT { left: w - 40, top: 8, right: w - 8, bottom: 40 };
                            let mut wide_x: [u16; 1] = [0x00D7]; // ×
                            DrawTextW(mem_dc, &mut wide_x, &mut x_rect, windows::Win32::Graphics::Gdi::DT_CENTER | DT_SINGLELINE);

                            // Content area
                            let content_rect = RECT { left: 16, top: 60, right: w - 16, bottom: h - 16 };
                            if let Some(crest) = &preview.image_crest {
                                // Draw image
                                let img_w = crest.size.0 as i32;
                                let img_h = crest.size.1 as i32;
                                let avail_w = w - 32;
                                let avail_h = h - 68;
                                let scale = (avail_w as f32 / img_w.max(1) as f32).min(avail_h as f32 / img_h.max(1) as f32).min(1.0);
                                let draw_w = (img_w as f32 * scale).round() as i32;
                                let draw_h = (img_h as f32 * scale).round() as i32;
                                let dest_x = (w - draw_w) / 2;
                                let dest_y = 52 + (avail_h - draw_h) / 2;

                                let mut bmi = BITMAPINFO {
                                    bmiHeader: BITMAPINFOHEADER {
                                        biSize: std::mem::size_of::<BITMAPINFOHEADER>() as u32,
                                        biWidth: img_w,
                                        biHeight: -img_h, // top-down
                                        biPlanes: 1,
                                        biBitCount: 32,
                                        biCompression: BI_RGB.0,
                                        biSizeImage: (img_w * img_h * 4) as u32,
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                };
                                StretchDIBits(
                                    mem_dc,
                                    dest_x,
                                    dest_y,
                                    draw_w,
                                    draw_h,
                                    0,
                                    0,
                                    img_w,
                                    img_h,
                                    Some(crest.px.as_ptr() as *const _),
                                    &mut bmi,
                                    DIB_RGB_COLORS,
                                    SRCCOPY,
                                );
                            } else if let Some(txt) = &preview.text_content {
                                // Draw text content
                                let code_font = create_app_font(14, FW_NORMAL.0 as i32, w!("Cascadia Code"));
                                SelectObject(mem_dc, code_font.into());
                                SetTextColor(mem_dc, COLORREF(0x00E0E0E0));
                                let mut text_box = content_rect;
                                let mut wide_txt: Vec<u16> = txt.encode_utf16().collect();
                                DrawTextW(mem_dc, &mut wide_txt, &mut text_box, DT_LEFT | DT_WORDBREAK | DT_NOPREFIX);
                                let _ = DeleteObject(code_font.into());
                            } else {
                                // Draw big icon for other files
                                if let Some(hicon) = preview.icon {
                                    let icon_x = (w - 64) / 2;
                                    let icon_y = 52 + (h - 68 - 100) / 2;
                                    let _ = DrawIconEx(mem_dc, icon_x, icon_y, hicon, 64, 64, 0, None, DI_NORMAL);

                                    let mut prompt_rect = RECT { left: 20, top: icon_y + 80, right: w - 20, bottom: icon_y + 110 };
                                    SetTextColor(mem_dc, COLORREF(0x00A0A0A0));
                                    SelectObject(mem_dc, small_font.into());
                                    let mut prompt: Vec<u16> = "Press Enter to open with default application".encode_utf16().collect();
                                    DrawTextW(mem_dc, &mut prompt, &mut prompt_rect, windows::Win32::Graphics::Gdi::DT_CENTER | DT_SINGLELINE);
                                }
                            }

                            SelectObject(mem_dc, old_font);
                            let _ = DeleteObject(font.into());
                            let _ = DeleteObject(small_font.into());
                        }
                    });

                    let _ = BitBlt(hdc, 0, 0, w, h, Some(mem_dc), 0, 0, SRCCOPY);
                    SelectObject(mem_dc, old_bmp);
                    let _ = DeleteObject(mem_bmp.into());
                    let _ = DeleteDC(mem_dc);
                    let _ = EndPaint(hwnd, &ps);
                }
            }
            LRESULT(0)
        }
        WM_ERASEBKGND => LRESULT(1),
        WM_DESTROY => {
            CURRENT_HWND.with(|h| *h.borrow_mut() = None);
            CURRENT_PATH.with(|p| *p.borrow_mut() = None);
            CACHED_PREVIEW.with(|c| *c.borrow_mut() = None);
            LRESULT(0)
        }
        _ => unsafe { DefWindowProcW(hwnd, msg, wp, lp) },
    }
}
