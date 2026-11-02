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

    /// The game's canvas, remembered once found: it's the one the game renders into even if
    /// the host adds another with the same id, and a canvas taken off the page can't be found
    /// by id any more.
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
