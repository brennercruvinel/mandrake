//! A one-line text field with a cursor.

use crate::theme;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use ratatui::style::{Modifier, Style};
use ratatui::text::Span;

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct TextField {
    value: String,
    /// In chars, 0 to the char count.
    cursor: usize,
    /// Accept only ASCII digits.
    digits: bool,
}

impl TextField {
    pub fn new(value: impl Into<String>) -> Self {
        let value = value.into();
        let cursor = value.chars().count();
        Self {
            value,
            cursor,
            digits: false,
        }
    }

    pub fn digits(value: impl Into<String>) -> Self {
        Self {
            digits: true,
            ..Self::new(value)
        }
    }

    pub fn value(&self) -> &str {
        &self.value
    }

    /// The trimmed value, or None when blank.
    pub fn some(&self) -> Option<String> {
        let v = self.value.trim();
        (!v.is_empty()).then(|| v.to_string())
    }

    pub fn set(&mut self, value: impl Into<String>) {
        *self = Self {
            digits: self.digits,
            ..Self::new(value)
        };
    }

    fn byte(&self, char_idx: usize) -> usize {
        self.value
            .char_indices()
            .nth(char_idx)
            .map(|(b, _)| b)
            .unwrap_or(self.value.len())
    }

    /// Applies an editing key; returns false when the key is not one the field takes.
    pub fn on_key(&mut self, k: &KeyEvent) -> bool {
        let len = self.value.chars().count();
        match k.code {
            KeyCode::Char('u') if k.modifiers.contains(KeyModifiers::CONTROL) => self.set(""),
            KeyCode::Char(_) if k.modifiers.contains(KeyModifiers::CONTROL) => return false,
            KeyCode::Char(c) if self.digits && !c.is_ascii_digit() => return false,
            KeyCode::Char(c) => {
                let at = self.byte(self.cursor);
                self.value.insert(at, c);
                self.cursor += 1;
            }
            KeyCode::Backspace if self.cursor > 0 => {
                self.cursor -= 1;
                let at = self.byte(self.cursor);
                self.value.remove(at);
            }
            KeyCode::Delete if self.cursor < len => {
                let at = self.byte(self.cursor);
                self.value.remove(at);
            }
            KeyCode::Backspace | KeyCode::Delete => {}
            KeyCode::Left => self.cursor = self.cursor.saturating_sub(1),
            KeyCode::Right => self.cursor = (self.cursor + 1).min(len),
            KeyCode::Home => self.cursor = 0,
            KeyCode::End => self.cursor = len,
            _ => return false,
        }
        true
    }

    /// The value, with the cursor cell shown when the field has focus.
    pub fn spans(&self, focused: bool, placeholder: &str) -> Vec<Span<'static>> {
        if self.value.is_empty() && !focused {
            return vec![Span::styled(placeholder.to_string(), theme::dim())];
        }
        let style = if focused {
            Style::new().add_modifier(Modifier::UNDERLINED)
        } else {
            Style::new()
        };
        if !focused {
            return vec![Span::styled(self.value.clone(), style)];
        }
        let at = self.byte(self.cursor);
        let (head, rest) = self.value.split_at(at);
        let mut chars = rest.chars();
        let under = chars.next().map(String::from).unwrap_or_else(|| " ".into());
        let mut spans = vec![
            Span::styled(head.to_string(), style),
            Span::styled(under, Style::new().add_modifier(Modifier::REVERSED)),
            Span::styled(chars.as_str().to_string(), style),
        ];
        if self.value.is_empty() {
            spans.push(Span::styled(format!(" {placeholder}"), theme::dim()));
        }
        spans
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn k(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::NONE)
    }

    #[test]
    fn edits_at_the_cursor() {
        let mut f = TextField::new("~/dév");
        assert!(f.on_key(&k(KeyCode::Left)));
        f.on_key(&k(KeyCode::Backspace));
        assert_eq!(f.value(), "~/dv");
        f.on_key(&k(KeyCode::Char('e')));
        f.on_key(&k(KeyCode::End));
        f.on_key(&k(KeyCode::Char('s')));
        assert_eq!(f.value(), "~/devs");
        f.on_key(&k(KeyCode::Home));
        f.on_key(&k(KeyCode::Delete));
        assert_eq!(f.value(), "/devs");
        f.on_key(&KeyEvent::new(KeyCode::Char('u'), KeyModifiers::CONTROL));
        assert_eq!(f.some(), None);
        assert!(!f.on_key(&k(KeyCode::Up)));
    }

    #[test]
    fn digit_fields_refuse_letters() {
        let mut f = TextField::digits("87");
        assert!(!f.on_key(&k(KeyCode::Char('q'))));
        assert!(f.on_key(&k(KeyCode::Char('8'))));
        assert_eq!(f.value(), "878");
        f.set("1");
        assert!(
            !f.on_key(&k(KeyCode::Char('x'))),
            "set keeps the digits rule"
        );
    }
}
