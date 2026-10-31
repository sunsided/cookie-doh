//! The few things that differ between native and the browser: hi-score storage, a seed, and
//! launch options (command-line flags natively, the URL query on the web).

use crate::gfx::modes::GfxMode;

/// Launch options: `--cracktro --gfx=cga` natively, `?cracktro&gfx=cga` in the browser.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Options {
    pub cracktro: bool,
    pub gfx: Option<GfxMode>,
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

    pub fn options() -> super::Options {
        let query = web_sys::window()
            .and_then(|w| w.location().search().ok())
            .unwrap_or_default();
        super::Options::parse(
            query
                .trim_start_matches('?')
                .split('&')
                .filter(|p| !p.is_empty()),
        )
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
