#![windows_subsystem = "windows"]
//! `swifttab-setup.exe`: installs the Swift Tab for this user (no elevation), upgrades or repairs it,
//! and, as `--uninstall`, takes it out again. One Slint window in the settings window's style;
//! `--quiet` does the same without it. The Swift Tab's exes travel inside (payload.bin).

mod install;
mod payload;
mod plan;
mod stage;
mod system;
mod uninstall;

use std::cell::RefCell;
use std::path::{Path, PathBuf};
use std::rc::Rc;
use std::time::Duration;

use raw_window_handle::{HasWindowHandle, RawWindowHandle};
use slint::winit_030::WinitWindowAccessor;
use slint::ComponentHandle;
use windows::core::HSTRING;
use windows::Win32::Foundation::{HWND, POINT};
use windows::Win32::Graphics::Gdi::{GetMonitorInfoW, MonitorFromPoint, MONITORINFO, MONITOR_DEFAULTTONEAREST};
use windows::Win32::System::Com::{CoInitializeEx, COINIT_APARTMENTTHREADED};
use windows::Win32::UI::HiDpi::{GetDpiForMonitor, MDT_EFFECTIVE_DPI};
use windows::Win32::UI::WindowsAndMessaging::{GetCursorPos, GetForegroundWindow, MessageBoxW, SetForegroundWindow, MB_ICONERROR, MB_OK};

use install::Options;

use plan::{kind, short_path, Kind};
use system::Places;

slint::include_modules!();

static PAYLOAD: &[u8] = include_bytes!(concat!(env!("OUT_DIR"), "/payload.bin"));
const VERSION: &str = nowbar::platform::updater::VERSION;
const SIZE: (f32, f32) = (700.0, 496.0);
const UNINSTALL_SIZE: (f32, f32) = (380.0, 380.0);
/// payload.rs's DAMAGED (the build script shares that file, so it can't reach i18n): marked here,
/// and shown through tr like the other step messages.
const _DAMAGED: &str = nowbar::i18n::n_("Setup file is damaged; please download again.");

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let has = |flag: &str| args.iter().any(|a| a == flag);
    let (quiet, no_launch) = (has("--quiet"), has("--no-launch"));
    let from = args.iter().position(|a| a == "--uninstall-from").and_then(|i| args.get(i + 1)).map(PathBuf::from);
    let uninstalling = has("--uninstall") || from.is_some();
    nowbar::platform::frame::register_app_identity();
    // The language the Swift Tab's settings pick (a first install has none: Windows' own).
    nowbar::i18n::set_language(nowbar::i18n::chosen(nowbar::settings::load().language));
    // Older Windows 10s lack what the Swift Tab runs on; removing it still works there.
    if !uninstalling && nowbar::platform::os::build().is_some_and(|b| !nowbar::platform::os::supported(b)) {
        if !quiet {
            message("Swift Tab requires Windows 10 version 1809 or later.");
        }
        std::process::exit(1);
    }
    unsafe {
        let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
    }
    let mut places = Places::current();
    if let Some(dir) = &from {
        places.dir = dir.clone();
    }
    // Run from the folder it is to delete: it goes on from a copy in %TEMP%.
    if uninstalling && from.is_none() && std::env::current_exe().is_ok_and(|me| system::inside(&me, &places.dir)) {
        if let Err(e) = system::relaunch_from_temp(&places.dir, quiet) {
            if !quiet {
                message(&e);
            }
            std::process::exit(1);
        }
        return;
    }
    let launch = !no_launch && !places.sandbox;
    if quiet {
        let ok = if uninstalling {
            uninstall::run(&places, false, |_, _| {}).is_ok()
        } else {
            // The API as it was (off on a first install): nobody was asked.
            let opts = Options { autostart: true, claude: false, desktop: false, api: None, launch };
            install::run(&places, &opts, PAYLOAD, VERSION, |_, _| {}).is_ok()
        };
        if from.is_some() {
            system::delete_self_later();
        }
        std::process::exit(if ok { 0 } else { 1 });
    }
    if let Err(e) = window(places, uninstalling, launch) {
        message(&format!("Failed to open setup window: {e}"));
    }
    if from.is_some() {
        system::delete_self_later();
    }
}

fn message(text: &str) {
    unsafe {
        MessageBoxW(None, &HSTRING::from(text), &HSTRING::from("Swift Tab"), MB_OK | MB_ICONERROR);
    }
}

/// The window's texts for this run (the kind of run decides the heading's line and the button).
fn describe(ui: &SetupWindow, places: &Places, uninstalling: bool) {
    let home = std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_default();
    ui.set_version(format!("v{VERSION}").into());
    ui.set_location(short_path(&places.dir, &home).into());
    ui.set_claude_available(false);
    if uninstalling {
        ui.set_mode("uninstall".into());
        ui.set_heading("Uninstall Swift Tab".into());
        ui.set_subtitle("Swift Tab and its shortcuts will be removed.".into());
        ui.set_primary("Uninstall".into());
        return;
    }
    let installed = places.installed();
    let old = installed.as_ref().map(|(_, v)| v.clone()).unwrap_or_default();
    let (subtitle, primary) = match kind(installed.as_ref().map(|(_, v)| v.as_str()), VERSION) {
        Kind::Fresh => ("Live activities in the taskbar".to_string(), "Install"),
        Kind::Upgrade => (format!("v{} installed · will update to v{}", old, VERSION), "Update"),
        Kind::Repair => ("This version is already installed; files will be rewritten.".to_string(), "Repair"),
        Kind::Downgrade => (format!("A newer version is installed (v{}).", old), "Install anyway"),
    };
    ui.set_subtitle(subtitle.into());
    ui.set_primary(primary.into());
    if installed.is_some() {
        // As it is set up now; the folder stays.
        let exe = places.dir.join("swifttab.exe");
        ui.set_location_visible(false);
        ui.set_autostart(nowbar::platform::autostart::run_value(&places.run_name).is_some_and(|v| nowbar::platform::autostart::points_at(&v, &exe)));
        ui.set_desktop(places.desktop.join("Swift Tab.lnk").exists() || places.desktop.join("Swift Tab.lnk").exists());
        ui.set_claude(false);
        ui.set_api(nowbar::settings::load_from(&places.roaming.join("settings.json")).api_enabled);
        ui.set_open_settings(false);
    }
}

#[allow(dead_code)]
fn has_our_hooks(path: &Path) -> bool {
    let text = std::fs::read_to_string(path).unwrap_or_default();
    let v: serde_json::Value = serde_json::from_str(text.trim_start_matches('\u{feff}')).unwrap_or_default();
    v["hooks"].as_object().is_some_and(|h| h.values().filter_map(|e| e.as_array()).flatten().filter_map(|e| e["hooks"].as_array()).flatten().any(nowbar::claude_hooks::is_ours))
}

fn window(places: Places, uninstalling: bool, launch: bool) -> Result<(), slint::PlatformError> {
    let shots = std::env::var("NOWBAR_SETUP_SHOTS").ok();
    let ui = SetupWindow::new()?;
    // Windows 10 has no Segoe Fluent Icons: the glyphs would be boxes.
    nowbar::set_slint_faces!(ui.global::<Theme>());
    let _ = slint::select_bundled_translation(if nowbar::i18n::lang() == nowbar::i18n::Language::English { "en" } else { "" });
    let (position, scale) = centre_at_mouse(if uninstalling { UNINSTALL_SIZE } else { SIZE });
    stage::fill(&ui, scale);
    describe(&ui, &places, uninstalling);
    let places = Rc::new(RefCell::new(places));

    {
        let weak = ui.as_weak();
        ui.on_drag(move || {
            if let Some(ui) = weak.upgrade() {
                ui.window().with_winit_window(|w| w.drag_window());
            }
        });
    }
    ui.on_close_clicked(|| {
        let _ = slint::quit_event_loop();
    });
    ui.on_open_url(|url| {
        unsafe {
            let _ = windows::Win32::UI::Shell::ShellExecuteW(
                None,
                windows::core::w!("open"),
                &windows::core::HSTRING::from(url.as_str()),
                None,
                None,
                windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL,
            );
        }
    });
    {
        let (weak, places) = (ui.as_weak(), places.clone());
        ui.on_change_location(move || {
            let Some(ui) = weak.upgrade() else { return };
            if let Some(dir) = system::pick_folder(hwnd_of(&ui)) {
                let home = std::env::var_os("USERPROFILE").map(PathBuf::from).unwrap_or_default();
                ui.set_location(short_path(&dir, &home).into());
                places.borrow_mut().dir = dir;
            }
        });
    }
    {
        let (weak, places) = (ui.as_weak(), places.clone());
        ui.on_primary_clicked(move || {
            let Some(ui) = weak.upgrade() else { return };
            if ui.get_phase() == "done" {
                let dir = places.borrow().dir.clone();
                if !uninstalling && launch && ui.get_open_settings() {
                    let _ = std::process::Command::new(dir.join("swifttab-settings.exe")).spawn();
                }
                let _ = slint::quit_event_loop();

                return;
            }
            start(&ui, places.borrow().clone(), uninstalling, launch);
        });
    }

    if shots.is_some() {
        ui.window().set_position(slint::PhysicalPosition::new(-6000, 120));
        ui.set_moving(false);
    } else {
        ui.window().set_position(position);
    }
    let had_focus = shots.as_ref().map(|_| unsafe { GetForegroundWindow() });
    nowbar::platform::frame::frame_from_creation(true);
    ui.show()?;
    let frame_timer = Rc::new(slint::Timer::default());
    {
        let (weak, timer) = (ui.as_weak(), Rc::downgrade(&frame_timer));
        let to_front = shots.is_none();
        frame_timer.start(slint::TimerMode::Repeated, Duration::from_millis(5), move || {
            let Some(ui) = weak.upgrade() else { return };
            let Some(hwnd) = hwnd_of(&ui) else { return };
            nowbar::platform::frame::adopt(hwnd);
            ui.set_framed(true);
            nowbar::platform::frame::style_frame(hwnd, true);
            if to_front {
                unsafe {
                    let _ = SetForegroundWindow(hwnd);
                }
            }
            if let Some(t) = timer.upgrade() {
                t.stop();
            }
        });
    }
    if let Some(dir) = shots {
        if let Some(prev) = had_focus.filter(|h| !h.is_invalid()) {
            unsafe {
                let _ = SetForegroundWindow(prev);
            }
        }
        return shoot(&ui, &dir);
    }
    slint::run_event_loop()
}

/// Runs the install or uninstall on a worker thread; the window follows its steps.
fn start(ui: &SetupWindow, places: Places, uninstalling: bool, launch: bool) {
    ui.set_phase("working".into());
    ui.set_progress(0.0);
    ui.set_step("Preparing".into());
    let weak = ui.as_weak();
    let opts = Options { autostart: ui.get_autostart(), claude: false, desktop: ui.get_desktop(), api: Some(ui.get_api()), launch };
    let wipe = ui.get_wipe();
    std::thread::spawn(move || {
        unsafe {
            let _ = CoInitializeEx(None, COINIT_APARTMENTTHREADED);
        }
        let progress = |step: &str, p: f32| {
            let step = step.to_string();
            let _ = weak.upgrade_in_event_loop(move |ui| {
                ui.set_step(step.into());
                ui.set_progress(p);
            });
        };
        let result = if uninstalling {
            uninstall::run(&places, wipe, progress).map(|_| "Swift Tab and its shortcuts have been removed.".to_string())
        } else {
            install::run(&places, &opts, PAYLOAD, VERSION, progress).map(|notes| match (notes.is_empty(), launch) {
                (true, true) => "Swift Tab is on the taskbar. You can customize everything from settings.".to_string(),
                (true, false) => "Files installed; Swift Tab was not launched.".to_string(),
                (false, _) => notes.join("\n"),
            })
        };
        let _ = weak.upgrade_in_event_loop(move |ui| match result {
            Ok(text) => {
                ui.set_phase("done".into());
                ui.set_message(text.into());
                ui.set_primary(if uninstalling { "Close" } else { "Finish" }.into());
            }
            Err(e) => {
                ui.set_phase("error".into());
                // The steps' fixed messages (payload.rs's too) are translated here.
                ui.set_message(e.into());
                ui.set_primary("Try again".into());
            }
        });
    });
}

/// Where the window opens (centred on the monitor under the mouse) and that monitor's scale.
fn centre_at_mouse(size: (f32, f32)) -> (slint::PhysicalPosition, f32) {
    unsafe {
        let mut mouse = POINT::default();
        let _ = GetCursorPos(&mut mouse);
        let monitor = MonitorFromPoint(mouse, MONITOR_DEFAULTTONEAREST);
        let mut info = MONITORINFO { cbSize: std::mem::size_of::<MONITORINFO>() as u32, ..Default::default() };
        let _ = GetMonitorInfoW(monitor, &mut info);
        let (mut dx, mut dy) = (96u32, 96u32);
        let _ = GetDpiForMonitor(monitor, MDT_EFFECTIVE_DPI, &mut dx, &mut dy);
        let scale = dx as f32 / 96.0;
        let w = info.rcWork;
        let (pw, ph) = ((size.0 * scale) as i32, (size.1 * scale) as i32);
        (slint::PhysicalPosition::new(w.left + (w.right - w.left - pw) / 2, w.top + (w.bottom - w.top - ph) / 2), scale)
    }
}

fn hwnd_of(ui: &SetupWindow) -> Option<HWND> {
    let handle = ui.window().window_handle();
    match handle.window_handle().ok()?.as_raw() {
        RawWindowHandle::Win32(h) => Some(HWND(h.hwnd.get() as *mut _)),
        _ => None,
    }
}

/// Dev: every state the window has, drawn by its own renderer into `dir`, then out.
fn shoot(ui: &SetupWindow, dir: &str) -> Result<(), slint::PlatformError> {
    let _ = std::fs::create_dir_all(dir);
    let pills = stage::count(ui).max(1) as i32;
    type SetUp = Box<dyn Fn(&SetupWindow)>;
    let states: Vec<(&str, SetUp)> = vec![
        ("1-install", Box::new(|_| {})),
        ("2-install-no-claude", Box::new(|ui| ui.set_claude_available(false))),
        ("3-installing", Box::new(|ui| {
            ui.set_claude_available(true);
            ui.set_phase("working".into());
            ui.set_step("Copying files".into());
            ui.set_progress(0.45);
        })),
        ("4-installed", Box::new(|ui| {
            ui.set_phase("done".into());
            ui.set_primary("Finish".into());
            ui.set_message("Swift Tab is on the taskbar. You can customize everything from settings.".into());
        })),
        ("5-error", Box::new(|ui| {
            ui.set_phase("error".into());
            ui.set_primary("Try again".into());
            ui.set_message("Swift Tab did not close; close it from the taskbar and try again.".into());
        })),
        ("6-update", Box::new(|ui| {
            ui.set_phase("ready".into());
            ui.set_subtitle(format!("v0.2.0 installed · will update to v{VERSION}").into());
            ui.set_primary("Update".into());
            ui.set_location_visible(false);
            ui.set_open_settings(false);
        })),
        ("7-repair", Box::new(|ui| {
            ui.set_subtitle("This version is already installed; files will be rewritten.".into());
            ui.set_primary("Repair".into());
        })),
        ("8-uninstall", Box::new(|ui| {
            ui.set_mode("uninstall".into());
            ui.set_heading("Uninstall Swift Tab".into());
            ui.set_subtitle("Swift Tab and its shortcuts will be removed.".into());
            ui.set_primary("Uninstall".into());
        })),
        ("9-uninstalling", Box::new(|ui| {
            ui.set_phase("working".into());
            ui.set_step("Removing files".into());
            ui.set_progress(0.7);
        })),
        ("10-uninstalled", Box::new(|ui| {
            ui.set_phase("done".into());
            ui.set_primary("Close".into());
            ui.set_message("Swift Tab and its shortcuts have been removed.".into());
        })),
    ];
    let step = Rc::new(std::cell::Cell::new(0usize));
    let timer = slint::Timer::default();
    let (weak, dir) = (ui.as_weak(), dir.to_string());
    timer.start(slint::TimerMode::Repeated, Duration::from_millis(700), move || {
        let Some(ui) = weak.upgrade() else { return };
        let n = step.get();
        // Each tick saves what the last one set up, then sets up the next.
        if n > 0 {
            if let Ok(snap) = ui.window().take_snapshot() {
                let mut bgra = Vec::with_capacity(snap.as_slice().len() * 4);
                for p in snap.as_slice() {
                    bgra.extend_from_slice(&[p.b, p.g, p.r, p.a]);
                }
                let _ = nowbar::slint_shot::save_png(&format!("{dir}/{}.png", states[n - 1].0), snap.width(), snap.height(), &bgra);
            }
        }
        let Some((_, set_up)) = states.get(n) else {
            std::process::exit(0);
        };
        ui.set_stage_index(n as i32 % pills);
        set_up(&ui);
        ui.window().request_redraw();
        step.set(n + 1);
    });
    slint::run_event_loop()
}
