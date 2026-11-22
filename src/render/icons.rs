//! Icons are glyphs of the system "Segoe Fluent Icons" font, so they stay sharp at any size.
//! Windows 10 has only its predecessor, "Segoe MDL2 Assets", with the same code points for
//! nearly all of them ([`in_mdl2`]).

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Icon {
    Play,
    Pause,
    Prev,
    Next,
    Shuffle,
    Repeat,
    Close,
    Add,
    Timer,
    Stopwatch,
    Flag,
    Headphones,
    Cast,
    Speaker,
    Earbuds,
    Headset,
    Monitor,
    Volume,
    Mute,
    Check,
    Back,
    Settings,
    Sync,
    Pencil,
    Prompt,
    Search,
    People,
    Wrench,
    Document,
    Folder,
    Image,
    Video,
    Music,
    Pdf,
    Archive,
    TextDoc,
    Code,
    Download,
    /// An app's notification without an icon of its own.
    Bell,
    /// "Sen yokken": what happened while away.
    History,
    /// The shelf's footer: Paylaş, Metni kopyala, Küçült.
    Share,
    Copy,
    Shrink,
    Link,
    /// Windows 11 Fluent Pin icon (`\u{E718}`).
    Pin,
    /// Windows 11 Fluent Weather icons (`\u{E753}`, `\u{E9C4}`, `\u{E706}`, `\u{E9C8}`).
    Cloud,
    Rain,
    Sun,
    Snow,
}

impl Icon {
    pub const ALL: [Icon; 49] = [
        Icon::Play,
        Icon::Pause,
        Icon::Prev,
        Icon::Next,
        Icon::Shuffle,
        Icon::Repeat,
        Icon::Close,
        Icon::Add,
        Icon::Timer,
        Icon::Stopwatch,
        Icon::Flag,
        Icon::Headphones,
        Icon::Cast,
        Icon::Speaker,
        Icon::Earbuds,
        Icon::Headset,
        Icon::Monitor,
        Icon::Volume,
        Icon::Mute,
        Icon::Check,
        Icon::Back,
        Icon::Settings,
        Icon::Sync,
        Icon::Pencil,
        Icon::Prompt,
        Icon::Search,
        Icon::People,
        Icon::Wrench,
        Icon::Document,
        Icon::Folder,
        Icon::Image,
        Icon::Video,
        Icon::Music,
        Icon::Pdf,
        Icon::Archive,
        Icon::TextDoc,
        Icon::Code,
        Icon::Download,
        Icon::Bell,
        Icon::History,
        Icon::Share,
        Icon::Copy,
        Icon::Shrink,
        Icon::Link,
        Icon::Pin,
        Icon::Cloud,
        Icon::Rain,
        Icon::Sun,
        Icon::Snow,
    ];
}

/// Segoe Fluent Icons code point for an icon (checked visually with the `glyphsheet` tool).
/// Transport controls use the Solid variants, which match Fluent's filled glyphs.
pub fn glyph(i: Icon) -> char {
    match i {
        Icon::Play => '\u{F5B0}',
        Icon::Pause => '\u{F8AE}',
        Icon::Prev => '\u{F8AC}',
        Icon::Next => '\u{F8AD}',
        Icon::Shuffle => '\u{E8B1}',
        Icon::Repeat => '\u{E8EE}',
        Icon::Close => '\u{E711}',
        Icon::Add => '\u{E710}',
        Icon::Timer => '\u{E823}',
        Icon::Stopwatch => '\u{E916}',
        Icon::Flag => '\u{E7C1}',
        Icon::Headphones => '\u{E7F6}',
        Icon::Cast => '\u{EC15}',
        Icon::Speaker => '\u{E7F5}',
        Icon::Earbuds => '\u{F4C0}',
        Icon::Headset => '\u{E95B}',
        Icon::Monitor => '\u{E7F4}',
        Icon::Volume => '\u{E767}',
        Icon::Mute => '\u{E74F}',
        Icon::Check => '\u{E73E}',
        Icon::Back => '\u{E76B}',
        Icon::Settings => '\u{E713}',
        Icon::Sync => '\u{E895}',
        Icon::Pencil => '\u{E70F}',
        Icon::Prompt => '\u{E756}',
        Icon::Search => '\u{E721}',
        Icon::People => '\u{E716}',
        Icon::Wrench => '\u{E90F}',
        Icon::Document => '\u{E8A5}',
        Icon::Folder => '\u{E8B7}',
        Icon::Image => '\u{EB9F}',
        Icon::Video => '\u{E714}',
        Icon::Music => '\u{EC4F}',
        Icon::Pdf => '\u{EA90}',
        Icon::Archive => '\u{F012}',
        Icon::TextDoc => '\u{F000}',
        Icon::Code => '\u{E943}',
        Icon::Download => '\u{E896}',
        Icon::Bell => '\u{EA8F}',
        Icon::History => '\u{E81C}',
        Icon::Share => '\u{E72D}',
        Icon::Copy => '\u{E8C8}',
        // BackToWindow: two arrows coming in.
        Icon::Shrink => '\u{E73F}',
        Icon::Link => '\u{E71B}',
        Icon::Pin => '\u{E718}',
        Icon::Cloud => '\u{E753}',
        Icon::Rain => '\u{E708}',
        Icon::Sun => '\u{E706}',
        Icon::Snow => '\u{E945}',
    }
}

pub const PIN_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="none" stroke="currentColor" stroke-width="1.7" stroke-linecap="round" stroke-linejoin="round" d="M12 17v5M8 3h8l-1 7 3 4H6l3-4z"/></svg>"##;

pub const SPOTIFY_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="#1ED760" d="M12 0C5.37 0 0 5.37 0 12s5.37 12 12 12 12-5.37 12-12S18.63 0 12 0zm5.5 17.3c-.22.36-.68.47-1.05.25-2.87-1.75-6.49-2.15-10.75-1.18-.41.09-.82-.16-.91-.57-.09-.41.16-.82.57-.91 4.67-1.06 8.67-.61 11.89 1.36.37.22.48.69.25 1.05zm1.47-3.26c-.28.45-.87.6-1.32.32-3.29-2.02-8.3-2.61-12.19-1.42-.51.15-1.05-.14-1.2-.65-.15-.51.14-1.05.65-1.2 4.45-1.35 9.97-.7 13.74 1.62.45.28.6.87.32 1.33zm.13-3.4c-3.95-2.35-10.45-2.57-14.22-1.42-.6.18-1.24-.16-1.43-.77-.18-.6.16-1.24.77-1.43 4.33-1.31 11.51-1.06 16.05 1.63.55.32.72 1.03.4 1.58-.32.55-1.03.73-1.57.41z"/></svg>"##;

pub const YOUTUBE_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path fill="#FF0000" d="M23.498 6.186a3.016 3.016 0 0 0-2.122-2.136C19.505 3.545 12 3.545 12 3.545s-7.505 0-9.377.505A3.017 3.017 0 0 0 .502 6.186C0 8.07 0 12 0 12s0 3.93.502 5.814a3.016 3.016 0 0 0 2.122 2.136c1.871.505 9.376.505 9.376.505s7.505 0 9.377-.505a3.015 3.015 0 0 0 2.122-2.136C24 15.93 24 12 24 12s0-3.93-.502-5.814z"/><polygon points="9.545,15.568 15.818,12 9.545,8.432" fill="#FFFFFF"/></svg>"##;

pub const YTMUSIC_SVG: &str = YOUTUBE_SVG;

pub const WEATHER_CLOUD_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M10 14h8a3.5 3.5 0 0 0 .5-6.96A5 5 0 0 0 9.2 8.5 3.5 3.5 0 0 0 10 14z" fill="#90A4AE"/><path d="M6.5 19h11a4 4 0 0 0 .6-7.95A6 6 0 0 0 5.7 10.6 4.2 4.2 0 0 0 6.5 19z" fill="#CFD8DC"/><path d="M6 18h10.5a3.8 3.8 0 0 0 .5-7.55A5.7 5.7 0 0 0 5.2 10 4 4 0 0 0 6 18z" fill="#ECEFF1"/></svg>"##;

pub const WEATHER_PARTLY_SUNNY_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><circle cx="15.5" cy="8.5" r="4.2" fill="#FFA000"/><circle cx="15.5" cy="8.5" r="3" fill="#FFC107"/><path d="M15.5 2.5v1.8M21.5 8.5h-1.8M19.7 4.3l-1.3 1.3M19.7 12.7l-1.3-1.3M11.3 4.3l1.3 1.3" stroke="#FFA000" stroke-width="1.8" stroke-linecap="round"/><path d="M7 19.5h10.5a3.8 3.8 0 0 0 .6-7.55A5.7 5.7 0 0 0 6.2 11.5 4.2 4.2 0 0 0 7 19.5z" fill="#B0BEC5"/><path d="M6.5 18.5h10a3.6 3.6 0 0 0 .5-7.16A5.4 5.4 0 0 0 5.7 11 4 4 0 0 0 6.5 18.5z" fill="#ECEFF1"/></svg>"##;

pub const WEATHER_SUN_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><circle cx="12" cy="12" r="5" fill="#FFA000"/><circle cx="12" cy="12" r="3.8" fill="#FFC107"/><path d="M12 2.5v2.2M12 19.3v2.2M2.5 12h2.2M19.3 12h2.2M5.3 5.3l1.6 1.6M17.1 17.1l1.6 1.6M18.7 5.3l-1.6 1.6M6.9 17.1l-1.6 1.6" stroke="#FFA000" stroke-width="2" stroke-linecap="round"/></svg>"##;

pub const WEATHER_RAIN_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M6.5 14h11a3.8 3.8 0 0 0 .6-7.55A5.7 5.7 0 0 0 5.7 8 4 4 0 0 0 6.5 14z" fill="#78909C"/><path d="M6 13h10.5a3.6 3.6 0 0 0 .5-7.16A5.4 5.4 0 0 0 5.2 7.5 3.8 3.8 0 0 0 6 13z" fill="#B0BEC5"/><path d="M8.5 16.5l-1.5 4M12.5 16.5l-1.5 4M16.5 16.5l-1.5 4" stroke="#29B6F6" stroke-width="2" stroke-linecap="round"/></svg>"##;

pub const WEATHER_SUN_RAIN_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><circle cx="15.5" cy="7.5" r="3.5" fill="#FFA000"/><path d="M15.5 2.5v1.5M20.5 7.5h-1.5M19 4l-1.1 1.1" stroke="#FFA000" stroke-width="1.6" stroke-linecap="round"/><path d="M6 14h10a3.5 3.5 0 0 0 .5-6.96A5 5 0 0 0 5.2 8.5 3.5 3.5 0 0 0 6 14z" fill="#90A4AE"/><path d="M5.5 13.2h9.5a3.3 3.3 0 0 0 .5-6.56A4.8 4.8 0 0 0 4.8 8 3.3 3.3 0 0 0 5.5 13.2z" fill="#CFD8DC"/><path d="M8 16.5l-1.2 3.5M12.5 16.5l-1.2 3.5" stroke="#29B6F6" stroke-width="1.8" stroke-linecap="round"/></svg>"##;

pub const WEATHER_MOON_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M15.5 3.5a8.5 8.5 0 1 0 5 13.5 8 8 0 0 1-9-9 8 8 0 0 1 4-4.5z" fill="#FFD54F"/><circle cx="9.8" cy="11.5" r="1.1" fill="#FFCA28" opacity="0.6"/><circle cx="12.5" cy="15" r="0.9" fill="#FFCA28" opacity="0.6"/><path d="M18.5 4.5l.5 1.5 1.5.5-1.5.5-.5 1.5-.5-1.5-1.5-.5 1.5-.5z" fill="#FFF59D"/></svg>"##;

pub const WEATHER_SNOW_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M6.5 14h11a3.8 3.8 0 0 0 .6-7.55A5.7 5.7 0 0 0 5.7 8 4 4 0 0 0 6.5 14z" fill="#78909C"/><path d="M6 13h10.5a3.6 3.6 0 0 0 .5-7.16A5.4 5.4 0 0 0 5.2 7.5 3.8 3.8 0 0 0 6 13z" fill="#B0BEC5"/><g stroke="#80DEEA" stroke-width="1.4" stroke-linecap="round"><path d="M8 17v3.5M6.3 18.7h3.4M12.5 17v3.5M10.8 18.7h3.4M17 17v3.5M15.3 18.7h3.4"/></g></svg>"##;

pub const WEATHER_STORM_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M6.5 14h11a3.8 3.8 0 0 0 .6-7.55A5.7 5.7 0 0 0 5.7 8 4 4 0 0 0 6.5 14z" fill="#37474F"/><path d="M6 13h10.5a3.6 3.6 0 0 0 .5-7.16A5.4 5.4 0 0 0 5.2 7.5 3.8 3.8 0 0 0 6 13z" fill="#455A64"/><path d="M12.5 12l-2.8 4.2h3.2l-1.4 4.8 4.2-5.5h-3.2l1.8-3.5z" fill="#FFD600" stroke="#FFEA00" stroke-width="0.5"/></svg>"##;

pub const WEATHER_FOG_SVG: &str = r##"<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 24 24"><path d="M5 8h14M3 11.5h18M5.5 15h13M7 18.5h10" stroke="#CFD8DC" stroke-width="2" stroke-linecap="round" opacity="0.9"/></svg>"##;

/// Maps a weather symbol string to its crisp multi-color Fluent SVG weather icon.
pub fn weather_svg_for_symbol(sym: &str) -> &'static str {
    if sym.contains('⛈') {
        WEATHER_STORM_SVG
    } else if sym.contains('🌦') {
        WEATHER_SUN_RAIN_SVG
    } else if sym.contains('🌧') || sym.contains('\u{E9C4}') {
        WEATHER_RAIN_SVG
    } else if sym.contains('❄') || sym.contains('\u{E9C8}') {
        WEATHER_SNOW_SVG
    } else if sym.contains('🌙') || sym.contains('\u{E708}') {
        WEATHER_MOON_SVG
    } else if sym.contains('⛅') || sym.contains('🌤') {
        WEATHER_PARTLY_SUNNY_SVG
    } else if sym.contains('☀') || sym.contains('\u{E706}') {
        WEATHER_SUN_SVG
    } else if sym.contains('🌫') {
        WEATHER_FOG_SVG
    } else {
        WEATHER_CLOUD_SVG
    }
}

/// Maps a weather symbol string (emoji or Fluent icon) to its Segoe Fluent Icons [`Icon`] so the UI
/// draws crisp monochrome glyphs (`#D6D6D6`) instead of emoji.
pub fn weather_icon_for_symbol(sym: &str) -> Icon {
    if sym.contains('🌧') || sym.contains('🌦') || sym.contains('⛈') || sym.contains('\u{E9C4}') {
        Icon::Rain
    } else if sym.contains('❄') || sym.contains('\u{E9C8}') {
        Icon::Snow
    } else if sym.contains('☀') || sym.contains('\u{E706}') {
        Icon::Sun
    } else {
        Icon::Cloud
    }
}

/// The code point to draw `c` with in Segoe MDL2 Assets: its own, but where Windows 10's MDL2
/// lacks a newer Fluent glyph (Microsoft's MDL2 list has no U+F012) or draws it otherwise.
/// ui/kit.slint's Icon does the same for the folder.
pub fn in_mdl2(c: char) -> char {
    match c {
        // ZipFolder: MDL2's Package, a box.
        '\u{F012}' => '\u{E7B8}',
        // MDL2's Folder stands upright; its FolderHorizontal is Fluent's Folder.
        '\u{E8B7}' => '\u{F12B}',
        _ => c,
    }
}

/// The size to draw `c` at in Segoe MDL2 Assets, as a factor on Fluent's: its batteries (made
/// for a phone's status bar) are wider than Fluent's 1 em, and come down to it.
pub fn mdl2_scale(c: char) -> f32 {
    match c {
        '\u{EBA0}'..='\u{EBAA}' => 1.0 / 1.3125,
        '\u{EBAB}'..='\u{EBB5}' => 1.0 / 1.4375,
        _ => 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Everything the app draws checked in both fonts with `glyphsheet` (`NOWBAR_FONTS=win10`).
    /// MDL2's batteries (made for a phone's status bar) are 1.31 em wide, charging 1.44 em;
    /// Fluent's 1 em, which the pills are laid out for.
    #[test]
    fn mdl2_batteries_draw_as_wide_as_fluents() {
        assert_eq!(mdl2_scale('\u{EBA0}'), 1.0 / 1.3125, "Battery0");
        assert_eq!(mdl2_scale('\u{EBAA}'), 1.0 / 1.3125, "Battery10");
        assert_eq!(mdl2_scale('\u{EBAB}'), 1.0 / 1.4375, "BatteryCharging0");
        assert_eq!(mdl2_scale('\u{EBB5}'), 1.0 / 1.4375, "BatteryCharging10");
        assert_eq!(mdl2_scale(glyph(Icon::Volume)), 1.0);
        assert_eq!(mdl2_scale('\u{EBB6}'), 1.0);
    }

    #[test]
    fn mdl2_draws_its_own_where_it_lacks_fluents() {
        assert_eq!(in_mdl2(glyph(Icon::Archive)), '\u{E7B8}', "the zip folder: a box");
        assert_eq!(in_mdl2(glyph(Icon::Folder)), '\u{F12B}', "lying, like Fluent's");
        for i in Icon::ALL.iter().filter(|i| ![Icon::Archive, Icon::Folder].contains(i)) {
            assert_eq!(in_mdl2(glyph(*i)), glyph(*i), "{i:?}");
        }
        assert_eq!(in_mdl2('\u{EBA5}'), '\u{EBA5}', "a battery level");
        assert_eq!(in_mdl2('a'), 'a');
    }

    #[test]
    fn all_distinct() {
        let all = Icon::ALL;
        for (i, a) in all.iter().enumerate() {
            let g = glyph(*a) as u32;
            assert!((0xE000..=0xF8FF).contains(&g), "{a:?} outside the private use area");
            for b in &all[i + 1..] {
                assert_ne!(glyph(*a), glyph(*b), "{a:?} and {b:?} share a glyph");
            }
        }
    }
}
