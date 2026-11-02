//! Downloads monitoring for the pill: tracks active browser and system downloads
//! (.crdownload for Chrome/Edge/Brave/Opera, .part for Firefox, .downloading) in the user's
//! Downloads folder, measures download progress and speed, and notifies when finished.

use std::ffi::{CStr, CString};
use std::path::{Path, PathBuf};
use std::time::Instant;
use windows::Win32::System::Com::CoTaskMemFree;
use windows::Win32::UI::Shell::{SHGetKnownFolderPath, FOLDERID_Downloads, KF_FLAG_DEFAULT};

#[link(name = "winsqlite3")]
extern "C" {
    fn sqlite3_open_v2(filename: *const std::ffi::c_char, ppDb: *mut *mut std::ffi::c_void, flags: i32, zVfs: *const std::ffi::c_char) -> i32;
    fn sqlite3_close(db: *mut std::ffi::c_void) -> i32;
    fn sqlite3_prepare_v2(db: *mut std::ffi::c_void, zSql: *const std::ffi::c_char, nByte: i32, ppStmt: *mut *mut std::ffi::c_void, pzTail: *mut *const std::ffi::c_char) -> i32;
    fn sqlite3_step(stmt: *mut std::ffi::c_void) -> i32;
    fn sqlite3_column_text(stmt: *mut std::ffi::c_void, iCol: i32) -> *const u8;
    fn sqlite3_column_int64(stmt: *mut std::ffi::c_void, iCol: i32) -> i64;
    fn sqlite3_finalize(stmt: *mut std::ffi::c_void) -> i32;
}

/// A finished download displays in the pill for this long.
pub const DOWNLOAD_DONE_MS: u64 = 5000;

#[derive(Clone, Debug, PartialEq)]
pub struct DownloadInfo {
    /// Friendly file name being downloaded (e.g. "VSCodeUserSetup-x64.exe").
    pub filename: String,
    /// Absolute path to the destination file.
    pub path: PathBuf,
    /// Temporary downloading file path (e.g. ".../file.exe.crdownload").
    pub temp_path: Option<PathBuf>,
    /// Directory containing the download.
    pub folder: PathBuf,
    /// Bytes downloaded so far.
    pub bytes: u64,
    /// Total bytes if known.
    pub total_bytes: Option<u64>,
    /// Current download speed in bytes per second.
    pub speed_bps: u64,
    /// Detail line (e.g. "45.2 MB · 3.5 MB/s" or "45.2 MB / 100 MB · 3.5 MB/s").
    pub detail: String,
    /// Progress fraction 0.0 .. 1.0 if known.
    pub progress: Option<f32>,
    /// Whether the download has completed.
    pub done: bool,
    /// Friendly name of downloading application (e.g. "Google Chrome").
    pub app_name: String,
    /// Executable name of downloading application (e.g. "chrome.exe").
    pub app_exe: String,
}

pub fn downloads_dir() -> Option<PathBuf> {
    unsafe {
        if let Ok(p) = SHGetKnownFolderPath(&FOLDERID_Downloads, KF_FLAG_DEFAULT, None) {
            let path = p.to_string().ok().map(PathBuf::from);
            CoTaskMemFree(Some(p.0 as *const _));
            if let Some(ref path) = path {
                if path.is_dir() {
                    return Some(path.clone());
                }
            }
        }
    }
    std::env::var_os("USERPROFILE")
        .map(PathBuf::from)
        .map(|u| u.join("Downloads"))
        .filter(|p| p.is_dir())
}

pub fn format_bytes(bytes: u64) -> String {
    const KB: u64 = 1024;
    const MB: u64 = 1024 * KB;
    const GB: u64 = 1024 * MB;
    if bytes >= GB {
        format!("{:.1} GB", bytes as f64 / GB as f64)
    } else if bytes >= MB {
        format!("{:.1} MB", bytes as f64 / MB as f64)
    } else if bytes >= KB {
        format!("{:.0} KB", bytes as f64 / KB as f64)
    } else {
        format!("{} B", bytes)
    }
}

pub fn clean_download_name(raw: &str) -> Option<(&str, &str)> {
    if let Some(stem) = raw.strip_suffix(".crdownload") {
        Some((stem, ".crdownload"))
    } else if let Some(stem) = raw.strip_suffix(".part") {
        Some((stem, ".part"))
    } else if let Some(stem) = raw.strip_suffix(".downloading") {
        Some((stem, ".downloading"))
    } else {
        None
    }
}

#[derive(Clone, Debug)]
struct BrowserSource {
    path: PathBuf,
    app_name: &'static str,
    app_exe: &'static str,
}

/// Known browser history SQLite database locations and associated browser executables.
fn browser_sources() -> Vec<BrowserSource> {
    let mut sources = Vec::new();
    if let Some(local) = std::env::var_os("LOCALAPPDATA").map(PathBuf::from) {
        // Google Chrome
        let chrome = local.join("Google").join("Chrome").join("User Data");
        sources.push(BrowserSource {
            path: chrome.join("Default").join("History"),
            app_name: "Google Chrome",
            app_exe: "chrome.exe",
        });
        for i in 1..=4 {
            sources.push(BrowserSource {
                path: chrome.join(format!("Profile {i}")).join("History"),
                app_name: "Google Chrome",
                app_exe: "chrome.exe",
            });
        }
        // Microsoft Edge
        let edge = local.join("Microsoft").join("Edge").join("User Data");
        sources.push(BrowserSource {
            path: edge.join("Default").join("History"),
            app_name: "Microsoft Edge",
            app_exe: "msedge.exe",
        });
        for i in 1..=4 {
            sources.push(BrowserSource {
                path: edge.join(format!("Profile {i}")).join("History"),
                app_name: "Microsoft Edge",
                app_exe: "msedge.exe",
            });
        }
        // Brave
        sources.push(BrowserSource {
            path: local.join("BraveSoftware").join("Brave-Browser").join("User Data").join("Default").join("History"),
            app_name: "Brave",
            app_exe: "brave.exe",
        });
    }
    if let Some(appdata) = std::env::var_os("APPDATA").map(PathBuf::from) {
        // Opera
        sources.push(BrowserSource {
            path: appdata.join("Opera Software").join("Opera Stable").join("History"),
            app_name: "Opera",
            app_exe: "opera.exe",
        });
    }
    sources.retain(|s| s.path.exists());
    sources
}

pub fn detect_download_app(ext: &str) -> (&'static str, &'static str) {
    if ext == ".part" {
        ("Firefox", "firefox.exe")
    } else {
        ("Google Chrome", "chrome.exe")
    }
}

/// Queries Chromium history database for active/recent download metadata.
/// Tries standard read-only first (to capture live WAL transactions), then falls back to URI immutable mode.
fn query_chromium_download(history_path: &Path) -> Option<(String, PathBuf, Option<u64>)> {
    let mut db: *mut std::ffi::c_void = std::ptr::null_mut();

    // 1. Try standard read-only open (which reads live WAL file)
    let c_path = CString::new(history_path.to_string_lossy().as_bytes()).ok()?;
    let mut rc = unsafe { sqlite3_open_v2(c_path.as_ptr(), &mut db, 0x00000001, std::ptr::null()) };

    // 2. Fallback to URI immutable=1 if locked
    if rc != 0 || db.is_null() {
        if !db.is_null() {
            unsafe { sqlite3_close(db) };
            db = std::ptr::null_mut();
        }
        let uri = format!("file:///{}?immutable=1", history_path.to_string_lossy().replace('\\', "/"));
        let c_uri = CString::new(uri).ok()?;
        rc = unsafe { sqlite3_open_v2(c_uri.as_ptr(), &mut db, 0x00000041, std::ptr::null()) };
        if rc != 0 || db.is_null() {
            return None;
        }
    }

    let mut result = None;

    // A. Active download (state == 0)
    let sql_active = CString::new("SELECT target_path, total_bytes, state FROM downloads WHERE state = 0 ORDER BY start_time DESC LIMIT 1").ok();
    if let Some(sql) = sql_active {
        let mut stmt: *mut std::ffi::c_void = std::ptr::null_mut();
        if unsafe { sqlite3_prepare_v2(db, sql.as_ptr(), -1, &mut stmt, std::ptr::null_mut()) } == 0 && !stmt.is_null() {
            if unsafe { sqlite3_step(stmt) } == 100 { // SQLITE_ROW
                let path_ptr = unsafe { sqlite3_column_text(stmt, 0) };
                let tot = unsafe { sqlite3_column_int64(stmt, 1) };
                if !path_ptr.is_null() {
                    let path_str = unsafe { CStr::from_ptr(path_ptr as *const std::ffi::c_char) }.to_string_lossy();
                    let path = PathBuf::from(path_str.as_ref());
                    if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                        result = Some((filename.to_string(), path, if tot > 0 { Some(tot as u64) } else { None }));
                    }
                }
            }
            unsafe { sqlite3_finalize(stmt) };
        }
    }

    // B. Unfinished download (end_time == 0)
    if result.is_none() {
        let sql_unfinished = CString::new("SELECT target_path, total_bytes, state FROM downloads WHERE end_time = 0 ORDER BY start_time DESC LIMIT 1").ok();
        if let Some(sql) = sql_unfinished {
            let mut stmt: *mut std::ffi::c_void = std::ptr::null_mut();
            if unsafe { sqlite3_prepare_v2(db, sql.as_ptr(), -1, &mut stmt, std::ptr::null_mut()) } == 0 && !stmt.is_null() {
                if unsafe { sqlite3_step(stmt) } == 100 {
                    let path_ptr = unsafe { sqlite3_column_text(stmt, 0) };
                    let tot = unsafe { sqlite3_column_int64(stmt, 1) };
                    if !path_ptr.is_null() {
                        let path_str = unsafe { CStr::from_ptr(path_ptr as *const std::ffi::c_char) }.to_string_lossy();
                        let path = PathBuf::from(path_str.as_ref());
                        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                            result = Some((filename.to_string(), path, if tot > 0 { Some(tot as u64) } else { None }));
                        }
                    }
                }
                unsafe { sqlite3_finalize(stmt) };
            }
        }
    }

    // C. Most recent download entry if not completed (state != 1)
    if result.is_none() {
        let sql_latest = CString::new("SELECT target_path, total_bytes, state FROM downloads WHERE state != 1 ORDER BY id DESC LIMIT 1").ok();
        if let Some(sql) = sql_latest {
            let mut stmt: *mut std::ffi::c_void = std::ptr::null_mut();
            if unsafe { sqlite3_prepare_v2(db, sql.as_ptr(), -1, &mut stmt, std::ptr::null_mut()) } == 0 && !stmt.is_null() {
                if unsafe { sqlite3_step(stmt) } == 100 {
                    let path_ptr = unsafe { sqlite3_column_text(stmt, 0) };
                    let tot = unsafe { sqlite3_column_int64(stmt, 1) };
                    if !path_ptr.is_null() {
                        let path_str = unsafe { CStr::from_ptr(path_ptr as *const std::ffi::c_char) }.to_string_lossy();
                        let path = PathBuf::from(path_str.as_ref());
                        if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                            result = Some((filename.to_string(), path, if tot > 0 { Some(tot as u64) } else { None }));
                        }
                    }
                }
                unsafe { sqlite3_finalize(stmt) };
            }
        }
    }

    unsafe { sqlite3_close(db) };
    result
}

/// Finds active browser download information across installed Chromium browsers.
fn query_active_browser_download() -> Option<(String, PathBuf, Option<u64>, &'static str, &'static str)> {
    for source in browser_sources() {
        if let Some((name, path, tot)) = query_chromium_download(&source.path) {
            return Some((name, path, tot, source.app_name, source.app_exe));
        }
    }
    None
}

#[derive(Default)]
pub struct DownloadTracker {
    last_path: Option<PathBuf>,
    last_bytes: u64,
    last_time: Option<Instant>,
    last_app_name: String,
    last_app_exe: String,
}

impl DownloadTracker {
    pub fn scan(&mut self, folder: &Path) -> (Option<DownloadInfo>, Option<DownloadInfo>) {
        let entries = match std::fs::read_dir(folder) {
            Ok(e) => e,
            Err(_) => return (None, None),
        };

        let now = Instant::now();
        let mut active: Option<(PathBuf, String, PathBuf, u64, Option<u64>, String, String)> = None;

        let browser_download = query_active_browser_download();

        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_file() {
                continue;
            }
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or_default();
            if let Some((clean_name, ext)) = clean_download_name(name) {
                let raw_target = folder.join(clean_name);
                let bytes = entry.metadata().map(|m| m.len()).unwrap_or(0);

                let (fallback_app, fallback_exe) = detect_download_app(ext);

                // Use rich browser history data if available
                let (display_name, target_file, total_bytes, app_name, app_exe) = if let Some((b_name, b_path, b_tot, b_app, b_exe)) = &browser_download {
                    (b_name.clone(), b_path.clone(), *b_tot, b_app.to_string(), b_exe.to_string())
                } else if clean_name.starts_with("Unconfirmed ") {
                    ("Downloading file...".to_string(), raw_target, None, fallback_app.to_string(), fallback_exe.to_string())
                } else {
                    (clean_name.to_string(), raw_target, None, fallback_app.to_string(), fallback_exe.to_string())
                };

                active = Some((path, display_name, target_file, bytes, total_bytes, app_name, app_exe));
                break;
            }
        }

        let mut current_download = None;
        let mut finished_download = None;

        if let Some((path, name, target_file, bytes, total_bytes, app_name, app_exe)) = active {
            let elapsed = self.last_time.map_or(1.0, |t| now.duration_since(t).as_secs_f64());
            let speed = if self.last_path.as_ref() == Some(&path) && elapsed > 0.4 && bytes >= self.last_bytes {
                ((bytes - self.last_bytes) as f64 / elapsed) as u64
            } else {
                0
            };

            let progress = total_bytes.map(|tot| (bytes as f32 / tot.max(1) as f32).clamp(0.0, 1.0));

            let detail = match (total_bytes, speed > 10_000) {
                (Some(tot), true) => {
                    let eta_s = tot.saturating_sub(bytes) / speed.max(1);
                    let eta = if eta_s >= 3600 {
                        format!("{}h {}m left", eta_s / 3600, (eta_s % 3600) / 60)
                    } else if eta_s >= 60 {
                        format!("{}m left", eta_s / 60)
                    } else {
                        format!("{}s left", eta_s)
                    };
                    format!("{} / {} · {}/s · {}", format_bytes(bytes), format_bytes(tot), format_bytes(speed), eta)
                }
                (Some(tot), false) => format!("{} / {}", format_bytes(bytes), format_bytes(tot)),
                (None, true) => format!("{} · {}/s", format_bytes(bytes), format_bytes(speed)),
                (None, false) => format_bytes(bytes),
            };

            current_download = Some(DownloadInfo {
                filename: name,
                path: target_file,
                temp_path: Some(path.clone()),
                folder: folder.to_path_buf(),
                bytes,
                total_bytes,
                speed_bps: speed,
                detail,
                progress,
                done: false,
                app_name: app_name.clone(),
                app_exe: app_exe.clone(),
            });

            self.last_path = Some(path);
            self.last_bytes = bytes;
            self.last_time = Some(now);
            self.last_app_name = app_name;
            self.last_app_exe = app_exe;
        } else if let Some(prev) = self.last_path.take() {
            // Active download disappeared: check if the finished target file exists
            let target = if let Some(stem) = prev.file_name().and_then(|n| n.to_str()).and_then(clean_download_name).map(|(s, _)| s) {
                folder.join(stem)
            } else {
                folder.join("download")
            };

            let found_target = if target.exists() {
                Some(target)
            } else if let Some((_, b_path, _, _, _)) = browser_download {
                if b_path.exists() { Some(b_path) } else { None }
            } else {
                None
            };

            if let Some(dest) = found_target {
                let final_name = dest.file_name().and_then(|n| n.to_str()).unwrap_or("download").to_string();
                let bytes = std::fs::metadata(&dest).map(|m| m.len()).unwrap_or(self.last_bytes);
                finished_download = Some(DownloadInfo {
                    filename: final_name,
                    path: dest,
                    temp_path: None,
                    folder: folder.to_path_buf(),
                    bytes,
                    total_bytes: Some(bytes),
                    speed_bps: 0,
                    detail: format_bytes(bytes),
                    progress: Some(1.0),
                    done: true,
                    app_name: self.last_app_name.clone(),
                    app_exe: self.last_app_exe.clone(),
                });
            }
            self.last_bytes = 0;
            self.last_time = None;
        }

        (current_download, finished_download)
    }
}

/// Cancels an active download: deletes temp files or schedules cleanup once released.
pub fn cancel_active_download(info: &DownloadInfo) {
    if let Some(temp) = &info.temp_path {
        let _ = std::fs::remove_file(temp);
    }
    let _ = std::fs::remove_file(&info.path);

    // If still locked by a browser, try deleting again in a detached worker once released
    if let Some(temp) = info.temp_path.clone() {
        if temp.exists() {
            std::thread::spawn(move || {
                for _ in 0..12 {
                    std::thread::sleep(std::time::Duration::from_millis(500));
                    if !temp.exists() || std::fs::remove_file(&temp).is_ok() {
                        break;
                    }
                }
            });
        }
    }
}

pub fn try_cancel_download(path: &Path) -> bool {
    if !path.exists() {
        return true;
    }
    if std::fs::remove_file(path).is_ok() {
        return true;
    }
    if let Ok(f) = std::fs::OpenOptions::new().write(true).truncate(true).open(path) {
        drop(f);
    }
    std::fs::remove_file(path).is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_browser_download_query() {
        let sources = browser_sources();
        println!("Browser sources count: {}", sources.len());
        for s in &sources {
            let res = query_chromium_download(&s.path);
            println!("Source {} ({}) -> {res:?}", s.app_name, s.app_exe);
        }
        let active = query_active_browser_download();
        println!("query_active_browser_download() = {active:?}");
    }
}

