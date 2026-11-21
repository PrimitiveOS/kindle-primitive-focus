<div align="center">

# Focus

**Focus timers and a study log**

[![Release](https://img.shields.io/github/v/release/PrimitiveOS/kindle-primitive-focus?label=release)](https://github.com/PrimitiveOS/kindle-primitive-focus/releases) [![Licence: PolyForm-Noncommercial-1.0.0](https://img.shields.io/badge/licence-PolyForm--Noncommercial--1.0.0-blue)](LICENSE) [![Store downloads](https://img.shields.io/badge/Store%20downloads-51-brightgreen)](https://primitiveos.github.io/kindle-primitive-catalog/v1/index.json)

[**Download**](https://github.com/PrimitiveOS/kindle-primitive-focus/releases/latest) · [**Install guide**](INSTALL.md) · [**Changelog**](CHANGELOG.md)

</div>

---

Choose a 25/5 or 50/10 focus and break preset or set your own timer. Label sessions by subject, then see weekly time and recent sessions in the study log.

## Quick start

After setting up PrimitiveOS, open **Store** from Primitive Launcher. Choose **Focus** under **Apps**,
tap **Get**, and open it from the Launcher grid. The [install guide](INSTALL.md) also covers the standalone USB package.

## Screenshots

<table>
  <tr>
    <td align="center"><img src="docs/images/focus-custom.png" alt="Focus and break presets with custom length controls" width="260"><br><sub>Presets and custom lengths</sub></td>
    <td align="center"><img src="docs/images/focus-running-dark.png" alt="Running focus countdown with Pause and Reset in dark mode" width="260"><br><sub>A running timer in dark mode</sub></td>
    <td align="center"><img src="docs/images/focus-complete.png" alt="Focus complete with Start break and a recorded 25-minute session" width="260"><br><sub>Session complete</sub></td>
  </tr>
  <tr>
    <td align="center"><img src="docs/images/focus-study-log.png" alt="Study log showing focus time grouped by subject" width="260"></td>
    <td align="center"><img src="docs/images/focus-sessions-dark.png" alt="Recent focus sessions in dark mode" width="260"></td>
  </tr>
</table>

<sub>Desktop simulation at 1264 × 1680 with sample timers.</sub>

## Highlights

- Use the 25/5 or 50/10 minute presets, or set separate focus and break lengths.
- Start, pause, and reset the timer. Choose **Set subject** before starting focus and reuse recent subject labels.
- Open **This week** for Monday-to-Sunday totals by subject or a list of completed sessions. Breaks stay out of the study log.
- Session state is saved on sleep and exit. The Kindle driver refreshes the idle timer during a running session.

## Develop locally

Keep `doannt-kindle-tools` and `doannt-kindle-launcher` next to this checkout. The desktop command writes sample PNGs
to `build/previews`. The local `app.json` contains draft listing copy.

```sh
cargo test --offline --locked --workspace
cargo run --offline --locked -p focus-desktop -- build/previews
```

## Status

| Device | Status |
|---|---|
| Kindle Oasis 3 (1264 × 1680, firmware 5.18.2) | Tested on hardware |

**Current version: 0.1.2.** Changes are in [CHANGELOG.md](CHANGELOG.md).

## Downloads

**51 downloads** from Store, as of 2026-11-21. Store shows the same count on the app page.
The count is GitHub downloads of this app's release packages, from the [Store catalog](https://primitiveos.github.io/kindle-primitive-catalog/v1/index.json).

## Documentation

[Install guide](INSTALL.md) · [Changelog](CHANGELOG.md) · [Device notes](docs/device-notes.md)

## Licence

Copyright (C) 2026 Doan Nguyen.

App code is licensed under PolyForm Noncommercial 1.0.0. [Code license](LICENSE)

Original documentation and screenshots are licensed under Creative Commons Attribution-NonCommercial-ShareAlike 4.0
International (CC BY-NC-SA 4.0). [Documentation and image license](LICENSE-DOCS)
