use anyhow::Result;
use egui_notify::Toast;
use std::time::Duration;

use crate::ui::app::App;

pub fn initialize(toasts: Vec<Toast>) -> Result<()> {
    edio11::set_overlay(
        env!("CARGO_PKG_NAME"),
        Box::new(|ctx| {
            let mut app = Box::new(App::new(ctx.clone()));
            for mut toast in toasts {
                toast.duration(Some(Duration::from_secs(5)));
                app.notifs.add(toast);
            }
            app
        }),
    )?;
    Ok(())
}