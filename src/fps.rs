use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};

use anyhow::{Context, Result};

use crate::kreide::types::{UnityEngine_Application, UnityEngine_QualitySettings};

pub const UNCAPPED_FPS: i32 = -1;
pub const MIN_FPS: i32 = 1;
pub const MAX_FPS: i32 = 360;
pub const DEFAULT_FPS: i32 = 60;

static LAST_LOGGED_FPS: AtomicI32 = AtomicI32::new(i32::MIN);
static LAST_LOGGED_VSYNC: AtomicI32 = AtomicI32::new(i32::MIN);
static ERROR_LOGGED: AtomicBool = AtomicBool::new(false);

pub fn clamp_fps(fps: i32) -> i32 {
    if is_uncapped(fps) {
        UNCAPPED_FPS
    } else {
        fps.max(MIN_FPS)
    }
}

pub fn is_uncapped(fps: i32) -> bool {
    fps == UNCAPPED_FPS
}

pub fn format_fps(fps: i32) -> String {
    if is_uncapped(fps) {
        "uncapped".to_owned()
    } else {
        fps.to_string()
    }
}

pub fn runtime_state() -> (Option<i32>, Option<i32>) {
    (
        UnityEngine_Application::get_target_framerate().ok(),
        UnityEngine_QualitySettings::get_v_sync_count().ok(),
    )
}

pub fn apply(fps: i32) -> Result<()> {
    let fps = clamp_fps(fps);
    let fps_label = format_fps(fps);

    UnityEngine_QualitySettings::set_v_sync_count(0)
        .context("failed to disable vSync via QualitySettings.vSyncCount")?;
    UnityEngine_Application::set_target_framerate(fps)
        .with_context(|| format!("failed to set Unity targetFrameRate to {fps_label}"))?;

    let (actual_fps, actual_vsync) = runtime_state();
    log_if_changed(actual_fps.unwrap_or(fps), actual_vsync.unwrap_or(0));
    ERROR_LOGGED.store(false, Ordering::Relaxed);

    Ok(())
}

pub fn ensure_applied(fps: i32) {
    let fps = clamp_fps(fps);
    let (actual_fps, actual_vsync) = runtime_state();

    if actual_fps == Some(fps) && actual_vsync == Some(0) {
        ERROR_LOGGED.store(false, Ordering::Relaxed);
        return;
    }

    if let Err(error) = apply(fps) {
        if !ERROR_LOGGED.swap(true, Ordering::Relaxed) {
            log::warn!("Failed to enforce FPS cap {}: {error}", format_fps(fps));
        }
    }
}

fn log_if_changed(fps: i32, v_sync_count: i32) {
    let prev_fps = LAST_LOGGED_FPS.swap(fps, Ordering::Relaxed);
    let prev_vsync = LAST_LOGGED_VSYNC.swap(v_sync_count, Ordering::Relaxed);

    if prev_fps != fps || prev_vsync != v_sync_count {
        log::info!(
            "FPS limiter applied: targetFrameRate={} vSyncCount={}",
            format_fps(fps),
            v_sync_count,
        );
    }
}