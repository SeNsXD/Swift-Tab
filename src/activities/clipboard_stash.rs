//! Clipboard stash: keeps recent copies (plain & rich text, links/URLs, images & screenshots)
//! for quick preview and re-pasting.

use std::collections::VecDeque;
use std::path::PathBuf;
use serde::{Deserialize, Serialize};
use crate::render::crest::Crest;

pub const CLIPBOARD_MOMENT_MS: u64 = 2500;
pub const MAX_STASH_ITEMS: usize = 20;

#[derive(Serialize, Deserialize)]
pub struct PinnedClipboardRecord {
    pub kind: String,
    pub has_html: bool,
    pub domain: String,
    pub width: u32,
    pub height: u32,
    pub preview: String,
    pub text: String,
    pub html: Option<String>,
    pub raw_png_hex: Option<String>,
    pub raw_dib_hex: Option<String>,
    pub timestamp: u64,
}

fn pinned_clipboard_path() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("SwiftTab").join("pinned_clipboard.json")
}

fn to_hex(bytes: &[u8]) -> String {
    let mut s = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        use std::fmt::Write;
        let _ = write!(s, "{:02x}", b);
    }
    s
}

fn from_hex(s: &str) -> Option<Vec<u8>> {
    if s.len() % 2 != 0 {
        return None;
    }
    (0..s.len())
        .step_by(2)
        .map(|i| u8::from_str_radix(&s[i..i + 2], 16).ok())
        .collect()
}

#[derive(Clone, Debug, PartialEq)]
pub enum ClipboardKind {
    /// Plain or Rich text (HTML formatted).
    Text { has_html: bool },
    /// Hyperlinks and web addresses.
    Url { domain: String },
    /// Image bitmap or screenshot.
    Image { width: u32, height: u32 },
}

#[derive(Clone, Debug, PartialEq)]
pub struct ClipboardItem {
    pub id: u64,
    pub kind: ClipboardKind,
    pub preview: String,
    pub text: String,
    pub html: Option<String>,
    pub thumb: Option<Crest>,
    pub raw_png: Option<Vec<u8>>,
    pub raw_dib: Option<Vec<u8>>,
    pub timestamp: u64,
    pub pinned: bool,
}

#[derive(Clone, Debug, Default)]
pub struct ClipboardStash {
    items: VecDeque<ClipboardItem>,
    next_id: u64,
}

impl ClipboardStash {
    pub fn new() -> Self {
        Self {
            items: VecDeque::new(),
            next_id: 1,
        }
    }

    pub fn push(&mut self, mut item: ClipboardItem) {
        item.id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);

        let mut was_pinned = false;
        // Remove identical prior item so it floats to the front without duplicates
        self.items.retain(|existing| {
            let same = if matches!(item.kind, ClipboardKind::Image { .. }) && matches!(existing.kind, ClipboardKind::Image { .. }) {
                if let (Some(a), Some(b)) = (&item.raw_dib, &existing.raw_dib) {
                    a == b
                } else if let (Some(a), Some(b)) = (&item.raw_png, &existing.raw_png) {
                    a == b
                } else {
                    false
                }
            } else {
                !item.text.is_empty() && existing.text == item.text
            };
            if same && existing.pinned {
                was_pinned = true;
            }
            !same
        });

        item.pinned = item.pinned || was_pinned;
        let is_pinned = item.pinned;
        if is_pinned {
            self.items.push_front(item);
        } else {
            let insert_at = self.items.iter().take_while(|i| i.pinned).count();
            self.items.insert(insert_at, item);
        }
        while self.items.len() > MAX_STASH_ITEMS {
            if let Some(pos) = self.items.iter().rposition(|i| !i.pinned) {
                self.items.remove(pos);
            } else {
                self.items.pop_back();
            }
        }
        if is_pinned {
            self.save_pinned();
        }
    }

    pub fn toggle_pin(&mut self, i: usize) {
        let Some(mut item) = self.items.remove(i) else { return };
        item.pinned = !item.pinned;
        if item.pinned {
            self.items.push_front(item);
        } else {
            let insert_at = self.items.iter().take_while(|x| x.pinned).count();
            self.items.insert(insert_at, item);
        }
        self.save_pinned();
    }

    pub fn remove(&mut self, i: usize) {
        if i < self.items.len() {
            let item = self.items.remove(i);
            if item.as_ref().is_some_and(|it| it.pinned) {
                self.save_pinned();
            }
        }
    }

    pub fn remove_id(&mut self, id: u64) {
        let was_pinned = self.items.iter().any(|i| i.id == id && i.pinned);
        self.items.retain(|i| i.id != id);
        if was_pinned {
            self.save_pinned();
        }
    }

    pub fn clear(&mut self) {
        self.items.retain(|i| i.pinned);
        self.save_pinned();
    }

    pub fn save_pinned(&self) {
        let pinned_records: Vec<PinnedClipboardRecord> = self
            .items
            .iter()
            .filter(|i| i.pinned)
            .map(|item| {
                let (kind_str, has_html, domain, width, height) = match &item.kind {
                    ClipboardKind::Text { has_html } => ("text".to_string(), *has_html, String::new(), 0, 0),
                    ClipboardKind::Url { domain } => ("url".to_string(), false, domain.clone(), 0, 0),
                    ClipboardKind::Image { width, height } => ("image".to_string(), false, String::new(), *width, *height),
                };
                PinnedClipboardRecord {
                    kind: kind_str,
                    has_html,
                    domain,
                    width,
                    height,
                    preview: item.preview.clone(),
                    text: item.text.clone(),
                    html: item.html.clone(),
                    raw_png_hex: item.raw_png.as_deref().map(to_hex),
                    raw_dib_hex: item.raw_dib.as_deref().map(to_hex),
                    timestamp: item.timestamp,
                }
            })
            .collect();

        let path = pinned_clipboard_path();
        if let Some(dir) = path.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Ok(json) = serde_json::to_string_pretty(&pinned_records) {
            let tmp = path.with_extension(format!("json.{}.tmp", std::process::id()));
            if std::fs::write(&tmp, json).is_ok() {
                let _ = std::fs::rename(&tmp, &path);
            }
        }
    }

    pub fn load_pinned(&mut self) {
        let path = pinned_clipboard_path();
        let Ok(text) = std::fs::read_to_string(&path) else { return };
        let text = text.trim_start_matches('\u{feff}');
        let Ok(records) = serde_json::from_str::<Vec<PinnedClipboardRecord>>(text) else { return };

        for r in records.into_iter().rev() {
            let raw_png = r.raw_png_hex.as_deref().and_then(from_hex);
            let raw_dib = r.raw_dib_hex.as_deref().and_then(from_hex);
            let kind = match r.kind.as_str() {
                "url" => ClipboardKind::Url { domain: r.domain },
                "image" => ClipboardKind::Image { width: r.width, height: r.height },
                _ => ClipboardKind::Text { has_html: r.has_html },
            };
            let thumb = raw_png
                .as_ref()
                .or(raw_dib.as_ref())
                .and_then(|b| crate::render::crest::thumb_from_bytes(b, 75))
                .map(|(t, _, _)| t);
            let item = ClipboardItem {
                id: self.next_id,
                kind,
                preview: r.preview,
                text: r.text,
                html: r.html,
                thumb,
                raw_png,
                raw_dib,
                timestamp: r.timestamp,
                pinned: true,
            };
            self.next_id = self.next_id.wrapping_add(1);
            self.items.push_front(item);
        }
    }

    pub fn items(&self) -> &VecDeque<ClipboardItem> {
        &self.items
    }

    pub fn get(&self, i: usize) -> Option<&ClipboardItem> {
        self.items.get(i)
    }

    pub fn is_empty(&self) -> bool {
        self.items.is_empty()
    }

    pub fn len(&self) -> usize {
        self.items.len()
    }
}

pub fn extract_domain(url: &str) -> String {
    let trimmed = url.trim();
    let after_proto = if let Some(rest) = trimmed.strip_prefix("https://") {
        rest
    } else if let Some(rest) = trimmed.strip_prefix("http://") {
        rest
    } else {
        trimmed
    };
    let host = after_proto.split('/').next().unwrap_or(after_proto);
    let host = host.split(':').next().unwrap_or(host);
    let host = host.split('?').next().unwrap_or(host);
    host.strip_prefix("www.").unwrap_or(host).to_string()
}

pub fn rows(n: usize) -> usize {
    n.div_ceil(3).clamp(1, 3)
}

pub fn card_height_dip(n: usize) -> f32 {
    let r = rows(n);
    // Header (48 DIP), 74 DIP 4:3 tiles 8 DIP apart, and bottom padding (16 DIP)
    48.0 + 82.0 * r as f32 + 8.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pinned_clipboard_serialization_and_retrieval() {
        let mut stash = ClipboardStash::new();
        stash.push(ClipboardItem {
            id: 0,
            kind: ClipboardKind::Text { has_html: false },
            preview: "Important note".into(),
            text: "Important note".into(),
            timestamp: 12345,
            pinned: false,
            thumb: None,
            raw_dib: None,
            raw_png: None,
            html: None,
        });
        stash.toggle_pin(0);
        assert!(stash.items()[0].pinned);

        // Verify save and load round-trip
        let mut loaded = ClipboardStash::new();
        loaded.load_pinned();
        assert!(loaded.items().iter().any(|i| i.text == "Important note" && i.pinned));

        // Clean up
        stash.clear();
        stash.remove(0);
        assert_eq!(stash.len(), 0);
    }
}
