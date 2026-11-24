//! Persistent floating sticky notes stash and editor.
//! Notes are saved to %APPDATA%\SwiftTab\notes.json, with images stored in note_images\.

use std::path::{Path, PathBuf};
use serde::{Deserialize, Serialize};
use crate::render::crest::{load_cover, Crest};

pub const MAX_NOTE_LENGTH: usize = 10000;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct NoteRecord {
    pub id: u64,
    pub title: String,
    pub text: String,
    pub image_path: Option<String>,
    pub created_at: u64,
    pub updated_at: u64,
    pub pinned: bool,
}

#[derive(Clone, Debug)]
pub struct NoteItem {
    pub record: NoteRecord,
    pub thumb: Option<Crest>,
}

fn notes_dir() -> PathBuf {
    let base = std::env::var_os("APPDATA").map(PathBuf::from).unwrap_or_else(std::env::temp_dir);
    base.join("SwiftTab")
}

fn notes_path() -> PathBuf {
    notes_dir().join("notes.json")
}

pub fn note_images_dir() -> PathBuf {
    notes_dir().join("note_images")
}

#[derive(Clone, Debug, Default)]
pub struct NotesStash {
    notes: Vec<NoteItem>,
    next_id: u64,
}

impl NotesStash {
    pub fn new() -> Self {
        Self {
            notes: Vec::new(),
            next_id: 1,
        }
    }

    pub fn load() -> Self {
        let path = notes_path();
        let Ok(text) = std::fs::read_to_string(&path) else {
            return Self::new();
        };
        let text = text.trim_start_matches('\u{feff}');
        let Ok(records) = serde_json::from_str::<Vec<NoteRecord>>(text) else {
            return Self::new();
        };

        let mut max_id = 0u64;
        let mut notes = Vec::with_capacity(records.len());
        for record in records {
            if record.text.trim().is_empty() && record.image_path.is_none() {
                continue;
            }
            if record.id > max_id {
                max_id = record.id;
            }
            let thumb = record.image_path.as_deref().and_then(|p| load_cover(Path::new(p), 320, 160));
            notes.push(NoteItem { record, thumb });
        }

        Self {
            notes,
            next_id: max_id.wrapping_add(1).max(1),
        }
    }

    pub fn save(&self) {
        let dir = notes_dir();
        let _ = std::fs::create_dir_all(&dir);
        let records: Vec<&NoteRecord> = self.notes.iter().map(|n| &n.record)
            .filter(|r| !r.text.trim().is_empty() || r.image_path.is_some())
            .collect();
        if let Ok(json) = serde_json::to_string_pretty(&records) {
            let path = notes_path();
            let tmp = path.with_extension(format!("json.{}.tmp", std::process::id()));
            if std::fs::write(&tmp, json.as_bytes()).is_ok() {
                let _ = std::fs::rename(&tmp, &path);
            }
        }
    }

    pub fn prune_empty(&mut self) {
        let before = self.notes.len();
        self.notes.retain(|n| !n.record.text.trim().is_empty() || n.record.image_path.is_some());
        if self.notes.len() != before {
            self.save();
        }
    }

    pub fn create_note(&mut self) -> u64 {
        let id = self.next_id;
        self.next_id = self.next_id.wrapping_add(1);
        let now = now_epoch_ms();
        let record = NoteRecord {
            id,
            title: "New Note".to_string(),
            text: String::new(),
            image_path: None,
            created_at: now,
            updated_at: now,
            pinned: false,
        };
        // Insert after any pinned notes
        let insert_idx = self.notes.iter().position(|n| !n.record.pinned).unwrap_or(self.notes.len());
        self.notes.insert(insert_idx, NoteItem { record, thumb: None });
        id
    }

    pub fn get(&self, id: u64) -> Option<&NoteItem> {
        self.notes.iter().find(|n| n.record.id == id)
    }

    pub fn get_mut(&mut self, id: u64) -> Option<&mut NoteItem> {
        self.notes.iter_mut().find(|n| n.record.id == id)
    }

    pub fn toggle_pin(&mut self, id: u64) {
        let Some(pos) = self.notes.iter().position(|n| n.record.id == id) else { return };
        let mut note = self.notes.remove(pos);
        note.record.pinned = !note.record.pinned;
        note.record.updated_at = now_epoch_ms();
        if note.record.pinned {
            self.notes.insert(0, note);
        } else {
            let insert_idx = self.notes.iter().position(|n| !n.record.pinned).unwrap_or(self.notes.len());
            self.notes.insert(insert_idx, note);
        }
        self.save();
    }

    pub fn remove(&mut self, id: u64) {
        if let Some(pos) = self.notes.iter().position(|n| n.record.id == id) {
            let note = self.notes.remove(pos);
            if let Some(img) = note.record.image_path {
                let _ = std::fs::remove_file(img);
            }
            self.save();
        }
    }

    pub fn clear_unpinned(&mut self) {
        self.notes.retain(|n| {
            if n.record.pinned {
                true
            } else {
                if let Some(img) = &n.record.image_path {
                    let _ = std::fs::remove_file(img);
                }
                false
            }
        });
        self.save();
    }

    pub fn clear_all(&mut self) {
        for n in &self.notes {
            if let Some(img) = &n.record.image_path {
                let _ = std::fs::remove_file(img);
            }
        }
        self.notes.clear();
        self.save();
    }

    pub fn update_text(&mut self, id: u64, text: String) {
        let Some(note) = self.get_mut(id) else { return };
        note.record.text = text;
        note.record.title = derive_title(&note.record.text);
        note.record.updated_at = now_epoch_ms();
        self.save();
    }

    pub fn attach_image(&mut self, id: u64, image_path: String, thumb: Option<Crest>) {
        let Some(note) = self.get_mut(id) else { return };
        if let Some(old) = &note.record.image_path {
            if old != &image_path {
                let _ = std::fs::remove_file(old);
            }
        }
        note.record.image_path = Some(image_path);
        note.thumb = thumb;
        note.record.updated_at = now_epoch_ms();
        self.save();
    }

    pub fn remove_image(&mut self, id: u64) {
        let Some(note) = self.get_mut(id) else { return };
        if let Some(old) = note.record.image_path.take() {
            let _ = std::fs::remove_file(old);
        }
        note.thumb = None;
        note.record.updated_at = now_epoch_ms();
        self.save();
    }

    pub fn notes(&self) -> &[NoteItem] {
        &self.notes
    }

    pub fn is_empty(&self) -> bool {
        self.notes.is_empty()
    }

    pub fn count(&self) -> usize {
        self.notes.len()
    }
}

pub fn format_note_age(updated_at_epoch_ms: u64, _now_ms: u64) -> String {
    let now_epoch = now_epoch_ms();
    let diff_s = now_epoch.saturating_sub(updated_at_epoch_ms) / 1000;
    if diff_s < 60 {
        "Just now".to_string()
    } else if diff_s < 3600 {
        format!("{}m ago", diff_s / 60)
    } else if diff_s < 86400 {
        format!("{}h ago", diff_s / 3600)
    } else {
        format!("{}d ago", diff_s / 86400)
    }
}

pub fn derive_title(text: &str) -> String {
    for line in text.lines() {
        let trimmed = line.trim();
        if !trimmed.is_empty() {
            let title: String = trimmed.chars().take(30).collect();
            return if trimmed.chars().count() > 30 { format!("{title}…") } else { title };
        }
    }
    "Untitled Note".to_string()
}

fn now_epoch_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0)
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NoteEdit {
    None,
    Changed,
    Close,
    Paste,
}

/// Multi-line text field editor with caret support.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NoteField {
    pub text: String,
    pub caret: usize,
    high: Option<u16>,
}

impl NoteField {
    pub fn new(text: String) -> Self {
        let caret = text.chars().count();
        Self { text, caret, high: None }
    }

    pub fn key(&mut self, unit: u16) -> NoteEdit {
        match unit {
            // Backspace
            0x08 => {
                if self.caret > 0 {
                    let chars: Vec<char> = self.text.chars().collect();
                    let mut new_chars = Vec::with_capacity(chars.len());
                    for (i, c) in chars.into_iter().enumerate() {
                        if i + 1 != self.caret {
                            new_chars.push(c);
                        }
                    }
                    self.text = new_chars.into_iter().collect();
                    self.caret = self.caret.saturating_sub(1);
                    NoteEdit::Changed
                } else {
                    NoteEdit::None
                }
            }
            // Ctrl+Backspace
            0x7F => {
                if self.caret > 0 {
                    let chars: Vec<char> = self.text.chars().collect();
                    let before = &chars[..self.caret];
                    let mut del_count = 0;
                    let mut seen_non_ws = false;
                    for &c in before.iter().rev() {
                        if c.is_whitespace() {
                            if seen_non_ws {
                                break;
                            }
                            del_count += 1;
                        } else {
                            seen_non_ws = true;
                            del_count += 1;
                        }
                    }
                    if del_count > 0 {
                        let new_caret = self.caret - del_count;
                        let mut new_chars = Vec::with_capacity(chars.len() - del_count);
                        new_chars.extend_from_slice(&chars[..new_caret]);
                        new_chars.extend_from_slice(&chars[self.caret..]);
                        self.text = new_chars.into_iter().collect();
                        self.caret = new_caret;
                        return NoteEdit::Changed;
                    }
                }
                NoteEdit::None
            }
            // Enter: inserts newline!
            0x0D => {
                if self.room() {
                    self.insert_char('\n');
                    NoteEdit::Changed
                } else {
                    NoteEdit::None
                }
            }
            // Escape: Close
            0x1B => NoteEdit::Close,
            // Ctrl+V: Paste
            0x16 => NoteEdit::Paste,
            // UTF-16 surrogate pairs (emojis)
            0xD800..=0xDBFF => {
                self.high = Some(unit);
                NoteEdit::None
            }
            0xDC00..=0xDFFF => match self.high.take().and_then(|h| char::decode_utf16([h, unit]).next()?.ok()) {
                Some(c) if self.room() => {
                    self.insert_char(c);
                    NoteEdit::Changed
                }
                _ => NoteEdit::None,
            },
            u if u < 0x20 || u == 0x7F => NoteEdit::None,
            u => match char::from_u32(u as u32) {
                Some(c) if self.room() => {
                    self.insert_char(c);
                    NoteEdit::Changed
                }
                _ => NoteEdit::None,
            },
        }
    }

    pub fn insert_char(&mut self, c: char) {
        let chars: Vec<char> = self.text.chars().collect();
        let mut new_chars = Vec::with_capacity(chars.len() + 1);
        new_chars.extend_from_slice(&chars[..self.caret.min(chars.len())]);
        new_chars.push(c);
        if self.caret < chars.len() {
            new_chars.extend_from_slice(&chars[self.caret..]);
        }
        self.text = new_chars.into_iter().collect();
        self.caret += 1;
    }

    pub fn insert_str(&mut self, s: &str) {
        for c in s.chars() {
            if c != '\r' && (c == '\n' || !c.is_control()) {
                if !self.room() {
                    break;
                }
                self.insert_char(c);
            }
        }
    }

    pub fn move_left(&mut self) {
        self.caret = self.caret.saturating_sub(1);
    }

    pub fn move_right(&mut self) {
        let len = self.text.chars().count();
        if self.caret < len {
            self.caret += 1;
        }
    }

    pub fn move_up(&mut self) {
        let chars: Vec<char> = self.text.chars().collect();
        let caret = self.caret.min(chars.len());
        let mut line_start = caret;
        while line_start > 0 && chars[line_start - 1] != '\n' {
            line_start -= 1;
        }
        let col = caret - line_start;
        if line_start == 0 {
            self.caret = 0;
            return;
        }
        let prev_line_end = line_start - 1;
        let mut prev_line_start = prev_line_end;
        while prev_line_start > 0 && chars[prev_line_start - 1] != '\n' {
            prev_line_start -= 1;
        }
        let prev_line_len = prev_line_end - prev_line_start;
        self.caret = prev_line_start + col.min(prev_line_len);
    }

    pub fn move_down(&mut self) {
        let chars: Vec<char> = self.text.chars().collect();
        let caret = self.caret.min(chars.len());
        let mut line_start = caret;
        while line_start > 0 && chars[line_start - 1] != '\n' {
            line_start -= 1;
        }
        let col = caret - line_start;
        let mut line_end = caret;
        while line_end < chars.len() && chars[line_end] != '\n' {
            line_end += 1;
        }
        if line_end >= chars.len() {
            self.caret = chars.len();
            return;
        }
        let next_line_start = line_end + 1;
        let mut next_line_end = next_line_start;
        while next_line_end < chars.len() && chars[next_line_end] != '\n' {
            next_line_end += 1;
        }
        let next_line_len = next_line_end - next_line_start;
        self.caret = next_line_start + col.min(next_line_len);
    }

    pub fn move_home(&mut self) {
        self.caret = 0;
    }

    pub fn move_end(&mut self) {
        self.caret = self.text.chars().count();
    }

    fn room(&self) -> bool {
        self.text.chars().count() < MAX_NOTE_LENGTH
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn note_field_multiline_editing() {
        let mut f = NoteField::default();
        f.key('H' as u16);
        f.key('i' as u16);
        f.key(0x0D); // Enter
        f.key('!' as u16);
        assert_eq!(f.text, "Hi\n!");
        assert_eq!(f.caret, 4);

        f.key(0x08); // Backspace
        assert_eq!(f.text, "Hi\n");
        assert_eq!(f.caret, 3);
    }

    #[test]
    fn derive_title_picks_first_nonempty_line() {
        assert_eq!(derive_title("\n\n  Shopping List  \nEggs\nMilk"), "Shopping List");
        assert_eq!(derive_title(""), "Untitled Note");
    }
}
