//! Files dragged onto the pill or the card (the shelf), and dragged back out.
//! Copy only, both ways: a `MOVE` would let the other side delete the original.

use std::cell::{Cell, RefCell};
use std::ffi::OsString;
use std::mem::ManuallyDrop;
use std::os::windows::ffi::{OsStrExt, OsStringExt};
use std::path::PathBuf;

use windows::core::{implement, Ref, Result, BOOL, HRESULT};
use windows::Win32::Foundation::{
    GlobalFree, DATA_S_SAMEFORMATETC, DV_E_FORMATETC, E_NOTIMPL, E_POINTER, HWND, OLE_E_ADVISENOTSUPPORTED, POINT, POINTL, S_FALSE, S_OK,
};
use windows::Win32::System::Com::{
    IAdviseSink, IDataObject, IDataObject_Impl, IEnumFORMATETC, IEnumFORMATETC_Impl, IEnumSTATDATA, DVASPECT_CONTENT, FORMATETC, STGMEDIUM, STGMEDIUM_0,
    TYMED_HGLOBAL,
};
use windows::Win32::System::DataExchange::RegisterClipboardFormatW;
use windows::Win32::System::Memory::{GlobalAlloc, GlobalLock, GlobalUnlock, GMEM_MOVEABLE};
use windows::Win32::System::Ole::{
    DoDragDrop, IDropSource, IDropSource_Impl, IDropTarget, IDropTarget_Impl, OleInitialize, RegisterDragDrop, ReleaseStgMedium, RevokeDragDrop,
    CF_HDROP, DROPEFFECT, DROPEFFECT_COPY, DROPEFFECT_NONE,
};
use windows::Win32::System::SystemServices::{MK_LBUTTON, MODIFIERKEYS_FLAGS};
use windows::Win32::UI::Input::KeyboardAndMouse::ReleaseCapture;
use windows::Win32::UI::Shell::{DragQueryFileW, DROPFILES, HDROP};
use windows::Win32::UI::WindowsAndMessaging::{GetSystemMetrics, SM_CXDRAG, SM_CYDRAG};

use super::windows::{push_event, UiEvent};

const DRAGDROP_S_DROP: HRESULT = HRESULT(0x00040100);
const DRAGDROP_S_CANCEL: HRESULT = HRESULT(0x00040101);
const DRAGDROP_S_USEDEFAULTCURSORS: HRESULT = HRESULT(0x00040102);

/// Which window a drag is over.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DropZone {
    Pill,
    Card,
}

thread_local! {
    static HOVER: Cell<Option<DropZone>> = const { Cell::new(None) };
    static DROPPED: RefCell<Vec<PathBuf>> = const { RefCell::new(Vec::new()) };
    /// A drag out of the shelf is running: its files landing on the pill or card are ours.
    static OUTGOING: Cell<bool> = const { Cell::new(false) };
}

/// OLE on the UI thread (already an STA, so this only adds OLE's drag and drop).
pub fn init() -> Result<()> {
    unsafe { OleInitialize(None) }
}

/// Makes `hwnd` take file drops; its zone is what `hover()` reports.
pub fn register(hwnd: HWND, zone: DropZone) -> Result<()> {
    let target: IDropTarget = DropTarget { zone, accepts: Cell::new(false) }.into();
    unsafe { RegisterDragDrop(hwnd, &target) }
}

/// Lets go of `hwnd`'s drop target before the window goes (a no-op if it never had one).
pub fn revoke(hwnd: HWND) {
    let _ = unsafe { RevokeDragDrop(hwnd) };
}

/// Where an accepted file drag is right now.
pub fn hover() -> Option<DropZone> {
    HOVER.with(|h| h.get())
}

/// The paths dropped since the last call, in drop order.
pub fn take_dropped() -> Vec<PathBuf> {
    DROPPED.with(|d| std::mem::take(&mut *d.borrow_mut()))
}

/// Past the system's drag distance on either axis (the mouse went down `dx`, `dy` px ago).
pub fn drag_moved(dx: i32, dy: i32) -> bool {
    let (cx, cy) = unsafe { (GetSystemMetrics(SM_CXDRAG), GetSystemMetrics(SM_CYDRAG)) };
    dx.abs() >= cx || dy.abs() >= cy
}

fn preferred_drop_effect_format() -> u16 {
    static FMT: std::sync::OnceLock<u16> = std::sync::OnceLock::new();
    *FMT.get_or_init(|| unsafe { RegisterClipboardFormatW(windows::core::w!("Preferred DropEffect")) as u16 })
}

fn create_hdrop(paths: &[PathBuf]) -> Result<windows::Win32::Foundation::HGLOBAL> {
    let mut total_chars = 0;
    let mut wide_paths: Vec<Vec<u16>> = Vec::new();
    for p in paths {
        let mut wide: Vec<u16> = p.as_os_str().encode_wide().collect();
        wide.push(0);
        total_chars += wide.len();
        wide_paths.push(wide);
    }
    total_chars += 1;

    let header_size = std::mem::size_of::<DROPFILES>();
    let total_bytes = header_size + total_chars * std::mem::size_of::<u16>();

    unsafe {
        let mem = GlobalAlloc(GMEM_MOVEABLE, total_bytes)?;
        let ptr = GlobalLock(mem) as *mut u8;
        if ptr.is_null() {
            let _ = GlobalFree(Some(mem));
            return Err(windows::core::Error::empty());
        }

        let df = ptr as *mut DROPFILES;
        (*df).pFiles = header_size as u32;
        (*df).pt = POINT { x: 0, y: 0 };
        (*df).fNC = false.into();
        (*df).fWide = true.into();

        let mut dest = ptr.add(header_size) as *mut u16;
        for wide in wide_paths {
            std::ptr::copy_nonoverlapping(wide.as_ptr(), dest, wide.len());
            dest = dest.add(wide.len());
        }
        *dest = 0;

        let _ = GlobalUnlock(mem);
        Ok(mem)
    }
}

#[implement(IEnumFORMATETC)]
struct FormatEnumerator {
    formats: Vec<FORMATETC>,
    index: Cell<usize>,
}

impl IEnumFORMATETC_Impl for FormatEnumerator_Impl {
    fn Next(&self, celt: u32, rgelt: *mut FORMATETC, pcelt_fetched: *mut u32) -> HRESULT {
        if rgelt.is_null() {
            return E_POINTER;
        }
        let cur = self.index.get();
        let remaining = self.formats.len().saturating_sub(cur);
        let count = (celt as usize).min(remaining);
        for i in 0..count {
            unsafe {
                *rgelt.add(i) = self.formats[cur + i];
            }
        }
        self.index.set(cur + count);
        if !pcelt_fetched.is_null() {
            unsafe {
                *pcelt_fetched = count as u32;
            }
        }
        if count == celt as usize {
            S_OK
        } else {
            S_FALSE
        }
    }

    fn Skip(&self, celt: u32) -> Result<()> {
        let cur = self.index.get();
        let next = (cur + celt as usize).min(self.formats.len());
        self.index.set(next);
        Ok(())
    }

    fn Reset(&self) -> Result<()> {
        self.index.set(0);
        Ok(())
    }

    fn Clone(&self) -> Result<IEnumFORMATETC> {
        let cloned = FormatEnumerator {
            formats: self.formats.clone(),
            index: Cell::new(self.index.get()),
        };
        Ok(cloned.into())
    }
}

#[implement(IDataObject)]
struct FileDataObject {
    paths: Vec<PathBuf>,
}

impl IDataObject_Impl for FileDataObject_Impl {
    fn GetData(&self, pformatetc_in: *const FORMATETC) -> Result<STGMEDIUM> {
        let Some(fmt) = (unsafe { pformatetc_in.as_ref() }) else {
            return Err(windows::core::Error::from(DV_E_FORMATETC));
        };
        if fmt.cfFormat == CF_HDROP.0 && (fmt.tymed & TYMED_HGLOBAL.0 as u32 != 0) {
            let hmem = create_hdrop(&self.paths)?;
            return Ok(STGMEDIUM {
                tymed: TYMED_HGLOBAL.0 as u32,
                u: STGMEDIUM_0 { hGlobal: hmem },
                pUnkForRelease: ManuallyDrop::new(None),
            });
        }
        if fmt.cfFormat == preferred_drop_effect_format() && (fmt.tymed & TYMED_HGLOBAL.0 as u32 != 0) {
            let hmem = unsafe {
                let mem = GlobalAlloc(GMEM_MOVEABLE, 4)?;
                let ptr = GlobalLock(mem) as *mut u32;
                if ptr.is_null() {
                    let _ = GlobalFree(Some(mem));
                    return Err(windows::core::Error::empty());
                }
                *ptr = DROPEFFECT_COPY.0;
                let _ = GlobalUnlock(mem);
                mem
            };
            return Ok(STGMEDIUM {
                tymed: TYMED_HGLOBAL.0 as u32,
                u: STGMEDIUM_0 { hGlobal: hmem },
                pUnkForRelease: ManuallyDrop::new(None),
            });
        }
        Err(windows::core::Error::from(DV_E_FORMATETC))
    }

    fn GetDataHere(&self, _: *const FORMATETC, _: *mut STGMEDIUM) -> Result<()> {
        Err(windows::core::Error::from(E_NOTIMPL))
    }

    fn QueryGetData(&self, pformatetc: *const FORMATETC) -> HRESULT {
        let Some(fmt) = (unsafe { pformatetc.as_ref() }) else {
            return DV_E_FORMATETC;
        };
        if fmt.cfFormat == CF_HDROP.0 && (fmt.tymed & TYMED_HGLOBAL.0 as u32 != 0) {
            return S_OK;
        }
        if fmt.cfFormat == preferred_drop_effect_format() && (fmt.tymed & TYMED_HGLOBAL.0 as u32 != 0) {
            return S_OK;
        }
        DV_E_FORMATETC
    }

    fn GetCanonicalFormatEtc(&self, _: *const FORMATETC, _: *mut FORMATETC) -> HRESULT {
        DATA_S_SAMEFORMATETC
    }

    fn SetData(&self, _: *const FORMATETC, _: *const STGMEDIUM, _: BOOL) -> Result<()> {
        Err(windows::core::Error::from(E_NOTIMPL))
    }

    fn EnumFormatEtc(&self, dw_direction: u32) -> Result<IEnumFORMATETC> {
        // DATADIR_GET = 1
        if dw_direction == 1 {
            let formats = vec![
                FORMATETC {
                    cfFormat: CF_HDROP.0,
                    ptd: std::ptr::null_mut(),
                    dwAspect: DVASPECT_CONTENT.0,
                    lindex: -1,
                    tymed: TYMED_HGLOBAL.0 as u32,
                },
                FORMATETC {
                    cfFormat: preferred_drop_effect_format(),
                    ptd: std::ptr::null_mut(),
                    dwAspect: DVASPECT_CONTENT.0,
                    lindex: -1,
                    tymed: TYMED_HGLOBAL.0 as u32,
                },
            ];
            let enumerator = FormatEnumerator {
                formats,
                index: Cell::new(0),
            };
            Ok(enumerator.into())
        } else {
            Err(windows::core::Error::from(E_NOTIMPL))
        }
    }

    fn DAdvise(&self, _: *const FORMATETC, _: u32, _: windows::core::Ref<'_, IAdviseSink>) -> Result<u32> {
        Err(windows::core::Error::from(OLE_E_ADVISENOTSUPPORTED))
    }

    fn DUnadvise(&self, _: u32) -> Result<()> {
        Err(windows::core::Error::from(OLE_E_ADVISENOTSUPPORTED))
    }

    fn EnumDAdvise(&self) -> Result<IEnumSTATDATA> {
        Err(windows::core::Error::from(OLE_E_ADVISENOTSUPPORTED))
    }
}

#[implement(IDropSource)]
struct FileDropSource;

impl IDropSource_Impl for FileDropSource_Impl {
    fn QueryContinueDrag(&self, f_escape_pressed: BOOL, grf_key_state: MODIFIERKEYS_FLAGS) -> HRESULT {
        if f_escape_pressed.as_bool() {
            return DRAGDROP_S_CANCEL;
        }
        if (grf_key_state.0 & MK_LBUTTON.0) == 0 {
            return DRAGDROP_S_DROP;
        }
        S_OK
    }

    fn GiveFeedback(&self, _dw_effect: DROPEFFECT) -> HRESULT {
        DRAGDROP_S_USEDEFAULTCURSORS
    }
}

/// Drags `paths` out of the shelf, copy only; returns once they are dropped somewhere (or the
/// drag is cancelled). Paths that do not exist are filtered out.
pub fn drag_out(paths: &[PathBuf]) -> Result<()> {
    if paths.is_empty() {
        return Ok(());
    }
    let valid_paths: Vec<PathBuf> = paths.iter().filter(|p| p.exists()).cloned().collect();
    if valid_paths.is_empty() {
        return Ok(());
    }
    super::log::log(&format!("drag_out: starting drag with {} path(s)", valid_paths.len()));
    unsafe {
        let _ = ReleaseCapture();
        let data: IDataObject = FileDataObject { paths: valid_paths.clone() }.into();
        let source: IDropSource = FileDropSource.into();
        OUTGOING.with(|o| o.set(true));
        let mut effect = DROPEFFECT(0);
        let outcome = DoDragDrop(&data, &source, DROPEFFECT_COPY, &mut effect);
        OUTGOING.with(|o| o.set(false));
        if outcome.is_ok() {
            super::log::log(&format!("drag_out: success, effect {}", effect.0));
        } else {
            super::log::log(&format!("drag_out: DoDragDrop ended with {outcome:?}"));
        }
        outcome.ok()
    }
}

/// What a drag over the pill or card gets: a copy of files from elsewhere, offered as a copy;
/// nothing for text, a move-only drag or the shelf's own drag coming back (no hint, no drop).
pub fn drop_effect(has_files: bool, offered: DROPEFFECT, outgoing: bool) -> DROPEFFECT {
    if has_files && !outgoing {
        allowed_effect(offered)
    } else {
        DROPEFFECT_NONE
    }
}

/// Copy when the source offers it, otherwise nothing: never a move.
pub fn allowed_effect(offered: DROPEFFECT) -> DROPEFFECT {
    if offered.0 & DROPEFFECT_COPY.0 != 0 {
        DROPEFFECT_COPY
    } else {
        DROPEFFECT_NONE
    }
}

fn hdrop_format() -> FORMATETC {
    FORMATETC { cfFormat: CF_HDROP.0, ptd: std::ptr::null_mut(), dwAspect: DVASPECT_CONTENT.0, lindex: -1, tymed: TYMED_HGLOBAL.0 as u32 }
}

fn paths_of(data: &IDataObject) -> Vec<PathBuf> {
    unsafe {
        let Ok(mut medium) = data.GetData(&hdrop_format()) else { return Vec::new() };
        let drop = HDROP(medium.u.hGlobal.0);
        let count = DragQueryFileW(drop, u32::MAX, None);
        let paths = (0..count)
            .map(|i| {
                let mut buf = vec![0u16; DragQueryFileW(drop, i, None) as usize + 1];
                let n = DragQueryFileW(drop, i, Some(&mut buf)) as usize;
                PathBuf::from(OsString::from_wide(&buf[..n]))
            })
            .collect();
        ReleaseStgMedium(&mut medium);
        paths
    }
}

fn set_hover(zone: Option<DropZone>) {
    if HOVER.with(|h| h.replace(zone)) != zone {
        push_event(UiEvent::DropHover);
    }
}

#[implement(IDropTarget)]
struct DropTarget {
    zone: DropZone,
    /// The drag carries files (text and bare images are refused).
    accepts: Cell<bool>,
}

impl DropTarget {
    /// Answers the drag with `drop_effect`; true if it is taken.
    fn answer(&self, effect: *mut DROPEFFECT) -> bool {
        let Some(e) = (unsafe { effect.as_mut() }) else { return false };
        *e = drop_effect(self.accepts.get(), *e, OUTGOING.with(|o| o.get()));
        *e != DROPEFFECT_NONE
    }
}

impl IDropTarget_Impl for DropTarget_Impl {
    fn DragEnter(&self, data: Ref<IDataObject>, _: MODIFIERKEYS_FLAGS, _: &POINTL, effect: *mut DROPEFFECT) -> Result<()> {
        let files = data.as_ref().is_some_and(|d| unsafe { d.QueryGetData(&hdrop_format()) }.is_ok());
        self.accepts.set(files);
        if self.answer(effect) {
            set_hover(Some(self.zone));
        }
        Ok(())
    }

    fn DragOver(&self, _: MODIFIERKEYS_FLAGS, _: &POINTL, effect: *mut DROPEFFECT) -> Result<()> {
        self.answer(effect);
        Ok(())
    }

    fn DragLeave(&self) -> Result<()> {
        set_hover(None);
        Ok(())
    }

    fn Drop(&self, data: Ref<IDataObject>, _: MODIFIERKEYS_FLAGS, _: &POINTL, effect: *mut DROPEFFECT) -> Result<()> {
        set_hover(None);
        if let (true, Some(d)) = (self.answer(effect), data.as_ref()) {
            let paths = paths_of(d);
            if !paths.is_empty() {
                DROPPED.with(|q| q.borrow_mut().extend(paths));
                push_event(UiEvent::Dropped);
            }
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use windows::Win32::System::Ole::{DROPEFFECT_LINK, DROPEFFECT_MOVE};

    #[test]
    fn allowed_effect_never_moves() {
        assert_eq!(allowed_effect(DROPEFFECT_COPY | DROPEFFECT_MOVE), DROPEFFECT_COPY);
        assert_eq!(allowed_effect(DROPEFFECT_COPY | DROPEFFECT_LINK), DROPEFFECT_COPY);
        assert_eq!(allowed_effect(DROPEFFECT_MOVE), DROPEFFECT_NONE);
        assert_eq!(allowed_effect(DROPEFFECT_LINK), DROPEFFECT_NONE);
    }

    #[test]
    fn only_copyable_files_from_elsewhere_are_taken() {
        assert_eq!(drop_effect(true, DROPEFFECT_COPY | DROPEFFECT_MOVE, false), DROPEFFECT_COPY);
        assert_eq!(drop_effect(true, DROPEFFECT_MOVE, false), DROPEFFECT_NONE, "a move-only drag gets no hint and no drop");
        assert_eq!(drop_effect(false, DROPEFFECT_COPY, false), DROPEFFECT_NONE, "text or a bare image");
        assert_eq!(drop_effect(true, DROPEFFECT_COPY, true), DROPEFFECT_NONE, "the shelf's own drag coming back");
    }

    #[test]
    fn drag_threshold_is_the_systems() {
        let (cx, cy) = unsafe { (GetSystemMetrics(SM_CXDRAG), GetSystemMetrics(SM_CYDRAG)) };
        assert!(!drag_moved(cx - 1, 0));
        assert!(drag_moved(cx, 0));
        assert!(drag_moved(0, -cy));
        assert!(!drag_moved(0, 0));
    }

    #[test]
    fn test_create_data_object_hdrop() {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        assert!(p.exists());
        let data: IDataObject = FileDataObject { paths: vec![p.clone()] }.into();
        let query = unsafe { data.QueryGetData(&hdrop_format()) };
        assert_eq!(query.0, 0);
        let paths = paths_of(&data);
        assert_eq!(paths, vec![p]);
    }

    #[test]
    fn test_enum_format_etc() {
        let p = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
        let data: IDataObject = FileDataObject { paths: vec![p] }.into();
        let enumerator = unsafe { data.EnumFormatEtc(1) }.expect("EnumFormatEtc(1) should succeed");
        let mut formats = [FORMATETC::default(); 2];
        let mut fetched = 0;
        let hr = unsafe { enumerator.Next(&mut formats, Some(&mut fetched)) };
        assert_eq!(hr.0, 0, "S_OK when all requested items are returned");
        assert_eq!(fetched, 2);
        assert_eq!(formats[0].cfFormat, CF_HDROP.0);
        assert_eq!(formats[1].cfFormat, preferred_drop_effect_format());
    }
}
