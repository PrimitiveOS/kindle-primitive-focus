<div align="center">

# Focus

**A focus and break timer for Kindle e-readers.**

[Install guide](INSTALL.md) · [Device notes](docs/device-notes.md) · [Changelog](CHANGELOG.md)

</div>

---

Pick a focus session and a break, then check the day's completed sessions and focus time. The countdown changes at
minute boundaries to limit e-ink refreshes.

## Screenshots

<table>
  <tr>
    <td align="center"><img src="docs/images/focus-custom.png" alt="Focus and break presets with custom length controls" width="260"><br><sub>Presets and custom lengths</sub></td>
    <td align="center"><img src="docs/images/focus-running-dark.png" alt="Running focus countdown with Pause and Reset in dark mode" width="260"><br><sub>A running timer in dark mode</sub></td>
    <td align="center"><img src="docs/images/focus-complete.png" alt="Focus complete with Start break and a recorded 25-minute session" width="260"><br><sub>Session complete</sub></td>
  </tr>
</table>

<sub>Desktop simulation at 1264 × 1680 with sample timers.</sub>

## Highlights

- Use the 25/5 or 50/10 minute presets, or set separate focus and break lengths.
- Start, pause, and reset the timer. Completed sessions count toward today's total.
- Session state is saved on sleep and exit. The Kindle driver refreshes the idle timer during a running session.

## Quick start

Copy the `focus` and `documents` folders from the local ARMHF package to your Kindle over USB, then eject and open
**Focus** from Primitive Launcher. The app uses the fonts installed by Launcher. [Install guide](INSTALL.md)

## Status

Desktop simulation and local ARMHF package checks passed on the Mac.
On an ARMHF Kindle Oasis 3 running firmware 5.18.2, Focus displayed the custom timer controls, restored the
original idle preset, and returned to Launcher. Battery-powered idle, sleep and wake, and rotation checks are
pending. [Device notes](docs/device-notes.md)

## Develop locally

Keep `doannt-kindle-tools` and `doannt-kindle-launcher` next to this checkout. The desktop command writes sample PNGs
to `build/previews`. The local `app.json` contains draft listing copy.

```sh
cargo test --offline --locked --workspace
cargo run --offline --locked -p focus-desktop -- build/previews
```

## Licence

Copyright (C) 2026 Doan Nguyen.

App code is licensed under PolyForm Noncommercial 1.0.0. [Code license](LICENSE)

Original documentation and screenshots are licensed under Creative Commons Attribution-NonCommercial-ShareAlike 4.0
International (CC BY-NC-SA 4.0). [Documentation and image license](LICENSE-DOCS)
