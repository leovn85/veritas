use std::{fs::File, io::Write, path::PathBuf};

use anyhow::{Result, anyhow};
use directories::ProjectDirs;
use egui::Theme;
use egui_plot::Corner;
use serde::{Deserialize, Serialize};

use crate::ui::themes::EGUI_THEME;


const CONFIG_FILENAME: &'static str = "config.json";

#[derive(Clone, Deserialize, Serialize)]
pub struct Config {
    pub version: String,
    pub locale: String,
    // pub fps: i32,
    pub widget_opacity: f32,
    pub streamer_mode: bool,
    pub streamer_msg: String,
    pub streamer_msg_size_pt: f32,
    pub theme: egui_colors::Theme,
    pub theme_mode: egui::Theme,
    pub legend_text_style: egui::TextStyle,
    pub legend_position: Corner,
    pub legend_opacity: f32,
    pub pie_chart_opacity: f32,
    pub defender_exclusion: bool
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: String::new(),
            locale: rust_i18n::locale().to_string(),
            // fps: 60,
            widget_opacity: 0.30,
            streamer_mode: true,
            streamer_msg: env!("CARGO_PKG_NAME").to_string(),
            theme: EGUI_THEME,
            theme_mode: egui::Theme::Dark,
            streamer_msg_size_pt: 1.0,
            legend_text_style: egui::TextStyle::Small,
            legend_position: Corner::RightTop,
            legend_opacity: 1.0,
            pie_chart_opacity: 0.05,
            defender_exclusion: true,
        }
    }
}

impl Config {
    pub fn new(ctx: &egui::Context) -> Result<Self> {
        match ProjectDirs::from("", "", env!("CARGO_PKG_NAME")) {
            Some(proj_dirs) => {
                let config_local_dir = proj_dirs.config_local_dir();
                let config_path = config_local_dir.join(CONFIG_FILENAME);

                if !config_local_dir.exists() {
                    std::fs::create_dir_all(config_local_dir)?;
                }

                if !config_path.exists() {
                    Self::initialize(&config_path, ctx)
                } else {
                    let mut file = File::open(&config_path)?;
                    match serde_json::from_reader(&file) {
                        Ok(v) => Ok(v),
                        Err(_) => {
                            file.flush()?;
                            Self::initialize(&config_path, ctx)
                        }
                    }
                }
            }
            None => Err(anyhow!("Failed to load/create config project dirs.")),
        }
    }

    fn initialize(config_path: &PathBuf, ctx: &egui::Context) -> Result<Self> {
        let mut config: Config = Config {
            theme_mode: ctx.theme(),
            ..Default::default()
        };

        if config.theme_mode == Theme::Light {
            config.widget_opacity = 0.75;
        }

        let mut file = File::create(config_path)?;
        serde_json::to_writer(&mut file, &config)?;
        file.flush()?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        match ProjectDirs::from("", "", env!("CARGO_PKG_NAME")) {
            Some(proj_dirs) => {
                let config_local_dir = proj_dirs.config_local_dir();
                let config_path = config_local_dir.join(CONFIG_FILENAME);

                if !config_path.exists() {
                    std::fs::create_dir_all(config_local_dir)?;
                }

                let mut file = File::create(config_path)?;
                serde_json::to_writer(&mut file, self)?;
                file.flush()?;
                Ok(())
            }
            None => Err(anyhow!("Failed to load/create config project dirs.")),
        }
    }
}