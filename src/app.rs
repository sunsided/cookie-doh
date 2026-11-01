//! The game-toolkit glue: input, audio device, and presenting the framebuffer.

use game_toolkit::prelude::*;
use winit::event::{TouchPhase, WindowEvent};

use crate::gfx::modes::Converter;
use crate::gfx::{H, Screen, W};
use crate::logic::layout::{LANE_Y, LANES};
use crate::logic::{CookieKind, DohCause, Event, Phase, World};
use crate::platform::{self, HostCommand, HostEvent, Options};
use crate::render::cracktro::Cracktro;
use crate::render::fx::Fx;
use crate::render::title::{Title, TitleAction};
use crate::render::{GameView, Renderer};
use crate::sound::tracker::{Player, Song};
use crate::sound::{SAMPLE_RATE, sfx, songs};

pub const CANVAS_ID: &str = "cookie-doh";
const MUSIC_VOLUME: f32 = 0.32;
const SFX_VOLUME: f32 = 0.6;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
enum Mode {
    Cracktro,
    Title,
    Play,
    Over,
}

/// Where the framebuffer sits in the window.
#[derive(Clone, Copy, Debug)]
struct Viewport {
    x: f32,
    y: f32,
    scale: f32,
}

impl Viewport {
    fn fit(win_w: u32, win_h: u32) -> Self {
        let s = (win_w as f32 / W as f32).min(win_h as f32 / H as f32);
        // Integer scaling keeps every pixel the same size; below 1x there's no choice.
        let scale = if s >= 1.0 { s.floor() } else { s };
        Self {
            x: ((win_w as f32 - W as f32 * scale) / 2.0).floor(),
            y: ((win_h as f32 - H as f32 * scale) / 2.0).floor(),
            scale,
        }
    }

    fn to_fb(self, x: f32, y: f32) -> (i32, i32) {
        (
            ((x - self.x) / self.scale) as i32,
            ((y - self.y) / self.scale) as i32,
        )
    }
}

struct Music {
    handle: Option<StreamHandle>,
    volume: f32,
}

impl Music {
    fn play(&mut self, audio: &mut Option<Audio>, song: Song) {
        self.stop();
        let Some(audio) = audio else { return };
        let mut player = Player::new(song);
        let stream = move |out: &mut [f32]| player.fill(out);
        match audio.play_stream_with(
            stream,
            SAMPLE_RATE,
            PlayOptions::default().volume(self.volume).deferred(),
        ) {
            Ok(h) => self.handle = Some(h),
            Err(e) => log::warn!("music: {e}"),
        }
    }

    fn stop(&mut self) {
        if let Some(h) = self.handle.take() {
            h.stop();
        }
    }

    fn set_volume(&mut self, v: f32) {
        if (v - self.volume).abs() > 1e-3 {
            self.volume = v;
            if let Some(h) = &self.handle {
                h.set_volume(v);
            }
        }
    }
}

pub struct CookieDoh {
    screen: Screen,
    rgba: Vec<u8>,
    tex: TextureId,
    renderer: Renderer,
    viewport: Viewport,
    mode: Mode,
    title: Title,
    world: World,
    fx: Fx,
    over_t: f32,
    record: bool,
    hiscore: u32,
    sfx: sfx::Bank,
    music: Music,
    scanlines: bool,
    paused: bool,
    muted: bool,
    /// Last vertical stick direction, for edge-triggering the lift with an analog stick.
    stick: i32,
    /// Touches don't reach the toolkit's `Input`; they're queued from `raw_window_event`.
    taps: Vec<(f32, f32)>,
    t: f32,
    cracktro: Cracktro,
    /// Reduces frames to the selected historical [`crate::gfx::modes::GfxMode`].
    gfx: Converter,
    /// Seconds left on the "graphics mode" banner.
    toast_t: f32,
    /// Audio was running last frame (on the web it starts on the first key press, which must
    /// not also skip the cracktro).
    sound_was_on: bool,
    /// Exited to the host page (web): frozen and silent until the player comes back.
    dormant: bool,
    /// Audio is muted because the host has the game hidden.
    host_muted: bool,
    /// A cracktro key press was already spent on unlocking audio.
    unlock_press_spent: bool,
}

const CRACKTRO_VOLUME: f32 = 0.24;

impl CookieDoh {
    fn sound(&self, audio: &mut Option<Audio>, pcm: &[f32], volume: f32) {
        if let Some(a) = audio {
            // Before the first gesture on the web this fails; a missed blip is fine.
            let _ = a.play_samples_with(
                pcm,
                SAMPLE_RATE,
                PlayOptions::default().volume(volume * SFX_VOLUME),
            );
        }
    }

    fn start_game(&mut self, ctx: &mut Context) {
        platform::notify(HostEvent::Start);
        self.world = World::new(platform::seed());
        self.fx = Fx::new();
        self.mode = Mode::Play;
        self.paused = false;
        self.music.play(&mut ctx.audio, songs::game());
    }

    /// Start over as if freshly launched (the host asked for it). A run in progress keeps
    /// its hi-score; the player's mute and scanline choices stay.
    fn restart(&mut self, ctx: &mut Context, options: Options) {
        if self.mode == Mode::Play {
            self.save_record();
        }
        self.dormant = false;
        self.paused = false;
        self.stick = 0;
        self.taps.clear();
        self.fx = Fx::new();
        self.toast_t = 0.0;
        self.gfx.set_mode(options.gfx.unwrap_or_default());
        // The host page may have had a gesture since launch (the player typed a command).
        platform::probe_audio();
        // Audio unlocked by an earlier launch stays unlocked: the first cracktro press skips.
        self.sound_was_on = ctx.audio.as_ref().is_none_or(Audio::is_started);
        self.unlock_press_spent = false;
        if options.cracktro {
            self.goto_cracktro(ctx);
        } else {
            self.goto_title(ctx);
        }
    }

    fn goto_cracktro(&mut self, ctx: &mut Context) {
        self.mode = Mode::Cracktro;
        self.cracktro = Cracktro::default();
        self.music.volume = CRACKTRO_VOLUME;
        self.music.play(&mut ctx.audio, songs::cracktro());
    }

    fn update_cracktro(&mut self, ctx: &mut Context, dt: f32) {
        self.cracktro.update(dt);
        let sound_on = ctx.audio.as_ref().is_none_or(Audio::is_started);
        let i = &ctx.input;
        let pad = i
            .first_gamepad()
            .is_some_and(|p| p.button_pressed(Button::South) || p.button_pressed(Button::Start));
        let pressed = i.key_pressed(Key::Space)
            || i.key_pressed(Key::Enter)
            || i.key_pressed(Key::Escape)
            || pad
            || !self.pointer_presses(&ctx.input).is_empty();
        // On the web the first press only switches the sound on. Any later press skips, even
        // if the browser never let the audio start.
        if pressed {
            if (self.sound_was_on && sound_on) || self.unlock_press_spent {
                self.goto_title(ctx);
            } else {
                self.unlock_press_spent = true;
            }
        }
        self.sound_was_on = sound_on;
    }

    fn goto_title(&mut self, ctx: &mut Context) {
        self.mode = Mode::Title;
        self.title = Title::new();
        self.music.volume = MUSIC_VOLUME;
        self.music.play(&mut ctx.audio, songs::title());
    }

    /// Pointer presses this frame (mouse clicks and touches), in framebuffer pixels.
    fn pointer_presses(&mut self, input: &Input) -> Vec<(i32, i32)> {
        let mut out: Vec<(i32, i32)> = self
            .taps
            .drain(..)
            .map(|(x, y)| self.viewport.to_fb(x, y))
            .collect();
        if input.mouse_pressed(MouseButton::Left) {
            let (x, y) = input.mouse_pos();
            out.push(self.viewport.to_fb(x, y));
        }
        // Presses on the letterbox bars (or a stray press at the window origin) aren't taps.
        out.retain(|&(x, y)| (0..W).contains(&x) && (0..H).contains(&y));
        out
    }

    fn update_title(&mut self, ctx: &mut Context, dt: f32) {
        self.title.update(dt);
        let i = &ctx.input;
        let pad = i.first_gamepad();
        let pad_btn = |b: Button| pad.is_some_and(|p| p.button_pressed(b));
        let mut actions = Vec::new();
        if i.key_pressed(Key::ArrowLeft) || i.key_pressed(Key::KeyA) || pad_btn(Button::DPadLeft) {
            actions.push(self.title.select(0));
        }
        if i.key_pressed(Key::ArrowRight) || i.key_pressed(Key::KeyD) || pad_btn(Button::DPadRight)
        {
            actions.push(self.title.select(1));
        }
        if i.key_pressed(Key::Space)
            || i.key_pressed(Key::Enter)
            || pad_btn(Button::South)
            || pad_btn(Button::Start)
        {
            actions.push(self.title.confirm());
        }
        let presses = self.pointer_presses(&ctx.input);
        for (x, y) in presses {
            if let Some(b) = self.title.hit(x, y) {
                self.title.select(b);
                actions.push(self.title.confirm());
            }
        }
        if ctx.input.key_pressed(Key::Escape) {
            self.exit(ctx);
            return;
        }
        for a in actions {
            match a {
                TitleAction::Start => {
                    self.sound(&mut ctx.audio, &self.sfx.popup_ok, 1.0);
                    self.start_game(ctx);
                    return;
                }
                TitleAction::Nope => self.sound(&mut ctx.audio, &self.sfx.nope, 1.0),
                TitleAction::Moved => self.sound(&mut ctx.audio, &self.sfx.menu, 0.6),
                TitleAction::None => {}
            }
        }
    }

    fn update_play(&mut self, ctx: &mut Context, dt: f32) {
        let i = &ctx.input;
        let pad = i.first_gamepad();
        let pad_btn = |b: Button| pad.is_some_and(|p| p.button_pressed(b));
        // Esc pauses; Esc again (or Q / pad Select) while paused quits to the title.
        let quit = self.paused
            && (i.key_pressed(Key::Escape) || i.key_pressed(Key::KeyQ) || pad_btn(Button::Select));
        if quit {
            self.save_record();
            self.goto_title(ctx);
            return;
        }
        if i.key_pressed(Key::KeyP) || i.key_pressed(Key::Escape) || pad_btn(Button::Start) {
            self.paused = !self.paused;
            if self.paused {
                self.music.set_volume(0.0);
            }
        }
        if self.paused {
            return;
        }
        let w = &mut self.world;
        if i.key_pressed(Key::ArrowUp)
            || i.key_pressed(Key::KeyW)
            || i.key_pressed(Key::KeyK)
            || pad_btn(Button::DPadUp)
        {
            w.move_up();
        }
        if i.key_pressed(Key::ArrowDown)
            || i.key_pressed(Key::KeyS)
            || i.key_pressed(Key::KeyJ)
            || pad_btn(Button::DPadDown)
        {
            w.move_down();
        }
        for (k, lane) in [
            (Key::Digit1, 0),
            (Key::Digit2, 1),
            (Key::Digit3, 2),
            (Key::Digit4, 3),
        ] {
            if i.key_pressed(k) {
                w.move_to(lane);
            }
        }
        let stick = pad.map_or(0.0, |p| p.axis(Axis::LeftStickY));
        let dir = if stick > 0.5 {
            -1
        } else if stick < -0.5 {
            1
        } else {
            0
        };
        if dir != self.stick {
            match dir {
                -1 => w.move_up(),
                1 => w.move_down(),
                _ => {}
            }
            self.stick = dir;
        }
        if i.key_pressed(Key::Space)
            || i.key_pressed(Key::Enter)
            || pad_btn(Button::South)
            || pad_btn(Button::East)
        {
            w.accept_all();
        }
        let presses = self.pointer_presses(&ctx.input);
        let w = &mut self.world;
        for (x, y) in presses {
            let on_popup = w.popup.is_some_and(|p| {
                let top = LANE_Y[p.lane] as i32 - 22;
                (74..238).contains(&x) && (top..top + 70).contains(&y)
            });
            if on_popup {
                w.accept_all();
            } else {
                // Tap a belt to send the lift there.
                let lane = (0..LANES)
                    .min_by_key(|&l| (LANE_Y[l] as i32 - 8 - y).abs())
                    .unwrap_or(0);
                w.move_to(lane);
            }
        }

        self.world.update(dt);
        let x = self.world.cleaner.x;
        self.fx.vacuum(x - 8.0, x + 8.0);
        let events = std::mem::take(&mut self.world.events);
        for e in &events {
            self.fx.on_event(e, &self.world);
            self.react(ctx, e);
        }

        let duck = matches!(
            self.world.phase,
            Phase::Doh { .. } | Phase::Clear { .. } | Phase::Intro { .. }
        );
        self.music.set_volume(if duck { 0.0 } else { MUSIC_VOLUME });
    }

    /// Sounds and mode changes for simulation events.
    fn react(&mut self, ctx: &mut Context, e: &Event) {
        let audio = &mut ctx.audio;
        let b = &self.sfx;
        match *e {
            Event::Spit { .. } => self.sound(audio, &b.spit, 0.25),
            Event::Moved => self.sound(audio, &b.step, 0.35),
            Event::Accepted {
                kind: CookieKind::Plain,
                saved: false,
                ..
            } => self.sound(audio, &b.accept, 0.8),
            Event::Accepted {
                kind: CookieKind::Plain,
                saved: true,
                ..
            } => self.sound(audio, &b.save, 0.9),
            Event::Accepted {
                kind: CookieKind::Golden,
                ..
            } => self.sound(audio, &b.golden, 0.9),
            Event::JarFull { .. } => self.sound(audio, &b.jar_full, 0.8),
            Event::TippedOff { .. } => self.sound(audio, &b.tip, 0.7),
            Event::Crumbled { .. } => self.sound(audio, &b.crumble, 1.0),
            Event::Doh(cause) => {
                match cause {
                    DohCause::Bitten => self.sound(audio, &b.bite, 1.0),
                    DohCause::Bumped => self.sound(audio, &b.bonk, 1.0),
                    DohCause::Dropped(_) => {}
                }
                self.sound(audio, &b.doh, 1.0);
            }
            Event::SpiderComing { .. } => self.sound(audio, &b.spider, 0.7),
            Event::SpiderAte { .. } => self.sound(audio, &b.munch, 0.8),
            Event::CleanerComing => self.sound(audio, &b.cleaner, 0.7),
            Event::CleanerAte => self.sound(audio, &b.slurp, 0.8),
            Event::PopupOpened => self.sound(audio, &b.popup, 0.8),
            Event::PopupAccepted { .. } => self.sound(audio, &b.popup_ok, 0.9),
            Event::LevelClear { .. } => self.sound(audio, &b.level_clear, 0.9),
            Event::LevelStart { .. } => self.sound(audio, &b.level_start, 0.7),
            Event::GameOver => {
                self.sound(audio, &b.game_over, 0.9);
                self.music.stop();
                self.mode = Mode::Over;
                self.over_t = 0.0;
                self.record = self.save_record();
                platform::notify(HostEvent::GameOver {
                    score: self.world.score,
                    level: self.world.level,
                    hiscore: self.hiscore,
                    record: self.record,
                });
            }
        }
    }

    /// Leave the game. Natively that closes the window. On the web there is nothing to close:
    /// the page gets a `cookie-doh:exit` event and the game goes quiet until it's needed again.
    fn exit(&mut self, ctx: &mut Context) {
        platform::notify(HostEvent::Exit);
        if cfg!(target_arch = "wasm32") {
            self.dormant = true;
        } else {
            ctx.quit();
        }
    }

    /// On the web, start audio without waiting for a key in the game when the page already
    /// had a gesture and the browser lets it play. Otherwise the toolkit starts it on the
    /// first key press as usual.
    fn start_audio_early(&mut self, ctx: &mut Context) {
        if let Some(a) = &mut ctx.audio
            && !a.is_started()
            && platform::audio_allowed()
        {
            match a.start() {
                Ok(()) => self.sound_was_on = true,
                Err(e) => log::warn!("early audio start: {e}"),
            }
        }
    }

    /// Mute while the host has the game hidden, without touching the player's own mute.
    fn set_host_muted(&mut self, ctx: &mut Context, muted: bool) {
        if self.host_muted != muted {
            self.host_muted = muted;
            self.apply_volume(ctx);
        }
    }

    fn apply_volume(&self, ctx: &mut Context) {
        if let Some(a) = &mut ctx.audio {
            a.set_master_volume(if self.muted || self.host_muted {
                0.0
            } else {
                1.0
            });
        }
    }

    /// A deliberate "I'm back" from the player: a key the game uses, a click or a tap.
    fn wake_pressed(&mut self, ctx: &Context) -> bool {
        let i = &ctx.input;
        let keys = [
            Key::Space,
            Key::Enter,
            Key::ArrowUp,
            Key::ArrowDown,
            Key::ArrowLeft,
            Key::ArrowRight,
        ];
        let pad = i
            .first_gamepad()
            .is_some_and(|p| p.button_pressed(Button::South) || p.button_pressed(Button::Start));
        keys.iter().any(|&k| i.key_pressed(k))
            || pad
            || i.mouse_pressed(MouseButton::Left)
            || !self.taps.is_empty()
    }

    /// Keep the score if it beats the hi-score. Returns whether it did.
    fn save_record(&mut self) -> bool {
        let record = self.world.score > self.hiscore;
        if record {
            self.hiscore = self.world.score;
            platform::save_hiscore(self.hiscore);
        }
        record
    }

    fn update_over(&mut self, ctx: &mut Context, dt: f32) {
        self.over_t += dt;
        let i = &ctx.input;
        let pad_ok = i
            .first_gamepad()
            .is_some_and(|p| p.button_pressed(Button::South) || p.button_pressed(Button::Start));
        let go = i.key_pressed(Key::Space)
            || i.key_pressed(Key::Enter)
            || pad_ok
            || i.key_pressed(Key::Escape);
        let tapped = !self.pointer_presses(&ctx.input).is_empty();
        if self.over_t > 1.5 && (go || tapped) {
            self.goto_title(ctx);
        }
    }
}

impl Game for CookieDoh {
    fn init(ctx: &mut Context) -> Result<Self> {
        let options = platform::options();
        let screen = Screen::new();
        let rgba = vec![0; (W * H * 4) as usize];
        let tex = ctx
            .gfx
            .create_texture_rgba(W as u32, H as u32, &rgba, Some("framebuffer"));
        let (ww, wh) = ctx.gfx.size();
        let mut game = Self {
            screen,
            rgba,
            tex,
            renderer: Renderer::new(),
            viewport: Viewport::fit(ww, wh),
            mode: Mode::Title,
            title: Title::new(),
            world: World::new(1),
            fx: Fx::new(),
            over_t: 0.0,
            record: false,
            hiscore: platform::load_hiscore(),
            sfx: sfx::Bank::new(),
            music: Music {
                handle: None,
                volume: MUSIC_VOLUME,
            },
            scanlines: true,
            paused: false,
            muted: false,
            stick: 0,
            taps: Vec::new(),
            t: 0.0,
            cracktro: Cracktro::default(),
            gfx: Converter::new(options.gfx.unwrap_or_default()),
            toast_t: 0.0,
            sound_was_on: false,
            unlock_press_spent: false,
            dormant: false,
            host_muted: false,
        };
        platform::probe_audio();
        if options.cracktro {
            game.goto_cracktro(ctx);
        } else {
            game.goto_title(ctx);
        }
        Ok(game)
    }

    fn update(&mut self, ctx: &mut Context, dt: f32) {
        for cmd in platform::commands() {
            match cmd {
                HostCommand::Restart(options) => self.restart(ctx, options),
            }
        }
        self.start_audio_early(ctx);
        // Hidden by the host (exited, or the canvas was taken off the page): freeze and stay
        // silent. A run in progress waits behind the pause screen.
        if self.dormant || !platform::is_active() {
            let woke = self.dormant && platform::is_active() && self.wake_pressed(ctx);
            self.taps.clear();
            if woke {
                self.dormant = false;
                self.set_host_muted(ctx, false);
            } else {
                if self.mode == Mode::Play {
                    self.paused = true;
                }
                self.set_host_muted(ctx, true);
            }
            // The waking press only wakes.
            return;
        }
        self.set_host_muted(ctx, false);
        self.t += dt;
        if ctx.input.key_pressed(Key::KeyM) {
            self.muted = !self.muted;
            self.apply_volume(ctx);
        }
        if ctx.input.key_pressed(Key::KeyC) {
            self.scanlines = !self.scanlines;
        }
        if ctx.input.key_pressed(Key::KeyG) {
            self.gfx.set_mode(self.gfx.mode.next());
            self.toast_t = 2.0;
        }
        self.toast_t = (self.toast_t - dt).max(0.0);
        match self.mode {
            Mode::Cracktro => self.update_cracktro(ctx, dt),
            Mode::Title => self.update_title(ctx, dt),
            Mode::Play => self.update_play(ctx, dt),
            Mode::Over => self.update_over(ctx, dt),
        }
        if !self.paused {
            self.fx.update(dt);
        }
    }

    fn event(&mut self, _ctx: &mut Context, e: &GameEvent) {
        match *e {
            GameEvent::Resized { width, height } => self.viewport = Viewport::fit(width, height),
            GameEvent::FocusChanged(false) if self.mode == Mode::Play => self.paused = true,
            _ => {}
        }
    }

    fn raw_window_event(&mut self, _ctx: &mut Context, e: &WindowEvent) -> bool {
        if let WindowEvent::Touch(t) = e
            && t.phase == TouchPhase::Started
        {
            self.taps.push((t.location.x as f32, t.location.y as f32));
        }
        false
    }

    fn render(&mut self, ctx: &mut Context, frame: &mut Frame) {
        match self.mode {
            Mode::Cracktro => {
                let sound_on =
                    ctx.audio.as_ref().is_none_or(Audio::is_started) || self.unlock_press_spent;
                self.renderer
                    .draw_cracktro(&mut self.screen, &self.cracktro, sound_on);
            }
            Mode::Title => self
                .renderer
                .draw_title(&mut self.screen, &self.title, self.hiscore),
            Mode::Play | Mode::Over => {
                let view = GameView {
                    world: &self.world,
                    fx: &self.fx,
                    t: self.t,
                    paused: self.paused,
                };
                self.renderer.draw_game(&mut self.screen, &view);
                if self.mode == Mode::Over {
                    let w = &self.world;
                    self.renderer.draw_game_over(
                        &mut self.screen,
                        w.score,
                        w.level,
                        self.record,
                        self.over_t,
                    );
                }
            }
        }
        if self.dormant {
            self.screen
                .blend_rect(0, 0, W, H, crate::gfx::palette::NAVY, 0.7);
            crate::render::toast(&mut self.screen, "SEE YOU! PRESS SPACE TO COME BACK");
        } else if self.toast_t > 0.0 {
            crate::render::toast(&mut self.screen, self.gfx.mode.blurb());
        }
        if let Some((c, a)) = self.fx.flash {
            self.screen.tint(c, a);
        }
        self.gfx.apply(&mut self.screen);
        self.screen.to_rgba(&mut self.rgba, None);
        ctx.gfx.update_texture_rgba(self.tex, &self.rgba);

        let (ww, wh) = ctx.gfx.size();
        self.viewport = Viewport::fit(ww, wh);
        let vp = self.viewport;
        let (sx, sy) = self.fx.shake_offset(self.t);
        let (dw, dh) = (W as f32 * vp.scale, H as f32 * vp.scale);
        let mut p = frame.painter(&mut ctx.gfx);
        p.clear([0.0, 0.0, 0.0, 1.0]);
        p.sprite(
            self.tex,
            [vp.x + sx as f32 * vp.scale, vp.y + sy as f32 * vp.scale],
            [dw, dh],
        );
        if self.scanlines && vp.scale >= 3.0 {
            let thick = (vp.scale / 3.0).floor();
            for row in 0..H {
                let y = vp.y + (row as f32 + 1.0) * vp.scale - thick;
                p.rect_ex(
                    [vp.x, y],
                    [dw, thick],
                    [0.0, 0.0, 0.0, 0.28],
                    1,
                    BlendMode::Alpha,
                );
            }
        }
    }
}

pub fn config() -> AppConfig {
    AppConfig {
        title: "Cookie D'oh! - Accept all Cookies".into(),
        width: (W * 4) as u32,
        height: (H * 4) as u32,
        asset_root: asset_root!(),
        canvas_id: Some(CANVAS_ID.into()),
        // Everything is generated in code: no asset files to fetch, not even the manifest,
        // so the build works from any URL an embedding page puts it at.
        web_preload: Some(Vec::new()),
        random_seed: platform::seed(),
        ..Default::default()
    }
}
