//! Bounded chat history and renderer-independent text entry/fade behavior.
use std::collections::VecDeque;

pub const MAX_CHAT_CHARS: usize = 160;
pub const MAX_CHAT_BYTES: usize = MAX_CHAT_CHARS * 4;
pub const CHAT_HISTORY: usize = 64;
pub const CHAT_IDLE_SECONDS: f32 = 10.0;
pub const CHAT_FADE_SECONDS: f32 = 2.0;

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[repr(u8)]
pub enum ChatChannel {
    #[default]
    Team = 0,
    Match = 1,
}
impl ChatChannel {
    pub fn from_wire(value: u8) -> Option<Self> {
        match value {
            0 => Some(Self::Team),
            1 => Some(Self::Match),
            _ => None,
        }
    }
    pub fn toggle(&mut self) {
        *self = match self {
            Self::Team => Self::Match,
            Self::Match => Self::Team,
        };
    }
    pub fn color(self) -> [u8; 3] {
        match self {
            Self::Team => [100, 175, 255],
            Self::Match => [255, 165, 70],
        }
    }
    pub fn label(self, korean: bool) -> &'static str {
        match (self, korean) {
            (Self::Team, true) => "팀",
            (Self::Match, true) => "경기",
            (Self::Team, false) => "TEAM",
            (Self::Match, false) => "MATCH",
        }
    }
    pub fn visible_to(self, sender: usize, receiver: usize) -> bool {
        self == Self::Match || crate::game::team(sender) == crate::game::team(receiver)
    }
}

pub fn clean_text(text: &str) -> String {
    text.chars()
        .filter(|c| !c.is_control())
        .take(MAX_CHAT_CHARS)
        .collect::<String>()
        .trim()
        .to_owned()
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChatMessage {
    pub id: u64,
    pub sender: usize,
    pub character: u8,
    pub channel: ChatChannel,
    pub text: String,
}

#[derive(Debug, Default)]
pub struct ChatHistory {
    pub messages: VecDeque<ChatMessage>,
    pub revision: u64,
    discarded_through: u64,
}
impl ChatHistory {
    /// Reordered UDP deliveries appear in server order; retries never appear twice.
    pub fn insert(&mut self, message: ChatMessage) {
        if message.id <= self.discarded_through || self.messages.iter().any(|m| m.id == message.id)
        {
            return;
        }
        let index = self.messages.partition_point(|m| m.id < message.id);
        self.messages.insert(index, message);
        if self.messages.len() > CHAT_HISTORY {
            self.discarded_through = self.messages.pop_front().unwrap().id;
        }
        self.revision += 1;
    }
}

/// Local transcript shared by every screen, independent of the current room's
/// wire IDs. Transport histories still own ordering, retries and recipient checks.
#[derive(Debug, Default)]
pub struct ChatTranscript {
    pub history: ChatHistory,
    session_offset: u64,
    source_revision: u64,
}
impl ChatTranscript {
    pub fn sync(&mut self, source: &ChatHistory) {
        if source.revision == self.source_revision {
            return;
        }
        for message in &source.messages {
            let mut message = message.clone();
            message.id += self.session_offset;
            self.history.insert(message);
        }
        self.source_revision = source.revision;
    }

    /// Call after syncing the old room and before consuming a new room's history.
    /// Old messages remain local; they are never replayed to the new recipients.
    pub fn next_session(&mut self) {
        self.session_offset = self.history.messages.back().map_or(0, |m| m.id);
        self.source_revision = 0;
    }
}

#[derive(Debug, Default)]
pub struct ChatInput {
    pub open: bool,
    pub channel: ChatChannel,
    pub draft: String,
    /// Byte boundary in draft, always on a UTF-8 character boundary.
    pub caret: usize,
    pub preedit: String,
    pub idle: f32,
}
impl ChatInput {
    pub fn touch(&mut self) {
        self.idle = 0.0;
    }
    pub fn tick(&mut self, dt: f32) {
        if self.open {
            self.touch();
        } else {
            self.idle = (self.idle + dt.max(0.0)).min(CHAT_IDLE_SECONDS + CHAT_FADE_SECONDS);
        }
    }
    pub fn opacity(&self) -> f32 {
        if self.open {
            1.0
        } else {
            (1.0 - (self.idle - CHAT_IDLE_SECONDS) / CHAT_FADE_SECONDS).clamp(0.0, 1.0)
        }
    }
    pub fn insert(&mut self, value: &str) {
        let remaining = MAX_CHAT_CHARS.saturating_sub(self.draft.chars().count());
        let value: String = value
            .chars()
            .filter(|c| !c.is_control())
            .take(remaining)
            .collect();
        self.draft.insert_str(self.caret, &value);
        self.caret += value.len();
        self.touch();
    }
    pub fn left(&mut self) {
        self.caret = self.draft[..self.caret]
            .char_indices()
            .next_back()
            .map_or(0, |(i, _)| i);
    }
    pub fn right(&mut self) {
        if let Some(c) = self.draft[self.caret..].chars().next() {
            self.caret += c.len_utf8();
        }
    }
    pub fn backspace(&mut self) {
        let end = self.caret;
        self.left();
        self.draft.replace_range(self.caret..end, "");
    }
    pub fn delete(&mut self) {
        if self.caret < self.draft.len() {
            self.draft.remove(self.caret);
        }
    }
    pub fn clear(&mut self) {
        self.draft.clear();
        self.preedit.clear();
        self.caret = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fade_waits_then_interpolates_and_input_restores_visibility() {
        let mut input = ChatInput::default();
        input.tick(10.0);
        assert_eq!(input.opacity(), 1.0);
        input.tick(1.0);
        assert_eq!(input.opacity(), 0.5);
        input.tick(10.0);
        assert_eq!(input.opacity(), 0.0);
        input.open = true;
        assert_eq!(input.opacity(), 1.0);
        input.tick(100.0);
        input.open = false;
        assert_eq!(input.opacity(), 1.0);
    }

    #[test]
    fn unicode_editing_limits_and_empty_messages() {
        let mut input = ChatInput::default();
        input.insert("안녕😀");
        input.left();
        input.backspace();
        input.insert("녕하세요");
        assert_eq!(input.draft, "안녕하세요😀");
        input.delete();
        assert_eq!(input.draft, "안녕하세요");
        input.insert(&"가".repeat(200));
        assert_eq!(input.draft.chars().count(), MAX_CHAT_CHARS);
        assert_eq!(clean_text(" \r\n\t "), "");
        assert_eq!(clean_text(" 안녕\n하세요\0 "), "안녕하세요");
    }

    #[test]
    fn history_orders_deduplicates_and_remains_bounded() {
        let message = |id| ChatMessage {
            id,
            sender: 0,
            character: 0,
            channel: ChatChannel::Team,
            text: "hello".into(),
        };
        let mut history = ChatHistory::default();
        history.insert(message(2));
        history.insert(message(1));
        history.insert(message(2));
        assert_eq!(history.revision, 2);
        assert_eq!(history.messages[0].id, 1);
        for id in 3..=100 {
            history.insert(message(id));
        }
        history.insert(message(1));
        assert_eq!(history.messages.len(), CHAT_HISTORY);
        assert_eq!(history.messages[0].id, 37);
        assert_eq!(history.revision, 100);
    }

    #[test]
    fn transcript_keeps_rooms_separate_even_with_reused_and_reordered_ids() {
        let message = |id, text: &str| ChatMessage {
            id,
            sender: 0,
            character: 0,
            channel: ChatChannel::Match,
            text: text.into(),
        };
        let mut transcript = ChatTranscript::default();
        let mut room = ChatHistory::default();
        room.insert(message(2, "old second"));
        transcript.sync(&room);
        room.insert(message(1, "old first"));
        transcript.sync(&room);
        transcript.sync(&room);
        transcript.next_session();
        room = ChatHistory::default();
        room.insert(message(2, "new second"));
        transcript.sync(&room);
        room.insert(message(1, "new first"));
        transcript.sync(&room);
        assert_eq!(
            transcript
                .history
                .messages
                .iter()
                .map(|m| m.text.as_str())
                .collect::<Vec<_>>(),
            ["old first", "old second", "new first", "new second"]
        );
        for _ in 0..100 {
            transcript.next_session();
            let mut room = ChatHistory::default();
            room.insert(message(1, "next room"));
            transcript.sync(&room);
            transcript.sync(&room);
        }
        assert_eq!(transcript.history.messages.len(), CHAT_HISTORY);
        assert_eq!(transcript.history.revision, 104);
        assert_eq!(transcript.history.messages.back().unwrap().id, 104);
    }
}
