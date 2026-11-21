use std::collections::BTreeMap;
use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use focus_core::{Focus, Phase, Preset, Session, Timer, MAX_SUBJECT};
use primitive_app::{App, Change};
use primitive_ui::{
    back_button, header, keyboard_key, progress, screen_rect, Canvas, Editor, Input, Layout, Press,
    Rect, TextStyle, BODY, CAPTION,
};

const DAY: i64 = 86_400;
const PRESETS: [(Preset, &str); 3] = [
    (Preset::Classic, "25 / 5"),
    (Preset::Long, "50 / 10"),
    (Preset::Custom, "Custom"),
];

#[derive(Clone, Copy, PartialEq, Eq)]
enum Page {
    Timer,
    Subjects,
    SubjectEditor,
    Week,
}

const LOG_ROWS: usize = 7;

pub struct View {
    pub focus: Focus,
    data: Option<PathBuf>,
    zone: Vec<u8>,
    now: i64,
    preview: bool,
    confirm_reset: bool,
    notice: Option<String>,
    page: Page,
    subject_editor: Editor,
    log_page: usize,
    show_sessions: bool,
}

impl View {
    pub fn open(data: PathBuf) -> Self {
        let focus = fs::read(&data)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Focus>(&bytes).ok())
            .map(Focus::restored)
            .unwrap_or_default();
        Self::new(focus, Some(data), wall_now(), false)
    }

    pub fn preview(now: i64) -> Self {
        Self::new(Focus::default(), None, now, true)
    }

    fn new(mut focus: Focus, data: Option<PathBuf>, now: i64, preview: bool) -> Self {
        let completed = focus.advance(now);
        let mut view = Self {
            focus,
            data,
            zone: fs::read("/etc/localtime").unwrap_or_default(),
            now,
            preview,
            confirm_reset: false,
            notice: None,
            page: Page::Timer,
            subject_editor: Editor::new(MAX_SUBJECT),
            log_page: 0,
            show_sessions: false,
        };
        if completed {
            view.persist();
        }
        view
    }

    pub fn at(&mut self, now: i64) -> bool {
        let day = self.day(self.now);
        let minutes = self.focus.remaining(self.now).div_ceil(60);
        self.now = now;
        let completed = self.focus.advance(now);
        if completed {
            self.persist();
        }
        if self.day(now) != day {
            self.log_page = 0;
        }
        completed || self.day(now) != day || self.focus.remaining(now).div_ceil(60) != minutes
    }

    fn persist(&mut self) {
        self.notice = self
            .save()
            .err()
            .map(|_| "Could not save the timer.".into());
    }

    fn offset(&self, when: i64) -> i64 {
        kl_core::tz::offset_at(&self.zone, when).map_or(0, i64::from)
    }

    fn day(&self, when: i64) -> i64 {
        when.saturating_add(self.offset(when)).div_euclid(DAY)
    }

    fn next_midnight(&self, now: i64) -> u32 {
        let local_midnight = (self.day(now) + 1) * DAY;
        let estimated = local_midnight - self.offset(now);
        let midnight = local_midnight - self.offset(estimated);
        midnight.saturating_sub(now).clamp(1, DAY + 3600) as u32
    }

    fn primary_label(&self) -> &'static str {
        match (self.focus.timer, self.focus.phase) {
            (Timer::Ready, _) => "Start focus",
            (Timer::Running { .. }, _) => "Pause",
            (Timer::Paused { .. }, _) => "Resume",
            (Timer::Finished, Phase::Focus) => "Start break",
            (Timer::Finished, Phase::Break) => "Start focus",
        }
    }

    fn draw_timer(&self, canvas: &mut Canvas, width: i32) {
        canvas.card(rect(width, 40, 175, 1184, 415));
        let phase = match self.focus.phase {
            Phase::Focus => "Focus session",
            Phase::Break => "Break",
        };
        let heading = if self.focus.phase == Phase::Focus && !self.focus.subject().is_empty() {
            format!("{phase} · {}", self.focus.subject())
        } else {
            phase.to_owned()
        };
        canvas.text(rect(width, 72, 234, 1120, 54), &heading, BODY.center());
        let remaining = self.focus.remaining(self.now);
        let caption;
        let timer = match (self.focus.timer, self.focus.phase) {
            (Timer::Finished, Phase::Focus) => {
                caption = "Take a break when you're ready.";
                "Focus complete".to_owned()
            }
            (Timer::Finished, Phase::Break) => {
                caption = "Start another session when you're ready.";
                "Break complete".to_owned()
            }
            (Timer::Paused { .. }, _) => {
                caption = "Paused";
                format!("{}:{:02}", remaining / 60, remaining % 60)
            }
            (_, _) => {
                caption = if remaining.div_ceil(60) == 1 {
                    "minute left"
                } else {
                    "minutes left"
                };
                remaining.div_ceil(60).to_string()
            }
        };
        let size = if self.focus.timer == Timer::Finished {
            82
        } else {
            138
        };
        canvas.text(
            rect(width, 72, 299, 1120, 170),
            &timer,
            TextStyle::new(size).bold().center(),
        );
        canvas.text(rect(width, 72, 474, 1120, 64), caption, CAPTION.center());
        if let Timer::Running { length, .. } | Timer::Paused { length, .. } = self.focus.timer {
            let filled = length.saturating_sub(remaining) * 10 / length.max(1);
            progress(canvas, rect(width, 260, 545, 744, 26), filled as usize);
        }
    }

    fn draw_settings(&self, canvas: &mut Canvas, width: i32) {
        canvas.text(
            rect(width, 40, 797, 1184, 50),
            "Focus and break lengths",
            CAPTION,
        );
        for (index, (preset, label)) in PRESETS.into_iter().enumerate() {
            canvas.button(
                preset_button(width, index),
                label,
                self.focus.preset == preset,
                false,
            );
        }
        if self.focus.preset == Preset::Custom {
            for (index, phase) in [Phase::Focus, Phase::Break].into_iter().enumerate() {
                let y = 1010 + 126 * index as i32;
                let name = if phase == Phase::Focus {
                    "Focus"
                } else {
                    "Break"
                };
                canvas.text(rect(width, 40, y + 21, 400, 68), name, BODY);
                let value = format!("{} min", self.focus.custom(phase));
                canvas.button(rect(width, 600, y, 130, 100), "-", false, false);
                canvas.text(rect(width, 738, y + 21, 340, 68), &value, BODY.center());
                canvas.button(rect(width, 1094, y, 130, 100), "+", false, false);
            }
        }
    }

    fn recent_subjects(&self) -> Vec<&str> {
        let mut subjects = Vec::new();
        for session in self.focus.sessions().iter().rev() {
            let subject = session.subject.as_str();
            if !subject.is_empty() && !subjects.contains(&subject) {
                subjects.push(subject);
                if subjects.len() == 5 {
                    break;
                }
            }
        }
        subjects
    }

    fn this_week(&self) -> impl Iterator<Item = &Session> {
        let today = self.day(self.now);
        let monday = today - (today + 3).rem_euclid(7);
        self.focus
            .sessions()
            .iter()
            .rev()
            .filter(move |session| (monday..=today).contains(&self.day(session.finished)))
    }

    fn subject_totals(&self) -> Vec<(&str, u32)> {
        let mut totals = BTreeMap::<&str, u32>::new();
        for session in self.this_week() {
            *totals.entry(labelled(&session.subject)).or_default() += session.seconds;
        }
        let mut totals = totals.into_iter().collect::<Vec<_>>();
        totals.sort_by(|left, right| right.1.cmp(&left.1).then(left.0.cmp(right.0)));
        totals
    }

    fn draw_log(&self, canvas: &mut Canvas, width: i32) {
        header(canvas, "This week", "Back");
        let sessions = self.this_week().collect::<Vec<_>>();
        let minutes = sessions.iter().map(|session| session.seconds).sum::<u32>() / 60;
        let count = sessions.len();
        let noun = if count == 1 { "session" } else { "sessions" };
        canvas.text(
            rect(width, 40, 162, 1184, 62),
            &format!("{count} focus {noun}  ·  {minutes} min"),
            BODY,
        );
        canvas.button(
            rect(width, 40, 255, 568, 100),
            "Subjects",
            !self.show_sessions,
            false,
        );
        canvas.button(
            rect(width, 656, 255, 568, 100),
            "Sessions",
            self.show_sessions,
            false,
        );
        let skip = self.log_page * LOG_ROWS;
        let len = if self.show_sessions {
            for (row, session) in sessions.iter().skip(skip).take(LOG_ROWS).enumerate() {
                let local = session
                    .finished
                    .saturating_add(self.offset(session.finished));
                let weekdays = ["Mon", "Tue", "Wed", "Thu", "Fri", "Sat", "Sun"];
                let weekday = weekdays[((self.day(session.finished) + 3).rem_euclid(7)) as usize];
                let hour = local.rem_euclid(DAY) / 3600;
                let minute = local.rem_euclid(3600) / 60;
                log_row(
                    canvas,
                    width,
                    row,
                    labelled(&session.subject),
                    &format!(
                        "{weekday} {hour:02}:{minute:02}  ·  {} min",
                        session.seconds / 60
                    ),
                );
            }
            count
        } else {
            let totals = self.subject_totals();
            for (row, &(subject, time)) in totals.iter().skip(skip).take(LOG_ROWS).enumerate() {
                log_row(canvas, width, row, subject, &format!("{} min", time / 60));
            }
            totals.len()
        };
        if count == 0 {
            canvas.text(
                rect(width, 40, 420, 1184, 90),
                "No finished sessions this week.",
                BODY.center(),
            );
        }
        if self.log_page > 0 {
            canvas.button(rect(width, 40, 1480, 400, 100), "Previous", false, false);
        }
        if (self.log_page + 1) * LOG_ROWS < len {
            canvas.button(rect(width, 824, 1480, 400, 100), "Next", false, false);
        }
    }

    fn draw_subjects(&self, canvas: &mut Canvas, width: i32) {
        header(canvas, "Subject", "Back");
        canvas.text(
            rect(width, 40, 165, 1184, 70),
            "Give your next focus session a label.",
            CAPTION,
        );
        canvas.button(rect(width, 40, 270, 568, 100), "No subject", false, false);
        canvas.button(rect(width, 656, 270, 568, 100), "New subject", true, false);
        for (index, subject) in self.recent_subjects().into_iter().enumerate() {
            canvas.button(
                subject_row(width, index),
                subject,
                subject == self.focus.subject(),
                false,
            );
        }
    }

    fn draw_subject_editor(&self, canvas: &mut Canvas, width: i32) {
        header(canvas, "New subject", "Cancel");
        canvas.card(rect(width, 40, 200, 1184, 200));
        canvas.text(rect(width, 70, 233, 1110, 65), "Subject name", CAPTION);
        let display = self.subject_editor.string();
        canvas.text(rect(width, 70, 305, 1110, 65), &display, BODY);
        canvas.keyboard(self.subject_editor.layer, "Save", None);
    }

    fn use_subject(&mut self, subject: &str) -> Change {
        self.focus.choose_subject(subject);
        self.page = Page::Timer;
        self.persist();
        Change::Full
    }

    fn log_page(&mut self, amount: isize) -> Change {
        let count = if self.show_sessions {
            self.this_week().count()
        } else {
            self.subject_totals().len()
        };
        let next = self
            .log_page
            .saturating_add_signed(amount)
            .min(count.div_ceil(LOG_ROWS).saturating_sub(1));
        if next == self.log_page {
            return Change::None;
        }
        self.log_page = next;
        Change::Partial
    }

    fn secondary_tap(&mut self, x: f32, y: f32) -> Change {
        let width = primitive_app::WIDTH as i32;
        if back_button(width).contains(x, y) {
            self.page = if self.page == Page::SubjectEditor {
                Page::Subjects
            } else {
                Page::Timer
            };
            return Change::Full;
        }
        match self.page {
            Page::Subjects => {
                if rect(width, 40, 270, 568, 100).contains(x, y) {
                    return self.use_subject("");
                }
                if rect(width, 656, 270, 568, 100).contains(x, y) {
                    self.subject_editor = Editor::new(MAX_SUBJECT);
                    self.page = Page::SubjectEditor;
                    return Change::Full;
                }
                let selected = self
                    .recent_subjects()
                    .into_iter()
                    .enumerate()
                    .find(|(index, _)| subject_row(width, *index).contains(x, y))
                    .map(|(_, value)| value.to_owned());
                if let Some(subject) = selected {
                    return self.use_subject(&subject);
                }
            }
            Page::SubjectEditor => {
                let layout = Layout::new(1264, 1680, 4, 5);
                if let Some(key) = keyboard_key(&layout, &self.subject_editor, "Save", x, y) {
                    return match self.subject_editor.press(key) {
                        Press::Nothing => Change::None,
                        Press::Go => self.use_subject(&self.subject_editor.string()),
                        _ => Change::Partial,
                    };
                }
            }
            Page::Week => {
                let subjects = rect(width, 40, 255, 568, 100).contains(x, y);
                let sessions = rect(width, 656, 255, 568, 100).contains(x, y);
                if subjects && self.show_sessions || sessions && !self.show_sessions {
                    self.show_sessions = sessions;
                    self.log_page = 0;
                    return Change::Partial;
                }
                if rect(width, 40, 1480, 400, 100).contains(x, y) {
                    return self.log_page(-1);
                }
                if rect(width, 824, 1480, 400, 100).contains(x, y) {
                    return self.log_page(1);
                }
            }
            Page::Timer => {}
        }
        Change::None
    }

    fn tap(&mut self, x: f32, y: f32) -> Change {
        let width = primitive_app::WIDTH as i32;
        if self.page != Page::Timer {
            return self.secondary_tap(x, y);
        }
        if self.confirm_reset {
            self.confirm_reset = false;
            if reset_confirm_button(width).contains(x, y) {
                self.focus.reset();
                self.persist();
            }
            return Change::Full;
        }
        if back_button(width).contains(x, y) {
            return Change::Exit;
        }
        if week_button(width).contains(x, y) {
            self.page = Page::Week;
            self.log_page = 0;
            self.show_sessions = false;
            return Change::Full;
        }
        if self.focus.can_choose_subject() && subject_button(width).contains(x, y) {
            self.page = Page::Subjects;
            return Change::Full;
        }
        if primary_button(width, self.focus.timer).contains(x, y) {
            self.focus.primary(self.now);
        } else if self.focus.timer != Timer::Ready && reset_button(width).contains(x, y) {
            self.confirm_reset = true;
            return Change::Full;
        } else if self.focus.timer == Timer::Ready {
            if let Some((_, (preset, _))) = PRESETS
                .into_iter()
                .enumerate()
                .find(|(index, _)| preset_button(width, *index).contains(x, y))
            {
                self.focus.choose(preset);
            } else if self.focus.preset == Preset::Custom {
                let Some((phase, step)) = [Phase::Focus, Phase::Break]
                    .into_iter()
                    .enumerate()
                    .find_map(|(index, phase)| {
                        let y_button = 1010 + 126 * index as i32;
                        [(-1, 600), (1, 1094)]
                            .into_iter()
                            .find(|(_, x_button)| {
                                rect(width, *x_button, y_button, 130, 100).contains(x, y)
                            })
                            .map(|(step, _)| (phase, step))
                    })
                else {
                    return Change::None;
                };
                self.focus.adjust(phase, step);
            } else {
                return Change::None;
            }
        } else {
            return Change::None;
        }
        self.persist();
        Change::Partial
    }
}

impl App for View {
    fn draw(&self, canvas: &mut Canvas) {
        let width = canvas.bounds().w;
        match self.page {
            Page::Subjects => return self.draw_subjects(canvas, width),
            Page::SubjectEditor => return self.draw_subject_editor(canvas, width),
            Page::Week => return self.draw_log(canvas, width),
            Page::Timer => {}
        }
        header(canvas, "Focus", "Exit");
        self.draw_timer(canvas, width);
        canvas.button(
            primary_button(width, self.focus.timer),
            self.primary_label(),
            true,
            false,
        );
        if self.focus.timer != Timer::Ready {
            canvas.button(reset_button(width), "Reset", false, false);
        } else {
            self.draw_settings(canvas, width);
        }

        if self.focus.can_choose_subject() {
            let subject = self.focus.subject();
            let label = if subject.is_empty() {
                "Set subject".to_owned()
            } else {
                format!("Subject · {subject}")
            };
            canvas.button(subject_button(width), &label, false, false);
        }
        let today = self.day(self.now);
        let (count, total) = self.focus.totals(|when| self.day(when) == today);
        canvas.card(rect(width, 40, 1370, 1184, 194));
        canvas.text(rect(width, 76, 1400, 740, 52), "Today", CAPTION);
        let sessions = if count == 1 { "session" } else { "sessions" };
        let totals = format!("{count} focus {sessions}  ·  {} min", total / 60);
        canvas.text(rect(width, 76, 1460, 760, 65), &totals, BODY);
        canvas.button(week_button(width), "This week", false, false);
        if let Some(notice) = &self.notice {
            canvas.text(rect(width, 40, 1560, 1184, 60), notice, CAPTION.center());
        }
        if self.confirm_reset {
            canvas.popup(rect(width, 190, 580, 884, 375));
            canvas.text(
                rect(width, 238, 632, 788, 70),
                "Reset this timer?",
                TextStyle::new(50).bold(),
            );
            canvas.text(
                rect(width, 238, 720, 788, 64),
                "Your completed sessions will stay in Today.",
                CAPTION,
            );
            canvas.button(rect(width, 238, 808, 360, 100), "Cancel", false, false);
            canvas.button(reset_confirm_button(width), "Reset", true, false);
        }
    }

    fn input(&mut self, input: Input) -> Change {
        let prior = self.focus.timer;
        let clock_changed = !self.preview && self.at(wall_now());
        if self.page == Page::Timer
            && matches!(prior, Timer::Running { .. })
            && self.focus.timer == Timer::Finished
            && matches!(input, Input::Tap(_, _))
        {
            return Change::Partial;
        }
        match input {
            Input::Tap(x, y) => match self.tap(x, y) {
                Change::None if clock_changed => Change::Partial,
                change => change,
            },
            Input::PageBack if self.confirm_reset => {
                self.confirm_reset = false;
                Change::Full
            }
            Input::PageBack if self.page != Page::Timer => {
                self.page = if self.page == Page::SubjectEditor {
                    Page::Subjects
                } else {
                    Page::Timer
                };
                Change::Full
            }
            Input::PageForward if self.page == Page::Week => self.log_page(1),
            Input::PageBack => Change::Exit,
            _ => {
                if clock_changed {
                    Change::Partial
                } else {
                    Change::None
                }
            }
        }
    }

    fn deadline(&self) -> Option<Instant> {
        let now = wall_now();
        let seconds = self
            .focus
            .next_change(now)
            .unwrap_or(u32::MAX)
            .min(self.next_midnight(now));
        Some(Instant::now() + Duration::from_secs(u64::from(seconds)))
    }

    fn tick(&mut self, _: Instant) -> Change {
        if self.at(wall_now()) {
            Change::Partial
        } else {
            Change::None
        }
    }

    fn storage_returned(&mut self) -> io::Result<()> {
        if !self.preview {
            self.at(wall_now());
        }
        self.save()
    }

    fn save(&mut self) -> io::Result<()> {
        if let Some(path) = &self.data {
            primitive_app::save(path, &serde_json::to_vec(&self.focus)?)?;
        }
        Ok(())
    }
}

fn labelled(subject: &str) -> &str {
    if subject.is_empty() {
        "No subject"
    } else {
        subject
    }
}

fn log_row(canvas: &mut Canvas, width: i32, index: usize, label: &str, detail: &str) {
    let y = 390 + index as i32 * 146;
    canvas.card(rect(width, 40, y, 1184, 123));
    canvas.text(rect(width, 72, y + 14, 1120, 46), label, BODY);
    canvas.text(rect(width, 72, y + 65, 1120, 36), detail, CAPTION);
}

fn subject_row(width: i32, index: usize) -> Rect {
    rect(width, 40, 410 + index as i32 * 126, 1184, 104)
}

fn subject_button(width: i32) -> Rect {
    rect(width, 40, 1250, 1184, 100)
}

fn week_button(width: i32) -> Rect {
    rect(width, 854, 1420, 334, 100)
}

fn wall_now() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64
}

fn rect(width: i32, x: i32, y: i32, w: i32, h: i32) -> Rect {
    screen_rect(width, x, y, w, h)
}

fn primary_button(width: i32, timer: Timer) -> Rect {
    rect(
        width,
        40,
        635,
        if timer == Timer::Ready { 1184 } else { 570 },
        106,
    )
}

fn reset_button(width: i32) -> Rect {
    rect(width, 654, 635, 570, 106)
}

fn reset_confirm_button(width: i32) -> Rect {
    rect(width, 666, 808, 360, 100)
}

fn preset_button(width: i32, index: usize) -> Rect {
    rect(width, 40 + 400 * index as i32, 871, 384, 100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn old_sessions_remain_visible_and_week_totals_use_local_monday() {
        let saved = r#"{"preset":"Classic","phase":"Focus","timer":"Ready","custom_focus":25,"custom_break":5,"completions":[[200,1500]]}"#;
        let focus = serde_json::from_str::<Focus>(saved).unwrap().restored();
        let mut app = View::new(focus, None, DAY * 4, true);
        app.zone.clear();
        assert_eq!(app.focus.totals(|_| true), (1, 1500));
        assert_eq!(app.focus.sessions()[0].subject, "");
        assert!(app.subject_totals().is_empty());
        app.focus.choose_subject("Reading");
        app.focus.primary(app.now);
        app.focus
            .choose_subject("Attempt to rename a running session");
        app.at(app.now + 1500);
        assert_eq!(app.subject_totals(), vec![("Reading", 1500)]);
        assert_eq!(app.focus.sessions().last().unwrap().subject, "Reading");
        app.at(DAY * 11);
        assert!(app.subject_totals().is_empty());
    }

    #[test]
    fn a_timer_that_expired_offscreen_is_recorded_only_once() -> io::Result<()> {
        let now = wall_now();
        let stamp = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("primitive-focus-test-{stamp}.json"));
        let mut app = View::open(path.clone());
        app.focus.primary(now - 1600);
        app.save()?;

        let first = View::open(path.clone());
        assert_eq!(first.focus.timer, Timer::Finished);
        assert_eq!(first.focus.totals(|_| true), (1, 1500));
        let second = View::open(path.clone());
        assert_eq!(second.focus.totals(|_| true), (1, 1500));
        fs::remove_file(path)
    }
}
