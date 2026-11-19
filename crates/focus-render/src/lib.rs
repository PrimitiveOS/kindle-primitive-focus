use std::fs;
use std::io;
use std::path::PathBuf;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

use focus_core::{Focus, Phase, Preset, Timer};
use primitive_app::{App, Change};
use primitive_ui::{
    back_button, header, progress, screen_rect, Canvas, Input, Rect, TextStyle, BODY, CAPTION,
};

const DAY: i64 = 86_400;
const PRESETS: [(Preset, &str); 3] = [
    (Preset::Classic, "25 / 5"),
    (Preset::Long, "50 / 10"),
    (Preset::Custom, "Custom"),
];

pub struct View {
    pub focus: Focus,
    data: Option<PathBuf>,
    zone: Vec<u8>,
    now: i64,
    preview: bool,
    confirm_reset: bool,
    notice: Option<String>,
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
        canvas.text(rect(width, 72, 234, 1120, 54), phase, BODY.center());
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

    fn tap(&mut self, x: f32, y: f32) -> Change {
        let width = primitive_app::WIDTH as i32;
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
        header(canvas, "Focus", "Done");
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

        let today = self.day(self.now);
        let (count, total) = self.focus.totals(|when| self.day(when) == today);
        canvas.card(rect(width, 40, 1300, 1184, 223));
        canvas.text(rect(width, 76, 1340, 1080, 52), "Today", CAPTION);
        let sessions = if count == 1 { "session" } else { "sessions" };
        let totals = format!("{count} focus {sessions}  ·  {} min", total / 60);
        canvas.text(rect(width, 76, 1400, 1080, 65), &totals, BODY);
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
        if matches!(prior, Timer::Running { .. })
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

    fn save(&mut self) -> io::Result<()> {
        if let Some(path) = &self.data {
            primitive_app::save(path, &serde_json::to_vec(&self.focus)?)?;
        }
        Ok(())
    }
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
