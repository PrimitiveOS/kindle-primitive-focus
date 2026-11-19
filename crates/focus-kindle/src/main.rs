use std::{
    io,
    path::PathBuf,
    time::{Duration, Instant},
};

use focus_core::Timer;
use focus_render::View;
use primitive_app::{App, Change};
use primitive_ui::{Canvas, Input};

const SCREEN_REFRESH_INTERVAL: Duration = Duration::from_secs(4 * 60);

#[derive(Default)]
struct ScreenActivity {
    next: Option<Instant>,
    asleep: bool,
}

impl ScreenActivity {
    fn refresh_due(&mut self, running: bool, now: Instant) -> bool {
        if self.asleep || !running {
            self.next = None;
            return false;
        }
        if self.next.is_some_and(|next| now < next) {
            return false;
        }
        self.next = Some(now + SCREEN_REFRESH_INTERVAL);
        true
    }
}

struct KindleFocus {
    view: View,
    screen: ScreenActivity,
}

impl KindleFocus {
    fn open(path: PathBuf) -> Self {
        let view = View::open(path);
        let screen = ScreenActivity {
            next: matches!(view.focus.timer, Timer::Running { .. }).then(Instant::now),
            ..ScreenActivity::default()
        };
        Self { view, screen }
    }

    fn maintain_screen(&mut self, now: Instant) {
        if self
            .screen
            .refresh_due(matches!(self.view.focus.timer, Timer::Running { .. }), now)
        {
            kl_kindle::device::refresh_screen_timeout();
        }
    }
}

impl App for KindleFocus {
    fn draw(&self, canvas: &mut Canvas) {
        self.view.draw(canvas);
    }

    fn input(&mut self, input: Input) -> Change {
        match input {
            Input::Sleep => self.screen.asleep = true,
            Input::Wake => self.screen.asleep = false,
            _ => {}
        }
        let change = self.view.input(input);
        if change != Change::Exit {
            self.maintain_screen(Instant::now());
        }
        change
    }

    fn deadline(&self) -> Option<Instant> {
        self.view
            .deadline()
            .into_iter()
            .chain(self.screen.next)
            .min()
    }

    fn tick(&mut self, now: Instant) -> Change {
        let change = self.view.tick(now);
        self.maintain_screen(now);
        change
    }

    fn save(&mut self) -> io::Result<()> {
        self.view.save()
    }
}

fn main() -> Result<(), String> {
    let directory = std::env::var_os("PRIMITIVE_APP_DIR")
        .map_or_else(|| PathBuf::from("/mnt/us/focus"), PathBuf::from);
    primitive_app::run(KindleFocus::open(directory.join("focus.json")), &directory)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pulse_is_rate_limited_and_stops_on_pause_or_explicit_sleep() {
        let now = Instant::now();
        let mut screen = ScreenActivity::default();
        assert!(!screen.refresh_due(false, now));
        assert!(screen.refresh_due(true, now));
        let later = now + Duration::from_secs(60);
        assert!(!screen.refresh_due(true, later));
        assert!(screen.refresh_due(true, now + SCREEN_REFRESH_INTERVAL));
        screen.asleep = true;
        assert!(!screen.refresh_due(true, later));
        assert_eq!(screen.next, None);
        screen.asleep = false;
        assert!(screen.refresh_due(true, later));
        assert!(!screen.refresh_due(false, later));
        assert_eq!(screen.next, None);
    }
}
