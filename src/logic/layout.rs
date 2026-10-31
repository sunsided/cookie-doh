//! Playfield geometry in framebuffer pixels, shared by the simulation and the renderer.

/// Number of conveyor lanes.
pub const LANES: usize = 4;

/// Y of each belt's top surface (cookies sit on it).
pub const LANE_Y: [f32; LANES] = [52.0, 91.0, 130.0, 169.0];

/// Where a spat-out cookie lands on the belt (cookie's left edge).
pub const BELT_START_X: f32 = 42.0;
/// The belt's right end; a cookie whose left edge passes `CATCH_X` leaves the belt.
pub const BELT_END_X: f32 = 256.0;
/// Cookie size (square).
pub const COOKIE_SIZE: f32 = 8.0;
/// Left edge of a cookie that is just tipping over the belt end.
pub const CATCH_X: f32 = BELT_END_X - COOKIE_SIZE;
/// Where the window's mouth spits from (cookie's left edge, relative to the lane's belt Y).
pub const SPIT_FROM: (f32, f32) = (26.0, -14.0);
/// Floor height: missed cookies crumble here.
pub const FLOOR_Y: f32 = 190.0;

/// The spider hangs at the jar, centered on this X.
pub const SPIDER_X: f32 = 262.0;
/// Top of the spider thread.
pub const CEILING_Y: f32 = 15.0;

/// Where the spider's body top sits when it hangs at `lane`.
pub fn spider_hang_y(lane: usize) -> f32 {
    LANE_Y[lane] - 10.0
}
