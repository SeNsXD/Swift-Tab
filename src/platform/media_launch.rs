use std::path::Path;
use windows::core::{w, HSTRING};
use windows::Win32::System::Registry::{RegCloseKey, RegOpenKeyExW, HKEY_CURRENT_USER, HKEY_LOCAL_MACHINE, KEY_READ};
use windows::Win32::UI::Shell::ShellExecuteW;
use windows::Win32::UI::WindowsAndMessaging::SW_SHOWNORMAL;

pub fn is_spotify_installed() -> bool {
    let mut key = Default::default();
    if unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, w!("Software\\Classes\\spotify"), Some(0), KEY_READ, &mut key) }.is_ok() {
        unsafe { let _ = RegCloseKey(key); }
        return true;
    }
    if unsafe { RegOpenKeyExW(HKEY_LOCAL_MACHINE, w!("SOFTWARE\\Classes\\spotify"), Some(0), KEY_READ, &mut key) }.is_ok() {
        unsafe { let _ = RegCloseKey(key); }
        return true;
    }
    if let Ok(local) = std::env::var("LOCALAPPDATA") {
        if Path::new(&local).join("Microsoft\\WindowsApps\\Spotify.exe").exists() {
            return true;
        }
    }
    if let Ok(appdata) = std::env::var("APPDATA") {
        if Path::new(&appdata).join("Spotify\\Spotify.exe").exists() {
            return true;
        }
    }
    false
}

pub fn open_browser_url(url: &str) {
    unsafe {
        let _ = ShellExecuteW(
            None,
            w!("open"),
            &HSTRING::from(url),
            None,
            None,
            SW_SHOWNORMAL,
        );
    }
}

pub fn launch_spotify() {
    if is_spotify_installed() {
        unsafe {
            let res = ShellExecuteW(None, w!("open"), w!("spotify:"), None, None, SW_SHOWNORMAL);
            if res.0 as usize <= 32 {
                open_browser_url("https://open.spotify.com");
            }
        }
    } else {
        open_browser_url("https://open.spotify.com");
    }
}

pub fn is_ytmusic_installed() -> bool {
    let mut key = Default::default();
    if unsafe { RegOpenKeyExW(HKEY_CURRENT_USER, w!("Software\\Classes\\youtubemusic"), Some(0), KEY_READ, &mut key) }.is_ok() {
        unsafe { let _ = RegCloseKey(key); }
        return true;
    }
    if unsafe { RegOpenKeyExW(HKEY_LOCAL_MACHINE, w!("SOFTWARE\\Classes\\youtubemusic"), Some(0), KEY_READ, &mut key) }.is_ok() {
        unsafe { let _ = RegCloseKey(key); }
        return true;
    }
    false
}

pub fn launch_ytmusic() {
    if is_ytmusic_installed() {
        unsafe {
            let res = ShellExecuteW(None, w!("open"), w!("youtubemusic:"), None, None, SW_SHOWNORMAL);
            if res.0 as usize <= 32 {
                open_browser_url("https://music.youtube.com");
            }
        }
    } else {
        open_browser_url("https://music.youtube.com");
    }
}

pub fn launch_youtube() {
    open_browser_url("https://www.youtube.com");
}
