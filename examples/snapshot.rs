//! Headless screenshots of the framebuffer, for checking art without opening a window:
//! `cargo run --example snapshot -- out_dir` writes title.png, play.png, doh.png, ... and the
//! audio as WAV files (doh.wav, music_game.wav, ...) to listen to without playing.

use cookie_doh::gfx::{Screen, png};
use cookie_doh::logic::{Phase, SpiderState, World};
use cookie_doh::render::fx::Fx;
use cookie_doh::render::title::Title;
use cookie_doh::render::{GameView, Renderer};

fn main() {
    let dir = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "snapshots".into());
    std::fs::create_dir_all(&dir).expect("create output dir");
    let r = Renderer::new();
    let mut s = Screen::new();
    let save = |s: &Screen, name: &str| {
        let path = format!("{dir}/{name}.png");
        std::fs::write(&path, png::encode(s, 3)).expect("write png");
        println!("{path}");
    };

    let mut title = Title::new();
    for _ in 0..78 {
        title.update(1.0 / 60.0);
    }
    r.draw_title(&mut s, &title, 12340);
    save(&s, "title");
    // The spider's visit, every ten seconds.
    for _ in 0..(6.8 * 60.0) as usize {
        title.update(1.0 / 60.0);
    }
    r.draw_title(&mut s, &title, 12340);
    save(&s, "title_spider");

    // Let a bot play a while so the belts fill up naturally.
    let mut w = World::new(42);
    w.hazards = false;
    let mut fx = Fx::new();
    let dt = 1.0 / 60.0;
    for _ in 0..(14.0 / dt) as usize {
        let next = w
            .cookies
            .iter()
            .filter_map(|c| w.arrival_in(c).map(|a| (a, c.lane)))
            .min_by(|a, b| a.0.total_cmp(&b.0));
        if let Some((_, lane)) = next {
            w.move_to(lane);
        }
        w.update(dt);
        for e in std::mem::take(&mut w.events) {
            fx.on_event(&e, &w);
        }
        fx.update(dt);
    }
    let shot = |s: &mut Screen, w: &World, fx: &Fx, name: &str| {
        r.draw_game(
            s,
            &GameView {
                world: w,
                fx,
                t: w.time,
                paused: false,
            },
        );
        save(s, name);
    };
    shot(&mut s, &w, &fx, "play");

    let mut w2 = w.clone();
    w2.spider = SpiderState::Down {
        lane: w2.player.nearest(),
        y: 30.0,
    };
    shot(&mut s, &w2, &fx, "spider_down");
    w2.spider = SpiderState::Hang { lane: 0, t: 1.0 };
    w2.popup = Some(cookie_doh::logic::Popup { lane: 1, age: 1.0 });
    shot(&mut s, &w2, &fx, "popup");

    let mut w3 = w.clone();
    w3.phase = Phase::Doh {
        t: 1.5,
        cause: cookie_doh::logic::DohCause::Dropped(0),
    };
    shot(&mut s, &w3, &fx, "doh");

    let mut w4 = World::new(1);
    w4.update(0.1);
    shot(&mut s, &w4, &Fx::new(), "intro");

    r.draw_game(
        &mut s,
        &GameView {
            world: &w3,
            fx: &fx,
            t: 2.0,
            paused: false,
        },
    );
    r.draw_game_over(&mut s, 12340, 3, true, 2.0);
    save(&s, "game_over");

    let bank = cookie_doh::sound::sfx::Bank::new();
    let music = |name: &str, song| {
        let mut p = cookie_doh::sound::tracker::Player::new(song);
        let mut pcm = vec![0.0; cookie_doh::sound::SAMPLE_RATE as usize * 16];
        p.fill(&mut pcm);
        wav(&format!("{dir}/music_{name}.wav"), &pcm);
    };
    music("title", cookie_doh::sound::songs::title());
    music("game", cookie_doh::sound::songs::game());
    for (name, pcm) in [
        ("doh", &bank.doh),
        ("accept", &bank.accept),
        ("golden", &bank.golden),
        ("crumble", &bank.crumble),
        ("bite", &bank.bite),
        ("level_clear", &bank.level_clear),
        ("game_over", &bank.game_over),
    ] {
        wav(&format!("{dir}/{name}.wav"), pcm);
    }
}

/// 16-bit mono WAV.
fn wav(path: &str, pcm: &[f32]) {
    let rate = cookie_doh::sound::SAMPLE_RATE;
    let data_len = (pcm.len() * 2) as u32;
    let mut out = Vec::with_capacity(44 + data_len as usize);
    out.extend_from_slice(b"RIFF");
    out.extend_from_slice(&(36 + data_len).to_le_bytes());
    out.extend_from_slice(b"WAVEfmt ");
    out.extend_from_slice(&16u32.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&rate.to_le_bytes());
    out.extend_from_slice(&(rate * 2).to_le_bytes());
    out.extend_from_slice(&2u16.to_le_bytes());
    out.extend_from_slice(&16u16.to_le_bytes());
    out.extend_from_slice(b"data");
    out.extend_from_slice(&data_len.to_le_bytes());
    for s in pcm {
        out.extend_from_slice(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes());
    }
    std::fs::write(path, out).expect("write wav");
    println!("{path}");
}
