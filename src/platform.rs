//! The few things that differ between native and the browser: hi-score storage and a seed.

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

    pub fn seed() -> u64 {
        (js_sys::Date::now() * 1000.0) as u64 ^ (js_sys::Math::random() * 1e15) as u64
    }
}

pub use imp::*;
