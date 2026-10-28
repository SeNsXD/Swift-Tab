//! Clipboard stash: keeps recent copies (plain & rich text, links/URLs, images & screenshots)
//! for quick preview and re-pasting.

use std::collections::VecDeque;
use crate::render::crest::Crest;

pub const CLIPBOARD_MOMENT_MS: u64 = 2500;
pub const MAX_STASH_ITEMS: usize = 20;

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

        // Remove identical prior item so it floats to the front without duplicates
        self.items.retain(|existing| {
            if matches!(item.kind, ClipboardKind::Image { .. }) && matches!(existing.kind, ClipboardKind::Image { .. }) {
                // For images, check raw bytes if available
                if let (Some(a), Some(b)) = (&item.raw_dib, &existing.raw_dib) {
                    return a != b;
                }
                if let (Some(a), Some(b)) = (&item.raw_png, &existing.raw_png) {
                    return a != b;
                }
            }
            existing.text != item.text || item.text.is_empty()
        });

        self.items.push_front(item);
        if self.items.len() > MAX_STASH_ITEMS {
            self.items.pop_back();
        }
    }

    pub fn remove(&mut self, i: usize) {
        if i < self.items.len() {
            self.items.remove(i);
        }
    }

    pub fn remove_id(&mut self, id: u64) {
        self.items.retain(|i| i.id != id);
    }

    pub fn clear(&mut self) {
        self.items.clear();
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
    n.div_ceil(4).clamp(1, 3)
}

pub fn card_height_dip(n: usize) -> f32 {
    let r = rows(n);
    // Header (44 DIP), 75 DIP tiles 10 DIP apart, and bottom margin (16 DIP)
    44.0 + 85.0 * r as f32 + 16.0
}
