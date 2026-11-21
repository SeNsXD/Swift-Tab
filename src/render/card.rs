//! The big One UI cards above the taskbar. Each card paints its content into one surface and
//! records where its buttons are, so clicks can be mapped back to actions.

use windows::core::Result;
use windows::UI::Composition::CompositionDrawingSurface;
use windows::Win32::Graphics::Direct2D::Common::D2D_RECT_F;

use super::gfx::{neutral_icon, new_surface, paint_surface, rect, rgba, sec_text, white, Face, Gfx, Ink, TextStyle, Weight};
use super::icons::{glyph, weather_svg_for_symbol, Icon, PIN_SVG, SPOTIFY_SVG, YOUTUBE_SVG};
use super::layout::Scale;
use super::color::{media_gradient, tint_mix_24};
use super::pill::{
    alert_style, lighten, paint_app_icon, paint_live_mark, store_accent, timer_accent, BUDS_ACCENT, CLAUDE_ACCENT, CLAUDE_ICON, LOW_ACCENT, RAIN_ACCENT, RAIN_ICON, SNOW_ACCENT, SNOW_ICON,
    WELLBEING_ACCENT, WELLBEING_GLYPH,
};
use crate::activities::away::AwayKind;
use crate::activities::live::AppIcon;
use crate::activities::games::Store;
use crate::activities::media::IDLE_MEDIA;
use crate::activities::weather_alert::AlertKind;
use crate::activities::claude::StepKind;
use crate::activities::live::LiveState;
use crate::activities::buds::{low_battery, percent, BudsSnapshot, NoiseControl};
use crate::activities::weather::{fx_accent, WeatherFx};
use crate::activities::football::{EventKind, MatchEvent, Side};
use crate::platform::audio::{OutputBus, OutputKind};
use crate::i18n::tr;
use super::color::TeamPalette;
use super::crest::Crest;
use crate::activities::media::{format_media_time, shown_position, MediaSnapshot, SeekHold};
pub use crate::activities::shelf::{FileKind, ShelfOp};

#[derive(Clone, Debug, PartialEq)]
pub struct TimerCard {
    pub text: String,
    pub label: String,
    pub paused: bool,
    pub done: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct StopwatchCard {
    pub text: String,
    pub last_lap: Option<String>,
    pub paused: bool,
}

/// The timer/stopwatch picker: which tab, and the time set on the wheel fields.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PickerState {
    pub stopwatch_tab: bool,
    pub h: u32,
    pub m: u32,
    pub s: u32,
}

impl Default for PickerState {
    fn default() -> Self {
        PickerState { stopwatch_tab: false, h: 0, m: 10, s: 0 }
    }
}

impl PickerState {
    /// Turns field 0 (hours, 0–23), 1 (minutes) or 2 (seconds) by `notches`, wrapping around.
    pub fn wheel(&mut self, field: u8, notches: i32) {
        let (value, modulo) = match field {
            0 => (&mut self.h, 24),
            1 => (&mut self.m, 60),
            _ => (&mut self.s, 60),
        };
        *value = (*value as i32 + notches).rem_euclid(modulo) as u32;
    }

    pub fn preset(&mut self, minutes: u32) {
        *self = PickerState { stopwatch_tab: false, h: 0, m: minutes, s: 0 };
    }

    pub fn total_ms(&self) -> u64 {
        ((self.h * 3600 + self.m * 60 + self.s) as u64) * 1000
    }
}


/// The media card: the snapshot, the position to draw (interpolated by the app), the output
/// device name, a seek preview while the thumb is dragged, and (index, count) of the Swift Tab
/// items for the page dots.
#[derive(Clone, Debug, PartialEq)]
pub struct MediaCard {
    pub m: MediaSnapshot,
    pub position: Option<i64>,
    pub output: Option<String>,
    pub seeking: Option<f32>,
    pub pages: (usize, usize),
    pub page: MediaPage,
    /// Synced lyrics around the position, when the track has them and the setting is on.
    pub lyrics: Option<LyricsView>,
    /// The cover shows the lyrics (the "Sözler" chip switches).
    pub show_lyrics: bool,
    /// Pinned: the card stays open until the pin or the pill is pressed.
    pub pinned: bool,
    /// Per-app volume (level 0.0..1.0, muted bool)
    pub app_volume: Option<(f32, bool)>,
}

/// The lyric lines around the position: the one before (dim), the one on, the next (dim), and
/// the one before that, leaving at the top while a new line comes in.
#[derive(Clone, Debug, PartialEq)]
pub struct LyricsView {
    pub before: Option<String>,
    pub prev: Option<String>,
    /// `None` before the first line starts.
    pub current: Option<String>,
    pub next: Option<String>,
    /// How far the lines have moved up since the current one started (0 → 1, then at rest).
    pub progress: f32,
}

/// A new lyric line takes this long to move into place.
pub const LYRIC_MOVE_MS: u64 = 420;

/// What the media card shows: the player, or our output picker in its place.
#[derive(Clone, Debug, PartialEq)]
pub enum MediaPage {
    Player,
    Output(Box<OutputPage>),
}

/// How a device row's second line reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SubTone {
    Normal,
    Busy,
    Failed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OutputRow {
    pub name: String,
    pub kind: OutputKind,
    pub earbuds: bool,
    pub sub: String,
    pub tone: SubTone,
}

/// The output picker: the current device first (with the volume), then the others.
#[derive(Clone, Debug, PartialEq)]
pub struct OutputPage {
    pub subtitle: String,
    pub rows: Vec<OutputRow>,
    pub volume: Option<f32>,
    pub muted: bool,
    pub scroll: usize,
}

/// Other devices shown under the current one; more scroll with the wheel.
pub const OUTPUT_VISIBLE_ROWS: usize = 3;

pub fn clamp_scroll(scroll: i32, others: usize) -> usize {
    scroll.clamp(0, others.saturating_sub(OUTPUT_VISIBLE_ROWS) as i32) as usize
}

/// Where `x` falls on a track from `x0` to `x1`, 0–1.
pub fn fraction_on(x: f32, x0: f32, x1: f32) -> f32 {
    ((x - x0) / (x1 - x0)).clamp(0.0, 1.0)
}

/// A device row's second line: how it's attached and, for Galaxy Buds, their battery.
pub fn output_subline(bus: OutputBus, buds: Option<&BudsSnapshot>) -> String {
    let attached = match bus {
        OutputBus::Bluetooth => "Bluetooth",
        OutputBus::Usb => "USB",
        OutputBus::Internal => "Built-in",
        OutputBus::Virtual => "Virtual",
    };
    match buds {
        Some(b) => format!("{} · Left {} · Right {}", attached, percent(b.left), percent(b.right)),
        None => attached.to_string(),
    }
}

fn output_icon(kind: OutputKind, earbuds: bool) -> Icon {
    match kind {
        OutputKind::Bluetooth if earbuds => Icon::Earbuds,
        OutputKind::Bluetooth | OutputKind::Headphones => Icon::Headphones,
        OutputKind::Headset => Icon::Headset,
        OutputKind::Speakers => Icon::Speaker,
        OutputKind::Display => Icon::Monitor,
        OutputKind::Other => Icon::Volume,
    }
}

impl MediaCard {
    pub fn new(m: MediaSnapshot, now_filetime: i64, output: Option<String>, seeking: Option<f32>, pages: (usize, usize), hold: Option<SeekHold>) -> Self {
        let position = m.timeline.as_ref().and_then(|t| shown_position(t, hold, now_filetime));
        MediaCard { m, position, output, seeking, pages, page: MediaPage::Player, lyrics: None, show_lyrics: false, pinned: false, app_volume: None }
    }
}

/// Width of the app name on the media card: from 42 DIP up to 8 DIP before the output chip,
/// which ends 16 DIP from the right edge of the 340 DIP card.
pub fn media_app_name_width_dip(chip_w_dip: Option<f32>) -> f32 {
    let right = 340.0 - 16.0 - chip_w_dip.map_or(0.0, |w| w + 8.0);
    (right - 42.0).max(0.0)
}

/// An app's item (the API): its icon and title, who sent it (the name it gave and the exe it
/// runs from, which it can't change), its text, progress and buttons; or, `prompt`, the question
/// whether it may show anything at all (`text`: the exe's path).
#[derive(Clone, Debug, PartialEq)]
pub struct AppCard {
    pub key: u32,
    pub title: String,
    /// Who sent it: the name it gave and its exe's file name (never cut: the name gives way).
    pub name: String,
    pub exe: String,
    pub text: String,
    /// "%40", "4:12", "Bitti"; empty for a notification.
    pub status: String,
    pub progress: Option<u8>,
    /// A picture at [`APP_CARD_ICON_DIP`]'s pixels, or a glyph.
    pub icon: Option<AppIcon>,
    pub accent: (u8, u8, u8),
    pub buttons: Vec<String>,
    pub prompt: bool,
}

/// An app's icon on its card, in DIP.
pub const APP_CARD_ICON_DIP: f32 = 40.0;

/// Where an app card's buttons go (left and right, px): right-aligned, 8 DIP apart, each its
/// label (`label_px`, measured) plus 32 DIP and 72 DIP at least, and all of them clear of
/// "Sustur" (16 + 84 DIP and a gap): long labels share what is left and get cut.
pub fn app_button_spans(w: f32, label_px: &[f32], s: &Scale) -> Vec<(f32, f32)> {
    let d = |v: f32| v * s.0;
    let n = label_px.len() as f32;
    let room = w - d(16.0) - d(16.0 + 84.0 + 8.0) - d(8.0) * (n - 1.0).max(0.0);
    let natural: Vec<f32> = label_px.iter().map(|l| (l + d(32.0)).max(d(72.0))).collect();
    let widths: Vec<f32> = if natural.iter().sum::<f32>() > room { vec![room / n; label_px.len()] } else { natural };
    let mut right = w - d(16.0);
    let mut spans: Vec<(f32, f32)> = widths.iter().rev().map(|w| {
        let span = (right - w, right);
        right -= w + d(8.0);
        span
    }).collect();
    spans.reverse();
    spans
}

/// The lines an app's text takes on its card (up to three; about 44 characters a line, the
/// painter cuts what doesn't fit).
pub fn app_text_lines(text: &str) -> u8 {
    match text.chars().count() {
        0 => 0,
        n => n.div_ceil(44).clamp(1, 3) as u8,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct DownloadCard {
    pub name: String,
    pub path: std::path::PathBuf,
    pub speed: String,
    pub status: String,
    pub size_text: String,
    pub progress: Option<f32>,
    pub done: bool,
    pub app_name: String,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CardContent {
    Claude(Box<ClaudeCard>),
    App(Box<AppCard>),
    Live(LiveCard),
    Timer(TimerCard),
    Stopwatch(StopwatchCard),
    Picker(PickerState),
    Media(Box<MediaCard>),
    Buds(BudsSnapshot),
    Match(Box<MatchCard>),
    Weather(Box<WeatherCard>),
    Shelf(ShelfCard),
    Clipboard(ClipboardCard),
    Brief(Box<BriefCard>),
    Game(Box<GameCard>),
    Wellbeing(Box<WellbeingCard>),
    Away(Box<AwayCard>),
    Music(Box<MusicCard>),
    Question(Box<QuestionCard>),
    Download(DownloadCard),
    Note(Box<NoteCard>),
    NotesList(Box<NotesListCard>),
}

/// Floating sticky note card.
#[derive(Clone, Debug, PartialEq)]
pub struct NoteCard {
    pub id: u64,
    pub title: String,
    pub text: String,
    pub caret: usize,
    pub caret_visible: bool,
    pub pinned: bool,
    pub thumb: Option<Crest>,
}

/// Stored sticky notes list card.
#[derive(Clone, Debug, PartialEq)]
pub struct NotesListCard {
    pub notes: Vec<NoteListItem>,
    pub hover: Option<usize>,
    pub scroll: f32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct NoteListItem {
    pub id: u64,
    pub title: String,
    pub preview: String,
    pub time_str: String,
    pub pinned: bool,
    pub thumb: Option<Crest>,
}

/// "Sen yokken": what happened while the PC was locked.
#[derive(Clone, Debug, PartialEq)]
pub struct AwayCard {
    /// "14:02 – 17:14 · 3 sa 12 dk"
    pub span: String,
    /// Newest first, at most `away::CARD_ROWS`.
    pub rows: Vec<AwayRow>,
    /// The older ones left out: "+3 daha".
    pub more: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AwayRow {
    pub kind: AwayKind,
    /// "Claude bitti · now-bar"
    pub text: String,
    /// "17:05"
    pub time: String,
    /// A match row's followed team's crest, at the row mark's pixels.
    pub crest: Option<Crest>,
}

// The away card's parts (DIP): a row, the "+3 daha" line.
const AWAY_ROW: f32 = 44.0;
const AWAY_MORE: f32 = 28.0;

/// The rows' panel: 8 above and below, each row, the "+N daha" line.
fn away_panel_dip(c: &AwayCard) -> f32 {
    16.0 + c.rows.len().max(1) as f32 * AWAY_ROW + if c.more > 0 { AWAY_MORE } else { 0.0 }
}

/// Dijital Denge: today's total, the week, the apps used longest.
#[derive(Clone, Debug, PartialEq)]
pub struct WellbeingCard {
    /// "3 sa 40 dk"
    pub total: String,
    /// "Bugün · dünden 25 dk az"
    pub caption: String,
    /// Each day's total as a share of the week's longest, oldest first (today last).
    pub week: [f32; 7],
    /// "Pt" … "Pz" under the bars.
    pub days: [&'static str; 7],
    /// At most five.
    pub apps: Vec<WellbeingApp>,
    /// The week's music row: ("Müzik · 6 sa 20 dk", "Şımarık · Tarkan"), its card behind it.
    pub music: Option<(String, String)>,
}

/// The week's music: how long, the tracks played most, the artists listened to longest.
#[derive(Clone, Debug, PartialEq)]
pub struct MusicCard {
    /// "6 sa 20 dk"
    pub total: String,
    /// "142 parça · en çok Tarkan"
    pub caption: String,
    /// At most five, most played first.
    pub tracks: Vec<MusicRow>,
    /// At most three: (name, "2 sa 10 dk").
    pub artists: Vec<(String, String)>,
    /// The top track's cover colour (the mark's).
    pub accent: (u8, u8, u8),
}

#[derive(Clone, Debug, PartialEq)]
pub struct MusicRow {
    pub title: String,
    pub artist: String,
    /// "12 kez"
    pub plays: String,
    pub accent: (u8, u8, u8),
}

/// A quick question to Claude: the field as typed, then the answer as it's written.
#[derive(Clone, Debug, PartialEq)]
pub struct QuestionCard {
    pub question: String,
    /// The caret's blink: on or off now.
    pub caret: bool,
    pub show: QuestionShow,
    /// The answer so far, or what went wrong.
    pub answer: String,
    /// The answer's area, in lines (from its estimate); the card grows with it.
    pub lines: usize,
    /// How far the answer is scrolled (px); `None` follows its end.
    pub scroll: Option<f32>,
    /// The copy chip's label once the answer is whole ("Kopyala", or "Kopyalandı" for a moment).
    pub copy: Option<String>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QuestionShow {
    /// Not asked yet: the field alone.
    Idle,
    Thinking,
    Answer,
    /// What went wrong, in the answer's place.
    Problem,
}

// The question card's parts (DIP): the header, the field, a line of the answer, the most lines.
const QUESTION_HEADER: f32 = 52.0;
const QUESTION_FIELD: f32 = 40.0;
pub const QUESTION_LINE: f32 = 19.0;
pub const QUESTION_MAX_LINES: usize = 14;

fn question_height_dip(c: &QuestionCard) -> f32 {
    match c.show {
        QuestionShow::Idle => QUESTION_HEADER + QUESTION_FIELD + 16.0,
        _ => {
            let footer = if c.copy.is_some() { 10.0 + 28.0 + 14.0 } else { 16.0 };
            QUESTION_HEADER + QUESTION_FIELD + 12.0 + c.lines.clamp(1, QUESTION_MAX_LINES) as f32 * QUESTION_LINE + 8.0 + footer
        }
    }
}

/// Music's colour: the shelf's audio files' pink.
pub const MUSIC_ACCENT: (u8, u8, u8) = (0xFF, 0x5E, 0x8A);

// The music card's parts (DIP): a track's row, an artist's.
const MUSIC_ROW: f32 = 44.0;
const ARTIST_ROW: f32 = 40.0;

fn music_height_dip(c: &MusicCard) -> f32 {
    let artists = if c.artists.is_empty() { 0.0 } else { BRIEF_GAP + 16.0 + c.artists.len() as f32 * ARTIST_ROW };
    WELL_HEADER + 16.0 + c.tracks.len().max(1) as f32 * MUSIC_ROW + artists + 12.0
}

#[derive(Clone, Debug, PartialEq)]
pub struct WellbeingApp {
    pub name: String,
    pub icon: Option<Crest>,
    /// "1 sa 50 dk"
    pub time: String,
    /// Its share of the longest app's time.
    pub share: f32,
}

// The Dijital Denge card's parts (DIP): the header, the week's panel, an app's row.
const WELL_HEADER: f32 = 100.0;
const WELL_WEEK: f32 = 104.0;
const WELL_ROW: f32 = 40.0;

fn wellbeing_height_dip(c: &WellbeingCard) -> f32 {
    let music = if c.music.is_some() { BRIEF_GAP + BRIEF_MUSIC } else { 0.0 };
    WELL_HEADER + WELL_WEEK + 10.0 + c.apps.len().max(1) as f32 * WELL_ROW + music + 14.0
}

/// A Steam or Epic game's card: the box art, the store, the title, what it does and the figures.
#[derive(Clone, Debug, PartialEq)]
pub struct GameCard {
    pub store: Store,
    pub title: String,
    /// The tall box art, cut to the card's 3:4 box at its pixel size.
    pub cover: Option<Crest>,
    /// "İndiriliyor · %48", "Duraklatıldı · %18", "Güncelleniyor · 4:12", "Oynuyor · 34 dk",
    /// "Güncellendi".
    pub status: String,
    /// "1,2 GB / 2,5 GB · 18 MB/sn · ~2 dk kaldı", "Başladı 21:10", "Bugün 2 sa 10 dk".
    pub detail: String,
    pub progress: Option<f32>,
    /// The download is paused: its bar goes grey.
    pub paused: bool,
}

/// The game card's box art (DIP): 3:4, as Epic's tall art and close to Steam's capsule.
pub const GAME_COVER_DIP: (f32, f32) = (69.0, 92.0);

/// One shelf item on the card.
#[derive(Clone, Debug, PartialEq)]
pub struct ShelfTile {
    pub name: String,
    pub thumb: Option<Crest>,
    pub kind: FileKind,
    /// Clicked for the footer's chips: the accent ring and selection.
    pub selected: bool,
    /// Pinned to the top PINNED section.
    pub pinned: bool,
}

/// The shelf's grid (newest first); `hover` shows that tile's close button. `chips`: the footer's
/// ops for the selection (or everything), each with its label or, for a moment, its result.
#[derive(Clone, Debug, PartialEq)]
pub struct ShelfCard {
    pub tiles: Vec<ShelfTile>,
    pub hover: Option<usize>,
    pub chips: Vec<(ShelfOp, String)>,
    /// The chip saying its result: alone on the footer while the others don't fit beside it.
    pub said: Option<ShelfOp>,
    /// The pin is on: the shelf stays in the stack while it is empty.
    pub pinned: bool,
    pub scroll: f32,
}

/// One clipboard stash item on the card.
#[derive(Clone, Debug, PartialEq)]
pub struct ClipboardTile {
    pub id: u64,
    pub title: String,
    pub preview: String,
    pub badge: &'static str,
    pub thumb: Option<Crest>,
    pub is_url: bool,
    pub is_image: bool,
    pub has_html: bool,
    pub pinned: bool,
}

/// The clipboard stash card.
#[derive(Clone, Debug, PartialEq)]
pub struct ClipboardCard {
    pub tiles: Vec<ClipboardTile>,
    pub hover: Option<usize>,
    pub toast: Option<String>,
    pub scroll: f32,
}

/// Maximum vertical scroll in DIP when there are more than 9 clipboard items (3 rows).
pub fn clipboard_max_scroll_dip(n: usize) -> f32 {
    let rows = n.min(crate::activities::clipboard_stash::MAX_STASH_ITEMS).div_ceil(3);
    ((rows.saturating_sub(3)) as f32 * 82.0).max(0.0)
}

/// A footer chip's glyph.
pub fn op_icon(op: ShelfOp) -> Icon {
    match op {
        ShelfOp::Share => Icon::Share,
        ShelfOp::CopyText => Icon::Copy,
        ShelfOp::Base64 => Icon::Copy,
        ShelfOp::Shrink => Icon::Shrink,
        ShelfOp::Zip => Icon::Archive,
    }
}

/// The shelf footer's chips, left to right in the 44 DIP footer band under the last row:
/// 12 DIP in, the glyph (14), 6, the label (`label_px` wide), 12 out; 8 DIP apart.
pub fn shelf_chips_at(label_px: &[f32], top: f32, s: &Scale) -> Vec<D2D_RECT_F> {
    let d = |v: f32| (v * s.0).round();
    let mut x = d(20.0);
    label_px
        .iter()
        .map(|w| {
            let chip = D2D_RECT_F { left: x, top, right: x + d(44.0) + w.ceil(), bottom: top + d(crate::activities::shelf::FOOTER_CHIP_DIP) };
            x = chip.right + d(8.0);
            chip
        })
        .collect()
}

pub fn shelf_chips(label_px: &[f32], rows: usize, s: &Scale) -> Vec<D2D_RECT_F> {
    let d = |v: f32| (v * s.0).round();
    let top = d(48.0 + 106.0 * rows as f32 + crate::activities::shelf::FOOTER_GAP_DIP + 8.0);
    shelf_chips_at(label_px, top, s)
}

/// A tile's colour and glyph when it has no preview (Fluent neutral or semantic PDF red).
pub fn kind_style(kind: FileKind) -> ((u8, u8, u8), Icon) {
    match kind {
        FileKind::Image => ((0xD6, 0xD6, 0xD6), Icon::Image),
        FileKind::Video => ((0xD6, 0xD6, 0xD6), Icon::Video),
        FileKind::Audio => ((0xD6, 0xD6, 0xD6), Icon::Music),
        FileKind::Pdf => ((0xFF, 0x99, 0xA4), Icon::Pdf),
        FileKind::Archive => ((0xD6, 0xD6, 0xD6), Icon::Archive),
        FileKind::Document => ((0xD6, 0xD6, 0xD6), Icon::TextDoc),
        FileKind::Code => ((0xD6, 0xD6, 0xD6), Icon::Code),
        FileKind::Folder => ((0xD6, 0xD6, 0xD6), Icon::Folder),
        FileKind::Other => ((0xD6, 0xD6, 0xD6), Icon::Document),
    }
}

/// Where the shelf card's parts are, in px: the tiles, their close buttons, the "Clear" chip.
#[derive(Clone, Debug, PartialEq)]
pub struct ShelfLayout {
    pub tiles: Vec<D2D_RECT_F>,
    pub closes: Vec<D2D_RECT_F>,
    pub clear: D2D_RECT_F,
}

/// Three 104×98 DIP square file tiles a row (min 88px), 9 DIP apart horizontally and 8 DIP
/// vertically, flush with the header's 20 DIP padding, from 48 DIP down; the close button 5 DIP
/// inside the top-right corner.
pub fn shelf_layout(n: usize, s: &Scale) -> ShelfLayout {
    let d = |v: f32| v * s.0;
    let (tile_w, tile_h, close) = (d(104.0).round(), d(98.0).round(), d(20.0).round());
    let (mut tiles, mut closes) = (Vec::new(), Vec::new());
    for i in 0..n.min(crate::activities::shelf::SHELF_MAX) {
        let (col, row) = ((i % 3) as f32, (i / 3) as f32);
        let tile = rect(d(20.0 + col * 113.0).round(), d(48.0 + row * 106.0).round(), tile_w, tile_h);
        closes.push(rect(tile.right - d(5.0).round() - close, tile.top + d(5.0).round(), close, close));
        tiles.push(tile);
    }
    let clear = rect(d(350.0 - 78.0), d(14.0), d(78.0), d(26.0));
    ShelfLayout { tiles, closes, clear }
}

/// Three 104×74 DIP (4:3) clipboard tiles a row, 9 DIP apart horizontally and 8 DIP vertically,
/// flush with the header's 20 DIP padding, from 48 DIP down.
pub fn clipboard_layout(n: usize, s: &Scale) -> ShelfLayout {
    let d = |v: f32| v * s.0;
    let (tile_w, tile_h, close) = (d(104.0).round(), d(74.0).round(), d(20.0).round());
    let (mut tiles, mut closes) = (Vec::new(), Vec::new());
    for i in 0..n.min(crate::activities::clipboard_stash::MAX_STASH_ITEMS) {
        let (col, row) = ((i % 3) as f32, (i / 3) as f32);
        let tile = rect(d(20.0 + col * 113.0).round(), d(48.0 + row * 82.0).round(), tile_w, tile_h);
        closes.push(rect(tile.right - d(5.0).round() - close, tile.top + d(5.0).round(), close, close));
        tiles.push(tile);
    }
    let clear = rect(d(350.0 - 78.0), d(14.0), d(78.0), d(26.0));
    ShelfLayout { tiles, closes, clear }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ShelfSectionHeader {
    pub title: &'static str,
    pub top_dip: f32,
    pub is_pinned: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CategorizedShelfLayout {
    pub tiles: Vec<D2D_RECT_F>,
    pub closes: Vec<D2D_RECT_F>,
    pub headers: Vec<ShelfSectionHeader>,
    pub max_scroll_dip: f32,
    pub clear: D2D_RECT_F,
}

pub fn shelf_categorized_layout(c: &ShelfCard, s: &Scale) -> CategorizedShelfLayout {
    let d = |v: f32| v * s.0;
    let (tile_w, tile_h, close) = (d(96.0).round(), d(88.0).round(), d(18.0).round());
    let mut tiles = vec![D2D_RECT_F::default(); c.tiles.len()];
    let mut closes = vec![D2D_RECT_F::default(); c.tiles.len()];
    let mut headers = Vec::new();

    let pinned_indices: Vec<usize> = (0..c.tiles.len()).filter(|&i| c.tiles[i].pinned).collect();
    let unpinned_indices: Vec<usize> = (0..c.tiles.len()).filter(|&i| !c.tiles[i].pinned).collect();

    let mut y = 48.0;
    if !pinned_indices.is_empty() {
        headers.push(ShelfSectionHeader { title: "PINNED", top_dip: y, is_pinned: true });
        y += 24.0;

        for (local_i, &orig_idx) in pinned_indices.iter().enumerate() {
            let (col, row) = ((local_i % 3) as f32, (local_i / 3) as f32);
            let t = rect(d(16.0 + col * 106.0).round(), d(y + row * 98.0).round(), tile_w, tile_h);
            let cl = rect(t.right - d(4.0).round() - close, t.top + d(4.0).round(), close, close);
            tiles[orig_idx] = t;
            closes[orig_idx] = cl;
        }
        let pinned_rows = pinned_indices.len().div_ceil(3);
        y += pinned_rows as f32 * 98.0 - 10.0 + 14.0;
    }

    if !unpinned_indices.is_empty() {
        headers.push(ShelfSectionHeader { title: "RECENT", top_dip: y, is_pinned: false });
        y += 24.0;

        for (local_i, &orig_idx) in unpinned_indices.iter().enumerate() {
            let (col, row) = ((local_i % 3) as f32, (local_i / 3) as f32);
            let t = rect(d(16.0 + col * 106.0).round(), d(y + row * 98.0).round(), tile_w, tile_h);
            let cl = rect(t.right - d(4.0).round() - close, t.top + d(4.0).round(), close, close);
            tiles[orig_idx] = t;
            closes[orig_idx] = cl;
        }
    }

    let card_h = shelf_card_height_dip(c);
    let needed_h = shelf_content_height(pinned_indices.len(), unpinned_indices.len());
    let max_scroll_dip = (needed_h - card_h).max(0.0);
    let clear = rect(d(340.0 - 16.0 - 68.0), d(14.0), d(68.0), d(26.0));
    CategorizedShelfLayout { tiles, closes, headers, max_scroll_dip, clear }
}

pub fn shelf_categorized_card_height(pinned: usize, unpinned: usize) -> f32 {
    if pinned == 0 && unpinned == 0 {
        return 204.0;
    }
    shelf_content_height(pinned, unpinned).clamp(204.0, 390.0)
}

pub fn shelf_card_height_dip(c: &ShelfCard) -> f32 {
    let pinned = c.tiles.iter().filter(|t| t.selected).count();
    let unpinned = c.tiles.len().saturating_sub(pinned);
    shelf_categorized_card_height(pinned, unpinned)
}

pub fn shelf_content_height(pinned: usize, unpinned: usize) -> f32 {
    if pinned == 0 && unpinned == 0 {
        return 204.0;
    }
    let mut h = 48.0;
    if pinned > 0 {
        h += 24.0 + (pinned.div_ceil(3) as f32 * 98.0 - 10.0);
    }
    if unpinned > 0 {
        if pinned > 0 {
            h += 14.0;
        }
        h += 24.0 + (unpinned.div_ceil(3) as f32 * 98.0 - 10.0);
    }
    h + 8.0 + crate::activities::shelf::FOOTER_BAND_DIP
}

pub fn shelf_categorized_max_scroll(pinned: usize, unpinned: usize) -> f32 {
    let card_h = shelf_categorized_card_height(pinned, unpinned);
    let needed_h = shelf_content_height(pinned, unpinned);
    (needed_h - card_h).max(0.0)
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClipboardSectionHeader {
    pub title: &'static str,
    pub top_dip: f32,
    pub is_pinned: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CategorizedClipboardLayout {
    pub tiles: Vec<D2D_RECT_F>,
    pub closes: Vec<D2D_RECT_F>,
    pub headers: Vec<ClipboardSectionHeader>,
    pub max_scroll_dip: f32,
    pub clear: D2D_RECT_F,
}

pub fn clipboard_categorized_layout(c: &ClipboardCard, s: &Scale) -> CategorizedClipboardLayout {
    let d = |v: f32| v * s.0;
    let (tile_w, tile_h, close) = (d(96.0).round(), d(72.0).round(), d(18.0).round());
    let mut tiles = vec![D2D_RECT_F::default(); c.tiles.len()];
    let mut closes = vec![D2D_RECT_F::default(); c.tiles.len()];
    let mut headers = Vec::new();

    let pinned_indices: Vec<usize> = (0..c.tiles.len()).filter(|&i| c.tiles[i].pinned).collect();
    let unpinned_indices: Vec<usize> = (0..c.tiles.len()).filter(|&i| !c.tiles[i].pinned).collect();

    let mut y = 48.0;
    if !pinned_indices.is_empty() {
        headers.push(ClipboardSectionHeader { title: "PINNED", top_dip: y, is_pinned: true });
        y += 24.0;

        for (local_i, &orig_idx) in pinned_indices.iter().enumerate() {
            let (col, row) = ((local_i % 3) as f32, (local_i / 3) as f32);
            let t = rect(d(16.0 + col * 106.0).round(), d(y + row * 80.0).round(), tile_w, tile_h);
            let cl = rect(t.right - d(4.0).round() - close, t.top + d(4.0).round(), close, close);
            tiles[orig_idx] = t;
            closes[orig_idx] = cl;
        }
        let pinned_rows = pinned_indices.len().div_ceil(3);
        y += pinned_rows as f32 * 80.0 - 8.0 + 14.0;
    }

    if !unpinned_indices.is_empty() {
        headers.push(ClipboardSectionHeader { title: "RECENT", top_dip: y, is_pinned: false });
        y += 24.0;

        for (local_i, &orig_idx) in unpinned_indices.iter().enumerate() {
            let (col, row) = ((local_i % 3) as f32, (local_i / 3) as f32);
            let t = rect(d(16.0 + col * 106.0).round(), d(y + row * 80.0).round(), tile_w, tile_h);
            let cl = rect(t.right - d(4.0).round() - close, t.top + d(4.0).round(), close, close);
            tiles[orig_idx] = t;
            closes[orig_idx] = cl;
        }
    }

    let card_h = clipboard_card_height_dip(c);
    let needed_h = clipboard_content_height(pinned_indices.len(), unpinned_indices.len());
    let max_scroll_dip = (needed_h - card_h).max(0.0);
    let clear = rect(d(340.0 - 16.0 - 68.0), d(14.0), d(68.0), d(26.0));
    CategorizedClipboardLayout { tiles, closes, headers, max_scroll_dip, clear }
}

pub fn clipboard_categorized_card_height(pinned: usize, unpinned: usize) -> f32 {
    if pinned == 0 && unpinned == 0 {
        return 204.0;
    }
    clipboard_content_height(pinned, unpinned).clamp(204.0, 390.0)
}

pub fn clipboard_card_height_dip(c: &ClipboardCard) -> f32 {
    let pinned = c.tiles.iter().filter(|t| t.pinned).count();
    let unpinned = c.tiles.len().saturating_sub(pinned);
    clipboard_categorized_card_height(pinned, unpinned)
}

pub fn clipboard_content_height(pinned: usize, unpinned: usize) -> f32 {
    if pinned == 0 && unpinned == 0 {
        return 204.0;
    }
    let mut h = 48.0;
    if pinned > 0 {
        h += 24.0 + (pinned.div_ceil(3) as f32 * 80.0 - 8.0);
    }
    if unpinned > 0 {
        if pinned > 0 {
            h += 14.0;
        }
        h += 24.0 + (unpinned.div_ceil(3) as f32 * 80.0 - 8.0);
    }
    h + 16.0
}

pub fn clipboard_categorized_max_scroll(pinned: usize, unpinned: usize) -> f32 {
    let card_h = clipboard_categorized_card_height(pinned, unpinned);
    let needed_h = clipboard_content_height(pinned, unpinned);
    (needed_h - card_h).max(0.0)
}

/// A live item's card: the timer card's one-row pill with the item's title and last line.
#[derive(Clone, Debug, PartialEq)]
pub struct LiveCard {
    pub key: u32,
    pub title: String,
    pub text: String,
    /// The time or progress while it runs; empty otherwise (the text says it).
    pub status: String,
    pub progress: Option<u8>,
    pub claude: bool,
    pub state: LiveState,
}

/// A Claude Code session's card: what it does, the task list, what it asks or said at the end.
#[derive(Clone, Debug, PartialEq)]
pub struct ClaudeCard {
    pub key: u32,
    /// The project folder.
    pub title: String,
    pub state: LiveState,
    /// "Çalışıyor · 3:12", "İzin istiyor", "Bitti · 12 dk · 9 dosya", or the error.
    pub status: String,
    /// The last three steps, newest first (the first is what it does now).
    pub steps: Vec<(StepKind, String)>,
    /// (done, total, the task under way).
    pub tasks: Option<(usize, usize, Option<String>)>,
    /// The first line of its last answer, once done.
    pub summary: Option<String>,
    pub ask: Option<ClaudeAsk>,
    /// The window the prompt came from is known: "Pencereye git".
    pub can_focus: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClaudeAsk {
    /// The ask's id as a stack key: the buttons answer this ask, not one that came after.
    pub key: u32,
    pub tool: String,
    pub detail: String,
    /// A hook waits for the card (else the buttons would answer no one).
    pub answerable: bool,
}

/// The glyph a step's line starts with.
pub fn step_icon(kind: StepKind) -> Icon {
    match kind {
        StepKind::Read => Icon::Document,
        StepKind::Edit | StepKind::Write => Icon::Pencil,
        StepKind::Run => Icon::Prompt,
        StepKind::Search => Icon::Search,
        StepKind::Agent => Icon::People,
        StepKind::Tool => Icon::Wrench,
    }
}

/// Now Brief: the day's greeting, weather, the followed teams' matches and the open player.
#[derive(Clone, Debug, PartialEq)]
pub struct BriefCard {
    /// "Günaydın, Ekin"
    pub greeting: String,
    /// "Perşembe, 22 Ekim"
    pub date: String,
    pub weather: Option<BriefWeather>,
    /// At most three.
    pub matches: Vec<BriefMatchRow>,
    pub music: Option<BriefMusic>,
    /// Dijital Denge's row: ("Ekran süresi · 3 sa 40 dk", "Bugün · VS Code 1 sa 50 dk").
    pub screen: Option<(String, String)>,
    /// Just back from a lock: ("Sen yokken · 4 şey", "Claude bitti · now-bar").
    pub away: Option<(String, String)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BriefWeather {
    pub symbol: String,
    pub temp: String,
    /// "Parçalı bulutlu · 11° / 21°"
    pub summary: String,
    /// (hour, symbol, temperature), as the weather card's.
    pub hours: Vec<(String, String, String)>,
    /// "15:00 civarı yağmur"
    pub rain: Option<String>,
    pub fx: WeatherFx,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BriefMatchRow {
    pub home: CardTeam,
    pub away: CardTeam,
    /// "65′ · Canlı", "20:00 · Süper Lig", "2–1 · Dün"
    pub status: String,
    pub live: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BriefMusic {
    pub title: String,
    pub artist: String,
    pub art: Option<std::sync::Arc<Vec<u8>>>,
    pub art_size: (u32, u32),
    pub playing: bool,
}

/// The idle clock's card: the city's weather now and the next hours.
#[derive(Clone, Debug, PartialEq)]
pub struct WeatherCard {
    pub city: String,
    pub symbol: String,
    pub temp: String,
    /// "Açık · En düşük 11° · En yüksek 21°"
    pub summary: String,
    /// (hour "21:00", symbol, temperature "17°")
    pub hours: Vec<(String, String, String)>,
    /// The sky now: the card's tint and its moving background.
    pub fx: WeatherFx,
    /// The most pressing alert that applies: its kind, "Kuvvetli rüzgâr", "65 km/sa · 21:00".
    pub alert: Option<(AlertKind, String, String)>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardTeam {
    pub name: String,
    pub code: String,
    pub palette: TeamPalette,
    /// Scaled to the 46 DIP crest box, in pixels.
    pub crest: Option<Crest>,
}

/// The live-match card (sports notes §4): pre-match, live or full time.
#[derive(Clone, Debug, PartialEq)]
pub struct MatchCard {
    pub competition: String,
    pub home: CardTeam,
    pub away: CardTeam,
    /// `None` before kick-off.
    pub score: Option<(u8, u8)>,
    /// Local kick-off time ("21:30"), shown big before the match.
    pub kickoff: String,
    /// The status capsule: "67′", "18 dk sonra", "Devre arası", "Maç sonu", ...
    pub capsule: String,
    pub live: bool,
    /// A goal moment: the capsule shows "GOL!" on the scorer's flood colour.
    pub goal: Option<Side>,
    pub loser: Option<Side>,
    pub events: Vec<MatchEvent>,
    pub form: Option<(String, String)>,
    pub venue: Option<String>,
    pub has_page: bool,
    /// Basketball: points per quarter, shown instead of the goals.
    pub quarters: Option<(Vec<u8>, Vec<u8>)>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CardAction {
    TimerPause,
    TimerResume,
    TimerClose,
    TimerAddMinute,
    StopwatchPause,
    StopwatchResume,
    StopwatchLap,
    StopwatchClose,
    PickerTab(bool),
    PickPreset(u32),
    PickerStart,
    /// A wheel field of the picker: 0 hours, 1 minutes, 2 seconds.
    PickerField(u8),
    PlayPause,
    Prev,
    Next,
    Shuffle,
    Repeat,
    OutputDevice,
    OutputBack,
    /// A row of the output page (0 = the current device).
    OutputPick(usize),
    OutputMute,
    OutputSettings,
    LaunchSpotify,
    LaunchYouTube,
    MediaAppMute,
    /// The match card's ↗: the match page in the browser.
    MatchOpen,
    /// The media card's "Sözler" / "Kapak" chip.
    Lyrics,
    /// The media card's pin: keeps it open.
    Pin,
    /// A segment of the Buds card's noise control (the mode's byte).
    BudsNoise(u8),
    /// A shelf tile's close button (its index, newest first).
    ShelfRemove(usize),
    /// Pin / unpin a specific shelf item.
    ShelfItemPin(usize),
    /// The shelf card's "Temizle".
    ShelfClear,
    /// Keeps the shelf in the stack while it is empty (and lets go of it again).
    ShelfPin,
    /// A shelf footer chip.
    ShelfTool(ShelfOp),
    /// The live card's ×: the item goes.
    LiveClose,
    /// A Claude card's answers to the permission it waits for, and its "Pencereye git".
    ClaudeAllow(u32),
    ClaudeDeny(u32),
    ClaudeFocus,
    /// An app card's button (the item it was painted for, the button's place).
    AppButton(u32, u8),
    /// A prompt's answers (the item it was painted for).
    AppAllow(u32),
    AppDeny(u32),
    /// "Sustur": the app may show nothing more.
    AppMute(u32),
    /// A match row of the brief (its index).
    BriefMatch(usize),
    /// The brief's music play / pause.
    BriefPlay,
    /// The brief's screen time row: the Dijital Denge card.
    BriefScreen,
    /// The brief's "Sen yokken" row: the away card.
    BriefAway,
    /// Dijital Denge's music row: the week's music card.
    WellbeingMusic,
    /// The question card's "Kopyala": the answer onto the clipboard.
    QuestionCopy,
    /// Clipboard stash clear.
    ClipboardClear,
    /// Clipboard stash item removal.
    ClipboardRemove(usize),
    /// Pin / unpin a clipboard item.
    ClipboardPin(usize),
    /// Re-paste item from clipboard stash.
    ClipboardPaste(usize),
    /// Re-paste item stripped of formatting (plain text only).
    ClipboardPastePlain(usize),
    /// Open the downloading application or the completed file.
    DownloadOpen,
    /// Open the downloads folder in File Explorer.
    DownloadFolder,
    /// Close the floating sticky note.
    NoteClose,
    /// Toggle pin state of sticky note.
    NotePin(u64),
    /// Create a new sticky note.
    NoteNew,
    /// Delete a sticky note.
    NoteDelete(u64),
    /// Copy sticky note text.
    NoteCopy(u64),
    /// Share sticky note.
    NoteShare(u64),
    /// Open a note from the notes list into the floating editor.
    NoteOpen(u64),
    /// Open the all notes list card.
    NoteOpenList,
    /// Clear all notes.
    NoteClearAll,
    /// Remove attached image from note.
    NoteRemoveImage(u64),
}

/// Card size in DIP.
pub fn card_size_dip(c: &CardContent) -> (f32, f32) {
    match c {
        CardContent::Download(_) => (370.0, 164.0),
        CardContent::Timer(_) | CardContent::Stopwatch(_) | CardContent::Live(_) => (370.0, 77.0),
        // The footer only with a button in it.
        CardContent::Claude(c) if c.can_focus || c.ask.as_ref().is_some_and(|a| a.answerable) => (370.0, 236.0),
        CardContent::Claude(_) => (370.0, 192.0),
        CardContent::App(a) if a.prompt => (370.0, 236.0),
        // The header, the text, the bar, the footer.
        CardContent::App(a) => {
            let text = match app_text_lines(&a.text) {
                0 => 0.0,
                n => n as f32 * APP_LINE + 4.0,
            };
            (370.0, APP_BODY_TOP + text + if a.progress.is_some() { 14.0 } else { 0.0 } + 14.0 + 34.0 + 12.0)
        }
        CardContent::Picker(_) => (370.0, 260.0),
        CardContent::Media(m) if m.m.id == IDLE_MEDIA => (340.0, 204.0),
        CardContent::Media(m) if matches!(m.page, MediaPage::Output(_)) => (340.0, 350.0),
        CardContent::Media(_) => (340.0, 320.0),
        // The noise control row once the Buds have said their mode.
        CardContent::Buds(b) if b.noise.is_some() => (370.0, 238.0),
        CardContent::Buds(_) => (370.0, 150.0),
        CardContent::Weather(w) if w.alert.is_some() => (370.0, 196.0 + WEATHER_ALERT_ROW),
        CardContent::Weather(_) => (370.0, 196.0),
        CardContent::Brief(b) => (370.0, brief_height_dip(b)),
        // Without the stat rows (ESPN only has them in the heavy summary): 296; pre-match 290.
        CardContent::Match(m) if m.score.is_none() => (340.0, 290.0),
        CardContent::Match(_) => (340.0, 296.0),
        CardContent::Shelf(c) => (340.0, shelf_card_height_dip(c)),
        CardContent::Clipboard(c) => (340.0, clipboard_card_height_dip(c)),
        CardContent::Game(_) => (370.0, GAME_COVER_DIP.1 + 48.0),
        CardContent::Wellbeing(c) => (370.0, wellbeing_height_dip(c)),
        CardContent::Away(c) => (370.0, BRIEF_HEADER + BRIEF_GAP + away_panel_dip(c) + 12.0),
        CardContent::Music(c) => (370.0, music_height_dip(c)),
        CardContent::Question(c) => (370.0, question_height_dip(c)),
        CardContent::Note(c) => (340.0, if c.thumb.is_some() { 330.0 } else { 240.0 }),
        CardContent::NotesList(_) => (340.0, 320.0),
    }
}

/// Where the weather card's big emoji sits, as fractions of the card: near enough for the light
/// behind it (the temperature beside it is about 56 DIP wide).
pub const WEATHER_ICON_AT: (f32, f32) = (1.0 - (20.0 + 56.0 + 26.0) / 370.0, 36.0 / 196.0);

/// Where a card's sky gathers its light (fractions of the card): on its big weather emoji.
pub fn card_glow_at(c: &CardContent) -> (f32, f32) {
    match c {
        CardContent::Brief(b) => ((20.0 + 14.0) / 370.0, (BRIEF_HEADER + BRIEF_GAP + BRIEF_WEATHER_ROW / 2.0) / brief_height_dip(b)),
        // An alert line makes the card taller under the emoji, which stays put.
        CardContent::Weather(_) => (WEATHER_ICON_AT.0, WEATHER_ICON_AT.1 * 196.0 / card_size_dip(c).1),
        _ => WEATHER_ICON_AT,
    }
}

/// The brief's height: the greeting, then each section it has.
fn brief_height_dip(b: &BriefCard) -> f32 {
    let weather = b.weather.as_ref().map_or(0.0, |w| BRIEF_GAP + BRIEF_WEATHER_ROW + 4.0 + HOURS_PANEL + if w.rain.is_some() { BRIEF_RAIN } else { 0.0 });
    let matches = if b.matches.is_empty() { 0.0 } else { BRIEF_GAP + 16.0 + b.matches.len() as f32 * BRIEF_MATCH_ROW };
    let music = if b.music.is_some() { BRIEF_GAP + BRIEF_MUSIC } else { 0.0 };
    let screen = if b.screen.is_some() { BRIEF_GAP + BRIEF_MUSIC } else { 0.0 };
    let away = if b.away.is_some() { BRIEF_GAP + BRIEF_MUSIC } else { 0.0 };
    BRIEF_HEADER + weather + matches + music + screen + away + 12.0
}

// The brief's sections (DIP): the greeting to the date's bottom, the gap before each section,
// the weather's now row, the rain line, a match row, the music panel.
const BRIEF_HEADER: f32 = 72.0;
const BRIEF_GAP: f32 = 8.0;
const BRIEF_WEATHER_ROW: f32 = 56.0;
const BRIEF_RAIN: f32 = 22.0;
const BRIEF_MATCH_ROW: f32 = 44.0;
const BRIEF_MUSIC: f32 = 60.0;
/// The hours' panel, the weather card's and the brief's.
const HOURS_PANEL: f32 = 100.0;
/// The weather card's alert line under the hours.
const WEATHER_ALERT_ROW: f32 = 34.0;

/// Where an app card's text starts, and a line of it (DIP).
const APP_BODY_TOP: f32 = 74.0;
const APP_LINE: f32 = 18.0;

/// Corner radius in DIP: Windows 11 floating panels and rows use 8 DIP.
pub fn card_radius_dip(_c: &CardContent) -> f32 {
    8.0
}

/// Subtle dark Acrylic base for cards; Media and Weather paint a localized top-left radial tint
/// over the `#202020` base so the card stays dark and crisp like the Windows 11 mockup.
pub fn card_tint(c: &CardContent) -> Option<(windows::UI::Color, windows::UI::Color)> {
    let alpha = if crate::platform::os::solid_fallback() { 255 } else { 64 };
    let base = windows::UI::Color { A: alpha, R: 0x20, G: 0x20, B: 0x20 };
    let _ = tint_mix_24([0, 0, 0]);
    if let CardContent::Media(card) = c {
        let _ = media_gradient(card.m.accent);
    }
    Some((base, base))
}

pub fn card_accent(c: &CardContent) -> (u8, u8, u8) {
    match c {
        CardContent::Timer(_) | CardContent::Stopwatch(_) | CardContent::Picker(_) | CardContent::Shelf(_) | CardContent::Clipboard(_) | CardContent::Away(_) => timer_accent(),
        CardContent::Live(l) if l.claude => CLAUDE_ACCENT,
        CardContent::Claude(_) | CardContent::Question(_) => CLAUDE_ACCENT,
        CardContent::App(a) => a.accent,
        CardContent::Live(_) => timer_accent(),
        CardContent::Media(card) => (card.m.accent[0], card.m.accent[1], card.m.accent[2]),
        CardContent::Buds(_) => BUDS_ACCENT,
        CardContent::Weather(_) => (0x8A, 0x8A, 0x8A),
        CardContent::Brief(b) => b.weather.as_ref().map_or((0x8A, 0x8A, 0x8A), |w| fx_accent(w.fx)),
        CardContent::Match(m) => (m.home.palette.tint[0], m.home.palette.tint[1], m.home.palette.tint[2]),
        CardContent::Game(g) => store_accent(g.store),
        CardContent::Wellbeing(_) => WELLBEING_ACCENT,
        CardContent::Music(c) => c.accent,
        CardContent::Download(_) => (0x00, 0x78, 0xD4),
        CardContent::Note(_) | CardContent::NotesList(_) => (0xF5, 0x9E, 0x0B),
    }
}

pub struct CardVisual {
    pub surface: CompositionDrawingSurface,
    hits: Vec<(D2D_RECT_F, CardAction)>,
    /// Media seek bar: where it takes the mouse, and the x range of the track.
    seek: Option<(D2D_RECT_F, f32, f32)>,
    /// Output page volume slider, the same way.
    volume: Option<(D2D_RECT_F, f32, f32)>,
    /// Active media card per-app volume slider.
    app_volume: Option<(D2D_RECT_F, f32, f32)>,
    /// Clipboard vertical scrollbar track: hit area and vertical y0..y1 thumb center range.
    clipboard_scroll: Option<(D2D_RECT_F, f32, f32)>,
    /// Shelf / clipboard tiles: where a drag-out or hover can happen.
    tiles: Vec<D2D_RECT_F>,
    answer_overflow: f32,
    size_px: (i32, i32),
}

impl CardVisual {
    pub fn new(g: &Gfx) -> Result<Self> {
        Ok(CardVisual {
            surface: new_surface(g, 1, 1)?,
            hits: Vec::new(),
            seek: None,
            volume: None,
            app_volume: None,
            clipboard_scroll: None,
            tiles: Vec::new(),
            answer_overflow: 0.0,
            size_px: (1, 1),
        })
    }

    pub fn size_px(&self) -> (i32, i32) {
        self.size_px
    }

    pub fn set(&mut self, g: &Gfx, c: &CardContent, s: &Scale) -> Result<()> {
        let (wd, hd) = card_size_dip(c);
        let (w, h) = (s.even_px(wd) as i32, s.px(hd) as i32);
        let mut painted = Painted::default();
        paint_surface(g, &self.surface, w, h, |ink| {
            painted = paint_card(ink, c, s, w as f32, h as f32)?;
            Ok(())
        })?;
        self.hits = painted.hits;
        self.seek = painted.seek;
        self.volume = painted.volume;
        self.app_volume = painted.app_volume;
        self.clipboard_scroll = painted.clipboard_scroll;
        self.tiles = painted.tiles;
        self.answer_overflow = painted.answer.unwrap_or(0.0);
        self.size_px = (w, h);
        Ok(())
    }

    /// How far the question card's answer runs past its area (px), as last painted.
    pub fn answer_overflow(&self) -> f32 {
        self.answer_overflow
    }

    /// Seek bar fraction (0–1) under (x, y), if the point is on the seek bar.
    pub fn seek_fraction(&self, x: i32, y: i32) -> Option<f32> {
        track_fraction(self.seek, x, y)
    }

    /// Fraction for a drag that started on the seek bar, wherever the mouse is now.
    pub fn drag_fraction(&self, x: i32) -> Option<f32> {
        self.seek.map(|(_, x0, x1)| fraction_on(x as f32, x0, x1))
    }

    /// Output page: volume fraction under (x, y), if the point is on the slider.
    pub fn volume_fraction(&self, x: i32, y: i32) -> Option<f32> {
        track_fraction(self.volume, x, y)
    }

    pub fn volume_drag(&self, x: i32) -> Option<f32> {
        self.volume.map(|(_, x0, x1)| fraction_on(x as f32, x0, x1))
    }

    /// Media card: app volume fraction under (x, y), if the point is on the slider.
    pub fn app_volume_fraction(&self, x: i32, y: i32) -> Option<f32> {
        track_fraction(self.app_volume, x, y)
    }

    pub fn app_volume_drag(&self, x: i32) -> Option<f32> {
        self.app_volume.map(|(_, x0, x1)| fraction_on(x as f32, x0, x1))
    }

    /// Clipboard vertical scrollbar fraction (0–1) under (x, y), if the point is on the scrollbar.
    pub fn clipboard_scroll_fraction(&self, x: i32, y: i32) -> Option<f32> {
        let (area, y0, y1) = self.clipboard_scroll?;
        let (fx, fy) = (x as f32, y as f32);
        (fx >= area.left && fx < area.right && fy >= area.top && fy < area.bottom).then(|| fraction_on(fy, y0, y1))
    }

    pub fn clipboard_scroll_drag(&self, y: i32) -> Option<f32> {
        self.clipboard_scroll.map(|(_, y0, y1)| fraction_on(y as f32, y0, y1))
    }

    /// The shelf tile under (x, y), if any.
    pub fn tile_at(&self, x: i32, y: i32) -> Option<usize> {
        let (x, y) = (x as f32, y as f32);
        self.tiles.iter().position(|r| x >= r.left && x < r.right && y >= r.top && y < r.bottom)
    }

    /// Action under (x, y), in pixels from the card's top-left.
    pub fn hit_test(&self, x: i32, y: i32) -> Option<CardAction> {
        let (x, y) = (x as f32, y as f32);
        self.hits.iter().find(|(r, _)| x >= r.left && x < r.right && y >= r.top && y < r.bottom).map(|(_, a)| *a)
    }
}

/// What a paint left behind: the buttons and the draggable tracks.
#[derive(Default)]
pub struct Painted {
    hits: Vec<(D2D_RECT_F, CardAction)>,
    seek: Option<(D2D_RECT_F, f32, f32)>,
    volume: Option<(D2D_RECT_F, f32, f32)>,
    app_volume: Option<(D2D_RECT_F, f32, f32)>,
    clipboard_scroll: Option<(D2D_RECT_F, f32, f32)>,
    tiles: Vec<D2D_RECT_F>,
    /// The question card's answer: how far past its area it runs (px).
    answer: Option<f32>,
}

/// Paints `c` into a `w`×`h` px area; dev tools render cards to PNG with it.
pub fn paint_card(ink: &Ink, c: &CardContent, s: &Scale, w: f32, h: f32) -> Result<Painted> {
    let mut out = Painted::default();
    let mut p = Painter {
        ink,
        s,
        w,
        h,
        hits: &mut out.hits,
        seek: &mut out.seek,
        volume: &mut out.volume,
        app_volume: &mut out.app_volume,
        clipboard_scroll: &mut out.clipboard_scroll,
    };
    match c {
        CardContent::Timer(t) => p.timer(t),
        CardContent::Stopwatch(t) => p.stopwatch(t),
        CardContent::Picker(t) => p.picker(t),
        CardContent::Media(t) => match &t.page {
            MediaPage::Player => p.media(t),
            MediaPage::Output(o) => p.output(t, o),
        },
        CardContent::Buds(b) => p.buds(b),
        CardContent::Weather(w) => p.weather(w),
        CardContent::Brief(b) => p.brief(b),
        CardContent::Live(l) => p.live(l),
        CardContent::Claude(c) => p.claude(c),
        CardContent::App(a) => p.app(a),
        CardContent::Match(m) => p.match_card(m),
        CardContent::Shelf(c) => {
            let l = shelf_categorized_layout(c, s);
            let scroll_px = (c.scroll.clamp(0.0, l.max_scroll_dip) * s.0).round();
            let clip_top = (46.0 * s.0).round();
            let clip_bottom = ((shelf_card_height_dip(c) - crate::activities::shelf::FOOTER_BAND_DIP) * s.0).round();
            out.tiles = l
                .tiles
                .into_iter()
                .map(|mut r| {
                    r.top -= scroll_px;
                    r.bottom -= scroll_px;
                    let top = r.top.max(clip_top);
                    let bottom = r.bottom.min(clip_bottom);
                    if bottom <= top {
                        D2D_RECT_F::default()
                    } else {
                        D2D_RECT_F { top, bottom, ..r }
                    }
                })
                .collect();
            p.shelf(c)
        }
        CardContent::Clipboard(c) => {
            let l = clipboard_categorized_layout(c, s);
            let scroll_px = (c.scroll.clamp(0.0, l.max_scroll_dip) * s.0).round();
            let clip_top = (46.0 * s.0).round();
            let clip_bottom = (clipboard_card_height_dip(c) * s.0 - 6.0 * s.0).round();
            out.tiles = l
                .tiles
                .into_iter()
                .map(|mut r| {
                    r.top -= scroll_px;
                    r.bottom -= scroll_px;
                    let top = r.top.max(clip_top);
                    let bottom = r.bottom.min(clip_bottom);
                    if bottom <= top {
                        D2D_RECT_F::default()
                    } else {
                        D2D_RECT_F { top, bottom, ..r }
                    }
                })
                .collect();
            p.clipboard(c)
        }
        CardContent::Game(g) => p.game(g),
        CardContent::Wellbeing(c) => p.wellbeing(c),
        CardContent::Away(c) => p.away(c),
        CardContent::Music(c) => p.music(c),
        CardContent::Question(c) => p.question(c, &mut out.answer),
        CardContent::Download(d) => p.download(d),
        CardContent::Note(c) => p.note(c),
        CardContent::NotesList(c) => p.notes_list(c),
    }?;
    Ok(out)
}

/// The 20 DIP crest of a form row (the pixels are the 46 DIP crest, drawn smaller).
struct PillTeamLike<'a> {
    team: &'a CardTeam,
}

impl PillTeamLike<'_> {
    fn crest(&self, p: &Painter, x: f32, y: f32) -> Result<()> {
        let side = (20.0 * p.s.0).round();
        let circle = rect(x.round(), y.round(), side, side);
        let ([r, g, b], [hr, hg, hb]) = (self.team.palette.flood, self.team.palette.headline);
        match &self.team.crest {
            Some(c) => {
                p.ink.rrect(circle, side / 2.0, white(0.10))?;
                let src = D2D_RECT_F { left: 0.0, top: 0.0, right: c.size.0 as f32, bottom: c.size.1 as f32 };
                let inner = rect(circle.left + side * 0.1, circle.top + side * 0.1, side * 0.8, side * 0.8);
                p.ink.image_rounded(&c.px, c.size, src, inner, 0.0)
            }
            None => {
                p.ink.rrect(circle, side / 2.0, rgba(r, g, b, 1.0))?;
                p.ink.text(&self.team.code, circle, &TextStyle::new(Face::Text, Weight::Bold, side * 0.36, rgba(hr, hg, hb, 1.0)).centered())
            }
        }
    }
}

fn track_fraction(track: Option<(D2D_RECT_F, f32, f32)>, x: i32, y: i32) -> Option<f32> {
    let (area, x0, x1) = track?;
    let (fx, fy) = (x as f32, y as f32);
    (fx >= area.left && fx < area.right && fy >= area.top && fy < area.bottom).then(|| fraction_on(fx, x0, x1))
}

struct Painter<'a, 'b> {
    ink: &'a Ink<'b>,
    s: &'a Scale,
    w: f32,
    h: f32,
    hits: &'a mut Vec<(D2D_RECT_F, CardAction)>,
    seek: &'a mut Option<(D2D_RECT_F, f32, f32)>,
    volume: &'a mut Option<(D2D_RECT_F, f32, f32)>,
    app_volume: &'a mut Option<(D2D_RECT_F, f32, f32)>,
    clipboard_scroll: &'a mut Option<(D2D_RECT_F, f32, f32)>,
}

impl Painter<'_, '_> {
    fn d(&self, v: f32) -> f32 {
        v * self.s.0
    }

    fn accent_circle(&self, icon: Icon) -> Result<()> {
        let (r, g, b) = timer_accent();
        let circle = self.d(42.0);
        let cy = ((self.h - circle) / 2.0).round();
        let area = rect(self.d(15.0), cy, circle, circle);
        self.ink.rrect(area, self.d(4.0), rgba(r, g, b, 1.0))?;
        let st = TextStyle::new(Face::Icons, Weight::Regular, self.d(20.0), rgba(0x00, 0x00, 0x00, 1.0)).centered();
        self.ink.text(&glyph(icon).to_string(), area, &st)
    }

    fn big_time(&self, text: &str, label: &str) -> Result<()> {
        let time = TextStyle::new(Face::Display, Weight::SemiBold, self.d(26.0), white(1.0)).tabular().top();
        self.ink.text(text, rect(self.d(73.0), self.d(10.0), self.d(180.0), self.d(36.0)), &time)?;
        let st = TextStyle::new(Face::Text, Weight::Regular, self.d(13.0), sec_text()).top();
        self.ink.text(label, rect(self.d(74.0), self.d(44.0), self.d(200.0), self.d(20.0)), &st)
    }

    /// An icon button centred `from_right` DIP from the right edge.
    fn icon_button(&mut self, icon: Icon, from_right: f32, action: CardAction) -> Result<()> {
        let area = rect(self.w - self.d(from_right + 20.0), (self.h - self.d(32.0)) / 2.0, self.d(32.0), self.d(32.0));
        self.ink.rrect(area, self.d(4.0), white(0.06))?;
        let st = TextStyle::new(Face::Icons, Weight::Regular, self.d(16.0), white(1.0)).centered();
        self.ink.text(&glyph(icon).to_string(), area, &st)?;
        self.hits.push((area, action));
        Ok(())
    }

    fn chip(&mut self, area: D2D_RECT_F, text: &str, fill: [f32; 4], action: CardAction) -> Result<()> {
        let radius = self.d(4.0);
        self.ink.rrect(area, radius, fill)?;
        let st = TextStyle::new(Face::Text, Weight::SemiBold, self.d(13.0), white(1.0)).centered();
        self.ink.text(text, area, &st)?;
        self.hits.push((area, action));
        Ok(())
    }

    fn timer(&mut self, t: &TimerCard) -> Result<()> {
        self.accent_circle(Icon::Timer)?;
        self.big_time(&t.text, &t.label)?;
        if t.done {
            let y = (self.h - self.d(34.0)) / 2.0;
            let add = rect(self.w - self.d(148.0), y, self.d(64.0), self.d(34.0));
            let add_label = format!("+{} min", crate::settings::current().timer_add_minutes);
            self.chip(add, &add_label, white(0.14), CardAction::TimerAddMinute)?;
            let close = rect(self.w - self.d(78.0), y, self.d(64.0), self.d(34.0));
            self.chip(close, "Dismiss", white(0.14), CardAction::TimerClose)
        } else {
            self.icon_button(Icon::Close, 77.0, CardAction::TimerClose)?;
            let (icon, action) = if t.paused { (Icon::Play, CardAction::TimerResume) } else { (Icon::Pause, CardAction::TimerPause) };
            self.icon_button(icon, 35.0, action)
        }
    }

    /// Claude's mark and the project over its state; a panel with the ask, the steps and the task
    /// list, or the last answer; a footer with "Pencereye git" and the ask's buttons.
    fn claude(&mut self, c: &ClaudeCard) -> Result<()> {
        let d = |v: f32| v * self.s.0;
        let (r, g, b) = CLAUDE_ACCENT;
        let circle = d(40.0).round();
        let dot = rect(d(20.0).round(), d(18.0).round(), circle, circle);
        self.ink.rrect(dot, circle / 2.0, rgba(r, g, b, 1.0))?;
        paint_live_mark(self.ink, dot, true, c.state, d(20.0))?;
        let text_x = d(72.0);
        let text_w = self.w - text_x - d(60.0);
        let title = TextStyle::new(Face::Text, Weight::SemiBold, d(16.0), white(1.0)).top().ellipsis();
        self.ink.text(&c.title, rect(text_x, d(17.0), text_w, d(22.0)), &title)?;
        let status = match c.state {
            LiveState::Wait => TextStyle::new(Face::Text, Weight::SemiBold, d(12.5), lighten(CLAUDE_ACCENT)),
            _ => TextStyle::new(Face::Text, Weight::Regular, d(12.5), white(0.7)).tabular(),
        }
        .top()
        .ellipsis();
        self.ink.text(&c.status, rect(text_x, d(40.0), text_w, d(18.0)), &status)?;
        let close = rect(self.w - d(54.0), d(18.0), d(40.0), d(40.0));
        self.ink.text(&glyph(Icon::Close).to_string(), close, &TextStyle::new(Face::Icons, Weight::Regular, d(15.0), white(0.8)).centered())?;
        self.hits.push((close, CardAction::LiveClose));

        let panel = rect(d(16.0), d(74.0), self.w - d(32.0), d(102.0));
        self.ink.rrect(panel, d(22.0), white(0.06))?;
        let (px, pw) = (panel.left + d(16.0), panel.right - panel.left - d(32.0));
        let line = |size: f32, weight, a: f32| TextStyle::new(Face::Text, weight, d(size), white(a)).top().ellipsis();
        let icon_style = |a: f32| TextStyle::new(Face::Icons, Weight::Regular, d(14.0), [1.0, 1.0, 1.0, a]).centered();
        // The buttons only under a detail shown whole: three lines for it, else two and the hint.
        let detail = TextStyle::new(Face::Text, Weight::Regular, d(12.5), white(0.85)).top().wrap();
        let answerable = match &c.ask {
            Some(ask) if ask.answerable => self.ink.fits(&ask.detail, pw, d(52.0), &detail)?,
            _ => false,
        };
        if let Some(ask) = &c.ask {
            self.ink.text(&format!("Permission request · {}", ask.tool), rect(px, panel.top + d(14.0), pw, d(18.0)), &line(13.0, Weight::SemiBold, 1.0))?;
            if answerable {
                self.ink.text(&ask.detail, rect(px, panel.top + d(38.0), pw, d(52.0)), &detail)?;
            } else {
                self.ink.text(&ask.detail, rect(px, panel.top + d(38.0), pw, d(34.0)), &detail.ellipsis())?;
            }
            if !answerable {
                self.ink.text("Reply in Claude's window", rect(px, panel.top + d(76.0), pw, d(16.0)), &line(12.0, Weight::Regular, 0.55))?;
            }
        } else if let (LiveState::Done, Some(summary)) = (c.state, &c.summary) {
            let wrap = TextStyle::new(Face::Text, Weight::Regular, d(13.0), white(0.9)).top().wrap();
            self.ink.text(summary, rect(px, panel.top + d(14.0), pw, d(74.0)), &wrap)?;
        } else {
            // What it does now, then the task list (or the steps before).
            let mut y = panel.top + d(14.0);
            let row = |p: &mut Self, y: f32, kind: Option<StepKind>, text: &str, size: f32, weight, a: f32| -> Result<()> {
                if let Some(k) = kind {
                    p.ink.text(&glyph(step_icon(k)).to_string(), rect(px - d(2.0), y, d(18.0), d(18.0)), &icon_style(a))?;
                }
                let x = if kind.is_some() { px + d(24.0) } else { px };
                p.ink.text(text, rect(x, y, pw - (x - px), d(18.0)), &line(size, weight, a))
            };
            match c.steps.first() {
                Some((k, label)) => row(self, y, Some(*k), label, 13.0, Weight::SemiBold, 1.0)?,
                None => row(self, y, None, "Thinking…", 13.0, Weight::SemiBold, 1.0)?,
            }
            y += d(26.0);
            if let Some((done, total, current)) = &c.tasks {
                let text = match current {
                    Some(t) => format!("{done}/{total} · {t}"),
                    None => format!("{done}/{total}"),
                };
                row(self, y, None, &text, 12.5, Weight::Regular, 0.8)?;
                let bar = d(3.0).round();
                let track = rect(px, y + d(24.0), pw, bar);
                self.ink.rrect(track, bar / 2.0, white(0.16))?;
                let fill = (pw * *done as f32 / (*total).max(1) as f32).max(bar);
                self.ink.rrect(rect(px, y + d(24.0), fill, bar), bar / 2.0, rgba(r, g, b, 1.0))?;
                y += d(36.0);
                if let Some((k, label)) = c.steps.get(1) {
                    row(self, y, Some(*k), label, 12.0, Weight::Regular, 0.55)?;
                }
            } else {
                for (k, label) in c.steps.iter().skip(1).take(2) {
                    row(self, y, Some(*k), label, 12.0, Weight::Regular, 0.55)?;
                    y += d(22.0);
                }
            }
        }

        // The footer: "Go to window" on the left, the ask's answers on the right.
        let fy = panel.bottom + d(14.0);
        if c.can_focus {
            self.chip(rect(d(16.0), fy, d(124.0), d(34.0)), "Go to window", white(0.12), CardAction::ClaudeFocus)?;
        }
        if let Some(ClaudeAsk { key, .. }) = c.ask.as_ref().filter(|_| answerable) {
            let allow = rect(self.w - d(16.0) - d(96.0), fy, d(96.0), d(34.0));
            let deny = rect(allow.left - d(8.0) - d(84.0), fy, d(84.0), d(34.0));
            self.chip(deny, "Deny", white(0.12), CardAction::ClaudeDeny(*key))?;
            self.chip(allow, "Allow", rgba(r, g, b, 1.0), CardAction::ClaudeAllow(*key))?;
        }
        Ok(())
    }

    /// An app's card, laid out as Claude's: the icon and the title over who sent it, the ×; the
    /// text, the bar; "Sustur" on the left of the footer and the app's buttons on the right, the
    /// last in its accent. A prompt has the exe's path in the panel and Reddet / İzin ver.
    fn app(&mut self, c: &AppCard) -> Result<()> {
        let d = |v: f32| v * self.s.0;
        let side = d(APP_CARD_ICON_DIP).round();
        paint_app_icon(self.ink, c.icon.as_ref(), rect(d(20.0).round(), d(18.0).round(), side, side), c.accent, d(20.0))?;
        let text_x = d(72.0);
        let text_w = self.w - text_x - d(60.0);
        let title = TextStyle::new(Face::Text, Weight::SemiBold, d(16.0), white(1.0)).top().ellipsis();
        self.ink.text(&c.title, rect(text_x, d(17.0), text_w, d(22.0)), &title)?;
        let line = rect(text_x, d(40.0), text_w, d(18.0));
        if c.prompt {
            let asks = TextStyle::new(Face::Text, Weight::SemiBold, d(12.5), lighten(c.accent)).top().ellipsis();
            self.ink.text("wants to show notifications", line, &asks)?;
        } else {
            self.sender(c, &c.status, line, &TextStyle::new(Face::Text, Weight::Regular, d(12.5), white(0.7)).top().tabular())?;
        }
        let close = rect(self.w - d(54.0), d(18.0), d(40.0), d(40.0));
        self.ink.text(&glyph(Icon::Close).to_string(), close, &TextStyle::new(Face::Icons, Weight::Regular, d(15.0), white(0.8)).centered())?;
        self.hits.push((close, CardAction::LiveClose));
        let (r, g, b) = c.accent;

        if c.prompt {
            let panel = rect(d(16.0), d(APP_BODY_TOP), self.w - d(32.0), d(102.0));
            self.ink.rrect(panel, d(22.0), white(0.06))?;
            let (px, pw) = (panel.left + d(16.0), panel.right - panel.left - d(32.0));
            self.sender(c, "", rect(px, panel.top + d(14.0), pw, d(18.0)), &TextStyle::new(Face::Text, Weight::SemiBold, d(13.0), white(1.0)).top())?;
            // The path's end (the folder and the exe) stays; a long one loses its start.
            let path = TextStyle::new(Face::Text, Weight::Regular, d(12.5), white(0.85)).top();
            let shown = self.end_kept(&c.text, pw, &path)?;
            self.ink.text(&shown, rect(px, panel.top + d(40.0), pw, d(18.0)), &path)?;
            self.ink.text("Only allow apps you trust.", rect(px, panel.top + d(76.0), pw, d(16.0)), &TextStyle::new(Face::Text, Weight::Regular, d(12.0), white(0.55)).top().ellipsis())?;
            let fy = panel.bottom + d(14.0);
            let allow = rect(self.w - d(16.0) - d(96.0), fy, d(96.0), d(34.0));
            let deny = rect(allow.left - d(8.0) - d(84.0), fy, d(84.0), d(34.0));
            self.chip(deny, "Deny", white(0.12), CardAction::AppDeny(c.key))?;
            return self.chip(allow, "Allow", rgba(r, g, b, 1.0), CardAction::AppAllow(c.key));
        }

        let mut y = d(APP_BODY_TOP);
        let lines = app_text_lines(&c.text) as f32;
        if lines > 0.0 {
            let body = TextStyle::new(Face::Text, Weight::Regular, d(13.0), white(0.9)).top().wrap().ellipsis();
            self.ink.text(&c.text, rect(d(20.0), y, self.w - d(40.0), d(lines * APP_LINE)), &body)?;
            y += d(lines * APP_LINE + 4.0);
        }
        if let Some(p) = c.progress {
            let bar = d(4.0).round();
            let (x0, x1) = (d(20.0), self.w - d(20.0));
            self.ink.rrect(rect(x0, y + d(5.0), x1 - x0, bar), bar / 2.0, white(0.18))?;
            self.ink.rrect(rect(x0, y + d(5.0), ((x1 - x0) * p.min(100) as f32 / 100.0).max(bar), bar), bar / 2.0, lighten(c.accent))?;
            y += d(14.0);
        }
        let fy = y + d(14.0);
        self.chip(rect(d(16.0), fy, d(84.0), d(34.0)), "Mute", white(0.12), CardAction::AppMute(c.key))?;
        // The last button is the one in the accent; long labels are cut, never over "Sustur".
        let chip_text = TextStyle::new(Face::Text, Weight::SemiBold, d(13.5), white(1.0)).centered().ellipsis();
        let widths = c.buttons.iter().map(|l| self.ink.measure(l, &chip_text)).collect::<Result<Vec<f32>>>()?;
        for (i, ((left, right), label)) in app_button_spans(self.w, &widths, self.s).into_iter().zip(&c.buttons).enumerate() {
            let area = rect(left, fy, right - left, d(34.0));
            let fill = if i + 1 == c.buttons.len() { rgba(r, g, b, 1.0) } else { white(0.12) };
            self.ink.rrect(area, d(17.0), fill)?;
            self.ink.text(label, rect(left + d(12.0), fy, right - left - d(24.0), d(34.0)), &chip_text)?;
            self.hits.push((area, CardAction::AppButton(c.key, i as u8)));
        }
        Ok(())
    }

    /// "name · exe" (and " · status") on one line: the name gives way first, so who sent it
    /// always shows whole.
    fn sender(&self, c: &AppCard, status: &str, area: D2D_RECT_F, style: &TextStyle) -> Result<()> {
        let tail = if status.is_empty() { format!(" · {}", c.exe) } else { format!(" · {} · {status}", c.exe) };
        let w = area.right - area.left;
        let whole = format!("{}{tail}", c.name);
        // One run, cut here rather than by DirectWrite: its cut would leave a gap before the tail.
        let line = if self.ink.measure(&whole, style)? <= w {
            whole
        } else {
            let room = w - self.ink.measure(&tail, style)? - self.d(1.0);
            format!("{}{tail}", self.start_kept(&c.name, room, style)?)
        };
        self.ink.text(&line, area, &style.ellipsis())
    }

    /// `text` as much as fits in `w`, keeping its start: "WWWW…".
    fn start_kept(&self, text: &str, w: f32, style: &TextStyle) -> Result<String> {
        let chars: Vec<char> = text.chars().collect();
        for end in (1..chars.len()).rev() {
            let cut: String = chars[..end].iter().copied().chain(std::iter::once('…')).collect();
            if self.ink.measure(&cut, style)? <= w {
                return Ok(cut);
            }
        }
        Ok("…".into())
    }

    /// `text` as much as fits in `w`, keeping its end: "…\Apps\Ekin\ekin.exe".
    fn end_kept(&self, text: &str, w: f32, style: &TextStyle) -> Result<String> {
        if self.ink.measure(text, style)? <= w {
            return Ok(text.to_string());
        }
        let chars: Vec<char> = text.chars().collect();
        for start in 1..chars.len() {
            let cut: String = std::iter::once('…').chain(chars[start..].iter().copied()).collect();
            if self.ink.measure(&cut, style)? <= w {
                return Ok(cut);
            }
        }
        Ok("…".into())
    }

    /// The mark in the accent circle, the title over its last line, the status, the ×; with a
    /// progress, a thin bar under the text.
    fn live(&mut self, l: &LiveCard) -> Result<()> {
        let (r, g, b) = if l.claude { CLAUDE_ACCENT } else { timer_accent() };
        let circle = self.d(46.0);
        let dot = rect(self.d(15.0), ((self.h - circle) / 2.0).round(), circle, circle);
        self.ink.rrect(dot, circle / 2.0, rgba(r, g, b, 1.0))?;
        paint_live_mark(self.ink, dot, l.claude, l.state, self.d(24.0))?;
        self.icon_button(Icon::Close, 35.0, CardAction::LiveClose)?;
        let status = TextStyle::new(Face::Text, Weight::Regular, self.d(13.0), white(0.7)).tabular().trailing();
        let status_w = self.ink.measure(&l.status, &status)? + self.d(1.0);
        let right = self.w - self.d(62.0);
        self.ink.text(&l.status, rect(right - status_w, 0.0, status_w, self.h), &status)?;
        let text_w = right - status_w - self.d(12.0) - self.d(73.0);
        // With a bar the block moves up so the three stay centred.
        let lift = if l.progress.is_some() { self.d(5.0) } else { 0.0 };
        let title = TextStyle::new(Face::Display, Weight::SemiBold, self.d(17.0), white(1.0)).top().ellipsis();
        self.ink.text(&l.title, rect(self.d(73.0), self.d(16.0) - lift, text_w, self.d(24.0)), &title)?;
        let line = TextStyle::new(Face::Text, Weight::Regular, self.d(13.0), white(0.7)).top().ellipsis();
        self.ink.text(&l.text, rect(self.d(74.0), self.d(41.0) - lift, text_w, self.d(20.0)), &line)?;
        if let Some(p) = l.progress {
            let bar = self.d(3.0).round();
            let (x0, x1, y) = (self.d(74.0), right, self.h - self.d(15.0));
            self.ink.rrect(rect(x0, y, x1 - x0, bar), bar / 2.0, white(0.22))?;
            self.ink.rrect(rect(x0, y, ((x1 - x0) * p.min(100) as f32 / 100.0).max(bar), bar), bar / 2.0, white(1.0))?;
        }
        Ok(())
    }

    /// The box art on the left (the store's colour and a controller without it); beside it the
    /// store, the title, what it does, the bar while it downloads and the figures under it.
    fn game(&mut self, c: &GameCard) -> Result<()> {
        let d = |v: f32| v * self.s.0;
        let (r, g, b) = store_accent(c.store);
        let (cw, ch) = (d(GAME_COVER_DIP.0).round(), d(GAME_COVER_DIP.1).round());
        let cover = rect(d(24.0).round(), d(24.0).round(), cw, ch);
        match &c.cover {
            Some(a) => self.ink.image_rounded(&a.px, a.size, rect(0.0, 0.0, a.size.0 as f32, a.size.1 as f32), cover, d(14.0))?,
            None => {
                self.ink.rrect(cover, d(14.0), rgba(r, g, b, 1.0))?;
                self.ink.text("\u{E7FC}", cover, &TextStyle::new(Face::Icons, Weight::Regular, d(28.0), white(1.0)).centered())?;
            }
        }
        let x = cover.right + d(16.0);
        let tw = self.w - x - d(24.0);
        let store = TextStyle::new(Face::Text, Weight::Regular, d(12.0), white(0.55)).top().ellipsis();
        self.ink.text(c.store.name(), rect(x, d(27.0), tw, d(16.0)), &store)?;
        let title = TextStyle::new(Face::Display, Weight::SemiBold, d(18.0), white(1.0)).top().ellipsis();
        self.ink.text(&c.title, rect(x, d(43.0), tw, d(26.0)), &title)?;
        let status = TextStyle::new(Face::Text, Weight::SemiBold, d(13.0), white(0.9)).top().ellipsis().tabular();
        self.ink.text(&c.status, rect(x, d(72.0), tw, d(18.0)), &status)?;
        let mut y = d(96.0);
        if let Some(f) = c.progress {
            let bar = d(4.0).round();
            self.ink.rrect(rect(x, y, tw, bar), bar / 2.0, white(0.18))?;
            let fill = if c.paused { white(0.5) } else { lighten((r, g, b)) };
            self.ink.rrect(rect(x, y, (tw * f.clamp(0.0, 1.0)).max(bar), bar), bar / 2.0, fill)?;
            y += d(10.0);
        }
        let detail = TextStyle::new(Face::Text, Weight::Regular, d(12.0), white(0.6)).top().ellipsis().tabular();
        self.ink.text(&c.detail, rect(x, y, tw, d(16.0)), &detail)
    }

    fn stopwatch(&mut self, t: &StopwatchCard) -> Result<()> {
        self.accent_circle(Icon::Stopwatch)?;
        self.big_time(&t.text, t.last_lap.as_deref().unwrap_or("Kronometre"))?;
        self.icon_button(Icon::Flag, 119.0, CardAction::StopwatchLap)?;
        self.icon_button(Icon::Close, 77.0, CardAction::StopwatchClose)?;
        let (icon, action) = if t.paused { (Icon::Play, CardAction::StopwatchResume) } else { (Icon::Pause, CardAction::StopwatchPause) };
        self.icon_button(icon, 35.0, action)
    }

    fn match_crest(&self, t: &CardTeam, x: f32, y: f32) -> Result<()> {
        let d = |v: f32| v * self.s.0;
        let side = d(56.0).round();
        let circle = rect(x.round(), y.round(), side, side);
        match &t.crest {
            Some(c) => {
                self.ink.rrect(circle, side / 2.0, white(0.10))?;
                self.ink.image_px(&c.px, c.size, circle.left + (side - c.size.0 as f32) / 2.0, circle.top + (side - c.size.1 as f32) / 2.0)?;
            }
            None => {
                let ([r, g, b], [hr, hg, hb]) = (t.palette.flood, t.palette.headline);
                self.ink.rrect(circle, side / 2.0, rgba(r, g, b, 1.0))?;
                self.ink.text(&t.code, circle, &TextStyle::new(Face::Text, Weight::Bold, d(19.0), rgba(hr, hg, hb, 1.0)).centered())?;
            }
        }
        Ok(())
    }

    /// Competition header, crests with team glows, score (or kick-off), the status capsule, then
    /// the goals and red cards (live / full time) or the last five results and the venue.
    fn match_card(&mut self, m: &MatchCard) -> Result<()> {
        let d = |v: f32| v * self.s.0;
        let glow = |side: Side| -> f32 {
            if m.goal == Some(side) {
                0.70
            } else if m.loser == Some(side) {
                0.24
            } else if m.score.is_none() {
                0.32
            } else {
                0.42
            }
        };
        let tint = |t: &CardTeam, a: f32| rgba(t.palette.tint[0], t.palette.tint[1], t.palette.tint[2], a);
        self.ink.glow(d(70.0), d(84.0), d(180.0), d(150.0), tint(&m.home, glow(Side::Home)))?;
        self.ink.glow(d(270.0), d(84.0), d(180.0), d(150.0), tint(&m.away, glow(Side::Away)))?;

        // Header: competition badge and name, ↗ to the match page.
        let badge = rect(d(16.0), d(20.0), d(20.0), d(20.0));
        let comp = if m.competition.contains("Süper Lig") { rgba(0xF2, 0x7A, 0x1A, 1.0) } else { rgba(0x2B, 0x4C, 0xC8, 1.0) };
        self.ink.rrect(badge, d(10.0), comp)?;
        let initials: String = match m.competition.as_str() {
            c if c.contains("Süper Lig") => "SL".into(),
            // The name is in the language it was asked in.
            c if c.starts_with("Şampiyonlar") || c.starts_with("Champions") => tr("ŞL").into(),
            c if c.starts_with("Avrupa") || c.starts_with("Europa") => tr("AL").into(),
            c if c.starts_with("Konferans") || c.starts_with("Conference") => tr("KL").into(),
            c => c.split_whitespace().filter_map(|w| w.chars().next()).filter(|c| c.is_uppercase()).take(2).collect(),
        };
        self.ink.text(&initials, badge, &TextStyle::new(Face::Text, Weight::Bold, d(8.5), white(1.0)).centered())?;
        let header = TextStyle::new(Face::Text, Weight::Regular, d(12.5), white(0.85)).ellipsis();
        self.ink.text(&m.competition, rect(d(42.0), d(16.0), d(246.0), d(28.0)), &header)?;
        if m.has_page {
            let open = rect(d(296.0), d(16.0), d(28.0), d(28.0));
            self.ink.rrect(open, d(14.0), white(0.14))?;
            self.ink.text("\u{E8A7}", open, &TextStyle::new(Face::Icons, Weight::Regular, d(13.0), white(1.0)).centered())?;
            self.hits.push((open, CardAction::MatchOpen));
        }

        // Crests, names, score or kick-off.
        self.match_crest(&m.home, d(42.0), d(56.0))?;
        self.match_crest(&m.away, d(242.0), d(56.0))?;
        let name = |side: Side| TextStyle::new(Face::Text, Weight::SemiBold, d(13.0), white(if m.loser == Some(side) { 0.7 } else { 1.0 })).centered().ellipsis();
        self.ink.text(&m.home.name, rect(d(16.0), d(118.0), d(108.0), d(18.0)), &name(Side::Home))?;
        self.ink.text(&m.away.name, rect(d(216.0), d(118.0), d(108.0), d(18.0)), &name(Side::Away))?;
        let score_box = rect(d(124.0), d(58.0), d(92.0), d(52.0));
        match m.score {
            None => {
                self.ink.text(&m.kickoff, score_box, &TextStyle::new(Face::Display, Weight::Bold, d(34.0), white(1.0)).tabular().centered())?;
            }
            // Basketball's three digits: smaller, each side against the dash.
            Some((hs, aw)) if hs >= 100 || aw >= 100 => {
                let digit = |side: Side| TextStyle::new(Face::Display, Weight::Bold, d(28.0), white(if m.loser == Some(side) { 0.45 } else { 1.0 })).tabular();
                self.ink.text(&hs.to_string(), rect(d(104.0), d(58.0), d(54.0), d(52.0)), &digit(Side::Home).trailing())?;
                self.ink.text("–", rect(d(162.0), d(58.0), d(16.0), d(52.0)), &TextStyle::new(Face::Display, Weight::Medium, d(28.0), white(0.4)).centered())?;
                self.ink.text(&aw.to_string(), rect(d(182.0), d(58.0), d(54.0), d(52.0)), &digit(Side::Away))?;
            }
            Some((hs, aw)) => {
                let big = if hs >= 10 || aw >= 10 { 34.0 } else { 40.0 };
                let digit = |side: Side| TextStyle::new(Face::Display, Weight::Bold, d(big), white(if m.loser == Some(side) { 0.45 } else { 1.0 })).tabular().centered();
                self.ink.text(&hs.to_string(), rect(d(124.0), d(58.0), d(38.0), d(52.0)), &digit(Side::Home))?;
                self.ink.text("–", rect(d(162.0), d(58.0), d(16.0), d(52.0)), &TextStyle::new(Face::Display, Weight::Medium, d(32.0), white(0.4)).centered())?;
                self.ink.text(&aw.to_string(), rect(d(178.0), d(58.0), d(38.0), d(52.0)), &digit(Side::Away))?;
            }
        }

        // Status capsule, centred under the score.
        let cap_style = TextStyle::new(Face::Text, Weight::SemiBold, d(12.5), white(1.0)).tabular();
        let text = match m.goal {
            Some(_) => format!("GOL! {}", m.capsule),
            None => m.capsule.clone(),
        };
        let tw = self.ink.measure(&text, &cap_style)?;
        let dot = if m.live && m.goal.is_none() { d(11.0) } else { 0.0 };
        let cap_w = tw + dot + d(20.0);
        let cap = rect(d(170.0) - cap_w / 2.0, d(118.0), cap_w, d(22.0));
        let fill = match m.goal {
            Some(side) => {
                let f = if side == Side::Home { m.home.palette.flood } else { m.away.palette.flood };
                rgba(f[0], f[1], f[2], 1.0)
            }
            None if m.live => rgba(0xFF, 0x4B, 0x4B, 0.20),
            None => white(0.12),
        };
        self.ink.rrect(cap, d(11.0), fill)?;
        if dot > 0.0 {
            self.ink.rrect(rect(cap.left + d(10.0), cap.top + d(8.0), d(6.0), d(6.0)), d(3.0), rgba(0xFF, 0x4B, 0x4B, 1.0))?;
        }
        let cap_color = match m.goal {
            Some(side) => {
                let h = if side == Side::Home { m.home.palette.headline } else { m.away.palette.headline };
                rgba(h[0], h[1], h[2], 1.0)
            }
            None if m.score.is_none() => white(0.85),
            None => white(1.0),
        };
        let cap_text = TextStyle::new(Face::Text, if m.goal.is_some() { Weight::Bold } else { Weight::SemiBold }, d(12.5), cap_color).tabular();
        self.ink.text(&text, rect(cap.left + d(10.0) + dot, cap.top, tw + d(2.0), cap.bottom - cap.top), &cap_text)?;

        if m.score.is_none() {
            return self.match_pre(m);
        }
        // Goals and red cards: newest five, home on the left, away on the right.
        let panel = rect(d(12.0), d(152.0), d(316.0), d(112.0));
        self.ink.rrect(panel, d(20.0), white(0.06))?;
        if let Some(quarters) = &m.quarters {
            return self.quarter_table(m, quarters, panel);
        }
        let shown: Vec<&MatchEvent> = m.events.iter().rev().take(5).collect::<Vec<_>>().into_iter().rev().collect();
        if shown.is_empty() {
            self.ink.text("No goals yet", panel, &TextStyle::new(Face::Text, Weight::Regular, d(12.5), white(0.55)).centered())?;
            return Ok(());
        }
        let top = panel.top + (panel.bottom - panel.top - shown.len() as f32 * d(20.0)) / 2.0;
        for (i, e) in shown.iter().enumerate() {
            let y = top + i as f32 * d(20.0);
            let minute = match e.added {
                Some(a) => format!("{}+{}′", e.minute, a),
                None => format!("{}′", e.minute),
            };
            let mst = TextStyle::new(Face::Text, Weight::SemiBold, d(12.5), white(0.6)).tabular().centered();
            self.ink.text(&minute, rect(d(146.0), y, d(48.0), d(20.0)), &mst)?;
            let suffix = match e.kind {
                EventKind::PenaltyGoal => " (P)",
                EventKind::OwnGoal => " (KK)",
                _ => "",
            };
            let who = e.player.clone().unwrap_or_else(|| tr("Teknik ekip").into());
            let weight = if e.kind == EventKind::RedCard { Weight::Regular } else { Weight::SemiBold };
            let nst = TextStyle::new(Face::Text, weight, d(13.0), white(if e.kind == EventKind::RedCard { 0.88 } else { 1.0 })).ellipsis();
            let sst = TextStyle::new(Face::Text, Weight::Regular, d(13.0), white(0.55));
            let icon_w = d(14.0);
            let nw = self.ink.measure(&who, &nst)?.min(d(100.0));
            let sw = if suffix.is_empty() { 0.0 } else { self.ink.measure(suffix, &sst)? };
            let (icon_x, name_x) = match e.side {
                Side::Home => {
                    let end = d(146.0) - d(4.0);
                    (end - icon_w, end - icon_w - d(6.0) - sw - nw)
                }
                Side::Away => (d(194.0) + d(4.0), d(194.0) + d(4.0) + icon_w + d(6.0)),
            };
            if e.kind == EventKind::RedCard {
                self.ink.rrect(rect(icon_x + d(2.5), y + d(4.0), d(9.0), d(12.0)), d(1.5), rgba(0xF0, 0x32, 0x2D, 1.0))?;
            } else {
                self.ink.text("⚽", rect(icon_x, y, icon_w, d(20.0)), &TextStyle::new(Face::Text, Weight::Regular, d(12.0), white(1.0)).centered())?;
            }
            self.ink.text(&who, rect(name_x, y, nw + d(1.0), d(20.0)), &nst)?;
            if !suffix.is_empty() {
                self.ink.text(suffix, rect(name_x + nw, y, sw + d(2.0), d(20.0)), &sst)?;
            }
        }
        let hidden = m.events.len().saturating_sub(5);
        if hidden > 0 {
            let more = TextStyle::new(Face::Text, Weight::Medium, d(11.0), white(0.55)).trailing();
            self.ink.text(&format!("+{hidden}"), rect(panel.right - d(54.0), panel.top + d(5.0), d(40.0), d(14.0)), &more)?;
        }
        Ok(())
    }

    /// Basketball: a column per quarter (overtime as UZ) and the total, a row per team.
    fn quarter_table(&mut self, m: &MatchCard, (home, away): &(Vec<u8>, Vec<u8>), panel: D2D_RECT_F) -> Result<()> {
        let d = |v: f32| v * self.s.0;
        let n = home.len().max(away.len()).max(1);
        let labels: Vec<String> = (1..=n).map(|i| if i <= 4 { i.to_string() } else { "UZ".into() }).chain(Some("T".to_string())).collect();
        let (first, last) = (panel.left + d(150.0), panel.right - d(22.0));
        let step = if labels.len() > 1 { (last - first) / (labels.len() - 1) as f32 } else { 0.0 };
        let col = d(30.0);
        let head = TextStyle::new(Face::Text, Weight::Medium, d(11.5), white(0.55)).centered();
        for (i, l) in labels.iter().enumerate() {
            self.ink.text(l, rect(first + i as f32 * step - col / 2.0, panel.top + d(8.0), col, d(18.0)), &head)?;
        }
        let total = |v: &Vec<u8>| v.iter().map(|x| *x as u32).sum::<u32>();
        for (row, (team, points)) in [(&m.home, home), (&m.away, away)].into_iter().enumerate() {
            let y = panel.top + d(30.0) + row as f32 * d(38.0);
            PillTeamLike { team }.crest(self, d(26.0), y + d(9.0))?;
            let name = TextStyle::new(Face::Text, Weight::SemiBold, d(13.0), white(1.0)).ellipsis();
            self.ink.text(&team.name, rect(d(54.0), y, first - d(54.0) - col, d(38.0)), &name)?;
            for i in 0..n {
                let value = points.get(i).map(|p| p.to_string()).unwrap_or_default();
                let st = TextStyle::new(Face::Display, Weight::Medium, d(13.0), white(0.85)).tabular().centered();
                self.ink.text(&value, rect(first + i as f32 * step - col / 2.0, y, col, d(38.0)), &st)?;
            }
            let st = TextStyle::new(Face::Display, Weight::Bold, d(14.0), white(1.0)).tabular().centered();
            self.ink.text(&total(points).to_string(), rect(last - col / 2.0, y, col, d(38.0)), &st)?;
        }
        Ok(())
    }

    /// Before kick-off: each team's last five results (G/B/M) and the venue.
    fn match_pre(&mut self, m: &MatchCard) -> Result<()> {
        let d = |v: f32| v * self.s.0;
        if let Some((home_form, away_form)) = &m.form {
            self.ink.text("Last 5 matches", rect(d(16.0), d(154.0), d(200.0), d(16.0)), &TextStyle::new(Face::Text, Weight::Regular, d(12.0), white(0.62)).top())?;
            let panel = rect(d(12.0), d(174.0), d(316.0), d(68.0));
            self.ink.rrect(panel, d(20.0), white(0.06))?;
            for (row, (team, form)) in [(&m.home, home_form), (&m.away, away_form)].into_iter().enumerate() {
                let y = panel.top + d(6.0) + row as f32 * d(28.0);
                let small = PillTeamLike { team };
                small.crest(self, d(24.0), y + d(4.0))?;
                let st = TextStyle::new(Face::Text, Weight::SemiBold, d(13.0), white(1.0)).ellipsis();
                self.ink.text(&team.name, rect(d(52.0), y, d(140.0), d(28.0)), &st)?;
                let results: Vec<char> = form.chars().filter(|c| "WDL".contains(*c)).collect();
                for (i, r) in results.iter().enumerate() {
                    let x = d(316.0) - (results.len() - i) as f32 * d(24.0);
                    let (fill, letter) = match r {
                        'W' => (rgba(0x2E, 0x9E, 0x4F, 1.0), "W"),
                        'L' => (rgba(0xD9, 0x3A, 0x3A, 1.0), "L"),
                        _ => (white(0.18), "D"),
                    };
                    let chip = rect(x, y + d(4.0), d(20.0), d(20.0));
                    self.ink.rrect(chip, d(6.0), fill)?;
                    self.ink.text(letter, chip, &TextStyle::new(Face::Text, Weight::Bold, d(11.0), white(1.0)).centered())?;
                }
            }
        }
        if let Some(venue) = &m.venue {
            let y = d(254.0);
            self.ink.text("\u{E707}", rect(d(16.0), y, d(16.0), d(18.0)), &TextStyle::new(Face::Icons, Weight::Regular, d(12.0), white(0.75)).centered())?;
            self.ink.text(venue, rect(d(37.0), y, d(280.0), d(18.0)), &TextStyle::new(Face::Text, Weight::Regular, d(12.5), white(0.75)).ellipsis())?;
        }
        Ok(())
    }

    /// "Galaxy Buds2 / Connected to this PC", then a ring per bud and, while known, the case.
    fn buds(&mut self, b: &BudsSnapshot) -> Result<()> {
        let d = |v: f32| v * self.s.0;
        let (r, g, bl) = BUDS_ACCENT;
        let circle = d(46.0);
        let badge = rect(d(15.0), d(15.0), circle, circle);
        self.ink.rrect(badge, circle / 2.0, rgba(r, g, bl, 1.0))?;
        self.ink.text(&glyph(Icon::Headphones).to_string(), badge, &TextStyle::new(Face::Icons, Weight::Regular, d(24.0), white(1.0)).centered())?;
        let title = TextStyle::new(Face::Text, Weight::SemiBold, d(16.0), white(1.0)).top();
        self.ink.text("Galaxy Buds2", rect(d(73.0), d(17.0), d(280.0), d(22.0)), &title)?;
        let sub = TextStyle::new(Face::Text, Weight::Regular, d(13.0), white(0.8)).top();
        self.ink.text("Connected to this PC", rect(d(73.0), d(40.0), d(280.0), d(20.0)), &sub)?;

        let mut rings = vec![("Left", b.left), ("Right", b.right)];
        if let Some(case) = b.case {
            rings.push(("Case", case));
        }
        let (radius, cy, step) = (d(22.0), d(96.0), d(110.0));
        let n = rings.len() as f32;
        for (i, (label, value)) in rings.into_iter().enumerate() {
            let cx = self.w / 2.0 + (i as f32 - (n - 1.0) / 2.0) * step;
            let fill = if value > 0 && value < low_battery() { rgba(0xFF, 0x5A, 0x4F, 1.0) } else { rgba(r, g, bl, 1.0) };
            self.ink.ring(cx, cy, radius, d(5.0), value as f32 / 100.0, white(0.14), fill)?;
            let pct = TextStyle::new(Face::Display, Weight::SemiBold, d(13.0), white(1.0)).tabular().centered();
            self.ink.text(&percent(value), rect(cx - radius, cy - radius, 2.0 * radius, 2.0 * radius), &pct)?;
            let st = TextStyle::new(Face::Text, Weight::Regular, d(12.0), white(0.7)).centered();
            self.ink.text(label, rect(cx - d(40.0), cy + radius + d(6.0), d(80.0), d(16.0)), &st)?;
        }
        if let Some(mode) = b.noise {
            self.noise_control(mode)?;
        }
        Ok(())
    }

    /// The city and the weather now on top, the next hours in a row below.
    /// "Raf" with its count and the "Temizle" chip, then the tiles: the preview edge to edge with
    /// the name on a scrim, or the type's colour and glyph; the hovered tile gets a ring and its
    /// close button.
    fn shelf(&mut self, c: &ShelfCard) -> Result<()> {
        let d = |v: f32| v * self.s.0;
        let l = shelf_categorized_layout(c, self.s);
        let title = TextStyle::new(Face::Text, Weight::SemiBold, d(16.0), white(1.0)).top();
        let title_text = "Shelf";
        self.ink.text(title_text, rect(d(16.0), d(16.0), d(120.0), d(22.0)), &title)?;
        let count = TextStyle::new(Face::Text, Weight::Regular, d(12.0), sec_text()).top();
        let count_x = d(16.0) + self.ink.measure(title_text, &title)? + d(8.0);
        self.ink.text(&c.tiles.len().to_string(), rect(count_x, d(19.0), d(60.0), d(18.0)), &count)?;

        let chip = l.clear;
        let btn_r = d(4.0);
        self.ink.rrect(chip, btn_r, white(0.05))?;
        self.ink.text("Clear", chip, &TextStyle::new(Face::Text, Weight::Regular, d(12.0), sec_text()).centered())?;
        self.hits.push((chip, CardAction::ShelfClear));

        // Quiet pin button left of Clear: same upright pushpin SVG as Media.
        let pinned = c.pinned;
        let pin = rect(chip.left - d(36.0), chip.top, d(28.0), chip.bottom - chip.top);
        let (pr, pg, pb) = timer_accent();
        self.ink.rrect(pin, btn_r, if pinned { white(0.09) } else { white(0.04) })?;
        let pin_color = if pinned { rgba(pr, pg, pb, 1.0) } else { neutral_icon() };
        let pin_sz = d(15.0).round();
        let pin_icon = rect(
            (pin.left + (pin.right - pin.left - pin_sz) / 2.0).round(),
            (pin.top + (pin.bottom - pin.top - pin_sz) / 2.0).round(),
            pin_sz,
            pin_sz,
        );
        self.ink.svg(PIN_SVG, pin_icon, pin_color)?;
        self.hits.push((pin, CardAction::ShelfPin));

        let band_h = d(crate::activities::shelf::FOOTER_BAND_DIP);

        if c.tiles.is_empty() {
            let hint = TextStyle::new(Face::Text, Weight::Regular, d(13.0), sec_text()).centered();
            let empty_y = (d(48.0) + (self.h - band_h - d(48.0) - d(40.0)) / 2.0).round();
            self.ink.text("Drop files here", rect(d(16.0), empty_y, self.w - d(32.0), d(40.0)), &hint)?;
        } else {
            let scroll_dip = c.scroll.clamp(0.0, l.max_scroll_dip);
            let scroll_px = d(scroll_dip).round();
            let grid_clip = rect(0.0, d(46.0).round(), self.w, (self.h - band_h).round());

            unsafe {
                self.ink.dc.PushAxisAlignedClip(&grid_clip, windows::Win32::Graphics::Direct2D::D2D1_ANTIALIAS_MODE_ALIASED);
            }

            let header_style = TextStyle::new(Face::Text, Weight::SemiBold, d(11.0), sec_text()).top();
            for h in &l.headers {
                let hy = d(h.top_dip).round() - scroll_px;
                let h_bottom = hy + d(18.0);
                if h_bottom > grid_clip.top && hy < grid_clip.bottom {
                    if h.is_pinned {
                        let ir = rect(d(16.0), hy + d(2.0), d(12.0), d(12.0));
                        self.ink.svg(PIN_SVG, ir, rgba(pr, pg, pb, 1.0))?;
                        let hr = rect(d(32.0), hy, d(120.0), d(18.0));
                        self.ink.text(h.title, hr, &header_style)?;
                    } else {
                        let hr = rect(d(16.0), hy, d(120.0), d(18.0));
                        self.ink.text(h.title, hr, &header_style)?;
                    }
                }
            }

            let radius = d(4.0);
            let name_style = TextStyle::new(Face::Text, Weight::Regular, d(11.5), white(0.95)).centered().wrap().ellipsis();
            for orig_idx in 0..c.tiles.len() {
                let tile = &c.tiles[orig_idx];
                let mut t = l.tiles[orig_idx];
                t.top -= scroll_px;
                t.bottom -= scroll_px;
                if t.bottom <= grid_clip.top || t.top >= grid_clip.bottom {
                    continue;
                }
                let (tw, th) = (t.right - t.left, t.bottom - t.top);
                let hovered = c.hover == Some(orig_idx);

                let tile_fill = if tile.selected {
                    white(0.12)
                } else if hovered {
                    white(0.07)
                } else {
                    white(0.05)
                };
                self.ink.rrect(t, radius, tile_fill)?;
                let border_rect = rect(t.left + 0.5, t.top + 0.5, tw - 1.0, th - 1.0);
                if tile.selected {
                    self.ink.rrect_outline(border_rect, (radius - 0.5).max(1.0), 1.5, rgba(pr, pg, pb, 0.9))?;
                } else {
                    self.ink.rrect_outline(border_rect, (radius - 0.5).max(1.0), 1.0, white(0.07))?;
                }

                let ((r, g, b), icon) = kind_style(tile.kind);
                let icon_color = if tile.kind == FileKind::Pdf { rgba(r, g, b, 1.0) } else { neutral_icon() };
                let glyph_style = TextStyle::new(Face::Icons, Weight::Regular, d(24.0), icon_color).centered();
                self.ink.text(&glyph(icon).to_string(), rect(t.left, t.top + d(10.0), tw, d(32.0)), &glyph_style)?;

                self.ink.text(&tile.name, rect(t.left + d(6.0), t.top + d(46.0), tw - d(12.0), d(34.0)), &name_style)?;

                if tile.pinned && !hovered {
                    let pin_mark = rect((t.right - d(17.0)).round(), (t.top + d(5.0)).round(), d(12.0).round(), d(12.0).round());
                    self.ink.svg(PIN_SVG, pin_mark, rgba(pr, pg, pb, 1.0))?;
                }

                if hovered {
                    let mut close = l.closes[orig_idx];
                    close.top -= scroll_px;
                    close.bottom -= scroll_px;
                    let close_w = close.right - close.left;
                    let close_h = close.bottom - close.top;
                    let item_pin = rect(close.left - d(4.0).round() - close_w, close.top, close_w, close_h);
                    self.ink.rrect(item_pin, d(4.0), [0.0, 0.0, 0.0, 0.65])?;
                    let ip_sz = d(11.0).round();
                    let ip_icon = rect(
                        (item_pin.left + (close_w - ip_sz) / 2.0).round(),
                        (item_pin.top + (close_h - ip_sz) / 2.0).round(),
                        ip_sz,
                        ip_sz,
                    );
                    let ip_color = if tile.pinned { rgba(pr, pg, pb, 1.0) } else { white(1.0) };
                    self.ink.svg(PIN_SVG, ip_icon, ip_color)?;
                    self.hits.push((item_pin, CardAction::ShelfItemPin(orig_idx)));

                    self.ink.rrect(close, d(4.0), [0.0, 0.0, 0.0, 0.65])?;
                    let x = TextStyle::new(Face::Icons, Weight::Regular, d(8.5), white(1.0)).centered();
                    self.ink.text(&glyph(Icon::Close).to_string(), close, &x)?;
                    self.hits.push((close, CardAction::ShelfRemove(orig_idx)));
                }
            }

            unsafe {
                self.ink.dc.PopAxisAlignedClip();
            }
        }

        // Footer band: black at 22% with a 1px white 6% top border.
        let band = rect(0.0, self.h - band_h, self.w, band_h);
        self.ink.rrect(band, 0.0, [0.0, 0.0, 0.0, 0.22])?;
        self.ink.line(0.0, band.top + 0.5, self.w, band.top + 0.5, 1.0, white(0.06))?;

        let label = TextStyle::new(Face::Text, Weight::Regular, d(12.0), white(0.92));
        let mut shown: Vec<&(ShelfOp, String)> = c.chips.iter().collect();
        let mut widths = shown.iter().map(|(_, text)| self.ink.measure(text, &label)).collect::<Result<Vec<f32>>>()?;
        let chip_top = (band.top + (band_h - d(crate::activities::shelf::FOOTER_CHIP_DIP)) / 2.0).round();
        if shelf_chips_at(&widths, chip_top, self.s).last().is_some_and(|last| last.right > d(324.0).round()) {
            if let Some(i) = shown.iter().position(|(op, _)| Some(*op) == c.said) {
                shown = vec![shown[i]];
                widths = vec![widths[i]];
            }
        }
        let glyph_style = TextStyle::new(Face::Icons, Weight::Regular, d(13.0), neutral_icon()).centered();
        for ((op, text), chip) in shown.into_iter().zip(shelf_chips_at(&widths, chip_top, self.s)) {
            self.ink.rrect(chip, d(4.0), white(0.05))?;
            self.ink.text(&glyph(op_icon(*op)).to_string(), D2D_RECT_F { left: chip.left + d(12.0), right: chip.left + d(26.0), ..chip }, &glyph_style)?;
            self.ink.text(text, D2D_RECT_F { left: chip.left + d(32.0), ..chip }, &label)?;
            self.hits.push((chip, CardAction::ShelfTool(*op)));
        }
        Ok(())
    }

    fn clipboard(&mut self, c: &ClipboardCard) -> Result<()> {
        let d = |v: f32| v * self.s.0;
        let l = clipboard_categorized_layout(c, self.s);
        let title = TextStyle::new(Face::Text, Weight::SemiBold, d(16.0), white(1.0)).top();
        let title_text = "Clipboard";
        self.ink.text(title_text, rect(d(16.0), d(16.0), d(120.0), d(22.0)), &title)?;
        let count = TextStyle::new(Face::Text, Weight::Regular, d(12.0), sec_text()).top();
        let count_x = d(16.0) + self.ink.measure(title_text, &title)? + d(8.0);
        self.ink.text(&c.tiles.len().to_string(), rect(count_x, d(19.0), d(60.0), d(18.0)), &count)?;
        let chip = l.clear;
        self.ink.rrect(chip, d(4.0), white(0.05))?;
        self.ink.text("Clear", chip, &TextStyle::new(Face::Text, Weight::Regular, d(12.0), sec_text()).centered())?;
        self.hits.push((chip, CardAction::ClipboardClear));

        if c.tiles.is_empty() {
            let empty_style = TextStyle::new(Face::Text, Weight::Regular, d(13.0), sec_text()).centered();
            self.ink.text("Clipboard is empty", rect(d(16.0), (self.h - d(40.0)) / 2.0, self.w - d(32.0), d(40.0)), &empty_style)?;
            return Ok(());
        }

        let max_scroll = l.max_scroll_dip;
        let scroll_dip = c.scroll.clamp(0.0, max_scroll);
        let scroll_px = d(scroll_dip).round();
        let grid_clip = rect(0.0, d(46.0).round(), self.w, (self.h - d(46.0) - d(6.0)).max(0.0).round());

        let radius = d(4.0);
        let prev_style = TextStyle::new(Face::Text, Weight::Regular, d(10.5), white(0.85)).top().wrap().ellipsis();
        let (ar, ag, ab) = timer_accent();

        unsafe {
            self.ink.dc.PushAxisAlignedClip(&grid_clip, windows::Win32::Graphics::Direct2D::D2D1_ANTIALIAS_MODE_ALIASED);
        }
        let grid_res = (|| -> Result<()> {
            let header_style = TextStyle::new(Face::Text, Weight::SemiBold, d(11.0), sec_text()).top();
            for h in &l.headers {
                let hy = d(h.top_dip).round() - scroll_px;
                let h_bottom = hy + d(18.0);
                if h_bottom > grid_clip.top && hy < grid_clip.bottom {
                    if h.is_pinned {
                        let ir = rect(d(16.0), hy + d(2.0), d(12.0), d(12.0));
                        self.ink.svg(PIN_SVG, ir, rgba(ar, ag, ab, 1.0))?;
                        let hr = rect(d(32.0), hy, d(120.0), d(18.0));
                        self.ink.text(h.title, hr, &header_style)?;
                    } else {
                        let hr = rect(d(16.0), hy, d(120.0), d(18.0));
                        self.ink.text(h.title, hr, &header_style)?;
                    }
                }
            }

            for (orig_idx, tile) in c.tiles.iter().enumerate() {
                let mut t = l.tiles[orig_idx];
                t.top -= scroll_px;
                t.bottom -= scroll_px;
                let mut close = l.closes[orig_idx];
                close.top -= scroll_px;
                close.bottom -= scroll_px;

                if t.bottom <= grid_clip.top || t.top >= grid_clip.bottom {
                    continue;
                }

                let (tw, th) = (t.right - t.left, t.bottom - t.top);
                let selected = orig_idx == 0 || c.hover == Some(orig_idx);

                self.ink.rrect(t, radius, if selected || tile.pinned { white(0.09) } else { white(0.05) })?;
                let border_rect = rect(t.left + 0.5, t.top + 0.5, tw - 1.0, th - 1.0);
                self.ink.rrect_outline(border_rect, (radius - 0.5).max(1.0), 1.0, white(0.07))?;

                if let Some(img) = &tile.thumb {
                    self.ink.image_cover_rounded(&img.px, img.size, t, radius)?;
                } else if tile.is_url {
                    let glyph_style = TextStyle::new(Face::Icons, Weight::Regular, d(22.0), neutral_icon()).centered();
                    self.ink.text(&glyph(Icon::Link).to_string(), rect(t.left, t.top + d(10.0), tw, d(28.0)), &glyph_style)?;
                } else if !tile.is_image {
                    let prev_rect = rect(t.left + d(8.0), t.top + d(8.0), tw - d(16.0), th - d(28.0));
                    self.ink.text(&tile.preview, prev_rect, &prev_style)?;
                }

                // Pixel size (or domain/type) in a small dark label at bottom-left — no "IMG" badge.
                let label_text = if tile.is_image || tile.is_url {
                    tile.title.as_str()
                } else {
                    tile.badge
                };
                if !label_text.is_empty() {
                    let st = TextStyle::new(Face::Text, Weight::Regular, d(11.0), white(0.95)).centered().tabular();
                    let lw = (self.ink.measure(label_text, &st)? + d(10.0)).min(tw - d(12.0));
                    let badge = rect(t.left + d(6.0), t.bottom - d(22.0), lw.round(), d(16.0).round());
                    self.ink.rrect(badge, d(4.0), [0.0, 0.0, 0.0, 0.65])?;
                    self.ink.text(label_text, badge, &st.ellipsis())?;
                }

                if selected {
                    let inset = d(0.75);
                    let ring = rect(t.left + inset, t.top + inset, tw - 2.0 * inset, th - 2.0 * inset);
                    self.ink.rrect_outline(ring, (radius - inset).max(1.0), d(1.5), rgba(ar, ag, ab, 1.0))?;
                }

                if c.hover != Some(orig_idx) {
                    if tile.pinned {
                        let chk_side = d(18.0).round();
                        let chk = rect(t.right - d(6.0) - chk_side, t.top + d(6.0), chk_side, chk_side);
                        self.ink.rrect(chk, d(4.0), [0.0, 0.0, 0.0, 0.65])?;
                        let p_sz = d(11.0).round();
                        let p_box = rect(
                            (chk.left + (chk_side - p_sz) / 2.0).round(),
                            (chk.top + (chk_side - p_sz) / 2.0).round(),
                            p_sz,
                            p_sz,
                        );
                        self.ink.svg(PIN_SVG, p_box, rgba(ar, ag, ab, 1.0))?;
                    } else if selected {
                        let chk_side = d(18.0).round();
                        let chk = rect(t.right - d(6.0) - chk_side, t.top + d(6.0), chk_side, chk_side);
                        self.ink.rrect(chk, chk_side / 2.0, rgba(ar, ag, ab, 1.0))?;
                        let chk_st = TextStyle::new(Face::Icons, Weight::Bold, d(10.0), rgba(0x00, 0x00, 0x00, 1.0)).centered();
                        self.ink.text(&glyph(Icon::Check).to_string(), chk, &chk_st)?;
                    }
                }

                if c.hover == Some(orig_idx) {
                    let close_w = close.right - close.left;
                    let close_h = close.bottom - close.top;
                    let item_pin = rect(close.left - d(4.0).round() - close_w, close.top, close_w, close_h);
                    self.ink.rrect(item_pin, d(4.0), [0.0, 0.0, 0.0, 0.65])?;
                    let ip_sz = d(11.0).round();
                    let ip_icon = rect(
                        (item_pin.left + (close_w - ip_sz) / 2.0).round(),
                        (item_pin.top + (close_h - ip_sz) / 2.0).round(),
                        ip_sz,
                        ip_sz,
                    );
                    let ip_color = if tile.pinned { rgba(ar, ag, ab, 1.0) } else { white(1.0) };
                    self.ink.svg(PIN_SVG, ip_icon, ip_color)?;
                    if item_pin.bottom > grid_clip.top && item_pin.top < grid_clip.bottom {
                        self.hits.push((item_pin, CardAction::ClipboardPin(orig_idx)));
                    }

                    self.ink.rrect(close, d(4.0), [0.0, 0.0, 0.0, 0.65])?;
                    let x = TextStyle::new(Face::Icons, Weight::Regular, d(8.5), white(1.0)).centered();
                    self.ink.text(&glyph(Icon::Close).to_string(), close, &x)?;
                    if close.bottom > grid_clip.top && close.top < grid_clip.bottom {
                        self.hits.push((close, CardAction::ClipboardRemove(orig_idx)));
                    }
                }

                let vis_top = t.top.max(grid_clip.top);
                let vis_bottom = t.bottom.min(grid_clip.bottom);
                if vis_bottom > vis_top {
                    self.hits.push((D2D_RECT_F { top: vis_top, bottom: vis_bottom, ..t }, CardAction::ClipboardPaste(orig_idx)));
                }
            }
            Ok(())
        })();
        unsafe {
            self.ink.dc.PopAxisAlignedClip();
        }
        grid_res?;

        if max_scroll > 0.0 {
            let bar_w = d(5.0).round();
            let bar_x = d(357.0).round();
            let track_y = d(48.0).round();
            let track_h = (self.h - d(48.0) - d(12.0)).max(0.0).round();
            let track_rect = rect(bar_x, track_y, bar_w, track_h);
            self.ink.rrect(track_rect, bar_w / 2.0, white(0.08))?;
            let visible_h = self.h - d(48.0) - d(12.0);
            let thumb_h = ((visible_h / (visible_h + d(max_scroll))) * track_h).max(d(28.0)).min(track_h).round();
            let travel = (track_h - thumb_h).max(1.0);
            let frac = (scroll_dip / max_scroll).clamp(0.0, 1.0);
            let thumb_y = (track_y + frac * travel).round();
            self.ink.rrect(rect(bar_x, thumb_y, bar_w, thumb_h), bar_w / 2.0, rgba(ar, ag, ab, 0.85))?;
            let hit_area = rect(d(350.0).round(), track_y, d(18.0).round(), track_h);
            *self.clipboard_scroll = Some((hit_area, track_y + thumb_h / 2.0, track_y + track_h - thumb_h / 2.0));
        }

        if let Some(toast) = &c.toast {
            let st = TextStyle::new(Face::Text, Weight::Medium, d(11.0), white(0.95)).centered();
            let w = self.ink.measure(toast, &st)? + d(16.0);
            let right = chip.left - d(8.0);
            let pill = rect((right - w).round(), d(15.0).round(), w.round(), d(24.0).round());
            self.ink.rrect(pill, d(4.0), white(0.09))?;
            self.ink.text(toast, pill, &st)?;
        }
        Ok(())
    }

    /// Now Brief: the greeting and the date, then the weather, the matches and the music, each
    /// only when there is one (stacked as `brief_height_dip` counts them; `y` in DIP).
    fn brief(&mut self, c: &BriefCard) -> Result<()> {
        let d = |v: f32| v * self.s.0;
        let inner = self.w - d(40.0);
        let title = TextStyle::new(Face::Display, Weight::Bold, d(22.0), white(1.0)).top().ellipsis();
        self.ink.text(&c.greeting, rect(d(20.0), d(20.0), inner, d(30.0)), &title)?;
        let sub = TextStyle::new(Face::Text, Weight::Regular, d(12.5), white(0.7)).top().ellipsis();
        self.ink.text(&c.date, rect(d(20.0), d(52.0), inner, d(18.0)), &sub)?;
        let mut y = BRIEF_HEADER;
        if let Some(w) = &c.weather {
            y += BRIEF_GAP;
            let row = d(BRIEF_WEATHER_ROW);
            // On the greeting's left edge.
            let icon_s = d(28.0).round();
            let icon_box = rect(d(20.0), d(y) + ((row - icon_s) / 2.0).round(), icon_s, icon_s);
            self.ink.svg(weather_svg_for_symbol(&w.symbol), icon_box, neutral_icon())?;
            self.ink.text(&w.temp, rect(d(60.0), d(y), d(100.0), row), &TextStyle::new(Face::Display, Weight::Bold, d(34.0), white(1.0)).tabular())?;
            let summary = TextStyle::new(Face::Text, Weight::Regular, d(12.5), white(0.7)).trailing().ellipsis();
            self.ink.text(&w.summary, rect(self.w - d(20.0) - d(190.0), d(y), d(190.0), row), &summary)?;
            y += BRIEF_WEATHER_ROW + 4.0;
            self.hours_panel(&w.hours, d(y))?;
            y += HOURS_PANEL;
            if let Some(rain) = &w.rain {
                self.ink.text(rain, rect(d(20.0), d(y), inner, d(BRIEF_RAIN)), &TextStyle::new(Face::Text, Weight::Regular, d(12.5), white(0.85)))?;
                y += BRIEF_RAIN;
            }
        }
        if !c.matches.is_empty() {
            y += BRIEF_GAP;
            let panel = rect(d(12.0), d(y), self.w - d(24.0), d(16.0 + c.matches.len() as f32 * BRIEF_MATCH_ROW));
            self.ink.rrect(panel, d(24.0), white(0.06))?;
            for (i, m) in c.matches.iter().enumerate() {
                let top = y + 8.0 + i as f32 * BRIEF_MATCH_ROW;
                let mid = d(top + BRIEF_MATCH_ROW / 2.0 - 10.0);
                PillTeamLike { team: &m.home }.crest(self, d(28.0), mid)?;
                PillTeamLike { team: &m.away }.crest(self, d(52.0), mid)?;
                let line = |x: f32, width: f32| rect(x, d(top), width, d(BRIEF_MATCH_ROW));
                let codes = TextStyle::new(Face::Text, Weight::SemiBold, d(13.0), white(1.0)).ellipsis();
                self.ink.text(&format!("{} – {}", m.home.code, m.away.code), line(d(82.0), d(90.0)), &codes)?;
                let st = TextStyle::new(Face::Text, Weight::Regular, d(12.5), white(if m.live { 1.0 } else { 0.7 })).trailing().ellipsis();
                let right = self.w - d(28.0);
                self.ink.text(&m.status, line(right - d(170.0), d(170.0)), &st)?;
                if m.live {
                    // The live dot, just left of the status.
                    let tw = self.ink.measure(&m.status, &st)?.min(d(170.0));
                    let dot = d(6.0);
                    self.ink.rrect(rect(right - tw - d(8.0) - dot, d(top + BRIEF_MATCH_ROW / 2.0) - dot / 2.0, dot, dot), dot / 2.0, rgba(0xFF, 0x45, 0x3A, 1.0))?;
                }
                self.hits.push((rect(d(12.0), d(top), self.w - d(24.0), d(BRIEF_MATCH_ROW)), CardAction::BriefMatch(i)));
            }
            y += 16.0 + c.matches.len() as f32 * BRIEF_MATCH_ROW;
        }
        if let Some(m) = &c.music {
            y += BRIEF_GAP;
            self.ink.rrect(rect(d(12.0), d(y), self.w - d(24.0), d(BRIEF_MUSIC)), d(24.0), white(0.06))?;
            // 12 in from the panel on every side, as the play button.
            let art = rect(d(24.0), d(y + 12.0), d(36.0), d(36.0));
            match &m.art {
                Some(px) => {
                    let src = D2D_RECT_F { left: 0.0, top: 0.0, right: m.art_size.0 as f32, bottom: m.art_size.1 as f32 };
                    self.ink.image_rounded(px, m.art_size, src, art, d(10.0))?;
                }
                None => self.ink.rrect(art, d(10.0), white(0.12))?,
            }
            let text_w = self.w - d(72.0) - d(68.0);
            self.ink.text(&m.title, rect(d(72.0), d(y + 11.0), text_w, d(20.0)), &TextStyle::new(Face::Text, Weight::SemiBold, d(12.5), white(1.0)).top().ellipsis())?;
            self.ink.text(&m.artist, rect(d(72.0), d(y + 31.0), text_w, d(18.0)), &TextStyle::new(Face::Text, Weight::Regular, d(12.5), white(0.7)).top().ellipsis())?;
            let button = rect(self.w - d(24.0) - d(36.0), d(y + 12.0), d(36.0), d(36.0));
            self.ink.rrect(button, d(18.0), white(0.14))?;
            let icon = if m.playing { Icon::Pause } else { Icon::Play };
            self.ink.text(&glyph(icon).to_string(), button, &TextStyle::new(Face::Icons, Weight::Regular, d(16.0), white(1.0)).centered())?;
            self.hits.push((button, CardAction::BriefPlay));
            y += BRIEF_MUSIC;
        }
        // Dijital Denge, in the music panel's shape: the heart where the cover is.
        if let Some((head, sub)) = &c.screen {
            y += BRIEF_GAP;
            self.brief_link(y, WELLBEING_ACCENT, WELLBEING_GLYPH, (head, sub), CardAction::BriefScreen)?;
            y += BRIEF_MUSIC;
        }
        if let Some((head, sub)) = &c.away {
            y += BRIEF_GAP;
            self.brief_link(y, timer_accent(), &glyph(Icon::History).to_string(), (head, sub), CardAction::BriefAway)?;
        }
        Ok(())
    }

    /// A brief row that opens another card, in the music panel's shape: the mark where the cover
    /// is, two lines, a chevron (`y` in DIP).
    fn brief_link(&mut self, y: f32, (r, g, b): (u8, u8, u8), mark_glyph: &str, (head, sub): (&str, &str), action: CardAction) -> Result<()> {
        let d = |v: f32| v * self.s.0;
        let panel = rect(d(12.0), d(y), self.w - d(24.0), d(BRIEF_MUSIC));
        self.ink.rrect(panel, d(24.0), white(0.06))?;
        let mark = rect(d(24.0), d(y + 12.0), d(36.0), d(36.0));
        self.ink.rrect(mark, d(18.0), rgba(r, g, b, 1.0))?;
        self.ink.text(mark_glyph, mark, &TextStyle::new(Face::Icons, Weight::Regular, d(17.0), white(1.0)).centered())?;
        let text_w = self.w - d(72.0) - d(40.0);
        self.ink.text(head, rect(d(72.0), d(y + 11.0), text_w, d(20.0)), &TextStyle::new(Face::Text, Weight::SemiBold, d(12.5), white(1.0)).top().ellipsis())?;
        self.ink.text(sub, rect(d(72.0), d(y + 31.0), text_w, d(18.0)), &TextStyle::new(Face::Text, Weight::Regular, d(12.5), white(0.7)).top().ellipsis())?;
        let chevron = rect(self.w - d(24.0) - d(24.0), d(y + 18.0), d(24.0), d(24.0));
        self.ink.text("\u{E76C}", chevron, &TextStyle::new(Face::Icons, Weight::Regular, d(12.0), white(0.6)).centered())?;
        self.hits.push((panel, action));
        Ok(())
    }

    /// "Sen yokken" and the span with the accent's mark, then the rows: each one's mark, what
    /// happened and when, newest first.
    fn away(&mut self, c: &AwayCard) -> Result<()> {
        let d = |v: f32| v * self.s.0;
        let title = TextStyle::new(Face::Display, Weight::Bold, d(22.0), white(1.0)).top().ellipsis();
        self.ink.text("While you were away", rect(d(20.0), d(20.0), self.w - d(100.0), d(30.0)), &title)?;
        let sub = TextStyle::new(Face::Text, Weight::Regular, d(12.5), white(0.7)).top().ellipsis();
        self.ink.text(&c.span, rect(d(20.0), d(52.0), self.w - d(100.0), d(18.0)), &sub)?;
        let (r, g, b) = timer_accent();
        let mark = rect(self.w - d(20.0) - d(40.0), d(22.0), d(40.0), d(40.0));
        self.ink.rrect(mark, d(20.0), rgba(r, g, b, 1.0))?;
        self.ink.text(&glyph(Icon::History).to_string(), mark, &TextStyle::new(Face::Icons, Weight::Regular, d(19.0), white(1.0)).centered())?;
        let top = BRIEF_HEADER + BRIEF_GAP;
        self.ink.rrect(rect(d(12.0), d(top), self.w - d(24.0), d(away_panel_dip(c))), d(24.0), white(0.06))?;
        let mut y = top + 8.0;
        let text = TextStyle::new(Face::Text, Weight::SemiBold, d(13.0), white(1.0)).ellipsis();
        let time = TextStyle::new(Face::Text, Weight::Regular, d(12.5), white(0.7)).trailing().tabular();
        for row in &c.rows {
            let side = d(28.0).round();
            let dot = rect(d(24.0).round(), (d(y + AWAY_ROW / 2.0) - side / 2.0).round(), side, side);
            match &row.crest {
                // The match card's crest look: on a faint circle, a fifth in from its edge.
                Some(c) => {
                    self.ink.rrect(dot, side / 2.0, white(0.10))?;
                    let inner = rect(dot.left + side * 0.1, dot.top + side * 0.1, side * 0.8, side * 0.8);
                    self.ink.image_rounded(&c.px, c.size, rect(0.0, 0.0, c.size.0 as f32, c.size.1 as f32), inner, 0.0)?;
                }
                None => self.away_mark(row.kind, dot)?,
            }
            let tw = self.ink.measure(&row.time, &time)? + d(1.0);
            let right = self.w - d(28.0);
            self.ink.text(&row.time, rect(right - tw, d(y), tw, d(AWAY_ROW)), &time)?;
            self.ink.text(&row.text, rect(d(64.0), d(y), (right - tw - d(12.0) - d(64.0)).max(0.0), d(AWAY_ROW)), &text)?;
            y += AWAY_ROW;
        }
        if c.more > 0 {
            let more = TextStyle::new(Face::Text, Weight::Regular, d(12.5), white(0.6));
            self.ink.text(&format!("+{} more", c.more), rect(d(64.0), d(y), self.w - d(92.0), d(AWAY_MORE - 6.0)), &more)?;
        }
        Ok(())
    }

    /// A row's mark: its kind's colour and glyph, as its own pill draws it.
    fn away_mark(&self, kind: AwayKind, dot: D2D_RECT_F) -> Result<()> {
        let side = dot.right - dot.left;
        let fill = |(r, g, b): (u8, u8, u8)| self.ink.rrect(dot, side / 2.0, rgba(r, g, b, 1.0));
        let icon = |i: Icon| self.ink.text(&glyph(i).to_string(), dot, &TextStyle::new(Face::Icons, Weight::Regular, side * 0.5, white(1.0)).centered());
        match kind {
            AwayKind::Claude => {
                fill(CLAUDE_ACCENT)?;
                paint_live_mark(self.ink, dot, true, LiveState::Run, side * 0.5)
            }
            AwayKind::Live => {
                fill(timer_accent())?;
                icon(Icon::Check)
            }
            AwayKind::Match => {
                self.ink.rrect(dot, side / 2.0, white(0.12))?;
                self.ink.text("\u{26BD}", dot, &TextStyle::new(Face::Text, Weight::Regular, side * 0.55, white(1.0)).centered().emoji())
            }
            AwayKind::Rain { snow } => {
                fill(if snow { SNOW_ACCENT } else { RAIN_ACCENT })?;
                self.ink.svg(if snow { SNOW_ICON } else { RAIN_ICON }, dot, white(1.0))
            }
            AwayKind::Alert(k) => {
                let (colour, svg) = alert_style(k);
                fill(colour)?;
                self.ink.svg(svg, dot, white(1.0))
            }
            AwayKind::Game(store) => {
                fill(store_accent(store))?;
                icon(Icon::Download)
            }
            AwayKind::Timer => {
                fill(timer_accent())?;
                icon(Icon::Timer)
            }
            AwayKind::Battery => {
                fill(LOW_ACCENT)?;
                // Battery10: the low one.
                self.ink.text("\u{EBA1}", dot, &TextStyle::new(Face::Icons, Weight::Regular, side * 0.5, white(1.0)).centered())
            }
            AwayKind::App(accent) => {
                fill(accent)?;
                icon(Icon::Bell)
            }
        }
    }

    /// The day's total and how it compares; the week's bars (today in green); the apps used
    /// longest, each with its icon, its time and a bar of its share.
    fn wellbeing(&mut self, c: &WellbeingCard) -> Result<()> {
        let d = |v: f32| v * self.s.0;
        let (r, g, b) = WELLBEING_ACCENT;
        let green = rgba(r, g, b, 1.0);
        let label = TextStyle::new(Face::Text, Weight::Regular, d(12.5), white(0.6)).top();
        self.ink.text("Digital Wellbeing", rect(d(20.0), d(20.0), d(200.0), d(18.0)), &label)?;
        let total = TextStyle::new(Face::Display, Weight::Bold, d(30.0), white(1.0)).top().tabular();
        self.ink.text(&c.total, rect(d(20.0), d(38.0), self.w - d(100.0), d(40.0)), &total)?;
        let caption = TextStyle::new(Face::Text, Weight::Regular, d(12.5), white(0.7)).top().ellipsis();
        self.ink.text(&c.caption, rect(d(20.0), d(76.0), self.w - d(40.0), d(18.0)), &caption)?;
        let mark = rect(self.w - d(20.0) - d(40.0), d(22.0), d(40.0), d(40.0));
        self.ink.rrect(mark, d(20.0), green)?;
        self.ink.text(WELLBEING_GLYPH, mark, &TextStyle::new(Face::Icons, Weight::Regular, d(19.0), white(1.0)).centered())?;

        // The week: seven bars from the panel's baseline, the day under each.
        let panel = rect(d(12.0), d(WELL_HEADER), self.w - d(24.0), d(WELL_WEEK));
        self.ink.rrect(panel, d(24.0), white(0.06))?;
        let col = (panel.right - panel.left - d(24.0)) / 7.0;
        let (base, tall, bar) = (panel.top + d(72.0), d(52.0), d(14.0));
        for (i, share) in c.week.iter().enumerate() {
            let cx = panel.left + d(12.0) + col * (i as f32 + 0.5);
            let today = i == 6;
            let h = (tall * share.clamp(0.0, 1.0)).max(d(4.0));
            self.ink.rrect(rect(cx - bar / 2.0, base - h, bar, h), bar / 2.0, if today { green } else { white(0.26) })?;
            let day = TextStyle::new(Face::Text, if today { Weight::SemiBold } else { Weight::Regular }, d(11.5), white(if today { 1.0 } else { 0.6 })).centered();
            self.ink.text(c.days[i], rect(cx - col / 2.0, base + d(6.0), col, d(18.0)), &day)?;
        }

        // The apps: icon, name and time, the share's bar under the name.
        let mut y = WELL_HEADER + WELL_WEEK + 10.0;
        if c.apps.is_empty() {
            self.ink.text("No screen time yet today", rect(d(24.0), d(y), self.w - d(48.0), d(WELL_ROW)), &TextStyle::new(Face::Text, Weight::Regular, d(13.0), white(0.6)))?;
        }
        for a in &c.apps {
            let icon = rect(d(24.0), d(y + 8.0), d(24.0), d(24.0));
            match &a.icon {
                Some(px) => self.ink.image_rounded(&px.px, px.size, rect(0.0, 0.0, px.size.0 as f32, px.size.1 as f32), icon, 0.0)?,
                None => {
                    self.ink.rrect(icon, d(7.0), white(0.12))?;
                    let letter: String = a.name.chars().next().map(|c| c.to_uppercase().collect()).unwrap_or_default();
                    self.ink.text(&letter, icon, &TextStyle::new(Face::Text, Weight::SemiBold, d(12.0), white(0.9)).centered())?;
                }
            }
            let x = d(60.0);
            let time = TextStyle::new(Face::Text, Weight::Regular, d(12.5), white(0.7)).top().trailing().tabular();
            let tw = self.ink.measure(&a.time, &time)? + d(1.0);
            self.ink.text(&a.time, rect(self.w - d(24.0) - tw, d(y + 5.0), tw, d(18.0)), &time)?;
            let name = TextStyle::new(Face::Text, Weight::SemiBold, d(13.0), white(1.0)).top().ellipsis();
            self.ink.text(&a.name, rect(x, d(y + 4.0), self.w - d(24.0) - tw - d(10.0) - x, d(18.0)), &name)?;
            let track = self.w - d(24.0) - x;
            let line = d(3.0).round();
            self.ink.rrect(rect(x, d(y + 27.0), track, line), line / 2.0, white(0.12))?;
            self.ink.rrect(rect(x, d(y + 27.0), (track * a.share.clamp(0.0, 1.0)).max(line), line), line / 2.0, green)?;
            y += WELL_ROW;
        }
        // The week's music, as the brief's links: the note where the cover would be.
        if let Some((head, sub)) = &c.music {
            let y = WELL_HEADER + WELL_WEEK + 10.0 + c.apps.len().max(1) as f32 * WELL_ROW + BRIEF_GAP;
            self.brief_link(y, MUSIC_ACCENT, &glyph(Icon::Music).to_string(), (head, sub), CardAction::WellbeingMusic)?;
        }
        Ok(())
    }

    /// Claude's mark and "Claude'a sor"; the field with what's typed (its end when it runs long)
    /// and the caret; then the answer, scrolled to its end while it's written (`answer`: how far
    /// it runs past its area), and "Kopyala" once it's whole.
    fn question(&mut self, c: &QuestionCard, answer: &mut Option<f32>) -> Result<()> {
        let d = |v: f32| v * self.s.0;
        let (r, g, b) = CLAUDE_ACCENT;
        let mark = rect(d(20.0), d(14.0), d(28.0), d(28.0));
        self.ink.rrect(mark, d(14.0), rgba(r, g, b, 1.0))?;
        let inset = d(28.0) * 0.22;
        self.ink.svg(CLAUDE_ICON, D2D_RECT_F { left: mark.left + inset, top: mark.top + inset, right: mark.right - inset, bottom: mark.bottom - inset }, white(1.0))?;
        let title = TextStyle::new(Face::Text, Weight::SemiBold, d(15.0), white(1.0)).ellipsis();
        self.ink.text("Ask Claude", rect(d(58.0), d(14.0), self.w - d(58.0) - d(130.0), d(28.0)), &title)?;
        if c.show == QuestionShow::Idle {
            let hint = TextStyle::new(Face::Text, Weight::Regular, d(12.0), white(0.45)).trailing();
            self.ink.text("Press Enter to ask", rect(self.w - d(20.0) - d(130.0), d(14.0), d(130.0), d(28.0)), &hint)?;
        }

        let field = rect(d(12.0), d(QUESTION_HEADER), self.w - d(24.0), d(QUESTION_FIELD));
        let fh = field.bottom - field.top;
        self.ink.rrect(field, fh / 2.0, white(0.08))?;
        let (left, right) = (field.left + d(16.0), field.right - d(16.0));
        let typed = TextStyle::new(Face::Text, Weight::Regular, d(14.0), white(1.0));
        // Empty, the caret stands just before the placeholder's first letter.
        let mut caret_x = left - d(4.0);
        if c.question.is_empty() {
            self.ink.text("Ask anything…", D2D_RECT_F { left, right, ..field }, &TextStyle::new(Face::Text, Weight::Regular, d(14.0), white(0.45)))?;
        } else {
            // A long question shows its end, where the caret is.
            let tw = self.ink.measure(&c.question, &typed)?;
            let x = if tw > right - left { right - tw } else { left };
            unsafe { self.ink.dc.PushAxisAlignedClip(&D2D_RECT_F { left, right: right + d(2.0), ..field }, windows::Win32::Graphics::Direct2D::D2D1_ANTIALIAS_MODE_ALIASED) };
            let drawn = self.ink.text(&c.question, rect(x, field.top, tw + d(2.0), fh), &typed);
            unsafe { self.ink.dc.PopAxisAlignedClip() };
            drawn?;
            caret_x = (x + tw).min(right);
        }
        if c.caret {
            let caret_h = d(18.0).round();
            self.ink.rrect(rect(caret_x + d(1.0), (field.top + (fh - caret_h) / 2.0).round(), d(2.0).round(), caret_h), d(1.0), rgba(r, g, b, 1.0))?;
        }
        if c.show == QuestionShow::Idle {
            return Ok(());
        }

        let top = QUESTION_HEADER + QUESTION_FIELD + 12.0;
        let area = rect(d(20.0), d(top), self.w - d(40.0), d(c.lines.clamp(1, QUESTION_MAX_LINES) as f32 * QUESTION_LINE + 8.0));
        let (aw, ah) = (area.right - area.left, area.bottom - area.top);
        let (text, alpha) = match c.show {
            QuestionShow::Thinking => ("Thinking…", 0.6),
            QuestionShow::Problem => (c.answer.as_str(), 0.7),
            _ => (c.answer.as_str(), 0.92),
        };
        let words = TextStyle::new(Face::Text, Weight::Regular, d(13.5), white(alpha)).top().wrap();
        let h = self.ink.text_height(text, aw, &words)?;
        let overflow = (h - ah).max(0.0);
        // Scrolled by whole lines, so the top line is never cut (the last shows all the same).
        let line = self.ink.text_height("Ag", aw, &words)?.max(1.0);
        let offset = ((c.scroll.unwrap_or(overflow).clamp(0.0, overflow) / line).ceil() * line).min((h - line).max(0.0));
        *answer = Some(overflow);
        unsafe { self.ink.dc.PushAxisAlignedClip(&area, windows::Win32::Graphics::Direct2D::D2D1_ANTIALIAS_MODE_ALIASED) };
        let drawn = self.ink.text(text, rect(area.left, area.top - offset, aw, h + d(2.0)), &words);
        unsafe { self.ink.dc.PopAxisAlignedClip() };
        drawn?;

        if let Some(label) = &c.copy {
            let style = TextStyle::new(Face::Text, Weight::Medium, d(12.0), white(0.85));
            let tw = self.ink.measure(label, &style)?;
            let chip = rect(d(20.0), area.bottom + d(10.0), d(44.0) + tw.ceil(), d(28.0));
            let ch = chip.bottom - chip.top;
            self.ink.rrect(chip, ch / 2.0, white(0.09))?;
            self.ink.text(&glyph(Icon::Copy).to_string(), D2D_RECT_F { left: chip.left + d(12.0), right: chip.left + d(26.0), ..chip }, &TextStyle::new(Face::Icons, Weight::Regular, d(13.0), white(0.85)).centered())?;
            self.ink.text(label, D2D_RECT_F { left: chip.left + d(32.0), ..chip }, &style)?;
            self.hits.push((chip, CardAction::QuestionCopy));
        }
        Ok(())
    }

    /// The week's music: how long, with the top track's colour on the mark; the tracks played
    /// most (their place, their cover's colour, title and artist, plays); the artists listened to
    /// longest.
    fn music(&mut self, c: &MusicCard) -> Result<()> {
        let d = |v: f32| v * self.s.0;
        let label = TextStyle::new(Face::Text, Weight::Regular, d(12.5), white(0.6)).top();
        self.ink.text("This week", rect(d(20.0), d(20.0), d(200.0), d(18.0)), &label)?;
        let total = TextStyle::new(Face::Display, Weight::Bold, d(30.0), white(1.0)).top().tabular();
        self.ink.text(&c.total, rect(d(20.0), d(38.0), self.w - d(100.0), d(40.0)), &total)?;
        let caption = TextStyle::new(Face::Text, Weight::Regular, d(12.5), white(0.7)).top().ellipsis();
        self.ink.text(&c.caption, rect(d(20.0), d(76.0), self.w - d(40.0), d(18.0)), &caption)?;
        let mark = rect(self.w - d(20.0) - d(40.0), d(22.0), d(40.0), d(40.0));
        let (r, g, b) = c.accent;
        self.ink.rrect(mark, d(20.0), rgba(r, g, b, 1.0))?;
        self.ink.text(&glyph(Icon::Music).to_string(), mark, &TextStyle::new(Face::Icons, Weight::Regular, d(19.0), white(1.0)).centered())?;

        let right = self.w - d(28.0);
        let small = TextStyle::new(Face::Text, Weight::Regular, d(12.5), white(0.7)).trailing().tabular();
        let name = TextStyle::new(Face::Text, Weight::SemiBold, d(13.0), white(1.0)).top().ellipsis();
        let tracks_h = 16.0 + c.tracks.len().max(1) as f32 * MUSIC_ROW;
        self.ink.rrect(rect(d(12.0), d(WELL_HEADER), self.w - d(24.0), d(tracks_h)), d(24.0), white(0.06))?;
        let mut y = WELL_HEADER + 8.0;
        if c.tracks.is_empty() {
            self.ink.text("No music played this week", rect(d(24.0), d(y), self.w - d(48.0), d(MUSIC_ROW)), &TextStyle::new(Face::Text, Weight::Regular, d(13.0), white(0.6)))?;
        }
        let side = d(28.0).round();
        for (i, t) in c.tracks.iter().enumerate() {
            let rank = TextStyle::new(Face::Text, Weight::SemiBold, d(13.0), white(0.5)).centered().tabular();
            self.ink.text(&(i + 1).to_string(), rect(d(20.0), d(y), d(18.0), d(MUSIC_ROW)), &rank)?;
            let dot = rect(d(46.0).round(), (d(y + MUSIC_ROW / 2.0) - side / 2.0).round(), side, side);
            let (r, g, b) = t.accent;
            self.ink.rrect(dot, side / 2.0, rgba(r, g, b, 1.0))?;
            self.ink.text(&glyph(Icon::Music).to_string(), dot, &TextStyle::new(Face::Icons, Weight::Regular, side * 0.5, white(1.0)).centered())?;
            let pw = self.ink.measure(&t.plays, &small)? + d(1.0);
            self.ink.text(&t.plays, rect(right - pw, d(y), pw, d(MUSIC_ROW)), &small)?;
            let (x, text_w) = (d(86.0), right - pw - d(12.0) - d(86.0));
            self.ink.text(&t.title, rect(x, d(y + 5.0), text_w, d(18.0)), &name)?;
            self.ink.text(&t.artist, rect(x, d(y + 23.0), text_w, d(16.0)), &TextStyle::new(Face::Text, Weight::Regular, d(12.0), white(0.7)).top().ellipsis())?;
            y += MUSIC_ROW;
        }
        if c.artists.is_empty() {
            return Ok(());
        }
        let top = WELL_HEADER + tracks_h + BRIEF_GAP;
        self.ink.rrect(rect(d(12.0), d(top), self.w - d(24.0), d(16.0 + c.artists.len() as f32 * ARTIST_ROW)), d(24.0), white(0.06))?;
        let mut y = top + 8.0;
        let one_line = TextStyle::new(Face::Text, Weight::SemiBold, d(13.0), white(1.0)).ellipsis();
        for (i, (artist, time)) in c.artists.iter().enumerate() {
            let rank = TextStyle::new(Face::Text, Weight::SemiBold, d(13.0), white(0.5)).centered().tabular();
            self.ink.text(&(i + 1).to_string(), rect(d(20.0), d(y), d(18.0), d(ARTIST_ROW)), &rank)?;
            let dot = rect(d(46.0).round(), (d(y + ARTIST_ROW / 2.0) - side / 2.0).round(), side, side);
            self.ink.rrect(dot, side / 2.0, white(0.12))?;
            let letter: String = artist.chars().next().map(|c| c.to_uppercase().collect()).unwrap_or_default();
            self.ink.text(&letter, dot, &TextStyle::new(Face::Text, Weight::SemiBold, d(12.0), white(0.9)).centered())?;
            let tw = self.ink.measure(time, &small)? + d(1.0);
            self.ink.text(time, rect(right - tw, d(y), tw, d(ARTIST_ROW)), &small)?;
            self.ink.text(artist, rect(d(86.0), d(y), right - tw - d(12.0) - d(86.0), d(ARTIST_ROW)), &one_line)?;
            y += ARTIST_ROW;
        }
        Ok(())
    }

    fn weather(&mut self, c: &WeatherCard) -> Result<()> {
        let d = |v: f32| v * self.s.0;
        let sky = match c.fx {
            WeatherFx::Clouds | WeatherFx::Fog => [0x8A, 0x97, 0xA6],
            _ => [0x4A, 0x6A, 0xA0],
        };
        self.ink.glow(self.w * 0.78, d(36.0), self.w * 0.85, self.h * 0.70, rgba(sky[0], sky[1], sky[2], 0.22))?;

        let title = TextStyle::new(Face::Text, Weight::SemiBold, d(16.0), white(1.0)).top().ellipsis();
        self.ink.text(&c.city, rect(d(20.0), d(16.0), d(190.0), d(22.0)), &title)?;
        let sub = TextStyle::new(Face::Text, Weight::Regular, d(13.0), sec_text()).top().ellipsis();
        self.ink.text(&c.summary, rect(d(20.0), d(40.0), d(210.0), d(18.0)), &sub)?;

        // Large light-weight temperature (48px) with crisp SVG weather icon and small caption on the right.
        let temp = TextStyle::new(Face::Display, Weight::Light, d(46.0), white(1.0)).tabular().trailing();
        self.ink.text(&c.temp, rect(self.w - d(20.0) - d(104.0), d(6.0), d(104.0), d(52.0)), &temp)?;
        let tw = self.ink.measure(&c.temp, &temp)?;
        let icon_s = d(34.0).round();
        let icon_box = rect((self.w - d(20.0) - tw - icon_s - d(10.0)).round(), d(15.0).round(), icon_s, icon_s);
        self.ink.svg(weather_svg_for_symbol(&c.symbol), icon_box, neutral_icon())?;
        let caption = TextStyle::new(Face::Text, Weight::Regular, d(11.5), sec_text()).trailing();
        self.ink.text("Now", rect(self.w - d(20.0) - d(120.0), d(56.0), d(120.0), d(16.0)), &caption)?;

        self.hours_panel(&c.hours, d(80.0))?;
        // The alert under the hours, in the pill's words: its mark on its colour, the head, the rest.
        if let Some((kind, head, detail)) = &c.alert {
            let ((r, g, b), icon) = alert_style(*kind);
            let (size, y) = (d(20.0).round(), d(80.0 + HOURS_PANEL + 12.0).round());
            let dot = rect(d(20.0).round(), y, size, size);
            self.ink.rrect(dot, d(4.0), rgba(r, g, b, 1.0))?;
            self.ink.svg(icon, dot, white(1.0))?;
            let head_st = TextStyle::new(Face::Text, Weight::SemiBold, d(12.5), white(1.0));
            let x = dot.right + d(9.0);
            let head_w = self.ink.measure(head, &head_st)?;
            self.ink.text(head, rect(x, y, head_w + d(2.0), size), &head_st)?;
            let rest = TextStyle::new(Face::Text, Weight::Regular, d(12.5), sec_text()).ellipsis();
            self.ink.text(&format!(" · {detail}"), rect(x + head_w, y, (self.w - d(24.0) - x - head_w).max(0.0), size), &rest)?;
        }
        Ok(())
    }

    /// Hourly forecast as a plain row separated by one thin 1px divider (no box); the selected
    /// hour (first column) is raised (`white(0.09)`) with a 16×3px system accent pill under it.
    fn hours_panel(&mut self, hours: &[(String, String, String)], top: f32) -> Result<()> {
        let d = |v: f32| v * self.s.0;
        let panel = rect(d(16.0), top, self.w - d(32.0), d(HOURS_PANEL));
        self.ink.line(panel.left, panel.top + 0.5, panel.right, panel.top + 0.5, 1.0, white(0.07))?;
        let n = hours.len().max(1) as f32;
        let gap = d(4.0);
        let col = ((panel.right - panel.left) - gap * (n - 1.0).max(0.0)) / n;
        let (ar, ag, ab) = timer_accent();
        for (i, (hour, sym, t)) in hours.iter().enumerate() {
            let x = panel.left + i as f32 * (col + gap);
            let item_rect = rect(x, panel.top + d(10.0), col, d(HOURS_PANEL - 12.0));
            if i == 0 {
                self.ink.rrect(item_rect, d(4.0), white(0.09))?;
                let pill_w = d(16.0).round();
                let pill_h = d(3.0).round();
                let pill_x = (x + (col - pill_w) / 2.0).round();
                self.ink.rrect(rect(pill_x, item_rect.bottom - pill_h - d(4.0), pill_w, pill_h), d(1.5), rgba(ar, ag, ab, 1.0))?;
            }
            let hour_st = TextStyle::new(Face::Text, Weight::Regular, d(12.0), sec_text()).centered();
            self.ink.text(hour, rect(x, item_rect.top + d(6.0), col, d(16.0)), &hour_st)?;
            let icon_s = d(22.0).round();
            let icon_box = rect((x + (col - icon_s) / 2.0).round(), (item_rect.top + d(28.0)).round(), icon_s, icon_s);
            self.ink.svg(weather_svg_for_symbol(sym), icon_box, neutral_icon())?;
            let temp_st = TextStyle::new(Face::Text, Weight::SemiBold, d(13.0), white(1.0)).centered().tabular();
            self.ink.text(t, rect(x, item_rect.top + d(54.0), col, d(18.0)), &temp_st)?;
        }
        Ok(())
    }

    /// One UI 8's three modes: Samsung's icons on round buttons with their names under them, the
    /// current one filled with the Buds' blue (the quick panel's look, in the dark).
    fn noise_control(&mut self, mode: NoiseControl) -> Result<()> {
        let d = |v: f32| v * self.s.0;
        let (r, g, bl) = timer_accent();
        let (size, top, step) = (d(44.0), d(158.0), d(110.0));
        let n = NoiseControl::ALL.len() as f32;
        for (i, m) in NoiseControl::ALL.into_iter().enumerate() {
            let cx = self.w / 2.0 + (i as f32 - (n - 1.0) / 2.0) * step;
            let button = rect((cx - size / 2.0).round(), top, size, size);
            let on = m == mode;
            self.ink.rrect(button, d(4.0), if on { rgba(r, g, bl, 1.0) } else { white(0.06) })?;
            self.ink.svg(m.icon(), button, if on { rgba(0, 0, 0, 1.0) } else { neutral_icon() })?;
            let label = TextStyle::new(Face::Text, if on { Weight::SemiBold } else { Weight::Regular }, d(12.0), if on { white(1.0) } else { sec_text() }).centered();
            let label_box = rect(cx - step / 2.0, top + size + d(6.0), step, d(16.0));
            self.ink.text(m.label(), label_box, &label)?;
            self.hits.push((rect(cx - step / 2.0, top, step, label_box.bottom - top), CardAction::BudsNoise(m.byte())));
        }
        Ok(())
    }

    /// "Ses çıkışı": back, title and what's playing, Windows' sound settings; the current device
    /// with its volume; then the other devices (three at a time, the rest scroll).
    fn output(&mut self, _card: &MediaCard, page: &OutputPage) -> Result<()> {
        let d = |v: f32| v * self.s.0;
        let (ar, ag, ab) = timer_accent();
        let accent = rgba(ar, ag, ab, 1.0);
        let icon_st = |size: f32| TextStyle::new(Face::Icons, Weight::Regular, d(size), white(1.0)).centered();
        let sub_color = |tone: SubTone, normal: f32| match tone {
            SubTone::Normal => white(normal),
            SubTone::Busy => rgba(ar.saturating_add(60), ag.saturating_add(60), ab.saturating_add(60), 1.0),
            SubTone::Failed => rgba(0xFF, 0x8A, 0x80, 1.0),
        };

        let back = rect(d(16.0), d(16.0), d(32.0), d(32.0));
        self.ink.rrect(back, d(4.0), white(0.06))?;
        self.ink.text(&glyph(Icon::Back).to_string(), back, &icon_st(14.0))?;
        self.hits.push((back, CardAction::OutputBack));
        let title = TextStyle::new(Face::Text, Weight::SemiBold, d(16.0), white(1.0)).top();
        self.ink.text("Audio Output", rect(d(58.0), d(14.0), d(220.0), d(22.0)), &title)?;
        let sub = TextStyle::new(Face::Text, Weight::Regular, d(12.0), sec_text()).top().ellipsis();
        self.ink.text(&page.subtitle, rect(d(58.0), d(36.0), d(222.0), d(16.0)), &sub)?;
        let gear = rect(self.w - d(48.0), d(16.0), d(32.0), d(32.0));
        self.ink.rrect(gear, d(4.0), white(0.05))?;
        self.ink.text(&glyph(Icon::Settings).to_string(), gear, &icon_st(15.0))?;
        self.hits.push((gear, CardAction::OutputSettings));

        let Some(current) = page.rows.first() else { return Ok(()) };
        let row = rect(d(12.0), d(64.0), d(316.0), d(108.0));
        self.ink.rrect(row, d(4.0), white(0.09))?;
        self.ink.rrect_outline(rect(row.left + 0.5, row.top + 0.5, row.right - row.left - 1.0, row.bottom - row.top - 1.0), d(3.5), 1.0, white(0.07))?;
        let icon = rect(d(24.0), d(76.0), d(36.0), d(36.0));
        self.ink.rrect(icon, d(4.0), accent)?;
        let dark_icon = TextStyle::new(Face::Icons, Weight::Regular, d(17.0), rgba(0, 0, 0, 1.0)).centered();
        self.ink.text(&glyph(output_icon(current.kind, current.earbuds)).to_string(), icon, &dark_icon)?;
        let name = TextStyle::new(Face::Text, Weight::SemiBold, d(14.5), white(1.0)).top().ellipsis();
        self.ink.text(&current.name, rect(d(72.0), d(76.0), d(212.0), d(20.0)), &name)?;
        let line = TextStyle::new(Face::Text, Weight::Regular, d(12.0), sub_color(current.tone, 0.72)).top().ellipsis();
        self.ink.text(&current.sub, rect(d(72.0), d(96.0), d(212.0), d(16.0)), &line)?;
        let check = rect(d(294.0), d(84.0), d(20.0), d(20.0));
        self.ink.rrect(check, d(10.0), accent)?;
        self.ink.text(&glyph(Icon::Check).to_string(), check, &TextStyle::new(Face::Icons, Weight::Bold, d(11.0), rgba(0, 0, 0, 1.0)).centered())?;

        let track = rect(d(24.0), d(128.0), d(292.0), d(28.0));
        self.ink.rrect(track, d(4.0), white(0.12))?;
        if let Some(v) = page.volume {
            let fill = if page.muted { white(0.24) } else { accent };
            let fill_w = (track.right - track.left) * v;
            self.ink.rrect(rect(track.left, track.top, fill_w.max(d(28.0)), d(28.0)), d(4.0), fill)?;
            let speaker = rect(track.left, track.top, d(32.0), d(28.0));
            let glyph_icon = if page.muted { Icon::Mute } else { Icon::Volume };
            let spk_st = TextStyle::new(Face::Icons, Weight::Regular, d(14.0), if page.muted { white(1.0) } else { rgba(0, 0, 0, 1.0) }).centered();
            self.ink.text(&glyph(glyph_icon).to_string(), speaker, &spk_st)?;
            self.hits.push((speaker, CardAction::OutputMute));
            let label = if page.muted { "Muted".to_string() } else { crate::i18n::percent((v * 100.0).round() as u32) };
            let value = TextStyle::new(Face::Text, Weight::SemiBold, d(12.0), white(0.95)).tabular().trailing();
            self.ink.text(&label, rect(track.right - d(72.0), track.top, d(60.0), d(28.0)), &value)?;
            *self.volume = Some((rect(track.left + d(32.0), track.top - d(4.0), track.right - track.left - d(32.0), d(36.0)), track.left, track.right));
        }

        let others = &page.rows[1..];
        let name = TextStyle::new(Face::Text, Weight::Medium, d(14.0), white(1.0)).top().ellipsis();
        for (i, r) in others.iter().enumerate().skip(page.scroll).take(OUTPUT_VISIBLE_ROWS) {
            let y = d(178.0) + (i - page.scroll) as f32 * d(54.0);
            let row = rect(d(12.0), y, d(316.0), d(50.0));
            self.ink.rrect(row, d(4.0), white(0.05))?;
            self.ink.rrect_outline(rect(row.left + 0.5, row.top + 0.5, row.right - row.left - 1.0, row.bottom - row.top - 1.0), d(3.5), 1.0, white(0.07))?;
            let icon = rect(d(20.0), y + d(7.0), d(36.0), d(36.0));
            self.ink.rrect(icon, d(4.0), white(0.08))?;
            self.ink.text(&glyph(output_icon(r.kind, r.earbuds)).to_string(), icon, &icon_st(16.0))?;
            self.ink.text(&r.name, rect(d(66.0), y + d(7.0), d(252.0), d(19.0)), &name)?;
            let line = TextStyle::new(Face::Text, Weight::Regular, d(12.0), sub_color(r.tone, 0.65)).top().ellipsis();
            self.ink.text(&r.sub, rect(d(66.0), y + d(27.0), d(252.0), d(16.0)), &line)?;
            self.hits.push((row, CardAction::OutputPick(i + 1)));
        }
        let hidden = others.len().saturating_sub(page.scroll + OUTPUT_VISIBLE_ROWS);
        if hidden > 0 {
            let y = d(178.0) + (OUTPUT_VISIBLE_ROWS - 1) as f32 * d(54.0) + d(14.0);
            let capsule = rect(d(328.0) - d(12.0) - d(36.0), y, d(36.0), d(22.0));
            self.ink.rrect(capsule, d(4.0), white(0.12))?;
            let more = TextStyle::new(Face::Text, Weight::SemiBold, d(11.5), white(0.85)).centered();
            self.ink.text(&format!("+{hidden}"), capsule, &more)?;
        }
        Ok(())
    }

    #[allow(dead_code)]
    fn lyrics(&mut self, band: D2D_RECT_F, view: &LyricsView) -> Result<()> {
        let d = |v: f32| v * self.s.0;
        self.ink.rrect(band, d(4.0), [0.0, 0.0, 0.0, 0.68])?;
        let (x, w) = (band.left + d(14.0), band.right - band.left - d(28.0));
        let p = 1.0 - (1.0 - view.progress.clamp(0.0, 1.0)).powi(3);
        let places: [(f32, f32, f32, f32, bool); 5] = [
            (band.top + d(6.0), d(18.0), d(12.0), 0.0, false),
            (band.top + d(24.0), d(18.0), d(12.0), 0.45, false),
            (band.top + d(44.0), d(44.0), d(15.0), 1.0, true),
            (band.bottom - d(24.0), d(18.0), d(12.0), 0.45, false),
            (band.bottom - d(4.0), d(18.0), d(12.0), 0.0, false),
        ];
        let lerp = |a: f32, b: f32| a + (b - a) * p;
        let now = view.current.as_deref().filter(|t| !t.is_empty()).unwrap_or("♪");
        let lines = [view.before.as_deref(), view.prev.as_deref(), Some(now), view.next.as_deref()];
        for (i, line) in lines.iter().enumerate() {
            let Some(text) = line else { continue };
            let (from, to) = (places[i + 1], places[i]);
            let (top, height, size, alpha) = (lerp(from.0, to.0), lerp(from.1, to.1), lerp(from.2, to.2), lerp(from.3, to.3));
            if alpha <= 0.01 {
                continue;
            }
            let bright = if p < 0.5 { from.4 } else { to.4 };
            let weight = if bright { Weight::SemiBold } else { Weight::Medium };
            let st = TextStyle::new(Face::Text, weight, size, white(alpha)).centered();
            let st = if to.4 || from.4 { st.wrap() } else { st.ellipsis() };
            self.ink.text(text, rect(x, top, w, height), &st)?;
        }
        Ok(())
    }

    fn paint_idle_media(&mut self, card: &MediaCard) -> Result<()> {
        let d = |v: f32| v * self.s.0;
        let (ar, ag, ab) = timer_accent();

        // Pin button at top right
        let pin = rect(self.w - d(16.0) - d(28.0), d(16.0), d(28.0), d(28.0));
        if card.pinned {
            self.ink.rrect(pin, d(4.0), white(0.09))?;
        }
        let pin_color = if card.pinned { rgba(ar, ag, ab, 1.0) } else { neutral_icon() };
        let pin_icon = rect((pin.left + d(6.0)).round(), (pin.top + d(6.0)).round(), d(16.0).round(), d(16.0).round());
        self.ink.svg(PIN_SVG, pin_icon, pin_color)?;
        self.hits.push((pin, CardAction::Pin));

        // Title and subtitle
        let title = TextStyle::new(Face::Text, Weight::SemiBold, d(20.0), white(1.0)).top().ellipsis();
        self.ink.text(&crate::i18n::tr("Nothing playing"), rect(d(16.0), d(16.0), pin.left - d(22.0), d(26.0)), &title)?;
        let sub = TextStyle::new(Face::Text, Weight::Regular, d(13.0), sec_text()).top().ellipsis();
        self.ink.text(&crate::i18n::tr("Pick an app to start listening."), rect(d(16.0), d(44.0), self.w - d(32.0), d(20.0)), &sub)?;

        // Two tiles side by side
        let card_w = ((self.w - d(32.0) - d(12.0)) / 2.0).round();
        let card_h = d(74.0);
        let card_y = d(72.0);

        // --- Card 1: Spotify ---
        let c1 = rect(d(16.0), card_y, card_w, card_h);
        self.ink.rrect(c1, d(8.0), white(0.05))?;
        self.ink.rrect_outline(rect(c1.left + 0.5, c1.top + 0.5, card_w - 1.0, card_h - 1.0), d(7.5), 1.0, white(0.08))?;
        self.hits.push((c1, CardAction::LaunchSpotify));

        // Spotify icon (22x22)
        let icon1 = rect(c1.left + d(12.0), c1.top + d(9.0), d(22.0), d(22.0));
        self.ink.svg(SPOTIFY_SVG, icon1, white(1.0))?;

        // Spotify text
        let st_title = TextStyle::new(Face::Text, Weight::SemiBold, d(13.0), white(1.0)).top().ellipsis();
        self.ink.text("Spotify", rect(c1.left + d(12.0), c1.top + d(35.0), card_w - d(24.0), d(17.0)), &st_title)?;
        let st_sub = TextStyle::new(Face::Text, Weight::Regular, d(11.0), sec_text()).top().ellipsis();
        let sp_installed = crate::platform::media_launch::is_spotify_installed();
        let sp_label = if sp_installed { crate::i18n::tr("Open app") } else { crate::i18n::tr("Open in browser") };
        self.ink.text(&sp_label, rect(c1.left + d(12.0), c1.top + d(51.0), card_w - d(24.0), d(16.0)), &st_sub)?;

        // --- Card 2: YouTube ---
        let c2 = rect(c1.right + d(12.0), card_y, card_w, card_h);
        self.ink.rrect(c2, d(8.0), white(0.05))?;
        self.ink.rrect_outline(rect(c2.left + 0.5, c2.top + 0.5, card_w - 1.0, card_h - 1.0), d(7.5), 1.0, white(0.08))?;
        self.hits.push((c2, CardAction::LaunchYouTube));

        // YouTube icon (22x22)
        let icon2 = rect(c2.left + d(12.0), c2.top + d(9.0), d(22.0), d(22.0));
        self.ink.svg(YOUTUBE_SVG, icon2, white(1.0))?;

        // YouTube text
        self.ink.text("YouTube", rect(c2.left + d(12.0), c2.top + d(35.0), card_w - d(24.0), d(17.0)), &st_title)?;
        self.ink.text(&crate::i18n::tr("Open in browser"), rect(c2.left + d(12.0), c2.top + d(51.0), card_w - d(24.0), d(16.0)), &st_sub)?;

        // --- Bottom Row: Output Chip & Note ---
        let bot_y = d(158.0);
        let chip_h = d(28.0);
        let chip_text = TextStyle::new(Face::Text, Weight::Regular, d(12.0), white(0.84));
        let out_name = card.output.as_deref().unwrap_or("Headphone");
        let text_w = self.ink.measure(out_name, &chip_text)?.min(d(110.0));
        let out_w = (text_w + d(38.0)).round();
        let out_chip = rect(d(16.0), bot_y, out_w, chip_h);
        self.ink.rrect(out_chip, d(4.0), white(0.05))?;
        self.ink.rrect_outline(rect(out_chip.left + 0.5, out_chip.top + 0.5, out_w - 1.0, chip_h - 1.0), d(3.5), 1.0, white(0.08))?;
        let icon = if out_name.to_lowercase().contains("hoparl") || out_name.to_lowercase().contains("speaker") { Icon::Speaker } else { Icon::Headphones };
        let glyph_st = TextStyle::new(Face::Icons, Weight::Regular, d(13.0), neutral_icon()).centered();
        self.ink.text(&glyph(icon).to_string(), rect(out_chip.left + d(8.0), bot_y, d(16.0), chip_h), &glyph_st)?;
        self.ink.text(out_name, rect(out_chip.left + d(28.0), bot_y, text_w + d(2.0), chip_h), &chip_text.ellipsis())?;
        self.hits.push((out_chip, CardAction::OutputDevice));

        // Right side note
        let note = TextStyle::new(Face::Text, Weight::Regular, d(11.5), sec_text()).ellipsis();
        let note_rect = rect(out_chip.right + d(10.0), bot_y, (self.w - out_chip.right - d(22.0)).max(0.0), chip_h);
        self.ink.text(&crate::i18n::tr("Card switches to the player when audio starts"), note_rect, &note)?;

        Ok(())
    }

    fn media(&mut self, card: &MediaCard) -> Result<()> {
        if card.m.id == IDLE_MEDIA {
            return self.paint_idle_media(card);
        }
        let m = &card.m;
        let d = |v: f32| v * self.s.0;
        let (ar, ag, ab) = timer_accent();

        // Subtle radial ambient tint from album art dominant color at top-left (18%, 18%), fading into #202020.
        if m.art.is_some() {
            self.ink.glow(d(64.0), d(56.0), self.w * 0.95, self.h * 0.72, rgba(m.accent[0], m.accent[1], m.accent[2], 0.24))?;
        }

        // 96×96px album art with 4px corner radius at top-left.
        let art_rect = rect(d(16.0), d(16.0), d(96.0), d(96.0));
        match &m.art {
            Some(art) => {
                self.ink.image_cover_rounded(art, m.art_size, art_rect, d(4.0))?;
            }
            None => {
                self.ink.rrect(art_rect, d(4.0), white(0.06))?;
                let initial: String = m.app_name.chars().take(1).collect();
                let st = TextStyle::new(Face::Display, Weight::SemiBold, d(36.0), white(0.45)).centered();
                self.ink.text(&initial, art_rect, &st)?;
            }
        }
        self.ink.rrect_outline(
            rect(art_rect.left + 0.5, art_rect.top + 0.5, art_rect.right - art_rect.left - 1.0, art_rect.bottom - art_rect.top - 1.0),
            d(3.5),
            1.0,
            white(0.08),
        )?;

        // Top-right quiet Pin button (upright pushpin SVG) and optional Lyrics button stacked below it.
        let pin = rect(self.w - d(16.0) - d(28.0), d(16.0), d(28.0), d(28.0));
        if card.pinned {
            self.ink.rrect(pin, d(4.0), white(0.09))?;
        }
        let pin_color = if card.pinned { rgba(ar, ag, ab, 1.0) } else { neutral_icon() };
        let pin_icon = rect((pin.left + d(6.0)).round(), (pin.top + d(6.0)).round(), d(16.0).round(), d(16.0).round());
        self.ink.svg(PIN_SVG, pin_icon, pin_color)?;
        self.hits.push((pin, CardAction::Pin));

        let right_limit = pin.left - d(6.0);
        let artist_right_limit = self.w - d(16.0);

        // Title (20px semibold), artist in secondary text, and 4-bar system accent equalizer below.
        let info_x = d(126.0);
        let info_w = (right_limit - info_x).max(d(80.0));
        let title = TextStyle::new(Face::Text, Weight::SemiBold, d(20.0), white(1.0)).top().ellipsis();
        self.ink.text(&m.title, rect(info_x, d(18.0), info_w, d(26.0)), &title)?;
        if !m.artist.is_empty() {
            let artist = TextStyle::new(Face::Text, Weight::Regular, d(13.0), sec_text()).top().ellipsis();
            self.ink.text(&m.artist, rect(info_x, d(46.0), (artist_right_limit - info_x).max(d(60.0)), d(20.0)), &artist)?;
        }
        // Four 3px system accent equalizer bars under the artist line.
        let eq_bottom = d(88.0);
        let heights = if m.playing { [13.0, 7.0, 11.0, 5.0] } else { [3.5, 3.5, 3.5, 3.5] };
        for (i, bh) in heights.into_iter().enumerate() {
            let bx = (info_x + i as f32 * d(5.0)).round();
            let bh_px = d(bh).round();
            self.ink.rrect(rect(bx, eq_bottom - bh_px, d(3.0).round(), bh_px), d(1.5), rgba(ar, ag, ab, 1.0))?;
        }

        // Page indicator dots if multiple activities are stacked.
        let (page, pages) = card.pages;
        if pages > 1 {
            let (dot, gap) = (d(5.0), d(4.0));
            let dots_w = pages as f32 * dot + (pages - 1) as f32 * gap;
            let x0 = (self.w - d(16.0) - dots_w).round();
            let y = d(82.0).round();
            for i in 0..pages {
                let fill = if i == page { rgba(ar, ag, ab, 1.0) } else { white(0.3) };
                self.ink.rrect(rect(x0 + i as f32 * (dot + gap), y, dot, dot), dot / 2.0, fill)?;
            }
        }

        // Quiet source and output chips row (4px radius, white 5% fill, 1px white 8% border).
        let chip_y = d(124.0);
        let chip_h = d(28.0);
        let chip_text = TextStyle::new(Face::Text, Weight::Regular, d(12.0), white(0.84));
        let chip_text_w = match &card.output {
            Some(output) => Some(self.ink.measure(output, &chip_text)?.min(d(146.0))),
            None => None,
        };
        let show_spotify = crate::settings::current().media_spotify_button;
        let is_spotify = m.app_id.to_lowercase().contains("spotify") || m.app_name.to_lowercase().contains("spotify");
        let (display_name, display_accent, action) = if show_spotify {
            if is_spotify {
                // When Spotify is playing, offer YouTube
                ("YouTube", [0xFF, 0x00, 0x00], Some(CardAction::LaunchYouTube))
            } else {
                // When playing from another source (Chrome, etc.), offer Spotify
                ("Spotify", [0x1D, 0xB9, 0x54], Some(CardAction::LaunchSpotify))
            }
        } else {
            // When quick button is turned off, show original source app
            let orig_name = if m.app_name.is_empty() { "Media" } else { m.app_name.as_str() };
            (orig_name, crate::activities::media::app_accent(&m.app_id), None)
        };
        let app_text_w = self.ink.measure(display_name, &chip_text)?.min(d(media_app_name_width_dip(chip_text_w.map(|w| w / self.s.0 + 44.0))));
        let src_chip_w = (d(28.0) + app_text_w + d(10.0)).round();
        let src_chip = rect(d(16.0), chip_y, src_chip_w, chip_h);
        self.ink.rrect(src_chip, d(4.0), white(0.05))?;
        self.ink.rrect_outline(rect(src_chip.left + 0.5, src_chip.top + 0.5, src_chip_w - 1.0, chip_h - 1.0), d(3.5), 1.0, white(0.08))?;
        let dot_side = d(8.0).round();
        self.ink.rrect(rect(src_chip.left + d(10.0), chip_y + (chip_h - dot_side) / 2.0, dot_side, dot_side), dot_side / 2.0, rgba(display_accent[0], display_accent[1], display_accent[2], 1.0))?;
        self.ink.text(display_name, rect(src_chip.left + d(24.0), chip_y, app_text_w + d(4.0), chip_h), &chip_text.ellipsis())?;
        if let Some(act) = action {
            self.hits.push((src_chip, act));
        }

        if let (Some(output), Some(text_w)) = (&card.output, chip_text_w) {
            let out_w = (text_w + d(38.0)).round();
            let out_chip = rect(self.w - d(16.0) - out_w, chip_y, out_w, chip_h);
            self.ink.rrect(out_chip, d(4.0), white(0.05))?;
            self.ink.rrect_outline(rect(out_chip.left + 0.5, out_chip.top + 0.5, out_w - 1.0, chip_h - 1.0), d(3.5), 1.0, white(0.08))?;
            let icon = if output.to_lowercase().contains("hoparl") { Icon::Speaker } else { Icon::Headphones };
            let glyph_st = TextStyle::new(Face::Icons, Weight::Regular, d(13.0), neutral_icon()).centered();
            self.ink.text(&glyph(icon).to_string(), rect(out_chip.left + d(8.0), chip_y, d(16.0), chip_h), &glyph_st)?;
            self.ink.text(output, rect(out_chip.left + d(28.0), chip_y, text_w + d(2.0), chip_h), &chip_text.ellipsis())?;
            self.hits.push((out_chip, CardAction::OutputDevice));
        }

        // Slider: 4px track with system accent fill and a 20px thumb with a grey ring (#454545) and an accent center that grows on drag/hover.
        let (x0, x1, y) = (d(16.0), self.w - d(16.0), d(172.0));
        let track_h = d(4.0).round();
        let timeline = m.timeline.filter(|t| t.end > t.start);
        match (timeline, card.position) {
            (Some(t), Some(position)) => {
                let duration = (t.end - t.start) as f32;
                let frac = card.seeking.unwrap_or(((position - t.start) as f32 / duration).clamp(0.0, 1.0));
                let xp = x0 + frac * (x1 - x0);
                self.ink.rrect(rect(x0, y - track_h / 2.0, x1 - x0, track_h), track_h / 2.0, white(0.20))?;
                if xp > x0 {
                    self.ink.rrect(rect(x0, y - track_h / 2.0, (xp - x0).max(track_h), track_h), track_h / 2.0, rgba(ar, ag, ab, 1.0))?;
                }
                let thumb_r = d(10.0).round();
                let thumb = rect(xp - thumb_r, y - thumb_r, 2.0 * thumb_r, 2.0 * thumb_r);
                self.ink.rrect(thumb, thumb_r, rgba(0x45, 0x45, 0x45, 1.0))?;
                self.ink.rrect_outline(rect(thumb.left + 0.5, thumb.top + 0.5, 2.0 * thumb_r - 1.0, 2.0 * thumb_r - 1.0), thumb_r - 0.5, 1.0, white(0.22))?;
                let inner_r = if card.seeking.is_some() { d(6.0).round() } else { d(4.5).round() };
                self.ink.rrect(rect(xp - inner_r, y - inner_r, 2.0 * inner_r, 2.0 * inner_r), inner_r, rgba(ar, ag, ab, 1.0))?;

                let shown = card.seeking.map_or(position, |f| t.start + (duration * f) as i64);
                let times = TextStyle::new(Face::Text, Weight::Regular, d(11.5), sec_text()).tabular();
                self.ink.text(&format_media_time(shown - t.start), rect(x0, d(188.0), d(80.0), d(16.0)), &times)?;
                self.ink.text(&format_media_time(t.end - t.start), rect(x1 - d(80.0), d(188.0), d(80.0), d(16.0)), &times.trailing())?;
                if m.can.seek {
                    *self.seek = Some((rect(x0 - d(8.0), d(158.0), x1 - x0 + d(16.0), d(28.0)), x0, x1));
                }
            }
            _ => {
                self.ink.rrect(rect(x0, y - track_h / 2.0, x1 - x0, track_h), track_h / 2.0, white(0.20))?;
            }
        }

        // Per-app volume slider row (speaker icon with mute toggle, volume track with accent fill, and percentage label).
        let (app_vol, app_muted) = card.app_volume.unwrap_or((1.0, false));
        let vol_cy = d(226.0);
        let spk_box = rect(d(16.0), vol_cy - d(10.0), d(20.0), d(20.0));
        let spk_icon = if app_muted || app_vol <= 0.001 {
            Icon::Mute
        } else {
            Icon::Volume
        };
        let spk_color = if app_muted {
            white(0.40)
        } else {
            white(0.85)
        };
        self.ink.text(
            &glyph(spk_icon).to_string(),
            spk_box,
            &TextStyle::new(Face::Icons, Weight::Regular, d(13.0), spk_color).centered(),
        )?;
        self.hits.push((spk_box, CardAction::MediaAppMute));

        let (vx0, vx1) = (d(44.0), self.w - d(54.0));
        let v_track_h = d(3.5).round();
        let v_track_w = vx1 - vx0;
        let v_xp = vx0 + app_vol.clamp(0.0, 1.0) * v_track_w;

        // Track background (dim white)
        self.ink.rrect(
            rect(vx0, vol_cy - v_track_h / 2.0, v_track_w, v_track_h),
            v_track_h / 2.0,
            white(0.18),
        )?;
        // Filled track (system accent or white if muted)
        if v_xp > vx0 {
            let fill = if app_muted {
                white(0.35)
            } else {
                rgba(ar, ag, ab, 1.0)
            };
            self.ink.rrect(
                rect(vx0, vol_cy - v_track_h / 2.0, (v_xp - vx0).max(v_track_h), v_track_h),
                v_track_h / 2.0,
                fill,
            )?;
        }
        // Subtle circular thumb
        let thumb_r = d(5.5).round();
        let thumb = rect(v_xp - thumb_r, vol_cy - thumb_r, 2.0 * thumb_r, 2.0 * thumb_r);
        let thumb_fill = if app_muted {
            white(0.60)
        } else {
            rgba(ar, ag, ab, 1.0)
        };
        self.ink.rrect(thumb, thumb_r, thumb_fill)?;
        self.ink.rrect_outline(
            rect(thumb.left + 0.5, thumb.top + 0.5, 2.0 * thumb_r - 1.0, 2.0 * thumb_r - 1.0),
            thumb_r - 0.5,
            1.0,
            white(0.25),
        )?;

        // Percentage text (e.g. "80%")
        let pct_str = if app_muted {
            "Mute".to_string()
        } else {
            format!("{}%", (app_vol * 100.0).round() as i32)
        };
        let pct_style = TextStyle::new(Face::Text, Weight::Regular, d(11.0), sec_text()).tabular().trailing();
        self.ink.text(
            &pct_str,
            rect(self.w - d(50.0), vol_cy - d(8.0), d(34.0), d(16.0)),
            &pct_style,
        )?;

        // App volume slider hit target
        *self.app_volume = Some((
            rect(vx0 - d(6.0), vol_cy - d(12.0), v_track_w + d(12.0), d(24.0)),
            vx0,
            vx1,
        ));

        // Footer band controls: black at 28% with a 1px white 7% top border.
        let band_h = d(68.0);
        let band = rect(0.0, self.h - band_h, self.w, band_h);
        self.ink.rrect(band, 0.0, [0.0, 0.0, 0.0, 0.28])?;
        self.ink.line(0.0, band.top + 0.5, self.w, band.top + 0.5, 1.0, white(0.07))?;

        let cy = (band.top + band_h / 2.0).round();
        let enabled = |on: bool| if on { 1.0 } else { 0.35 };

        // Prev / Next: 40px quiet 4px-radius buttons.
        for (x, icon, on, action) in [
            (114.0, Icon::Prev, m.can.prev, CardAction::Prev),
            (226.0, Icon::Next, m.can.next, CardAction::Next),
        ] {
            let area = rect(d(x - 20.0), cy - d(20.0), d(40.0), d(40.0));
            let st = TextStyle::new(Face::Icons, Weight::Regular, d(16.0), white(enabled(on))).centered();
            self.ink.text(&glyph(icon).to_string(), area, &st)?;
            self.hits.push((area, action));
        }

        // Play / Pause: 48px system accent circle with a soft accent glow (~18px, 45%) while playing.
        let play_r = d(24.0).round();
        let play_cx = d(170.0).round();
        if m.playing {
            self.ink.glow(play_cx, cy, d(36.0), d(36.0), rgba(ar, ag, ab, 0.45))?;
        }
        let play_area = rect(play_cx - play_r, cy - play_r, 2.0 * play_r, 2.0 * play_r);
        self.ink.rrect(play_area, play_r, rgba(ar, ag, ab, 1.0))?;
        let play_icon = if m.playing { Icon::Pause } else { Icon::Play };
        let play_st = TextStyle::new(Face::Icons, Weight::Regular, d(18.0), rgba(0x00, 0x00, 0x00, enabled(m.can.play_pause))).centered();
        self.ink.text(&glyph(play_icon).to_string(), play_area, &play_st)?;
        self.hits.push((play_area, CardAction::PlayPause));

        // Shuffle and Repeat at the edges: 36px buttons, subtle highlight when active.
        for (x, icon, supported, active, is_shuffle, action) in [
            (36.0, Icon::Shuffle, m.can.shuffle, m.shuffle == Some(true), true, CardAction::Shuffle),
            (304.0, Icon::Repeat, m.can.repeat, m.repeat.is_some(), false, CardAction::Repeat),
        ] {
            let area = rect(d(x - 18.0), cy - d(18.0), d(36.0), d(36.0));
            if active && supported {
                self.ink.rrect(area, d(6.0), white(0.08))?;
                self.ink.rrect_outline(rect(area.left + 0.5, area.top + 0.5, d(36.0) - 1.0, d(36.0) - 1.0), d(5.5), 1.0, white(0.06))?;
            }
            let tint = if !supported {
                white(0.28)
            } else if active {
                rgba(ar, ag, ab, 1.0)
            } else {
                sec_text()
            };
            let st = TextStyle::new(Face::Icons, Weight::Regular, d(15.0), tint).centered();
            self.ink.text(&glyph(icon).to_string(), area, &st)?;
            if !is_shuffle && supported && m.repeat == Some(true) {
                let one_st = TextStyle::new(Face::Text, Weight::Bold, d(8.5), tint).centered();
                self.ink.text("1", area, &one_st)?;
            }
            self.hits.push((area, action));
        }
        Ok(())
    }

    fn picker(&mut self, p: &PickerState) -> Result<()> {
        let (l, r) = (self.d(18.0), self.w - self.d(18.0));
        let inner = r - l;
        // Tabs.
        let tabs = rect(l, self.d(16.0), inner, self.d(36.0));
        self.ink.rrect(tabs, self.d(4.0), white(0.06))?;
        let half = inner / 2.0;
        let (timer_tab, watch_tab) = (rect(l, self.d(16.0), half, self.d(36.0)), rect(l + half, self.d(16.0), half, self.d(36.0)));
        let selected = if p.stopwatch_tab { watch_tab } else { timer_tab };
        let pad = self.d(3.0);
        let sel = D2D_RECT_F { left: selected.left + pad, top: selected.top + pad, right: selected.right - pad, bottom: selected.bottom - pad };
        self.ink.rrect(sel, self.d(4.0), white(0.10))?;
        for (area, label, on, stopwatch) in [(timer_tab, "Timer", !p.stopwatch_tab, false), (watch_tab, "Stopwatch", p.stopwatch_tab, true)] {
            let st = TextStyle::new(Face::Text, Weight::SemiBold, self.d(13.0), white(if on { 1.0 } else { 0.65 })).centered();
            self.ink.text(label, area, &st)?;
            self.hits.push((area, CardAction::PickerTab(stopwatch)));
        }

        if p.stopwatch_tab {
            let st = TextStyle::new(Face::Display, Weight::Light, self.d(40.0), white(0.45)).centered().tabular();
            self.ink.text(&crate::activities::timer::format_stopwatch(0), rect(l, self.d(74.0), inner, self.d(56.0)), &st)?;
            let note = TextStyle::new(Face::Text, Weight::Regular, self.d(13.0), white(0.7)).centered();
            self.ink.text("Counting starts when you tap Start", rect(l, self.d(140.0), inner, self.d(24.0)), &note)?;
        } else {
            // Wheel fields: HH : MM : SS; the mouse wheel over a field turns it.
            let field_w = self.d(76.0);
            let gap = self.d(26.0);
            let x0 = (self.w - (3.0 * field_w + 2.0 * gap)) / 2.0;
            for (i, value) in [p.h, p.m, p.s].into_iter().enumerate() {
                let x = x0 + i as f32 * (field_w + gap);
                let area = rect(x, self.d(66.0), field_w, self.d(56.0));
                let (weight, alpha) = if value > 0 { (Weight::Medium, 1.0) } else { (Weight::Light, 0.45) };
                let st = TextStyle::new(Face::Display, weight, self.d(40.0), white(alpha)).centered().tabular();
                self.ink.text(&format!("{value:02}"), area, &st)?;
                let unit = TextStyle::new(Face::Text, Weight::Regular, self.d(11.5), white(0.6)).centered();
                self.ink.text(["hr", "min", "sec"][i], rect(x, self.d(118.0), field_w, self.d(16.0)), &unit)?;
                self.hits.push((area, CardAction::PickerField(i as u8)));
                if i < 2 {
                    let colon = TextStyle::new(Face::Display, Weight::Light, self.d(32.0), white(0.45)).centered();
                    self.ink.text(":", rect(x + field_w, self.d(66.0), gap, self.d(52.0)), &colon)?;
                }
            }
            // Presets, two rows of three.
            let chip_gap = self.d(8.0);
            let chip_w = (inner - 2.0 * chip_gap) / 3.0;
            for (i, minutes) in crate::settings::current().timer_presets_min.iter().enumerate() {
                let x = l + (i % 3) as f32 * (chip_w + chip_gap);
                let y = self.d(140.0) + (i / 3) as f32 * self.d(28.0);
                let area = rect(x, y, chip_w, self.d(24.0));
                let on = p.total_ms() == *minutes as u64 * 60_000;
                let (r, g, b) = timer_accent();
                let fill = if on { rgba(r, g, b, 0.55) } else { white(0.12) };
                self.chip(area, &format!("{minutes} min"), fill, CardAction::PickPreset(*minutes))?;
            }
        }
        // Start.
        let (r_, g_, b_) = timer_accent();
        let start = rect(l, self.h - self.d(18.0 + 46.0), inner, self.d(46.0));
        self.chip(start, "Start", rgba(r_, g_, b_, 1.0), CardAction::PickerStart)
    }

    fn download(&mut self, c: &DownloadCard) -> Result<()> {
        let (r, g, b) = (0x00, 0x78, 0xD4);

        // Icon badge on left
        let circle = self.d(40.0).round();
        let dot = rect(self.d(20.0).round(), self.d(18.0).round(), circle, circle);
        self.ink.rrect(dot, self.d(12.0), rgba(r, g, b, 1.0))?;
        let icon_st = TextStyle::new(Face::Icons, Weight::Regular, self.d(20.0), white(1.0)).centered();
        self.ink.text(&glyph(Icon::Download).to_string(), dot, &icon_st)?;

        // Filename and status
        let text_x = self.d(72.0);
        let text_w = self.w - text_x - self.d(20.0);
        let title = TextStyle::new(Face::Text, Weight::SemiBold, self.d(15.0), white(1.0)).top().ellipsis();
        self.ink.text(&c.name, rect(text_x, self.d(17.0), text_w, self.d(22.0)), &title)?;

        let status_st = TextStyle::new(Face::Text, Weight::Regular, self.d(12.5), white(0.7)).top().ellipsis().tabular();
        self.ink.text(&c.status, rect(text_x, self.d(40.0), text_w, self.d(18.0)), &status_st)?;

        // Progress bar
        let bar_y = self.d(72.0);
        let bar_h = self.d(6.0).round();
        let bar_x = self.d(20.0);
        let bar_w = self.w - self.d(40.0);
        let track = rect(bar_x, bar_y, bar_w, bar_h);
        self.ink.rrect(track, bar_h / 2.0, white(0.18))?;

        let progress_frac = if c.done {
            1.0
        } else {
            c.progress.unwrap_or(0.0)
        };
        let fill_w = if progress_frac > 0.0001 {
            (bar_w * progress_frac.clamp(0.0, 1.0)).max(bar_h)
        } else {
            0.0
        };
        let fill_color = if c.done { rgba(0x10, 0x7C, 0x10, 1.0) } else { rgba(r, g, b, 1.0) };
        if fill_w > 0.0 {
            self.ink.rrect(rect(bar_x, bar_y, fill_w, bar_h), bar_h / 2.0, fill_color)?;
        }

        // Size and speed under progress bar
        let detail_y = self.d(85.0);
        let size_st = TextStyle::new(Face::Text, Weight::Regular, self.d(12.0), white(0.75)).top().tabular();
        self.ink.text(&c.size_text, rect(bar_x, detail_y, self.d(180.0), self.d(18.0)), &size_st)?;

        if !c.speed.is_empty() {
            let speed_st = TextStyle::new(Face::Text, Weight::SemiBold, self.d(12.0), white(0.9)).trailing().top().tabular();
            self.ink.text(&c.speed, rect(self.w - self.d(20.0) - self.d(170.0), detail_y, self.d(170.0), self.d(18.0)), &speed_st)?;
        }

        // Action buttons
        let btn_y = self.d(114.0);
        let btn_h = self.d(34.0);
        let folder_w = self.d(96.0);
        let open_w = self.d(96.0);
        let folder_rect = rect(self.w - self.d(20.0) - folder_w, btn_y, folder_w, btn_h);
        let open_rect = rect(folder_rect.left - self.d(10.0) - open_w, btn_y, open_w, btn_h);

        let open_label = if c.done { "Open file" } else { "Open" };
        self.chip(open_rect, open_label, white(0.14), CardAction::DownloadOpen)?;
        let folder_fill = if c.done { rgba(0x10, 0x7C, 0x10, 1.0) } else { rgba(r, g, b, 1.0) };
        self.chip(folder_rect, "Folder", folder_fill, CardAction::DownloadFolder)?;

        Ok(())
    }

    fn note(&mut self, c: &NoteCard) -> Result<()> {
        let (ar, ag, ab) = (0xF5, 0x9E, 0x0B);

        // Header area
        let title_style = TextStyle::new(Face::Text, Weight::SemiBold, self.d(13.0), white(0.95)).ellipsis();
        let display_title = if c.title.is_empty() { "Sticky Note" } else { &c.title };
        self.ink.text(display_title, rect(self.d(18.0), self.d(14.0), self.d(160.0), self.d(22.0)), &title_style)?;

        // Top right buttons: Pin, New (+), Close (X)
        let btn_sz = self.d(22.0).round();
        let btn_y = self.d(12.0).round();

        // 1. Pin
        let pin_btn = rect(self.w - self.d(18.0) - btn_sz * 3.0 - self.d(16.0), btn_y, btn_sz, btn_sz);
        self.ink.rrect(pin_btn, self.d(4.0), white(if c.pinned { 0.18 } else { 0.08 }))?;
        let pin_color = if c.pinned { rgba(ar, ag, ab, 1.0) } else { white(0.85) };
        let pin_icon_r = rect(pin_btn.left + self.d(5.0), pin_btn.top + self.d(5.0), self.d(12.0), self.d(12.0));
        self.ink.svg(PIN_SVG, pin_icon_r, pin_color)?;
        self.hits.push((pin_btn, CardAction::NotePin(c.id)));

        // 2. New (+)
        let add_btn = rect(self.w - self.d(18.0) - btn_sz * 2.0 - self.d(8.0), btn_y, btn_sz, btn_sz);
        self.ink.rrect(add_btn, self.d(4.0), white(0.08))?;
        let plus_style = TextStyle::new(Face::Text, Weight::SemiBold, self.d(13.0), white(0.85)).centered();
        self.ink.text("+", add_btn, &plus_style)?;
        self.hits.push((add_btn, CardAction::NoteNew));

        // 3. Close (X)
        let close_btn = rect(self.w - self.d(18.0) - btn_sz, btn_y, btn_sz, btn_sz);
        self.ink.rrect(close_btn, self.d(4.0), white(0.08))?;
        let x_style = TextStyle::new(Face::Icons, Weight::Regular, self.d(10.0), white(0.85)).centered();
        self.ink.text(&glyph(Icon::Close).to_string(), close_btn, &x_style)?;
        self.hits.push((close_btn, CardAction::NoteClose));

        let mut content_top = self.d(42.0);

        // Attached image preview if any
        if let Some(thumb) = &c.thumb {
            let img_h = self.d(80.0);
            let img_rect = rect(self.d(16.0), content_top, self.w - self.d(32.0), img_h);
            self.ink.image_cover_rounded(&thumb.px, thumb.size, img_rect, self.d(6.0))?;

            let rm_sz = self.d(18.0);
            let rm_btn = rect(img_rect.right - rm_sz - self.d(6.0), img_rect.top + self.d(6.0), rm_sz, rm_sz);
            self.ink.rrect(rm_btn, self.d(3.0), [0.0, 0.0, 0.0, 0.65])?;
            self.ink.text(&glyph(Icon::Close).to_string(), rm_btn, &TextStyle::new(Face::Icons, Weight::Regular, self.d(8.0), white(0.9)).centered())?;
            self.hits.push((rm_btn, CardAction::NoteRemoveImage(c.id)));

            content_top += img_h + self.d(10.0);
        }

        // Text editor area
        let footer_h = self.d(36.0);
        let body_h = (self.h - content_top - footer_h - self.d(10.0)).max(self.d(80.0));
        let body_rect = rect(self.d(16.0), content_top, self.w - self.d(32.0), body_h);
        self.ink.rrect(body_rect, self.d(6.0), white(0.04))?;
        self.ink.rrect_outline(rect(body_rect.left + 0.5, body_rect.top + 0.5, body_rect.right - body_rect.left - 1.0, body_rect.bottom - body_rect.top - 1.0), (self.d(6.0) - 0.5).max(1.0), 1.0, white(0.07))?;

        let text_pad = self.d(10.0);
        let inner_rect = rect(body_rect.left + text_pad, body_rect.top + text_pad, body_rect.right - body_rect.left - 2.0 * text_pad, body_rect.bottom - body_rect.top - 2.0 * text_pad);
        let typed_style = TextStyle::new(Face::Text, Weight::Regular, self.d(13.0), white(0.95)).top().wrap();

        if c.text.is_empty() {
            let placeholder = TextStyle::new(Face::Text, Weight::Regular, self.d(13.0), white(0.35)).top().wrap();
            self.ink.text("Take a note…\nCtrl+V to paste image or text", inner_rect, &placeholder)?;
            if c.caret_visible {
                let caret_h = self.d(16.0).round();
                self.ink.rrect(rect(inner_rect.left, inner_rect.top + self.d(1.0), self.d(2.0).round(), caret_h), self.d(1.0), rgba(ar, ag, ab, 1.0))?;
            }
        } else {
            unsafe {
                self.ink.dc.PushAxisAlignedClip(&inner_rect, windows::Win32::Graphics::Direct2D::D2D1_ANTIALIAS_MODE_ALIASED);
            }
            let _ = self.ink.text(&c.text, inner_rect, &typed_style);
            unsafe {
                self.ink.dc.PopAxisAlignedClip();
            }

            if c.caret_visible {
                let chars: Vec<char> = c.text.chars().collect();
                let up_to: String = chars[..c.caret.min(chars.len())].iter().collect();
                let last_line = up_to.rsplit('\n').next().unwrap_or("");
                let line_count = up_to.matches('\n').count();
                let line_h = self.ink.text_height("Ag", inner_rect.right - inner_rect.left, &typed_style).unwrap_or(self.d(18.0)).max(self.d(18.0));
                let line_w = self.ink.measure(last_line, &typed_style).unwrap_or(0.0);
                let cy = inner_rect.top + (line_count as f32 * line_h);
                let cx = (inner_rect.left + line_w).min(inner_rect.right - self.d(4.0));
                if cy + line_h <= inner_rect.bottom + self.d(4.0) {
                    let caret_h = self.d(16.0).round();
                    self.ink.rrect(rect(cx, cy + (line_h - caret_h) / 2.0, self.d(2.0).round(), caret_h), self.d(1.0), rgba(ar, ag, ab, 1.0))?;
                }
            }
        }

        // Footer chips
        let footer_y = self.h - footer_h;
        let chip_style = TextStyle::new(Face::Text, Weight::Medium, self.d(11.5), white(0.85));

        let copy_w = self.ink.measure("Copy", &chip_style).unwrap_or(self.d(28.0));
        let copy_chip = rect(self.d(16.0), footer_y + self.d(4.0), copy_w + self.d(20.0), self.d(26.0));
        self.ink.rrect(copy_chip, self.d(13.0), white(0.08))?;
        self.ink.text("Copy", D2D_RECT_F { left: copy_chip.left + self.d(10.0), ..copy_chip }, &chip_style)?;
        self.hits.push((copy_chip, CardAction::NoteCopy(c.id)));

        let share_w = self.ink.measure("Share", &chip_style).unwrap_or(self.d(30.0));
        let share_chip = rect(copy_chip.right + self.d(8.0), footer_y + self.d(4.0), share_w + self.d(20.0), self.d(26.0));
        self.ink.rrect(share_chip, self.d(13.0), white(0.08))?;
        self.ink.text("Share", D2D_RECT_F { left: share_chip.left + self.d(10.0), ..share_chip }, &chip_style)?;
        self.hits.push((share_chip, CardAction::NoteShare(c.id)));

        let del_w = self.ink.measure("Delete", &chip_style).unwrap_or(self.d(34.0));
        let del_chip = rect(share_chip.right + self.d(8.0), footer_y + self.d(4.0), del_w + self.d(20.0), self.d(26.0));
        self.ink.rrect(del_chip, self.d(13.0), white(0.08))?;
        self.ink.text("Delete", D2D_RECT_F { left: del_chip.left + self.d(10.0), ..del_chip }, &chip_style)?;
        self.hits.push((del_chip, CardAction::NoteDelete(c.id)));

        let list_w = self.ink.measure("All Notes", &chip_style).unwrap_or(self.d(50.0));
        let list_chip = rect(self.w - self.d(16.0) - list_w - self.d(20.0), footer_y + self.d(4.0), list_w + self.d(20.0), self.d(26.0));
        self.ink.rrect(list_chip, self.d(13.0), white(0.08))?;
        self.ink.text("All Notes", D2D_RECT_F { left: list_chip.left + self.d(10.0), ..list_chip }, &chip_style)?;
        self.hits.push((list_chip, CardAction::NoteOpenList));

        Ok(())
    }

    fn notes_list(&mut self, c: &NotesListCard) -> Result<()> {
        let (ar, ag, ab) = (0xF5, 0x9E, 0x0B);

        // Header: "Sticky Notes"
        let title_style = TextStyle::new(Face::Text, Weight::SemiBold, self.d(16.0), white(1.0)).top();
        self.ink.text("Sticky Notes", rect(self.d(18.0), self.d(16.0), self.d(140.0), self.d(24.0)), &title_style)?;

        // New Note (+) button
        let btn_style = TextStyle::new(Face::Text, Weight::Medium, self.d(12.0), white(0.9));
        let nw = self.ink.measure("+ New Note", &btn_style).unwrap_or(self.d(60.0));
        let new_btn = rect(self.w - self.d(18.0) - nw - self.d(18.0), self.d(14.0), nw + self.d(18.0), self.d(26.0));
        self.ink.rrect(new_btn, self.d(13.0), rgba(ar, ag, ab, 0.35))?;
        self.ink.text("+ New Note", D2D_RECT_F { left: new_btn.left + self.d(9.0), ..new_btn }, &btn_style)?;
        self.hits.push((new_btn, CardAction::NoteNew));

        if c.notes.is_empty() {
            let empty_style = TextStyle::new(Face::Text, Weight::Regular, self.d(13.5), white(0.45)).centered();
            self.ink.text("No notes yet\nMiddle click the bar or tap + New Note", rect(self.d(20.0), self.d(100.0), self.w - self.d(40.0), self.d(60.0)), &empty_style)?;
            return Ok(());
        }

        let item_h = self.d(56.0);
        let item_gap = self.d(6.0);
        let list_clip = rect(self.d(14.0), self.d(50.0), self.w - self.d(28.0), self.h - self.d(50.0) - self.d(12.0));
        let scroll_px = c.scroll * self.s.0;

        unsafe {
            self.ink.dc.PushAxisAlignedClip(&list_clip, windows::Win32::Graphics::Direct2D::D2D1_ANTIALIAS_MODE_ALIASED);
        }

        for (idx, note) in c.notes.iter().enumerate() {
            let item_top = list_clip.top + idx as f32 * (item_h + item_gap) - scroll_px;
            let item_rect = rect(list_clip.left, item_top, list_clip.right - list_clip.left, item_h);
            if item_rect.bottom < list_clip.top || item_rect.top > list_clip.bottom {
                continue;
            }

            let hovered = c.hover == Some(idx);
            let fill = if hovered { white(0.10) } else { white(0.05) };
            self.ink.rrect(item_rect, self.d(6.0), fill)?;
            self.ink.rrect_outline(rect(item_rect.left + 0.5, item_rect.top + 0.5, item_rect.right - item_rect.left - 1.0, item_rect.bottom - item_rect.top - 1.0), (self.d(6.0) - 0.5).max(1.0), 1.0, white(0.07))?;

            let mut text_right = item_rect.right - self.d(12.0);

            if let Some(thumb) = &note.thumb {
                let th_sz = self.d(40.0);
                let th_rect = rect(item_rect.right - th_sz - self.d(8.0), item_rect.top + self.d(8.0), th_sz, th_sz);
                self.ink.image_cover_rounded(&thumb.px, thumb.size, th_rect, self.d(4.0))?;
                text_right = th_rect.left - self.d(8.0);
            }

            let note_title_style = TextStyle::new(Face::Text, Weight::SemiBold, self.d(12.5), white(0.95)).ellipsis();
            let title_rect = rect(item_rect.left + self.d(12.0), item_rect.top + self.d(8.0), text_right - item_rect.left - self.d(12.0), self.d(18.0));
            self.ink.text(&note.title, title_rect, &note_title_style)?;

            let note_prev_style = TextStyle::new(Face::Text, Weight::Regular, self.d(11.5), white(0.55)).ellipsis();
            let prev_rect = rect(item_rect.left + self.d(12.0), item_rect.top + self.d(28.0), text_right - item_rect.left - self.d(12.0), self.d(18.0));
            let prev_str = if note.preview.is_empty() { "(Empty note)" } else { &note.preview };
            self.ink.text(prev_str, prev_rect, &note_prev_style)?;

            if note.pinned {
                let pin_r = rect(item_rect.right - self.d(20.0), item_rect.top + self.d(4.0), self.d(10.0), self.d(10.0));
                self.ink.svg(PIN_SVG, pin_r, rgba(ar, ag, ab, 1.0))?;
            }

            self.hits.push((item_rect, CardAction::NoteOpen(note.id)));
        }

        unsafe {
            self.ink.dc.PopAxisAlignedClip();
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn download_card_size_and_accent() {
        let card = DownloadCard {
            name: "test.zip".into(),
            path: std::path::PathBuf::from(r"C:\Downloads\test.zip"),
            speed: "3.5 MB/s".into(),
            status: "Downloading · 3.5 MB/s".into(),
            size_text: "45 MB / 100 MB".into(),
            progress: Some(0.45),
            done: false,
            app_name: "Google Chrome".into(),
        };
        assert_eq!(card_size_dip(&CardContent::Download(card.clone())), (370.0, 164.0));
        assert_eq!(card_accent(&CardContent::Download(card)), (0x00, 0x78, 0xD4));
    }

    fn brief(weather: bool, rain: bool, matches: usize, music: bool, screen: bool) -> BriefCard {
        let team = || CardTeam { name: "FB".into(), code: "FB".into(), palette: TeamPalette { tint: [0x0B, 0x1E, 0x5B], flood: [0x0B, 0x1E, 0x5B], headline: [255, 255, 255] }, crest: None };
        BriefCard {
            greeting: "Günaydın, Ekin".into(),
            date: "Perşembe, 22 Ekim".into(),
            weather: weather.then(|| BriefWeather {
                symbol: "☀\u{FE0F}".into(),
                temp: "18°".into(),
                summary: "Açık · 11° / 21°".into(),
                hours: vec![],
                rain: rain.then(|| "15:00 civarı yağmur".into()),
                fx: WeatherFx::Sunny,
            }),
            matches: (0..matches).map(|_| BriefMatchRow { home: team(), away: team(), status: "20:00 · Süper Lig".into(), live: false }).collect(),
            music: music.then(|| BriefMusic { title: "t".into(), artist: "a".into(), art: None, art_size: (0, 0), playing: true }),
            screen: screen.then(|| ("Ekran süresi · 3 sa 40 dk".into(), "Bugün · VS Code 1 sa 50 dk".into())),
            away: None,
        }
    }

    #[test]
    fn the_away_row_is_the_screen_rows_shape() {
        let h = |b: BriefCard| card_size_dip(&CardContent::Brief(Box::new(b))).1;
        let back = BriefCard { away: Some(("Sen yokken · 4 şey".into(), "Claude bitti · now-bar".into())), ..brief(false, false, 0, false, false) };
        assert_eq!(h(back), 84.0 + 68.0);
    }

    #[test]
    fn the_question_card_grows_with_its_answer() {
        let card = |show, lines, copy: bool| {
            CardContent::Question(Box::new(QuestionCard { question: "?".into(), caret: true, show, answer: String::new(), lines, scroll: None, copy: copy.then(|| "Kopyala".into()) }))
        };
        assert_eq!(card_size_dip(&card(QuestionShow::Idle, 0, false)), (370.0, 52.0 + 40.0 + 16.0), "the field alone");
        assert_eq!(card_size_dip(&card(QuestionShow::Thinking, 1, false)).1, 52.0 + 40.0 + 12.0 + 19.0 + 8.0 + 16.0);
        assert_eq!(card_size_dip(&card(QuestionShow::Answer, 3, true)).1, 52.0 + 40.0 + 12.0 + 3.0 * 19.0 + 8.0 + 52.0, "and the copy chip");
        let tallest = card_size_dip(&card(QuestionShow::Answer, 99, true)).1;
        assert_eq!(tallest, 52.0 + 40.0 + 12.0 + 14.0 * 19.0 + 8.0 + 52.0, "14 lines, then it scrolls");
        assert!(tallest <= 570.0, "inside the card window");
    }

    #[test]
    fn the_weeks_music_is_as_tall_as_its_rows() {
        let row = || MusicRow { title: "t".into(), artist: "a".into(), plays: "3 kez".into(), accent: (0, 0, 0) };
        let card = |tracks: usize, artists: usize| {
            CardContent::Music(Box::new(MusicCard {
                total: String::new(),
                caption: String::new(),
                tracks: (0..tracks).map(|_| row()).collect(),
                artists: (0..artists).map(|_| (String::new(), String::new())).collect(),
                accent: (0, 0, 0),
            }))
        };
        assert_eq!(card_size_dip(&card(0, 0)).1, 100.0 + 16.0 + 44.0 + 12.0, "the empty week's line");
        assert_eq!(card_size_dip(&card(5, 3)).1, 100.0 + 16.0 + 5.0 * 44.0 + 8.0 + 16.0 + 3.0 * 40.0 + 12.0);
        let well = |music: bool| WellbeingCard {
            total: String::new(),
            caption: String::new(),
            week: [0.0; 7],
            days: [""; 7],
            apps: vec![],
            music: music.then(|| ("Haftanın müziği · 6 sa".into(), "t · a".into())),
        };
        let h = |c: WellbeingCard| card_size_dip(&CardContent::Wellbeing(Box::new(c))).1;
        assert_eq!(h(well(true)) - h(well(false)), 68.0, "the music row: the brief's link shape");
    }

    #[test]
    fn the_away_card_is_as_tall_as_its_rows() {
        let row = || AwayRow { kind: AwayKind::Timer, text: "Zamanlayıcı bitti".into(), time: "17:05".into(), crest: None };
        let card = |n: usize, more: usize| CardContent::Away(Box::new(AwayCard { span: String::new(), rows: (0..n).map(|_| row()).collect(), more }));
        assert_eq!(card_size_dip(&card(1, 0)), (370.0, 72.0 + 8.0 + 16.0 + 44.0 + 12.0), "the header, the panel round one row");
        assert_eq!(card_size_dip(&card(6, 3)).1, 72.0 + 8.0 + 16.0 + 6.0 * 44.0 + 28.0 + 12.0, "and the +3 daha line");
    }

    #[test]
    fn brief_height_follows_its_sections() {
        let h = |b: BriefCard| card_size_dip(&CardContent::Brief(Box::new(b))).1;
        assert_eq!(h(brief(false, false, 0, false, false)), 84.0, "the greeting alone: 20 + 52 + 12");
        assert_eq!(h(brief(true, false, 0, false, false)), 84.0 + 168.0, "weather: 8 + row 56 + 4 + hours 100");
        assert_eq!(h(brief(true, true, 0, false, false)), 84.0 + 190.0, "and the rain line, 22");
        assert_eq!(h(brief(false, false, 3, false, false)), 84.0 + 8.0 + 16.0 + 3.0 * 44.0);
        assert_eq!(h(brief(false, false, 0, true, false)), 84.0 + 68.0, "music: 8 + 60");
        assert_eq!(h(brief(false, false, 0, false, true)), 84.0 + 68.0, "screen time: the music panel's shape");
        assert_eq!(h(brief(true, true, 3, true, true)), 566.0);
    }

    fn shelf_card(n: usize) -> ShelfCard {
        let tiles = (0..n).map(|i| ShelfTile { name: format!("f{i}.txt"), thumb: None, kind: FileKind::Document, selected: false, pinned: false }).collect();
        ShelfCard { tiles, hover: None, chips: vec![], said: None, pinned: false, scroll: 0.0 }
    }

    #[test]
    fn shelf_chips_line_up_under_the_tiles() {
        let s = Scale(1.25);
        let tiles = shelf_layout(5, &s).tiles;
        let chips = shelf_chips(&[40.0, 80.5], 2, &s);
        assert_eq!(chips[0].left, tiles[0].left, "under the first column");
        assert!((chips[0].top - tiles[4].bottom - 30.0).abs() <= 1.0, "inside the footer band under the second row");
        assert_eq!(chips[0].bottom - chips[0].top, 35.0, "28 DIP tall");
        assert_eq!((chips[0].right - chips[0].left, chips[1].right - chips[1].left), (95.0, 136.0), "the label plus 44 DIP of glyph and padding");
        assert_eq!(chips[1].left - chips[0].right, 10.0, "8 DIP apart");
        let h = 312.0 * 1.25;
        assert!((h - chips[0].bottom - 10.0).abs() <= 1.0, "centred inside the 44 DIP footer band");
    }

    #[test]
    fn shelf_layout_rows_and_columns() {
        let s = Scale(1.25);
        let l = shelf_layout(5, &s);
        assert_eq!(l.tiles.len(), 5);
        assert!((l.tiles[3].top - l.tiles[0].top - 106.0 * 1.25).abs() <= 1.0, "row 2 starts one pitch lower");
        assert!((l.tiles[3].left - l.tiles[0].left).abs() <= 1.0, "row 2 starts in column 1");
        assert!((l.tiles[1].left - l.tiles[0].left - 113.0 * 1.25).abs() <= 1.0, "104 DIP tiles 9 DIP apart");
        assert!(((l.tiles[0].right - l.tiles[0].left) - 104.0 * 1.25).abs() <= 1.0);
    }

    #[test]
    fn shelf_tiles_line_up_with_the_header() {
        let s = Scale(1.25);
        let l = shelf_layout(3, &s);
        assert!((l.tiles[0].left - 20.0 * 1.25).abs() <= 1.0, "first tile under \"Raf\"'s left edge");
        assert!((l.tiles[2].right - 350.0 * 1.25).abs() <= 1.0, "last tile under the chip's right edge");
        assert!((l.clear.right - 350.0 * 1.25).abs() <= 1.0);
    }

    #[test]
    fn shelf_layout_gap_is_not_a_tile() {
        let s = Scale(1.25);
        let l = shelf_layout(2, &s);
        let x = (l.tiles[0].right + l.tiles[1].left) / 2.0;
        let y = (l.tiles[0].top + l.tiles[0].bottom) / 2.0;
        assert!(!l.tiles.iter().any(|t| x >= t.left && x < t.right && y >= t.top && y < t.bottom));
    }

    #[test]
    fn shelf_close_sits_inside_the_tiles_corner() {
        let s = Scale(1.25);
        let l = shelf_layout(1, &s);
        let (c, t) = (l.closes[0], l.tiles[0]);
        assert!((c.right - (t.right - 5.0 * 1.25)).abs() <= 1.0 && (c.top - (t.top + 5.0 * 1.25)).abs() <= 1.0, "{c:?} in {t:?}");
        assert!(((c.right - c.left) - 20.0 * 1.25).abs() <= 1.0, "a 20 DIP button");
    }

    fn app_card() -> AppCard {
        AppCard {
            key: 1,
            title: "Yedekleme".into(),
            name: "Ekin".into(),
            exe: "ekin.exe".into(),
            text: String::new(),
            status: String::new(),
            progress: None,
            icon: None,
            accent: (0x1D, 0xB9, 0x54),
            buttons: Vec::new(),
            prompt: false,
        }
    }

    #[test]
    fn an_app_card_is_as_tall_as_its_parts() {
        let h = |c: AppCard| card_size_dip(&CardContent::App(Box::new(c))).1;
        let bare = h(app_card());
        assert_eq!(bare, 134.0, "the header and the footer, 12 DIP under its chips as Claude's");
        assert_eq!(h(AppCard { text: "kısa".into(), ..app_card() }), bare + 22.0, "one line");
        assert_eq!(h(AppCard { text: "a".repeat(120), ..app_card() }), bare + 58.0, "three lines at most");
        assert_eq!(h(AppCard { progress: Some(40), ..app_card() }), bare + 14.0);
        assert_eq!(card_size_dip(&CardContent::App(Box::new(AppCard { prompt: true, ..app_card() }))), (370.0, 236.0), "as Claude's ask");
        assert_eq!(card_radius_dip(&CardContent::App(Box::new(app_card()))), 8.0);
        assert_eq!(card_accent(&CardContent::App(Box::new(app_card()))), (0x1D, 0xB9, 0x54));
    }

    #[test]
    fn app_buttons_keep_clear_of_sustur() {
        let s = Scale(1.25);
        let (w, d) = (370.0 * s.0, |v: f32| v * s.0);
        // Two labels near the 16-character cap: about 150 DIP each with their padding.
        let wide = app_button_spans(w, &[d(150.0), d(150.0)], &s);
        assert!(wide.iter().all(|(l, _)| *l >= d(16.0 + 84.0 + 8.0) - 0.5), "clear of Sustur: {wide:?}");
        assert!((wide[1].1 - (w - d(16.0))).abs() < 0.5, "flush right");
        assert!(wide[0].1 <= wide[1].0 - d(8.0) + 0.5, "8 DIP apart");
        let short = app_button_spans(w, &[d(60.0)], &s);
        assert!((short[0].1 - short[0].0 - d(92.0)).abs() < 0.5, "a short label keeps its width: {short:?}");
        assert!((app_button_spans(w, &[d(10.0)], &s)[0].1 - app_button_spans(w, &[d(10.0)], &s)[0].0 - d(72.0)).abs() < 0.5, "72 DIP at least");
    }

    #[test]
    fn app_text_lines_follow_its_length() {
        assert_eq!((app_text_lines(""), app_text_lines("kısa"), app_text_lines(&"ş".repeat(60)), app_text_lines(&"a".repeat(500))), (0, 1, 2, 3));
    }

    #[test]
    fn shelf_card_size() {
        assert_eq!(card_size_dip(&CardContent::Shelf(shelf_card(0))), (340.0, 204.0), "empty matches idle media");
        assert_eq!(card_size_dip(&CardContent::Shelf(shelf_card(5))), (340.0, 310.0), "two rows and the footer band");
        assert_eq!(card_radius_dip(&CardContent::Shelf(shelf_card(1))), 8.0);
    }

    fn claude(ask: bool, focus: bool) -> ClaudeCard {
        ClaudeCard {
            key: 1,
            title: "now-bar".into(),
            state: if ask { LiveState::Wait } else { LiveState::Run },
            status: "Çalışıyor · 3:12".into(),
            steps: vec![],
            tasks: None,
            summary: None,
            ask: ask.then(|| ClaudeAsk { key: 7, tool: "Bash".into(), detail: "cargo build".into(), answerable: true }),
            can_focus: focus,
        }
    }

    #[test]
    fn claude_card_grows_a_footer_for_its_buttons() {
        assert_eq!(card_size_dip(&CardContent::Claude(Box::new(claude(false, false)))), (370.0, 192.0));
        assert_eq!(card_size_dip(&CardContent::Claude(Box::new(claude(true, false)))), (370.0, 236.0), "İzin ver / Reddet");
        assert_eq!(card_size_dip(&CardContent::Claude(Box::new(claude(false, true)))), (370.0, 236.0), "Pencereye git");
        assert_eq!(card_radius_dip(&CardContent::Claude(Box::new(claude(false, false)))), 8.0);
    }

    #[test]
    fn slider_fraction_is_clamped() {
        assert_eq!(fraction_on(50.0, 0.0, 200.0), 0.25);
        assert_eq!(fraction_on(-5.0, 0.0, 200.0), 0.0);
        assert_eq!(fraction_on(500.0, 0.0, 200.0), 1.0);
    }

    /// Three other devices fit under the current one; the rest scroll.
    #[test]
    fn output_scroll_is_clamped() {
        assert_eq!(OUTPUT_VISIBLE_ROWS, 3);
        assert_eq!(clamp_scroll(5, 6), 3);
        assert_eq!(clamp_scroll(-1, 6), 0);
        assert_eq!(clamp_scroll(2, 2), 0);
    }

    #[test]
    fn output_sublines() {
        use crate::platform::audio::OutputBus;
        let buds = BudsSnapshot { left: 74, right: 69, case: None, noise: None, worn: Default::default() };
        assert_eq!(output_subline(OutputBus::Bluetooth, Some(&buds)), "Bluetooth · Left 74% · Right 69%");
        assert_eq!(output_subline(OutputBus::Bluetooth, None), "Bluetooth");
        assert_eq!(output_subline(OutputBus::Virtual, None), "Virtual");
        assert_eq!(output_subline(OutputBus::Internal, None), "Built-in");
        assert_eq!(output_subline(OutputBus::Usb, None), "USB");
    }

    #[test]
    fn media_app_name_stops_before_the_output_chip() {
        // The name starts at 42 DIP; the chip ends 16 DIP from the right of the 340 DIP card.
        let chip_w = 194.0;
        assert_eq!(media_app_name_width_dip(Some(chip_w)), 80.0);
        assert!(42.0 + media_app_name_width_dip(Some(chip_w)) + 8.0 <= 340.0 - 16.0 - chip_w);
        assert_eq!(media_app_name_width_dip(None), 282.0);
    }

    #[test]
    fn picker_starts_at_ten_minutes() {
        let p = PickerState::default();
        assert_eq!((p.stopwatch_tab, p.h, p.m, p.s), (false, 0, 10, 0));
        assert_eq!(p.total_ms(), 600_000);
    }

    #[test]
    fn picker_wheel_wraps_each_field() {
        let mut p = PickerState::default();
        p.wheel(1, 51);
        assert_eq!(p.m, 1);
        p.wheel(2, -1);
        assert_eq!(p.s, 59);
        p.wheel(0, -1);
        assert_eq!(p.h, 23);
        assert_eq!(p.total_ms(), (23 * 3600 + 60 + 59) * 1000);
    }

    #[test]
    fn picker_preset_sets_minutes_and_timer_tab() {
        let mut p = PickerState { stopwatch_tab: true, h: 1, m: 2, s: 3 };
        p.preset(15);
        assert_eq!((p.stopwatch_tab, p.h, p.m, p.s), (false, 0, 15, 0));
    }
}
