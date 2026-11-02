//! Team crests: ESPN's 500 px PNG (cached by the football worker) scaled once with WIC to the
//! exact physical size it is drawn at, so it stays sharp (no bitmap stretching at draw time).

use std::path::Path;
use std::sync::Arc;

use windows::core::{Interface, HSTRING};
use windows::Win32::Foundation::GENERIC_READ;
use windows::Win32::Graphics::Imaging::{
    CLSID_WICImagingFactory, GUID_WICPixelFormat32bppBGRA, GUID_WICPixelFormat32bppPBGRA, IWICBitmapSource, IWICImagingFactory, WICBitmapDitherTypeNone,
    WICBitmapInterpolationModeHighQualityCubic, WICBitmapPaletteTypeCustom, WICDecodeMetadataCacheOnDemand,
};
use windows::Win32::System::Com::{CoCreateInstance, CLSCTX_INPROC_SERVER};

/// A crest (square) or a cover (any shape) as premultiplied BGRA.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Crest {
    pub px: Arc<Vec<u8>>,
    pub size: (u32, u32),
}

/// The picture at `path` scaled to cover `w`×`h` px and cut to it around its centre: box art in
/// the pill's circle or the card's box; `None` if the file is missing or unreadable.
pub fn load_cover(path: &Path, w: u32, h: u32) -> Option<Crest> {
    if w == 0 || h == 0 || !path.exists() {
        return None;
    }
    unsafe {
        let wic: IWICImagingFactory = CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).ok()?;
        let decoder = wic.CreateDecoderFromFilename(&HSTRING::from(path.as_os_str()), None, GENERIC_READ, WICDecodeMetadataCacheOnDemand).ok()?;
        let frame = decoder.GetFrame(0).ok()?;
        let (mut iw, mut ih) = (0u32, 0u32);
        frame.GetSize(&mut iw, &mut ih).ok()?;
        let fill = (w as f32 / iw.max(1) as f32).max(h as f32 / ih.max(1) as f32);
        let (sw, sh) = (((iw as f32 * fill).ceil() as u32).max(w), ((ih as f32 * fill).ceil() as u32).max(h));
        let scaler = wic.CreateBitmapScaler().ok()?;
        scaler.Initialize(&frame, sw, sh, WICBitmapInterpolationModeHighQualityCubic).ok()?;
        let converter = wic.CreateFormatConverter().ok()?;
        converter.Initialize(&scaler, &GUID_WICPixelFormat32bppPBGRA, WICBitmapDitherTypeNone, None, 0.0, WICBitmapPaletteTypeCustom).ok()?;
        let cut = windows::Win32::Graphics::Imaging::WICRect { X: ((sw - w) / 2) as i32, Y: ((sh - h) / 2) as i32, Width: w as i32, Height: h as i32 };
        let mut px = vec![0u8; (w * h * 4) as usize];
        converter.CopyPixels(&cut, w * 4, &mut px).ok()?;
        Some(Crest { px: Arc::new(px), size: (w, h) })
    }
}

/// The PNG at `path` scaled (high-quality cubic) to fit `side`×`side` px, centred; `None` if
/// the file is missing or unreadable.
pub fn load_crest(path: &Path, side: u32) -> Option<Crest> {
    if side == 0 || !path.exists() {
        return None;
    }
    unsafe {
        let wic: IWICImagingFactory = CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).ok()?;
        let decoder = wic.CreateDecoderFromFilename(&HSTRING::from(path.as_os_str()), None, GENERIC_READ, WICDecodeMetadataCacheOnDemand).ok()?;
        let frame = decoder.GetFrame(0).ok()?;
        let (mut w, mut h) = (0u32, 0u32);
        frame.GetSize(&mut w, &mut h).ok()?;
        square(&wic, &frame.cast().ok()?, (w, h), side)
    }
}

/// A PNG or JPEG in memory (an app's icon) fit into `side`×`side` px, centred; `None` if it
/// can't be read or is wider or taller than the API allows (its size is read before decoding).
pub fn from_bytes(bytes: &[u8], side: u32) -> Option<Crest> {
    use crate::activities::api::MAX_ICON_PX;
    if side == 0 {
        return None;
    }
    unsafe {
        let wic: IWICImagingFactory = CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).ok()?;
        let stream = wic.CreateStream().ok()?;
        stream.InitializeFromMemory(bytes).ok()?;
        let decoder = wic.CreateDecoderFromStream(&stream, std::ptr::null(), WICDecodeMetadataCacheOnDemand).ok()?;
        let frame = decoder.GetFrame(0).ok()?;
        let (mut w, mut h) = (0u32, 0u32);
        frame.GetSize(&mut w, &mut h).ok()?;
        if w == 0 || h == 0 || w > MAX_ICON_PX || h > MAX_ICON_PX {
            return None;
        }
        square(&wic, &frame.cast().ok()?, (w, h), side)
    }
}

/// A picture of any size (the clipboard's) fit into `side`×`side` px, with its own size.
pub fn thumb_from_bytes(bytes: &[u8], side: u32) -> Option<(Crest, u32, u32)> {
    if side == 0 {
        return None;
    }
    let decoded = unsafe {
        (|| -> Option<(Crest, u32, u32)> {
            let wic: IWICImagingFactory = CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).ok()?;
            let stream = wic.CreateStream().ok()?;
            stream.InitializeFromMemory(bytes).ok()?;
            let decoder = wic.CreateDecoderFromStream(&stream, std::ptr::null(), WICDecodeMetadataCacheOnDemand).ok()?;
            let frame = decoder.GetFrame(0).ok()?;
            let (mut w, mut h) = (0u32, 0u32);
            frame.GetSize(&mut w, &mut h).ok()?;
            if w == 0 || h == 0 || w > 32_000 || h > 32_000 {
                return None;
            }
            let crest = square(&wic, &frame.cast().ok()?, (w, h), side)?;
            Some((crest, w, h))
        })()
    };
    decoded.or_else(|| dib_thumb_from_bmp(bytes, side))
}

fn dib_thumb_from_bmp(bytes: &[u8], side: u32) -> Option<(Crest, u32, u32)> {
    let dib = if bytes.starts_with(b"BM") && bytes.len() > 54 { &bytes[14..] } else { bytes };
    if dib.len() < 40 {
        return None;
    }
    let header_size = u32::from_le_bytes(dib[0..4].try_into().ok()?) as usize;
    if header_size < 40 || dib.len() < header_size {
        return None;
    }
    let width = i32::from_le_bytes(dib[4..8].try_into().ok()?);
    let height = i32::from_le_bytes(dib[8..12].try_into().ok()?);
    let bpp = u16::from_le_bytes(dib[14..16].try_into().ok()?);
    let compression = u32::from_le_bytes(dib[16..20].try_into().ok()?);
    let w = width.unsigned_abs();
    let h = height.unsigned_abs();
    if w == 0 || h == 0 || w > 16_384 || h > 16_384 || (bpp != 32 && bpp != 24) {
        return None;
    }
    let masks_extra = if compression == 3 && header_size == 40 {
        if (dib.len() - header_size) >= (w * h * 4) as usize + 16 { 16 } else { 12 }
    } else {
        0
    };
    let pixel_offset = header_size + masks_extra;
    if dib.len() <= pixel_offset {
        return None;
    }
    let src = &dib[pixel_offset..];
    let row_bytes = ((w as usize * bpp as usize + 31) / 32) * 4;
    if src.len() < row_bytes * h as usize {
        return None;
    }
    let top_down = height < 0;
    let mut out = vec![0u8; (w * h * 4) as usize];
    for y in 0..h as usize {
        let src_y = if top_down { y } else { h as usize - 1 - y };
        let row = &src[src_y * row_bytes..src_y * row_bytes + row_bytes];
        let dst_row = &mut out[y * (w as usize * 4)..(y + 1) * (w as usize * 4)];
        if bpp == 32 {
            for x in 0..w as usize {
                let b = row[x * 4];
                let g = row[x * 4 + 1];
                let r = row[x * 4 + 2];
                let a = row[x * 4 + 3];
                let alpha = if a == 0 { 255 } else { a };
                dst_row[x * 4] = ((b as u32 * alpha as u32) / 255) as u8;
                dst_row[x * 4 + 1] = ((g as u32 * alpha as u32) / 255) as u8;
                dst_row[x * 4 + 2] = ((r as u32 * alpha as u32) / 255) as u8;
                dst_row[x * 4 + 3] = alpha;
            }
        } else {
            for x in 0..w as usize {
                dst_row[x * 4] = row[x * 3];
                dst_row[x * 4 + 1] = row[x * 3 + 1];
                dst_row[x * 4 + 2] = row[x * 3 + 2];
                dst_row[x * 4 + 3] = 255;
            }
        }
    }
    let crest = unsafe { fit_pixels(&out, (w, h), side)? };
    Some((crest, w, h))
}

/// A PNG (or any picture WIC reads) as a DIB: a 40-byte header and bottom-up 32-bit pixels,
/// transparent parts laid on white, for the programs that only take the classic format.
pub fn to_dib(bytes: &[u8]) -> Option<Vec<u8>> {
    unsafe {
        let wic: IWICImagingFactory = CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).ok()?;
        let stream = wic.CreateStream().ok()?;
        stream.InitializeFromMemory(bytes).ok()?;
        let decoder = wic.CreateDecoderFromStream(&stream, std::ptr::null(), WICDecodeMetadataCacheOnDemand).ok()?;
        let frame = decoder.GetFrame(0).ok()?;
        let (mut w, mut h) = (0u32, 0u32);
        frame.GetSize(&mut w, &mut h).ok()?;
        if w == 0 || h == 0 || w > 16_384 || h > 16_384 {
            return None;
        }
        let source: IWICBitmapSource = frame.cast().ok()?;
        let converter = wic.CreateFormatConverter().ok()?;
        converter.Initialize(&source, &GUID_WICPixelFormat32bppBGRA, WICBitmapDitherTypeNone, None, 0.0, WICBitmapPaletteTypeCustom).ok()?;
        let stride = w * 4;
        let mut px = vec![0u8; (stride * h) as usize];
        converter.CopyPixels(std::ptr::null(), stride, &mut px).ok()?;
        for p in px.chunks_exact_mut(4) {
            let a = p[3] as u32;
            if a < 255 {
                for c in &mut p[..3] {
                    *c = ((*c as u32 * a + 255 * (255 - a)) / 255) as u8;
                }
                p[3] = 255;
            }
        }
        let mut dib = Vec::with_capacity(40 + px.len());
        dib.extend_from_slice(&40u32.to_le_bytes());
        dib.extend_from_slice(&(w as i32).to_le_bytes());
        dib.extend_from_slice(&(h as i32).to_le_bytes());
        dib.extend_from_slice(&1u16.to_le_bytes());
        dib.extend_from_slice(&32u16.to_le_bytes());
        dib.extend_from_slice(&0u32.to_le_bytes());
        dib.extend_from_slice(&(px.len() as u32).to_le_bytes());
        dib.extend_from_slice(&[0u8; 16]);
        for row in (0..h as usize).rev() {
            dib.extend_from_slice(&px[row * stride as usize..(row + 1) * stride as usize]);
        }
        Some(dib)
    }
}

/// The exe's icon as Explorer shows it, `side`×`side` px; `None` without one.
pub fn exe_icon(path: &Path, side: u32) -> Option<Crest> {
    exe_icon_or_why(path, side).ok()
}

/// `exe_icon`, saying which step failed (and its HRESULT).
fn exe_icon_or_why(path: &Path, side: u32) -> Result<Crest, String> {
    use windows::Win32::Foundation::SIZE;
    use windows::Win32::Graphics::Gdi::DeleteObject;
    use windows::Win32::UI::Shell::{IShellItemImageFactory, SHCreateItemFromParsingName, SIIGBF_BIGGERSIZEOK, SIIGBF_ICONONLY};
    if side == 0 || !path.exists() {
        return Err("no such file".into());
    }
    unsafe {
        let factory: IShellItemImageFactory = SHCreateItemFromParsingName(&HSTRING::from(path.as_os_str()), None).map_err(|e| format!("item: {e}"))?;
        let bitmap = factory.GetImage(SIZE { cx: side as i32, cy: side as i32 }, SIIGBF_ICONONLY | SIIGBF_BIGGERSIZEOK).map_err(|e| format!("image: {e}"))?;
        let pixels = crate::activities::shelf_worker::bitmap_pixels(bitmap);
        let _ = DeleteObject(bitmap.into());
        let (px, w, h) = pixels.ok_or("pixels")?;
        fit_pixels(&px, (w, h), side).ok_or_else(|| "scale".into())
    }
}

/// `c` at exactly `side`×`side` px, the size it is drawn at (no stretching at draw time).
pub fn fit(c: &Crest, side: u32) -> Option<Crest> {
    if c.size == (side, side) {
        return Some(c.clone());
    }
    unsafe { fit_pixels(&c.px, c.size, side) }
}

unsafe fn fit_pixels(px: &[u8], (w, h): (u32, u32), side: u32) -> Option<Crest> {
    if side == 0 || w == 0 || h == 0 || px.len() < (w * h * 4) as usize {
        return None;
    }
    unsafe {
        let wic: IWICImagingFactory = CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).ok()?;
        let bitmap = wic.CreateBitmapFromMemory(w, h, &GUID_WICPixelFormat32bppPBGRA, w * 4, px).ok()?;
        square(&wic, &bitmap.cast().ok()?, (w, h), side)
    }
}

/// `source` (`w`×`h`) scaled (high-quality cubic) to fit `side`×`side` px, centred.
unsafe fn square(wic: &IWICImagingFactory, source: &IWICBitmapSource, (w, h): (u32, u32), side: u32) -> Option<Crest> {
    let fit = side as f32 / w.max(h).max(1) as f32;
    let (sw, sh) = (((w as f32 * fit).round() as u32).clamp(1, side), ((h as f32 * fit).round() as u32).clamp(1, side));
    unsafe {
        let scaler = wic.CreateBitmapScaler().ok()?;
        scaler.Initialize(source, sw, sh, WICBitmapInterpolationModeHighQualityCubic).ok()?;
        let converter = wic.CreateFormatConverter().ok()?;
        converter.Initialize(&scaler, &GUID_WICPixelFormat32bppPBGRA, WICBitmapDitherTypeNone, None, 0.0, WICBitmapPaletteTypeCustom).ok()?;
        let mut scaled = vec![0u8; (sw * sh * 4) as usize];
        converter.CopyPixels(std::ptr::null(), sw * 4, &mut scaled).ok()?;
        // Centre a non-square picture in the square.
        let mut px = vec![0u8; (side * side * 4) as usize];
        let (ox, oy) = ((side - sw) / 2, (side - sh) / 2);
        for y in 0..sh {
            let from = (y * sw * 4) as usize;
            let to = (((y + oy) * side + ox) * 4) as usize;
            px[to..to + (sw * 4) as usize].copy_from_slice(&scaled[from..from + (sw * 4) as usize]);
        }
        Some(Crest { px: Arc::new(px), size: (side, side) })
    }
}

/// Writes a `w`×`h` BGRA PNG to `path`, `pixel(x, y)` for each pixel (tests only).
#[cfg(test)]
pub(crate) fn write_test_png(path: &Path, w: u32, h: u32, pixel: impl Fn(u32, u32) -> [u8; 4]) {
    use windows::Win32::Graphics::Imaging::*;
    use windows::Win32::System::Com::{CoInitializeEx, COINIT_MULTITHREADED};
    unsafe {
        let _ = CoInitializeEx(None, COINIT_MULTITHREADED);
        let wic: IWICImagingFactory = CoCreateInstance(&CLSID_WICImagingFactory, None, CLSCTX_INPROC_SERVER).unwrap();
        let mut px = vec![0u8; (w * h * 4) as usize];
        for y in 0..h {
            for x in 0..w {
                let i = ((y * w + x) * 4) as usize;
                px[i..i + 4].copy_from_slice(&pixel(x, y));
            }
        }
        let stream = wic.CreateStream().unwrap();
        stream.InitializeFromFilename(&HSTRING::from(path.as_os_str()), windows::Win32::Foundation::GENERIC_WRITE.0).unwrap();
        let encoder = wic.CreateEncoder(&GUID_ContainerFormatPng, std::ptr::null()).unwrap();
        encoder.Initialize(&stream, WICBitmapEncoderNoCache).unwrap();
        let mut frame = None;
        encoder.CreateNewFrame(&mut frame, std::ptr::null_mut()).unwrap();
        let frame = frame.unwrap();
        frame.Initialize(None).unwrap();
        frame.SetSize(w, h).unwrap();
        let mut format = GUID_WICPixelFormat32bppBGRA;
        frame.SetPixelFormat(&mut format).unwrap();
        frame.WritePixels(h, w * 4, &px).unwrap();
        frame.Commit().unwrap();
        encoder.Commit().unwrap();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A 500×500 PNG: transparent, with an opaque red disc in the middle (like a crest).
    fn write_png(path: &std::path::Path) {
        write_test_png(path, 500, 500, |x, y| {
            let (dx, dy) = (x as i32 - 250, y as i32 - 250);
            if dx * dx + dy * dy < 200 * 200 { [0, 0, 255, 255] } else { [0; 4] }
        });
    }

    fn test_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("nowbar-crest-{name}-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        dir
    }

    #[test]
    fn bytes_decode_to_a_centred_square() {
        let path = test_dir("bytes").join("wide.png");
        // 40×20, opaque red: fit into 32 it is 32×16, centred top to bottom.
        write_test_png(&path, 40, 20, |_, _| [0, 0, 255, 255]);
        let icon = from_bytes(&std::fs::read(&path).unwrap(), 32).expect("decoded");
        assert_eq!(icon.size, (32, 32));
        let at = |x: usize, y: usize| icon.px[(y * 32 + x) * 4..(y * 32 + x) * 4 + 4].to_vec();
        assert_eq!(at(16, 0)[3], 0, "above the picture: transparent");
        assert_eq!(at(16, 16), vec![0, 0, 255, 255], "the middle: the picture");
        assert!(from_bytes(b"not an image", 32).is_none());
    }

    #[test]
    fn oversized_images_are_refused() {
        let path = test_dir("big").join("big.png");
        write_test_png(&path, 600, 600, |_, _| [0, 255, 0, 255]);
        assert!(from_bytes(&std::fs::read(&path).unwrap(), 32).is_none(), "over 512 px: not decoded");
    }

    #[test]
    fn an_exe_has_its_icon() {
        let explorer = std::path::PathBuf::from(std::env::var("WINDIR").unwrap_or_else(|_| r"C:\Windows".into())).join("explorer.exe");
        unsafe {
            let _ = windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_MULTITHREADED);
        }
        // Failed once (only the first run after a rebuild, then 0 in 26): the step and HRESULT tell.
        let icon = exe_icon_or_why(&explorer, 48).unwrap_or_else(|why| panic!("explorer has an icon: {why}"));
        assert_eq!(icon.size, (48, 48));
        assert!(icon.px.chunks(4).any(|p| p[3] == 255), "something opaque in it");
        assert!(exe_icon(std::path::Path::new(r"C:\yok\yok.exe"), 48).is_none());
    }

    #[test]
    fn fit_scales_exactly() {
        unsafe {
            let _ = windows::Win32::System::Com::CoInitializeEx(None, windows::Win32::System::Com::COINIT_MULTITHREADED);
        }
        let big = Crest { px: Arc::new([0u8, 0, 255, 255].repeat(128 * 128)), size: (128, 128) };
        let small = fit(&big, 22).expect("scaled");
        assert_eq!((small.size, small.px.len()), ((22, 22), 22 * 22 * 4));
        assert_eq!(&small.px[(11 * 22 + 11) * 4..(11 * 22 + 11) * 4 + 4], &[0, 0, 255, 255]);
    }

    #[test]
    fn crest_scaled_to_exact_pixels_keeps_alpha() {
        let dir = std::env::temp_dir().join(format!("nowbar-crest-test-{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("436.png");
        write_png(&path);
        let crest = load_crest(&path, 25).expect("decoded");
        assert_eq!(crest.size, (25, 25));
        assert_eq!(crest.px.len(), 25 * 25 * 4);
        let at = |x: usize, y: usize| &crest.px[(y * 25 + x) * 4..(y * 25 + x) * 4 + 4];
        assert_eq!(at(0, 0)[3], 0, "corner stays transparent");
        let centre = at(12, 12);
        assert!(centre[3] == 255 && centre[2] > 240 && centre[0] < 15, "centre is opaque red: {centre:?}");
        assert!(load_crest(&dir.join("missing.png"), 25).is_none());
    }
}
