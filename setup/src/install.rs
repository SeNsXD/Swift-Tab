//! Installing (and upgrading, repairing): the steps of the spec's 2.4, in order, on a worker
//! thread. A failure before the exes are in place puts the old ones back.

use std::path::{Path, PathBuf};
use std::time::Duration;

use nowbar::i18n::{tr, trf};

use crate::payload::unpack;
use crate::plan::{place_steps, uninstall_values, SETUP_EXE, STAGED};
use crate::system::{quit_now_bar, close_settings, shortcut, start_and_watch, write_values, Places};

#[derive(Clone, Copy, Debug)]
pub struct Options {
    pub autostart: bool,
    pub claude: bool,
    pub desktop: bool,
    /// Other apps' notifications (the API); `None` leaves the setting as it is (`--quiet`).
    pub api: Option<bool>,
    /// Start the Swift Tab at the end (not in the sandbox, not with `--no-launch`).
    pub launch: bool,
}

pub const NO_FILES: &str = "This setup was built without files (no NOWBAR_SETUP_PAYLOAD).";
const NOT_WRITABLE: &str = "Cannot write to this folder; choose another location.";

/// Installs `payload` into `places.dir`; `progress(step, 0..1)` as it goes. The notes are what
/// didn't work but doesn't stop the Swift Tab (a shortcut, Claude Code).
pub fn run(places: &Places, opts: &Options, payload: &[u8], version: &str, progress: impl Fn(&str, f32)) -> Result<Vec<String>, String> {
    if payload.is_empty() {
        return Err(NO_FILES.into());
    }
    progress("Preparing", 0.05);
    let (_, files) = unpack(payload)?;
    let dir = places.dir.clone();
    std::fs::create_dir_all(&dir).map_err(|_| NOT_WRITABLE.to_string())?;

    if !places.sandbox {
        progress("Closing Swift Tab", 0.15);
        quit_now_bar(Duration::from_secs(10))?;
        close_settings(&dir, Duration::from_secs(3));
    }

    progress("Copying files", 0.3);
    let staged = dir.join(STAGED);
    let _ = std::fs::remove_dir_all(&staged);
    std::fs::create_dir_all(&staged).map_err(|_| NOT_WRITABLE.to_string())?;
    for f in &files {
        std::fs::write(staged.join(&f.name), &f.bytes).map_err(|_| NOT_WRITABLE.to_string())?;
    }
    for e in crate::plan::EXES {
        let _ = std::fs::remove_file(dir.join(format!("{e}.old.exe")));
    }
    let placed = place(&dir, &staged);
    let _ = std::fs::remove_dir_all(&staged);
    let done = placed?;
    copy_self(&dir).inspect_err(|_| undo(&done))?;

    progress("Registering application", 0.6);
    let date = today();
    write_values(&places.uninstall_key, &uninstall_values(&dir, version, folder_kb(&dir), &date))?;

    let mut notes = Vec::new();
    progress("Creating shortcuts", 0.7);
    let exe = dir.join("swifttab.exe");
    let _ = std::fs::remove_file(places.start_menu.join("Swift Tab.lnk"));
    if let Err(e) = shortcut(&places.start_menu.join("Swift Tab.lnk"), &exe) {
        notes.push(e);
    }
    let _ = std::fs::remove_file(places.desktop.join("Swift Tab.lnk"));
    let on_desktop = places.desktop.join("Swift Tab.lnk");
    if opts.desktop {
        if let Err(e) = shortcut(&on_desktop, &exe) {
            notes.push(e);
        }
    } else {
        let _ = std::fs::remove_file(&on_desktop);
    }

    progress("Setting up start with Windows", 0.78);
    set_autostart(places, &exe, opts.autostart)?;
    if let Some(on) = opts.api {
        set_api(&places.roaming.join("settings.json"), on)?;
    }

    if let Some(path) = &places.claude {
        progress("Claude Code", 0.86);
        let changed = if opts.claude {
            nowbar::claude_hooks::rewrite_at(path, |v| nowbar::claude_hooks::install(v, &exe.display().to_string()))
        } else {
            nowbar::claude_hooks::rewrite_at(path, without_ours)
        };
        if changed.is_err() {
            notes.push(tr("Could not connect to Claude Code: settings file is open or unreadable.").into());
        }
    }

    if opts.launch {
        progress("Starting Swift Tab", 0.94);
        if !start_and_watch(&exe, &[], Duration::from_secs(2)) {
            return Err("Swift Tab could not start. Please reboot and try again.".into());
        }
    }
    for e in crate::plan::EXES {
        let _ = std::fs::remove_file(dir.join(format!("{e}.old.exe")));
    }
    progress("Done", 1.0);
    Ok(notes)
}

/// Settings without our hooks; `None` when there were none (nothing to write).
pub fn without_ours(v: serde_json::Value) -> Option<serde_json::Value> {
    let next = nowbar::claude_hooks::remove(v.clone());
    (next != v).then_some(next)
}

/// The Swift Tab's own setting (its first start would otherwise turn starting with Windows back
/// on) and the Run value.
pub fn set_autostart(places: &Places, exe: &Path, on: bool) -> Result<(), String> {
    let file = places.roaming.join("settings.json");
    let mut s = nowbar::settings::load_from(&file);
    if s.preferred_idle_activity.is_empty() {
        s.preferred_idle_activity = "media".into();
    }
    if s.autostart != on || !file.exists() {
        s.autostart = on;
        nowbar::settings::save_to(&file, &s).map_err(|e| trf("Could not save settings: {}", &[&e]))?;
    }
    nowbar::platform::autostart::set_autostart_for(&places.run_name, exe, on).map_err(|e| trf("Could not configure start with Windows: {}", &[&e]))
}

/// The user's answer to "Enable application notifications", in Swift Tab's settings (the rest stays).
pub fn set_api(file: &Path, on: bool) -> Result<(), String> {
    // A file there that can't be read now (half written, locked) stays as it is: the defaults
    // must never go over the user's settings.
    let mut s = match (file.exists(), nowbar::settings::try_load_from(file)) {
        (false, _) => nowbar::settings::Settings::default(),
        (true, Some(s)) => s,
        (true, None) => return Ok(()),
    };
    if s.api_enabled == on && file.exists() {
        return Ok(());
    }
    s.api_enabled = on;
    nowbar::settings::save_to(file, &s).map_err(|e| trf("Could not save settings: {}", &[&e]))
}

/// Renames along `place_steps`; on a failure the done ones are undone and the error says so.
fn place(dir: &Path, staged: &Path) -> Result<Vec<(PathBuf, PathBuf)>, String> {
    let mut done = Vec::new();
    for (from, to) in place_steps(dir, staged, |p| p.exists()) {
        let moved = std::fs::rename(&from, &to).or_else(|_| std::fs::copy(&from, &to).map(|_| ()));
        if moved.is_err() {
            undo(&done);
            return Err("Could not replace Swift Tab files; please close Swift Tab and try again.".into());
        }
        done.push((from, to));
    }
    Ok(done)
}

fn undo(done: &[(PathBuf, PathBuf)]) {
    for (from, to) in done.iter().rev() {
        let _ = std::fs::rename(to, from);
    }
}

/// The setup beside the Swift Tab: the installed apps list runs it to uninstall. Skipped when it
/// is already the one running from there (a repair).
fn copy_self(dir: &Path) -> Result<(), String> {
    let me = std::env::current_exe().map_err(|e| e.to_string())?;
    let target = dir.join(SETUP_EXE);
    if crate::system::inside(&me, &target) {
        return Ok(());
    }
    let _ = std::fs::remove_file(dir.join("swifttab-setup.old.exe"));
    if target.exists() && std::fs::remove_file(&target).is_err() {
        // In use (an uninstaller window open): step it aside.
        let _ = std::fs::rename(&target, dir.join("swifttab-setup.old.exe"));
    }
    std::fs::copy(&me, &target).map(|_| ()).map_err(|_| NOT_WRITABLE.to_string())
}

/// The folder's files, in KB (the installed apps list shows it).
fn folder_kb(dir: &Path) -> u32 {
    let bytes: u64 = std::fs::read_dir(dir).into_iter().flatten().flatten().filter_map(|e| e.metadata().ok()).filter(|m| m.is_file()).map(|m| m.len()).sum();
    (bytes / 1024) as u32
}

/// yyyymmdd, local time.
fn today() -> String {
    let t = unsafe { windows::Win32::System::SystemInformation::GetLocalTime() };
    format!("{:04}{:02}{:02}", t.wYear, t.wMonth, t.wDay)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_api_choice_lands_in_the_settings_and_keeps_the_rest() {
        let dir = std::env::temp_dir().join(format!("nowbar-setup-api-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        let file = dir.join("settings.json");
        set_api(&file, true).unwrap();
        assert!(nowbar::settings::load_from(&file).api_enabled, "a first install writes it");
        let mut s = nowbar::settings::load_from(&file);
        s.idle_clock = false;
        nowbar::settings::save_to(&file, &s).unwrap();
        set_api(&file, false).unwrap();
        let s = nowbar::settings::load_from(&file);
        assert!(!s.api_enabled && !s.idle_clock, "only the API changes");
        // A file that can't be read now (half written, locked) is left as it is.
        std::fs::write(&file, "{\"idle_clock\": false, ").unwrap();
        set_api(&file, true).unwrap();
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "{\"idle_clock\": false, ", "never the defaults over the user's settings");
    }
}
