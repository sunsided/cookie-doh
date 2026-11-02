//! The few things that differ between native and the browser: hi-score storage, a seed,
//! launch options (command-line flags natively; the URL query or the canvas's `data-`
//! attributes on the web), and telling an embedding page what happened.

use crate::gfx::modes::GfxMode;

/// Launch options: `--cracktro --gfx=cga` natively, `?cracktro&gfx=cga` in the browser.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub cracktro: bool,
    pub gfx: Option<GfxMode>,
}

/// Things the embedding page may want to react to. On the web each becomes a bubbling
/// `cookie-doh:<name>` DOM event on the canvas (with the fields in `event.detail`), and a
/// `postMessage({ type: "cookie-doh:<name>", ... })` to the parent when running in an iframe.
/// Natively they are only logged.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostEvent {
    /// A game started (the player accepted the consent banner).
    Start,
    /// The player asked to leave (Esc on the title screen).
    Exit,
    GameOver {
        score: u32,
        level: u32,
        hiscore: u32,
        record: bool,
    },
}

impl HostEvent {
    pub fn name(self) -> &'static str {
        match self {
            HostEvent::Start => "cookie-doh:start",
            HostEvent::Exit => "cookie-doh:exit",
            HostEvent::GameOver { .. } => "cookie-doh:gameover",
        }
    }
}

/// Requests from the embedding page. On the web a `cookie-doh:restart` DOM event on the
/// canvas (or, in an iframe, a `postMessage({ type: "cookie-doh:restart" })` from the parent)
/// becomes [`HostCommand::Restart`]; `detail` (or the message) may carry `cracktro` and `gfx`,
/// which override the launch options. Natively there is no host and none arrive.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HostCommand {
    /// Start over as if freshly launched, but keep the running loop, GPU context and the
    /// already unlocked audio.
    Restart(Options),
}

impl HostCommand {
    pub const RESTART: &str = "cookie-doh:restart";
}

impl Options {
    /// Parse `key` / `key=value` pairs (flags without the leading dashes).
    pub fn parse<'a>(pairs: impl IntoIterator<Item = &'a str>) -> Self {
        let mut o = Options::default();
        for pair in pairs {
            let (key, value) = pair.split_once('=').unwrap_or((pair, ""));
            match key.to_ascii_lowercase().as_str() {
                "cracktro" | "intro" => o.cracktro = !matches!(value, "0" | "false" | "no"),
                "gfx" | "mode" => o.gfx = GfxMode::parse(value),
                _ => {}
            }
        }
        o
    }
}

#[cfg(not(target_arch = "wasm32"))]
mod imp {
    use std::path::PathBuf;

    fn hiscore_path() -> Option<PathBuf> {
        Some(dirs::data_dir()?.join("cookie-doh").join("hiscore"))
    }

    pub fn load_hiscore() -> u32 {
        hiscore_path()
            .and_then(|p| std::fs::read_to_string(p).ok())
            .and_then(|s| s.trim().parse().ok())
            .unwrap_or(0)
    }

    pub fn save_hiscore(score: u32) {
        let Some(p) = hiscore_path() else { return };
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        if let Err(e) = std::fs::write(&p, score.to_string()) {
            log::warn!("save hiscore to {}: {e}", p.display());
        }
    }

    pub fn notify(event: super::HostEvent) {
        log::info!("host event: {event:?}");
    }

    /// Natively the game always has the screen to itself.
    pub fn is_active() -> bool {
        true
    }

    pub fn options() -> super::Options {
        // `--gfx cga` and `--gfx=cga` both work.
        let args: Vec<String> = std::env::args().skip(1).collect();
        let mut pairs = Vec::new();
        let mut i = 0;
        while i < args.len() {
            let a = args[i].trim_start_matches('-');
            if !a.contains('=')
                && matches!(a, "gfx" | "mode")
                && let Some(v) = args.get(i + 1)
            {
                pairs.push(format!("{a}={v}"));
                i += 2;
                continue;
            }
            pairs.push(a.to_string());
            i += 1;
        }
        super::Options::parse(pairs.iter().map(String::as_str))
    }

    pub fn commands() -> Vec<super::HostCommand> {
        Vec::new()
    }

    /// Natively the window can't be taken away from under the game.
    pub fn watch_detach() {}

    /// Natively the audio device opens at startup; there is nothing to probe.
    pub fn probe_audio() {}

    pub fn audio_allowed() -> bool {
        false
    }

    pub fn seed() -> u64 {
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map_or(1, |d| d.as_nanos() as u64)
    }
}

#[cfg(target_arch = "wasm32")]
mod imp {
    const HISCORE_KEY: &str = "cookie-doh.hiscore";

    fn storage() -> Option<web_sys::Storage> {
        web_sys::window()?.local_storage().ok()?
    }

    pub fn load_hiscore() -> u32 {
        storage()
            .and_then(|s| s.get_item(HISCORE_KEY).ok()?)
            .and_then(|v| v.parse().ok())
            .unwrap_or(0)
    }

    pub fn save_hiscore(score: u32) {
        if let Some(s) = storage() {
            let _ = s.set_item(HISCORE_KEY, &score.to_string());
        }
    }

    use wasm_bindgen::JsValue;

    thread_local! {
        static CANVAS: std::cell::RefCell<Option<web_sys::Element>> =
            const { std::cell::RefCell::new(None) };
    }

    /// The game's canvas, remembered once found: by id, a canvas taken off the page can't be
    /// found any more, and it's the one the game renders into even if the host adds another.
    fn canvas() -> Option<web_sys::Element> {
        if let Some(c) = CANVAS.with_borrow(Clone::clone) {
            return Some(c);
        }
        let c = web_sys::window()?
            .document()?
            .get_element_by_id(crate::app::CANVAS_ID)?;
        CANVAS.set(Some(c.clone()));
        Some(c)
    }

    /// The URL query, then the canvas's `data-cracktro` / `data-gfx` attributes, so an
    /// embedding page can configure the game without touching its own URL.
    pub fn options() -> super::Options {
        let query = web_sys::window()
            .and_then(|w| w.location().search().ok())
            .unwrap_or_default();
        let mut pairs: Vec<String> = query
            .trim_start_matches('?')
            .split('&')
            .filter(|p| !p.is_empty())
            .map(str::to_owned)
            .collect();
        if let Some(c) = canvas() {
            for key in ["cracktro", "gfx"] {
                if let Some(v) = c.get_attribute(&format!("data-{key}")) {
                    pairs.push(if v.is_empty() {
                        key.to_owned()
                    } else {
                        format!("{key}={v}")
                    });
                }
            }
        }
        super::Options::parse(pairs.iter().map(String::as_str))
    }

    /// Paused while the canvas is detached from the page (the host component unmounted).
    pub fn is_active() -> bool {
        canvas().is_none_or(|c| c.is_connected())
    }

    pub fn notify(event: super::HostEvent) {
        let detail = js_sys::Object::new();
        let set = |k: &str, v: JsValue| {
            let _ = js_sys::Reflect::set(&detail, &JsValue::from_str(k), &v);
        };
        if let super::HostEvent::GameOver {
            score,
            level,
            hiscore,
            record,
        } = event
        {
            set("score", score.into());
            set("level", level.into());
            set("hiscore", hiscore.into());
            set("record", record.into());
        }
        if let Some(c) = canvas() {
            let init = web_sys::CustomEventInit::new();
            init.set_bubbles(true);
            init.set_detail(&detail);
            match web_sys::CustomEvent::new_with_event_init_dict(event.name(), &init) {
                Ok(e) => {
                    let _ = c.dispatch_event(&e);
                }
                Err(e) => log::warn!("host event {}: {e:?}", event.name()),
            }
        }
        // Inside an iframe, tell the parent page too.
        if let Some(w) = web_sys::window()
            && let Ok(Some(parent)) = w.parent()
            && parent != w
        {
            set("type", JsValue::from_str(event.name()));
            let _ = parent.post_message(&detail, "*");
        }
    }

    thread_local! {
        static COMMANDS: std::cell::RefCell<Vec<super::HostCommand>> =
            const { std::cell::RefCell::new(Vec::new()) };
        static LISTENING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }

    /// Launch options with `cracktro` / `gfx` from a restart request's payload on top.
    fn restart_options(payload: &JsValue) -> super::Options {
        let mut o = options();
        let get = |k: &str| js_sys::Reflect::get(payload, &JsValue::from_str(k)).ok();
        if let Some(c) = get("cracktro").and_then(|v| v.as_bool()) {
            o.cracktro = c;
        }
        if let Some(g) = get("gfx").and_then(|v| v.as_string()) {
            o.gfx = crate::gfx::modes::GfxMode::parse(&g);
        }
        o
    }

    fn queue_restart(payload: &JsValue) {
        let cmd = super::HostCommand::Restart(restart_options(payload));
        COMMANDS.with_borrow_mut(|q| q.push(cmd));
    }

    /// Listen for host requests (once), on the canvas and, in an iframe, from the parent.
    fn listen() {
        use wasm_bindgen::JsCast;
        use wasm_bindgen::closure::Closure;
        if LISTENING.replace(true) {
            return;
        }
        if let Some(c) = canvas() {
            let on_restart =
                Closure::<dyn FnMut(web_sys::CustomEvent)>::new(|e: web_sys::CustomEvent| {
                    queue_restart(&e.detail())
                });
            let _ = c.add_event_listener_with_callback(
                super::HostCommand::RESTART,
                on_restart.as_ref().unchecked_ref(),
            );
            on_restart.forget();
        }
        if let Some(w) = web_sys::window()
            && let Ok(Some(parent)) = w.parent()
            && parent != w
        {
            let on_message = Closure::<dyn FnMut(web_sys::MessageEvent)>::new(
                move |e: web_sys::MessageEvent| {
                    let from_parent = e.source().is_some_and(|s| js_sys::Object::is(&s, &parent));
                    let data = e.data();
                    let kind = js_sys::Reflect::get(&data, &JsValue::from_str("type"))
                        .ok()
                        .and_then(|t| t.as_string());
                    if from_parent && kind.as_deref() == Some(super::HostCommand::RESTART) {
                        queue_restart(&data);
                    }
                },
            );
            let _ =
                w.add_event_listener_with_callback("message", on_message.as_ref().unchecked_ref());
            on_message.forget();
        }
    }

    thread_local! {
        /// Every `AudioContext` resumed after [`watch_detach`]: the toolkit's (kira's) among them.
        static CONTEXTS: std::cell::RefCell<Vec<web_sys::AudioContext>> =
            const { std::cell::RefCell::new(Vec::new()) };
        /// Contexts [`watch_detach`] suspended because the canvas left the page.
        static SUSPENDED: std::cell::RefCell<Vec<web_sys::AudioContext>> =
            const { std::cell::RefCell::new(Vec::new()) };
        static WATCHING: std::cell::Cell<bool> = const { std::cell::Cell::new(false) };
    }

    /// Go silent the moment the canvas is taken off the page, and resume when it's back.
    ///
    /// [`is_active`] alone can't do it: the game checks it in `update`, and a canvas outside
    /// the document gets no more frames, so the music would play on until the next run. A
    /// `MutationObserver` fires without a frame and suspends the audio contexts directly.
    ///
    /// The toolkit doesn't hand out kira's context, and wrapping the `AudioContext`
    /// constructor wouldn't see it either: wasm-bindgen's glue grabs the constructor when the
    /// module loads. Every audio backend `resume()`s its context to start playing, though, and
    /// methods are looked up on the prototype at call time, so a `Proxy` around
    /// `AudioContext.prototype.resume` remembers each context that plays. Call this before
    /// audio starts.
    pub fn watch_detach() {
        use wasm_bindgen::JsCast;
        use wasm_bindgen::closure::Closure;
        if WATCHING.replace(true) {
            return;
        }
        let Some(window) = web_sys::window() else {
            return;
        };
        let get = |o: &JsValue, k: &str| js_sys::Reflect::get(o, &JsValue::from_str(k)).ok();
        let proto = get(&window, "AudioContext").and_then(|c| get(&c, "prototype"));
        let resume = proto.as_ref().and_then(|p| get(p, "resume"));
        if let (Some(proto), Some(resume)) = (proto, resume) {
            let apply =
                Closure::<dyn FnMut(js_sys::Function, JsValue, js_sys::Array) -> JsValue>::new(
                    |target: js_sys::Function, this: JsValue, args: js_sys::Array| -> JsValue {
                        if let Some(c) = this.dyn_ref::<web_sys::AudioContext>() {
                            CONTEXTS.with_borrow_mut(|cs| {
                                if !cs.iter().any(|known| known == c) {
                                    cs.push(c.clone());
                                }
                            });
                        }
                        match js_sys::Reflect::apply(&target, &this, &args) {
                            Ok(promise) => promise,
                            // Let the caller see the browser's own error.
                            Err(e) => wasm_bindgen::throw_val(e),
                        }
                    },
                );
            let handler = js_sys::Object::new();
            let _ = js_sys::Reflect::set(&handler, &JsValue::from_str("apply"), apply.as_ref());
            apply.forget();
            let proxy = js_sys::Proxy::new(&resume, &handler);
            if js_sys::Reflect::set(&proto, &JsValue::from_str("resume"), &proxy).is_err() {
                log::warn!("can't watch AudioContext; audio plays on when the canvas is removed");
            }
        }
        let Some(document) = window.document() else {
            return;
        };
        let on_change = Closure::<dyn FnMut()>::new(on_dom_change);
        match web_sys::MutationObserver::new(on_change.as_ref().unchecked_ref()) {
            Ok(observer) => {
                let init = web_sys::MutationObserverInit::new();
                init.set_child_list(true);
                init.set_subtree(true);
                if let Err(e) = observer.observe_with_options(&document, &init) {
                    log::warn!("watching the canvas: {e:?}");
                }
                on_change.forget();
            }
            Err(e) => log::warn!("watching the canvas: {e:?}"),
        }
    }

    /// Suspend running contexts when the canvas is gone, resume the ones we suspended when
    /// it's back. The game's own mute (`set_host_muted`) takes over again on the next frame.
    fn on_dom_change() {
        use web_sys::AudioContextState::{Closed, Running};
        let suspended = SUSPENDED.with_borrow(|s| !s.is_empty());
        if is_active() {
            if suspended {
                for c in SUSPENDED.take() {
                    let _ = c.resume();
                }
            }
        } else if !suspended {
            CONTEXTS.with_borrow_mut(|cs| {
                cs.retain(|c| c.state() != Closed);
                let running: Vec<_> = cs
                    .iter()
                    .filter(|c| c.state() == Running)
                    .cloned()
                    .collect();
                for c in &running {
                    let _ = c.suspend();
                }
                SUSPENDED.set(running);
            });
        }
    }

    thread_local! {
        /// A throwaway `AudioContext` and when it was made, watched by [`audio_allowed`].
        static PROBE: std::cell::RefCell<Option<(web_sys::AudioContext, f64)>> =
            const { std::cell::RefCell::new(None) };
    }

    /// How long a probe may stay suspended before we leave audio to the first key press.
    /// A fresh context reports `suspended` at first in Firefox and WebKit and turns
    /// `running` about 300 ms later (Chromium is `running` at once), so an early `suspended`
    /// is no verdict.
    const PROBE_MS: f64 = 1500.0;

    /// `navigator.userActivation.hasBeenActive`: the player already interacted with this
    /// document (e.g. typed into the host page). `false` where the API is missing.
    fn has_been_active() -> bool {
        let Some(nav) = web_sys::window().map(|w| w.navigator()) else {
            return false;
        };
        js_sys::Reflect::get(&nav, &JsValue::from_str("userActivation"))
            .ok()
            .filter(|a| a.is_object())
            .and_then(|a| js_sys::Reflect::get(&a, &JsValue::from_str("hasBeenActive")).ok())
            .and_then(|v| v.as_bool())
            .unwrap_or(false)
    }

    /// If the document already has user activation, open a probe `AudioContext` to find out
    /// whether the browser lets audio run without another gesture.
    pub fn probe_audio() {
        if !has_been_active() || PROBE.with_borrow(Option::is_some) {
            return;
        }
        match web_sys::AudioContext::new() {
            Ok(c) => PROBE.set(Some((c, js_sys::Date::now()))),
            Err(e) => log::debug!("audio probe: {e:?}"),
        }
    }

    /// The probe is running, so starting the game's audio now will be audible. Only ever
    /// `true` once; a probe still suspended after [`PROBE_MS`] is dropped (Safari wants a
    /// gesture of its own), and the toolkit then starts audio on the first key press.
    pub fn audio_allowed() -> bool {
        PROBE.with_borrow_mut(|p| {
            let Some((c, t0)) = p else { return false };
            let running = c.state() == web_sys::AudioContextState::Running;
            if running || js_sys::Date::now() - *t0 > PROBE_MS {
                let _ = c.close();
                *p = None;
            }
            running
        })
    }

    /// Host requests since the last call.
    pub fn commands() -> Vec<super::HostCommand> {
        listen();
        COMMANDS.with_borrow_mut(std::mem::take)
    }

    pub fn seed() -> u64 {
        (js_sys::Date::now() * 1000.0) as u64 ^ (js_sys::Math::random() * 1e15) as u64
    }
}

pub use imp::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn options_parse() {
        let o = Options::parse(["cracktro", "gfx=c64"]);
        assert!(o.cracktro);
        assert_eq!(o.gfx, Some(GfxMode::C64));
        assert_eq!(Options::parse(["cracktro=0"]), Options::default());
        assert_eq!(Options::parse(["gfx=nope", "foo=bar"]), Options::default());
    }
}
