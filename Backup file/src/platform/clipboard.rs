//! Text onto the clipboard (the shelf's "Metni kopyala", the question card's answer) and off it.

use windows::core::Result;
use windows::Win32::Foundation::{GlobalFree, HANDLE, HGLOBAL, HWND};
use windows::Win32::System::DataExchange::{
    AddClipboardFormatListener, CloseClipboard, EmptyClipboard, GetClipboardData, IsClipboardFormatAvailable, OpenClipboard,
    RegisterClipboardFormatW, RemoveClipboardFormatListener, SetClipboardData,
};
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalSize, GlobalUnlock, GMEM_MOVEABLE};

pub const CF_BITMAP: u32 = 2;
pub const CF_DIB: u32 = 8;
pub const UNICODE_TEXT: u32 = 13;
pub const CF_HDROP: u32 = 15;
pub const CF_DIBV5: u32 = 17;

pub fn html_format() -> u32 {
    unsafe { RegisterClipboardFormatW(windows::core::w!("HTML Format")) }
}

pub fn png_format() -> u32 {
    unsafe { RegisterClipboardFormatW(windows::core::w!("PNG")) }
}

pub fn listen(hwnd: HWND) -> bool {
    unsafe { AddClipboardFormatListener(hwnd).is_ok() }
}

pub fn unlisten(hwnd: HWND) -> bool {
    unsafe { RemoveClipboardFormatListener(hwnd).is_ok() }
}

/// Opens the clipboard, waiting a little while another program (a clipboard manager) holds it.
fn open(owner: HWND) -> Result<()> {
    let mut tries = 0;
    loop {
        match unsafe { OpenClipboard(Some(owner)) } {
            Err(_) if tries < 12 => {
                tries += 1;
                std::thread::sleep(std::time::Duration::from_millis(15));
            }
            other => return other,
        }
    }
}

/// The clipboard's Unicode text, if it holds some (the question card's Ctrl+V).
pub fn get_text(owner: HWND) -> Option<String> {
    unsafe {
        open(owner).ok()?;
        let read = || -> Option<String> {
            let memory = HGLOBAL(GetClipboardData(UNICODE_TEXT).ok()?.0);
            let units = GlobalSize(memory) / 2;
            let at = GlobalLock(memory) as *const u16;
            if at.is_null() {
                return None;
            }
            let all = std::slice::from_raw_parts(at, units);
            let text = String::from_utf16_lossy(&all[..all.iter().position(|u| *u == 0).unwrap_or(units)]);
            let _ = GlobalUnlock(memory);
            Some(text)
        };
        let text = read();
        let _ = CloseClipboard();
        text
    }
}

/// Puts `text` on the clipboard as Unicode text, owned by `owner`.
pub fn set_text(owner: HWND, text: &str) -> Result<()> {
    set_text_and_html(owner, text, None)
}

#[derive(Clone, Debug)]
pub enum RawClipboardContent {
    Text { text: String, html: Option<String> },
    Image { raw_png: Option<Vec<u8>>, raw_dib: Option<Vec<u8>>, bmp_bytes: Option<Vec<u8>>, width: u32, height: u32 },
}

/// Inspects the current clipboard content without taking ownership.
pub fn read_snapshot(owner: HWND) -> Option<RawClipboardContent> {
    unsafe {
        open(owner).ok()?;
        let res = (|| -> Option<RawClipboardContent> {
            let png_fmt = png_format();
            if png_fmt != 0 && IsClipboardFormatAvailable(png_fmt).is_ok() {
                if let Ok(handle) = GetClipboardData(png_fmt) {
                    let hmem = HGLOBAL(handle.0);
                    let size = GlobalSize(hmem);
                    let ptr = GlobalLock(hmem) as *const u8;
                    if !ptr.is_null() && size > 8 {
                        let bytes = std::slice::from_raw_parts(ptr, size).to_vec();
                        let _ = GlobalUnlock(hmem);
                        return Some(RawClipboardContent::Image {
                            raw_png: Some(bytes.clone()),
                            raw_dib: None,
                            bmp_bytes: Some(bytes),
                            width: 0,
                            height: 0,
                        });
                    }
                    let _ = GlobalUnlock(hmem);
                }
            }

            if IsClipboardFormatAvailable(CF_DIB).is_ok() || IsClipboardFormatAvailable(CF_DIBV5).is_ok() {
                let fmt = if IsClipboardFormatAvailable(CF_DIB).is_ok() { CF_DIB } else { CF_DIBV5 };
                if let Ok(handle) = GetClipboardData(fmt) {
                    let hmem = HGLOBAL(handle.0);
                    let size = GlobalSize(hmem);
                    let ptr = GlobalLock(hmem) as *const u8;
                    if !ptr.is_null() && size >= 40 {
                        let dib_slice = std::slice::from_raw_parts(ptr, size);
                        let header_size = u32::from_le_bytes(dib_slice[0..4].try_into().unwrap_or_default());
                        let width = i32::from_le_bytes(dib_slice[4..8].try_into().unwrap_or_default()).abs() as u32;
                        let height = i32::from_le_bytes(dib_slice[8..12].try_into().unwrap_or_default()).abs() as u32;
                        let bit_count = u16::from_le_bytes(dib_slice[14..16].try_into().unwrap_or_default());
                        let compression = u32::from_le_bytes(dib_slice[16..20].try_into().unwrap_or_default());
                        let clr_used = u32::from_le_bytes(dib_slice[32..36].try_into().unwrap_or_default());

                        let palette_colors = if clr_used != 0 {
                            clr_used
                        } else if bit_count <= 8 {
                            1 << bit_count
                        } else {
                            0
                        };
                        let masks_size = if compression == 3 && header_size == 40 { 12 } else { 0 };
                        let off_bits = 14 + header_size + palette_colors * 4 + masks_size;

                        let mut bmp = Vec::with_capacity(14 + size);
                        bmp.extend_from_slice(b"BM");
                        bmp.extend_from_slice(&((14 + size) as u32).to_le_bytes());
                        bmp.extend_from_slice(&[0u8; 4]);
                        bmp.extend_from_slice(&(off_bits as u32).to_le_bytes());
                        bmp.extend_from_slice(dib_slice);

                        let raw_dib = dib_slice.to_vec();
                        let _ = GlobalUnlock(hmem);

                        return Some(RawClipboardContent::Image {
                            raw_png: None,
                            raw_dib: Some(raw_dib),
                            bmp_bytes: Some(bmp),
                            width,
                            height,
                        });
                    }
                    let _ = GlobalUnlock(hmem);
                }
            }

            if IsClipboardFormatAvailable(UNICODE_TEXT).is_ok() {
                if let Ok(handle) = GetClipboardData(UNICODE_TEXT) {
                    let hmem = HGLOBAL(handle.0);
                    let units = GlobalSize(hmem) / 2;
                    let ptr = GlobalLock(hmem) as *const u16;
                    let text = if !ptr.is_null() && units > 0 {
                        let all = std::slice::from_raw_parts(ptr, units);
                        let end = all.iter().position(|u| *u == 0).unwrap_or(units);
                        Some(String::from_utf16_lossy(&all[..end]))
                    } else {
                        None
                    };
                    let _ = GlobalUnlock(hmem);

                    if let Some(text) = text {
                        if !text.is_empty() {
                            let html_fmt = html_format();
                            let html = if html_fmt != 0 && IsClipboardFormatAvailable(html_fmt).is_ok() {
                                if let Ok(h_handle) = GetClipboardData(html_fmt) {
                                    let h_hmem = HGLOBAL(h_handle.0);
                                    let h_size = GlobalSize(h_hmem);
                                    let h_ptr = GlobalLock(h_hmem) as *const u8;
                                    let h_str = if !h_ptr.is_null() && h_size > 0 {
                                        let h_slice = std::slice::from_raw_parts(h_ptr, h_size);
                                        let h_end = h_slice.iter().position(|b| *b == 0).unwrap_or(h_size);
                                        Some(String::from_utf8_lossy(&h_slice[..h_end]).to_string())
                                    } else {
                                        None
                                    };
                                    let _ = GlobalUnlock(h_hmem);
                                    h_str
                                } else {
                                    None
                                }
                            } else {
                                None
                            };

                            return Some(RawClipboardContent::Text { text, html });
                        }
                    }
                }
            }

            None
        })();
        let _ = CloseClipboard();
        res
    }
}

/// Puts `text` (and optional `html`) on the clipboard, owned by `owner`.
pub fn set_text_and_html(owner: HWND, text: &str, html: Option<&str>) -> Result<()> {
    let utf16: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
    unsafe {
        let text_mem = GlobalAlloc(GMEM_MOVEABLE, utf16.len() * 2)?;
        let to = GlobalLock(text_mem) as *mut u16;
        if to.is_null() {
            let _ = GlobalFree(Some(text_mem));
            return Err(windows::core::Error::from_thread());
        }
        std::ptr::copy_nonoverlapping(utf16.as_ptr(), to, utf16.len());
        let _ = GlobalUnlock(text_mem);

        let html_mem_and_fmt = if let Some(html) = html {
            let html_fmt = html_format();
            if html_fmt != 0 {
                let html_bytes = html.as_bytes();
                if let Ok(mem) = GlobalAlloc(GMEM_MOVEABLE, html_bytes.len() + 1) {
                    let h_to = GlobalLock(mem) as *mut u8;
                    if !h_to.is_null() {
                        std::ptr::copy_nonoverlapping(html_bytes.as_ptr(), h_to, html_bytes.len());
                        *h_to.add(html_bytes.len()) = 0;
                        let _ = GlobalUnlock(mem);
                        Some((mem, html_fmt))
                    } else {
                        let _ = GlobalFree(Some(mem));
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        if let Err(e) = open(owner) {
            let _ = GlobalFree(Some(text_mem));
            if let Some((mem, _)) = html_mem_and_fmt {
                let _ = GlobalFree(Some(mem));
            }
            return Err(e);
        }
        let _ = EmptyClipboard();
        let _ = SetClipboardData(UNICODE_TEXT, Some(HANDLE(text_mem.0)));
        if let Some((mem, fmt)) = html_mem_and_fmt {
            let _ = SetClipboardData(fmt, Some(HANDLE(mem.0)));
        }
        let _ = CloseClipboard();
    }
    Ok(())
}

/// Puts image (raw PNG and/or DIB) on the clipboard, owned by `owner`.
pub fn set_image(owner: HWND, raw_png: Option<&[u8]>, raw_dib: Option<&[u8]>) -> Result<()> {
    unsafe {
        let png_mem_and_fmt = if let Some(png) = raw_png {
            let fmt = png_format();
            if fmt != 0 {
                if let Ok(mem) = GlobalAlloc(GMEM_MOVEABLE, png.len()) {
                    let to = GlobalLock(mem) as *mut u8;
                    if !to.is_null() {
                        std::ptr::copy_nonoverlapping(png.as_ptr(), to, png.len());
                        let _ = GlobalUnlock(mem);
                        Some((mem, fmt))
                    } else {
                        let _ = GlobalFree(Some(mem));
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        let dib_mem = if let Some(dib) = raw_dib {
            if let Ok(mem) = GlobalAlloc(GMEM_MOVEABLE, dib.len()) {
                let to = GlobalLock(mem) as *mut u8;
                if !to.is_null() {
                    std::ptr::copy_nonoverlapping(dib.as_ptr(), to, dib.len());
                    let _ = GlobalUnlock(mem);
                    Some(mem)
                } else {
                    let _ = GlobalFree(Some(mem));
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        if dib_mem.is_none() && png_mem_and_fmt.is_none() {
            return Ok(());
        }

        if let Err(e) = open(owner) {
            if let Some((mem, _)) = png_mem_and_fmt {
                let _ = GlobalFree(Some(mem));
            }
            if let Some(mem) = dib_mem {
                let _ = GlobalFree(Some(mem));
            }
            return Err(e);
        }

        // A DIB with a bigger header (V5) goes under its own format.
        let dib_fmt = match raw_dib {
            Some(d) if d.len() >= 4 && u32::from_le_bytes([d[0], d[1], d[2], d[3]]) >= 108 => CF_DIBV5,
            _ => CF_DIB,
        };
        let _ = EmptyClipboard();
        if let Some((mem, fmt)) = png_mem_and_fmt {
            let _ = SetClipboardData(fmt, Some(HANDLE(mem.0)));
        }
        if let Some(mem) = dib_mem {
            let _ = SetClipboardData(dib_fmt, Some(HANDLE(mem.0)));
        }
        let _ = CloseClipboard();
    }
    Ok(())
}
