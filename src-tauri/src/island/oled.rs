//! OLED burn-in protection: the island steps a pixel or two along the top edge every few
//! minutes, so no column of pixels holds one brightness for hours on end.
//!
//! Along the edge, never away from it: the island's flat top flares into the screen edge, and
//! moving down would open a seam under it. Only while collapsed: an open panel lives on screen
//! for seconds, and moving a window under a drag reads as a jump.

use std::sync::atomic::{AtomicU8, Ordering};
use std::thread;
use std::time::Duration;

use tauri::AppHandle;

use super::geometry::apply_stage_geometry;
use super::state::{read_window_state, IslandWindowState};

/// How long the island holds one offset before stepping to the next.
pub(super) const SHIFT_INTERVAL: Duration = Duration::from_secs(180);

/// Logical px along the top edge, in the order they are used: still, right, further right,
/// then back. Walking out and back keeps the island's average place where the settings put it.
const STEPS: [i32; 4] = [0, 1, 2, 1];

static STEP: AtomicU8 = AtomicU8::new(0);

/// The offset of one step in the cycle; `0` for the first.
fn step_offset(step: u8) -> i32 {
    STEPS[usize::from(step % STEPS.len() as u8)]
}

pub(super) fn offset() -> i32 {
    step_offset(STEP.load(Ordering::Relaxed))
}

/// Back to the still step: a switch that comes back on should not resume halfway out.
pub(super) fn reset() {
    STEP.store(0, Ordering::Relaxed);
}

/// Moves to the next offset, and says whether it differs from the one before.
pub(super) fn advance() -> bool {
    let before = offset();
    STEP.fetch_add(1, Ordering::Relaxed);
    offset() != before
}

/// Whether the island should be standing on a shifted offset right now: only as the collapsed
/// strip, with the setting on, and never under the pointer of a drag. Tucking does not exempt
/// it — a tucked island still lights the same few pixels of its edge strip.
fn should_shift(state: IslandWindowState) -> bool {
    state.oled_shift && state.is_collapsed() && !state.is_dragging
}

/// A logical offset in physical px, rounded the way window positions are.
fn physical_offset(offset: i32, scale: f64) -> i32 {
    (f64::from(offset) * scale).round() as i32
}

/// `(x, y)` moved along the top edge by the current offset (physical px at `scale`).
pub(super) fn shifted(x: i32, y: i32, state: IslandWindowState, scale: f64) -> (i32, i32) {
    if !should_shift(state) {
        return (x, y);
    }
    let step = physical_offset(offset(), scale);
    if step == 0 {
        return (x, y);
    }
    // The step runs along whichever edge the island is docked to: across the screen at the top,
    // up and down the side when it hangs off the left or right edge.
    if state.dock_edge.is_side() {
        return (x, y + step);
    }
    (x + step, y)
}

/// Steps the island every [`SHIFT_INTERVAL`] while it stands collapsed and the setting is on.
pub(crate) fn start_oled_shift_loop(app: AppHandle) {
    thread::spawn(move || loop {
        thread::sleep(SHIFT_INTERVAL);
        let state = read_window_state();
        if !should_shift(state) {
            continue;
        }
        if !advance() {
            continue;
        }
        let Ok(window) = super::main_window(&app) else {
            continue;
        };
        if let Err(error) = apply_stage_geometry(&window, read_window_state(), false) {
            eprintln!("Failed to step the island for OLED protection: {error}");
        }
    });
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_offsets_walk_out_and_back_from_still() {
        assert_eq!(
            [0u8, 1, 2, 3, 4, 5, 6, 7].map(step_offset),
            [0, 1, 2, 1, 0, 1, 2, 1]
        );
    }

    #[test]
    fn a_collapsed_island_shifts_unless_it_is_being_dragged() {
        let mut state = IslandWindowState::default();
        assert!(should_shift(state));
        state.oled_shift = false;
        assert!(!should_shift(state));
        state.oled_shift = true;
        state.is_tucked = true;
        assert!(should_shift(state), "a tucked island still lights its edge strip");
        state.is_tucked = false;
        state.is_dragging = true;
        assert!(!should_shift(state));
        state.is_dragging = false;
        state.mode = super::super::state::IslandMode::Expanded;
        assert!(!should_shift(state));
    }

    #[test]
    fn an_offset_becomes_physical_pixels() {
        assert_eq!(physical_offset(0, 1.5), 0);
        assert_eq!(physical_offset(2, 1.0), 2);
        assert_eq!(physical_offset(1, 1.5), 2);
        assert_eq!(physical_offset(2, 1.25), 3);
    }
}
