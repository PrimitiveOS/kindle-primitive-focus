use serde::{Deserialize, Serialize};

pub const MAX_MINUTES: u16 = 180;
pub const MAX_SUBJECT: usize = 48;
const MAX_SECONDS: u32 = MAX_MINUTES as u32 * 60;

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum Preset {
    #[default]
    Classic,
    Long,
    Custom,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum Phase {
    #[default]
    Focus,
    Break,
}

#[derive(Clone, Copy, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
pub enum Timer {
    #[default]
    Ready,
    Running {
        deadline: i64,
        length: u32,
    },
    Paused {
        remaining: u32,
        length: u32,
    },
    Finished,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(from = "SavedSession")]
pub struct Session {
    pub finished: i64,
    pub seconds: u32,
    pub subject: String,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum SavedSession {
    Labelled {
        finished: i64,
        seconds: u32,
        #[serde(default)]
        subject: String,
    },
    Previous((i64, u32)),
}

impl From<SavedSession> for Session {
    fn from(saved: SavedSession) -> Self {
        match saved {
            SavedSession::Labelled {
                finished,
                seconds,
                subject,
            } => Self {
                finished,
                seconds,
                subject,
            },
            SavedSession::Previous((finished, seconds)) => Self {
                finished,
                seconds,
                subject: String::new(),
            },
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Focus {
    pub preset: Preset,
    pub phase: Phase,
    pub timer: Timer,
    custom_focus: u16,
    custom_break: u16,
    completions: Vec<Session>,
    #[serde(default)]
    subject: String,
}

impl Default for Focus {
    fn default() -> Self {
        Self {
            preset: Preset::Classic,
            phase: Phase::Focus,
            timer: Timer::Ready,
            custom_focus: 25,
            custom_break: 5,
            completions: Vec::new(),
            subject: String::new(),
        }
    }
}

impl Focus {
    pub fn restored(mut self) -> Self {
        self.custom_focus = self.custom_focus.clamp(1, MAX_MINUTES);
        self.custom_break = self.custom_break.clamp(1, MAX_MINUTES);
        let valid_length = |length: u32| (60..=MAX_SECONDS).contains(&length);
        let valid = match self.timer {
            Timer::Running { length, .. } => valid_length(length),
            Timer::Paused { remaining, length } => {
                valid_length(length) && (1..=length).contains(&remaining)
            }
            Timer::Ready | Timer::Finished => true,
        };
        if !valid {
            self.reset();
        }
        self.subject = subject_text(&self.subject);
        self.completions
            .retain(|session| valid_length(session.seconds));
        for session in &mut self.completions {
            session.subject = subject_text(&session.subject);
        }
        if self.completions.len() > 512 {
            self.completions.drain(..self.completions.len() - 512);
        }
        self
    }

    pub fn subject(&self) -> &str {
        &self.subject
    }

    pub fn can_choose_subject(&self) -> bool {
        self.timer == Timer::Ready || (self.timer == Timer::Finished && self.phase == Phase::Break)
    }

    pub fn choose_subject(&mut self, subject: &str) {
        if self.can_choose_subject() {
            self.subject = subject_text(subject);
        }
    }

    pub fn sessions(&self) -> &[Session] {
        &self.completions
    }

    pub fn custom(&self, phase: Phase) -> u16 {
        match phase {
            Phase::Focus => self.custom_focus,
            Phase::Break => self.custom_break,
        }
    }

    pub fn choose(&mut self, preset: Preset) {
        if self.timer == Timer::Ready {
            self.preset = preset;
        }
    }

    pub fn adjust(&mut self, phase: Phase, by: i16) {
        if self.timer == Timer::Ready && self.preset == Preset::Custom {
            let value = match phase {
                Phase::Focus => &mut self.custom_focus,
                Phase::Break => &mut self.custom_break,
            };
            *value = value.saturating_add_signed(by).clamp(1, MAX_MINUTES);
        }
    }

    pub fn primary(&mut self, now: i64) {
        self.advance(now);
        self.timer = match self.timer {
            Timer::Ready => Self::start(now, self.duration(), self.duration()),
            Timer::Running { deadline, length } => Timer::Paused {
                remaining: seconds_until(deadline, now, length),
                length,
            },
            Timer::Paused { remaining, length } => Self::start(now, remaining, length),
            Timer::Finished => {
                self.phase = match self.phase {
                    Phase::Focus => Phase::Break,
                    Phase::Break => Phase::Focus,
                };
                Self::start(now, self.duration(), self.duration())
            }
        };
    }

    pub fn reset(&mut self) {
        self.timer = Timer::Ready;
        self.phase = Phase::Focus;
    }

    pub fn advance(&mut self, now: i64) -> bool {
        let Timer::Running { deadline, length } = self.timer else {
            return false;
        };
        if now < deadline {
            return false;
        }
        if self.phase == Phase::Focus {
            self.completions.push(Session {
                finished: deadline,
                seconds: length,
                subject: self.subject.clone(),
            });
            if self.completions.len() > 512 {
                self.completions.remove(0);
            }
        }
        self.timer = Timer::Finished;
        true
    }

    pub fn remaining(&self, now: i64) -> u32 {
        match self.timer {
            Timer::Ready => self.duration(),
            Timer::Running { deadline, length } => seconds_until(deadline, now, length),
            Timer::Paused { remaining, .. } => remaining,
            Timer::Finished => 0,
        }
    }

    pub fn next_change(&self, now: i64) -> Option<u32> {
        if !matches!(self.timer, Timer::Running { .. }) {
            return None;
        }
        let remaining = self.remaining(now);
        Some(remaining.saturating_sub(1) % 60 + 1)
    }

    pub fn totals(&self, mut is_today: impl FnMut(i64) -> bool) -> (usize, u32) {
        self.completions
            .iter()
            .filter(|session| is_today(session.finished))
            .fold((0, 0_u32), |(count, seconds), session| {
                (count + 1, seconds.saturating_add(session.seconds))
            })
    }

    fn duration(&self) -> u32 {
        let minutes = match (self.preset, self.phase) {
            (Preset::Classic, Phase::Focus) => 25,
            (Preset::Classic, Phase::Break) => 5,
            (Preset::Long, Phase::Focus) => 50,
            (Preset::Long, Phase::Break) => 10,
            (Preset::Custom, phase) => self.custom(phase).clamp(1, MAX_MINUTES),
        };
        u32::from(minutes) * 60
    }

    fn start(now: i64, remaining: u32, length: u32) -> Timer {
        Timer::Running {
            deadline: now.saturating_add(i64::from(remaining)),
            length,
        }
    }
}

fn subject_text(text: &str) -> String {
    text.trim()
        .chars()
        .filter(|c| !c.is_control())
        .take(MAX_SUBJECT)
        .collect()
}

fn seconds_until(deadline: i64, now: i64, limit: u32) -> u32 {
    deadline.saturating_sub(now).clamp(0, i64::from(limit)) as u32
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pause_and_resume_keep_the_original_focus_duration() {
        let mut focus = Focus::default();
        focus.primary(100);
        assert_eq!(focus.next_change(101), Some(59));
        focus.primary(130);
        assert_eq!(focus.remaining(400), 1470);
        focus.primary(400);
        assert!(!focus.advance(1869));
        assert!(focus.advance(1870));
        assert_eq!(focus.totals(|_| true), (1, 1500));
        assert_eq!(focus.totals(|time| time > 1870), (0, 0));
    }

    #[test]
    fn completed_focus_is_counted_once_and_break_requires_a_new_start() {
        let mut focus = Focus::default();
        focus.choose(Preset::Long);
        focus.primary(0);
        assert!(focus.advance(3000));
        assert!(!focus.advance(3200));
        assert_eq!(focus.timer, Timer::Finished);
        focus.primary(3200);
        assert_eq!(focus.phase, Phase::Break);
        assert_eq!(focus.remaining(3200), 600);
        assert!(focus.advance(3800));
        assert_eq!(focus.totals(|_| true), (1, 3000));
        focus.primary(3800);
        assert_eq!(focus.phase, Phase::Focus);
    }

    #[test]
    fn custom_lengths_are_bounded_and_locked_when_running() {
        let mut focus = Focus::default();
        focus.choose(Preset::Custom);
        focus.adjust(Phase::Focus, -200);
        focus.adjust(Phase::Break, 200);
        assert_eq!(focus.custom(Phase::Focus), 1);
        assert_eq!(focus.custom(Phase::Break), 180);
        focus.primary(0);
        focus.adjust(Phase::Focus, 30);
        focus.choose(Preset::Classic);
        assert_eq!(focus.remaining(0), 60);
        focus.reset();
        assert_eq!(focus.preset, Preset::Custom);
    }
}
