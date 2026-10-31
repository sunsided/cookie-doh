//! Cookie D'oh! - Accept all Cookies.

use cookie_doh::app::{CookieDoh, config};
use game_toolkit::prelude::*;

fn main() -> Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn")).init();
    run::<CookieDoh>(config())
}
