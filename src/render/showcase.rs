//! The pills the setup's stage turns through: what the Swift Tab shows, drawn by its own painter.

use windows::core::Result;

use super::layout::Scale;
use super::offscreen::render;
use super::pill::{paint_pill_content, PillContent, PillThumb};
use crate::activities::buds::{BudsSnapshot, Worn};
use crate::activities::media::{Caps, MediaSnapshot};
use crate::activities::shelf::FileKind;
use crate::activities::weather::WeatherFx;

/// Music, Weather, the Buds, the shelf (names made up).
pub fn pills() -> Vec<PillContent> {
    let song = MediaSnapshot {
        id: 1,
        app_id: "Spotify.exe".into(),
        app_name: "Spotify".into(),
        title: "Party On My Mind".into(),
        artist: "Pritam".into(),
        playing: true,
        timeline: None,
        can: Caps { play_pause: true, prev: true, next: true, seek: true, shuffle: false, repeat: false },
        shuffle: None,
        repeat: None,
        art: None,
        art_size: (0, 0),
        accent: [0x1D, 0xB9, 0x54],
        changed_ms: 0,
        muted: false,
    };
    let file = |kind| PillThumb { thumb: None, kind };
    vec![
        PillContent::media(song),
        PillContent::Weather {
            symbol: "⛅".into(),
            temp: "24°".into(),
            desc: "Partly cloudy".into(),
            fx: WeatherFx::Clouds,
        },
        PillContent::Buds(BudsSnapshot { left: 80, right: 75, case: Some(60), noise: None, worn: Worn { left: true, right: true } }),
        PillContent::Clipboard {
            label: "swift-tab-v1.0.0.zip".into(),
            count: 3,
            thumb: None,
        },
        PillContent::Shelf {
            label: "presentation-final.pdf".into(),
            count: 3,
            thumbs: vec![file(FileKind::Pdf), file(FileKind::Image), file(FileKind::Archive)],
            drop_hint: false,
            min_width_dip: 0.0,
        },
    ]
}

/// One pill for the stage: its content alone (the body is the stage's, so it can glide), the
/// body's colours top to bottom (straight RGBA) and the accent sweeping in from the left.
pub struct Frame {
    pub w: u32,
    pub h: u32,
    /// Premultiplied BGRA.
    pub px: Vec<u8>,
    pub top: [u8; 4],
    pub bottom: [u8; 4],
    pub accent: Option<(u8, u8, u8)>,
    /// The pill's colour (a song's cover, a team's, Claude's), for the light behind it.
    pub glow: (u8, u8, u8),
}

/// `content` drawn at `scale` on a transparent ground.
pub fn frame(content: &PillContent, scale: f32) -> Result<Frame> {
    let s = Scale(scale);
    let (w, h) = (s.even_px(content.width_dip()), s.px(32.0).round());
    let px = render(w as u32, h as u32, [0.0; 4], |ink| paint_pill_content(ink, content, w, h, &s))?;
    let (top, bottom) = content.base();
    let c = |c: windows::UI::Color| [c.R, c.G, c.B, c.A];
    Ok(Frame { w: w as u32, h: h as u32, px, top: c(top), bottom: c(bottom), accent: content.has_accent_gradient().then(|| content.accent()), glow: content.accent() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_stage_takes_every_pill_at_full_size() {
        let widths: Vec<f32> = pills().iter().map(PillContent::width_dip).collect();
        assert!(widths.iter().all(|w| *w <= 300.0), "{widths:?}: the setup's stage shows 300 DIP at full size");
    }
}
