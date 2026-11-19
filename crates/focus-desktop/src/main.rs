use std::{
    fs,
    path::{Path, PathBuf},
};

use focus_core::{Phase, Timer};
use focus_render::View;
use primitive_app::{font_paths, frame, write_png, App, Change, HEIGHT, WIDTH};
use primitive_ui::{Canvas, Input};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output = PathBuf::from(
        std::env::args_os()
            .nth(1)
            .unwrap_or_else(|| "build/previews".into()),
    );
    fs::create_dir_all(&output)?;
    let fonts =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../doannt-kindle-launcher/assets/fonts");
    let mut canvas = Canvas::new(WIDTH, HEIGHT, &font_paths(&fonts))?;
    let mut app = View::preview(10_000);
    write_png(
        &output.join("focus.png"),
        WIDTH,
        HEIGHT,
        &frame(&app, &mut canvas, false),
    )?;
    assert_eq!(app.input(Input::Tap(1000.0, 925.0)), Change::Partial);
    write_png(
        &output.join("focus-custom.png"),
        WIDTH,
        HEIGHT,
        &frame(&app, &mut canvas, false),
    )?;
    assert_eq!(app.input(Input::Tap(450.0, 680.0)), Change::Partial);
    app.at(10_061);
    assert_eq!(app.focus.remaining(10_061), 1439);
    write_png(
        &output.join("focus-running-dark.png"),
        WIDTH,
        HEIGHT,
        &frame(&app, &mut canvas, true),
    )?;
    app.at(11_500);
    assert_eq!(app.focus.timer, Timer::Finished);
    assert_eq!(app.focus.totals(|_| true), (1, 1500));
    write_png(
        &output.join("focus-complete.png"),
        WIDTH,
        HEIGHT,
        &frame(&app, &mut canvas, false),
    )?;
    assert_eq!(app.input(Input::Tap(450.0, 680.0)), Change::Partial);
    assert_eq!(app.focus.phase, Phase::Break);
    Ok(())
}
