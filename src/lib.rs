//! Cookie D'oh! - Accept all Cookies.
//!
//! The crate is split so everything except [`app`] is free of toolkit types and can be unit
//! tested and rendered headlessly (see `src/bin/snapshot.rs`):
//!
//! - [`logic`]: the game rules and simulation, deterministic and tested.
//! - [`gfx`]: a 320x200 CPU framebuffer, palette, bitmap fonts and hand-drawn sprites.
//! - [`render`]: draws a [`logic::World`] (plus cosmetic [`render::Fx`]) into the framebuffer.
//! - [`sound`]: every sound is synthesized in code - chiptune tracker, SFX, the "D'oh!" voice.
//! - [`app`]: the game-toolkit glue (window, input, audio device, presenting the framebuffer).

// Pixel-drawing calls take x, y, w, h plus colors; bundling them would only add noise.
#![allow(clippy::too_many_arguments)]

pub mod app;
pub mod gfx;
pub mod logic;
pub mod platform;
pub mod render;
pub mod sound;
