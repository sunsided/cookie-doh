# Cookie D'oh!

### *Accept all Cookies*

A silly arcade game in the style of the C64 / Amiga / early-286 era (think *Tapper* meets
*Egg*, dressed like *Night Shift*). Built on [game-toolkit](https://github.com/sunsided/game-toolkit).

It's the night shift at the Internet. Websites hang off the mainframe and spit cookies onto
four conveyor belts. You ride the lift at the end of the line with a jar, and **every cookie
must be accepted**. Drop one and it crumbles on the floor: *"D'oh!"* (with a sad trombone).

- **Up / Down** (W/S, J/K, 1-4, D-pad, or tap a belt): ride the lift.
- A cookie tipping off the belt drops onto the end of the belt below and smashes there -
  unless you're parked exactly one lane down: that dive-save pays double.
- **Space** (Enter, pad A): *Accept all* - dismisses the cookie-consent popups that cover
  the belts from level 2 on.
- **The Web Crawler** drops down to the jar spot of a random lane (watch for the red `!`).
  Don't be standing there when it lands. Cookies arriving at its lane get eaten - no penalty.
- **The Cache Cleaner** patrols the floor clearing crumbs. Now and then it beeps and heads for
  the dock under the lift (red `!` on the bottom lane) and bumps whoever is parked there.
  Camp at the bottom and it comes three times as often. While docked, it clears cookies
  that drop off the bottom belt - no penalty.
- **Golden cookies** are worth 10x. Ten cookies fill the jar for a bonus.
- Three D'ohs and your session expires.
- **P** / Esc pause; **Esc** again while paused quits to the title (Esc on the title quits
  the game). **G** cycle graphics modes, **M** mute, **C** toggle scanlines.

## Screenshots

| | |
|---|---|
| ![Title screen](docs/screenshots/title.png) | ![Night shift at the Internet](docs/screenshots/play.png) |
| ![Consent popup](docs/screenshots/popup.png) | ![D'oh!](docs/screenshots/doh.png) |
| ![The Cache Cleaner docked under the lift](docs/screenshots/cleaner_docked.png) | ![Session expired](docs/screenshots/game_over.png) |

## Silly extras

- **Cracktro:** start with `cargo run -- --cracktro`, or open the web build with
  `?cracktro`. *Cookies & Crackers* proudly present... with copper bars, a parallax
  starfield, a wobbling logo, cookie bobs and a sine scroller. Space skips (in the browser
  the first key press only switches the sound on).
- **Graphics modes:** press **G** in-game, or start with `--gfx=ega` / `?gfx=ega`. Every
  frame is reduced to real hardware palettes with ordered dithering:

  | Mode | Looks like |
  |------|-----------|
  | `vga` | the game as drawn (default) |
  | `ega` | IBM EGA, 16 fixed colors |
  | `c64` | Commodore 64 multicolor: 16 colors, double-wide pixels, hi-res status bar |
  | `cga` | IBM CGA palette 1: black, cyan, magenta, white. Sorry. |
  | `hercules` | 1-bit amber monochrome |

  Options combine: `?cracktro&gfx=c64`.

![The Cookies & Crackers cracktro](docs/screenshots/cracktro.png)

| | |
|---|---|
| ![EGA](docs/screenshots/play_ega.png) | ![C64](docs/screenshots/play_c64.png) |
| ![CGA](docs/screenshots/play_cga.png) | ![Hercules](docs/screenshots/play_hercules.png) |

Shortcuts: `task run:cracked` (alias `intro`), `task run:ega` / `run:c64` / `run:cga` /
`run:hercules`, `task run:retro` (cracktro, then C64), or `task run GFX=cga`.

## Run

```sh
cargo run                  # or: task run
```

## Run in the browser

```sh
task setup                       # wasm target + trunk
task web                         # http://localhost:8080, rebuilds on save
PROFILE=release task web:build   # dist/ - zip it for itch.io
```

Sound starts after the first key press or click (browser rule).

## Embedding in a website

`task web:export DEST=../my-site/static/cookie-doh` copies a release build
(`cookie-doh.js` + `cookie-doh_bg.wasm`) into your site. It fetches no other files, so it
works from any path. For SvelteKit, drop in [`embed/CookieDoh.svelte`](embed/CookieDoh.svelte):

```svelte
<CookieDoh
  autofocus
  gfx="c64"
  onexit={() => goto('/games')}
  ongameover={({ score, level, record }) => saveScore(score)}
/>
```

Without Svelte, create `<canvas id="cookie-doh" tabindex="0">` yourself (optionally with
`data-cracktro` and `data-gfx="ega"`), `import('/path/cookie-doh.js')` and call its default
export. The game reports to the page with bubbling DOM events on the canvas, and with
`postMessage` to the parent when it runs in an iframe:

| Event | When | `detail` |
|---|---|---|
| `cookie-doh:start` | a game starts | - |
| `cookie-doh:exit` | Esc on the title screen | - |
| `cookie-doh:gameover` | the session expired | `{ score, level, hiscore, record }` |

On the web, exiting doesn't stop anything: the game goes quiet and frozen until the player
presses Space (or clicks) again, and it pauses itself while its canvas is off the page. What
"exit" means is up to the host - navigate away, close an overlay, show a leaderboard.

## How it's made

There are no asset files. Everything is drawn and synthesized in code:

- **Graphics** render into a 320x200 CPU framebuffer that is uploaded as one texture and
  scaled by whole pixels (plus optional scanlines). Sprites are ASCII art in
  `src/render/art.rs` against a shared ~40-color palette (`src/gfx/palette.rs`); fonts are
  hand-made 5x7 and 3x5 bitmaps.
- **Music** is a tiny 4-channel tracker (pulse / pulse / triangle / noise drums) playing
  songs written in text notation (`src/sound/songs.rs`) on the audio thread.
- **"D'oh!"** is a formant synthesizer (`src/sound/voice.rs`): a glottal pulse through
  three cascaded resonators gliding from "d" to "oh", crushed to 6 bits at 8 kHz.

`task snapshot` renders screenshots of every screen and WAVs of the audio into
`snapshots/` without opening a window.

## Layout

```
src/logic(.rs)  the rules, pure and unit-tested (tuning.rs holds every difficulty knob)
src/gfx(.rs)    framebuffer, palette, fonts, sprite parser, a tiny PNG encoder
src/render(.rs) scene, sprites, effects, title and game-over screens
src/sound(.rs)  synth, tracker, songs, SFX, the voice
src/app.rs      game-toolkit glue: input, audio device, presenting the framebuffer
src/platform.rs hi-score storage, launch options, events for an embedding page
embed/          CookieDoh.svelte, a SvelteKit wrapper for the web build
examples/       snapshot: headless screenshots + WAV export
```

## License

Copyright &copy; 2026 Markus Mayer.

Cookie D'oh! is licensed under the [European Union Public Licence v1.2](LICENSE) (EUPL-1.2),
like [game-toolkit](https://github.com/sunsided/game-toolkit). All graphics, music, sound
effects and fonts are generated by the code in this repository and fall under the same
license.

The game is built with open-source crates under their own licenses (mostly MIT and
Apache-2.0, plus MPL-2.0, BSD, ISC, Zlib and others). Their notices are collected in
[`THIRD_PARTY_LICENSES.html`](THIRD_PARTY_LICENSES.html), which every web build ships next to
the game (`task licenses` regenerates it after dependency changes).
