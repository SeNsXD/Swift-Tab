//! Uninstalling: cleanly removes Swift Tab, shortcuts, autostart, registry keys, and files.

use std::time::Duration;

use windows::core::HSTRING;
use windows::Win32::Storage::FileSystem::{MoveFileExW, MOVEFILE_DELAY_UNTIL_REBOOT};

use crate::install::without_ours;
use crate::plan::ours;
use crate::system::{delete_key, kill_all_in_dir, Places};

/// Takes Swift Tab out of `places.dir`; `wipe` also deletes its settings and log.
pub fn run(places: &Places, wipe: bool, progress: impl Fn(&str, f32)) -> Result<(), String> {
    let dir = places.dir.clone();
    if !places.sandbox {
        progress("Closing Swift Tab", 0.15);
        kill_all_in_dir(&dir);
    }
    progress("Shortcuts and registry", 0.4);
    let _ = nowbar::platform::autostart::set_autostart_for(&places.run_name, &dir.join("swifttab.exe"), false);
    if let Some(path) = &places.claude {
        let _ = nowbar::claude_hooks::rewrite_at(path, without_ours);
    }
    for lnk in [
        places.start_menu.join("Swift Tab.lnk"),
        places.desktop.join("Swift Tab.lnk"),
    ] {
        let _ = std::fs::remove_file(lnk);
    }
    progress("Removing files", 0.7);
    if dir.exists() {
        if let Ok(entries) = std::fs::read_dir(&dir) {
            for entry in entries.flatten() {
                delete(&entry.path());
            }
        }
        for p in ours(&dir) {
            delete(&p);
        }
        let _ = std::fs::remove_dir_all(&dir);
        if dir.exists() {
            let dir_h = HSTRING::from(dir.as_os_str());
            unsafe {
                let _ = MoveFileExW(&dir_h, None, MOVEFILE_DELAY_UNTIL_REBOOT);
            }
        }
    }
    delete_key(&places.uninstall_key);
    if wipe {
        let _ = std::fs::remove_dir_all(&places.roaming);
        let _ = std::fs::remove_dir_all(&places.local);
    }
    progress("Done", 1.0);
    Ok(())
}

/// Deletes `p`, clearing read-only flags and trying repeatedly before scheduling reboot cleanup if locked.
fn delete(p: &std::path::Path) -> bool {
    if !p.exists() {
        return true;
    }
    if let Ok(meta) = std::fs::metadata(p) {
        let mut perms = meta.permissions();
        if perms.readonly() {
            perms.set_readonly(false);
            let _ = std::fs::set_permissions(p, perms);
        }
    }
    for _ in 0..15 {
        let gone = if p.is_dir() {
            std::fs::remove_dir_all(p)
        } else {
            std::fs::remove_file(p)
        };
        if gone.is_ok() || !p.exists() {
            return true;
        }
        std::thread::sleep(Duration::from_millis(150));
    }
    // In Windows, a locked or in-use binary can often be renamed aside even when deletion fails
    let del_target = p.with_extension(format!("del_{}", std::process::id()));
    if std::fs::rename(p, &del_target).is_ok() {
        let _ = std::fs::remove_file(&del_target);
        if del_target.exists() {
            let path_h = HSTRING::from(del_target.as_os_str());
            unsafe {
                let _ = MoveFileExW(&path_h, None, MOVEFILE_DELAY_UNTIL_REBOOT);
            }
        }
        return true;
    }
    // If still in use, schedule delete on reboot via MoveFileExW
    let path_h = HSTRING::from(p.as_os_str());
    unsafe {
        let _ = MoveFileExW(&path_h, None, MOVEFILE_DELAY_UNTIL_REBOOT);
    }
    !p.exists()
}
