use std::{collections::BTreeMap, fs::File, io::Write, path::PathBuf};

use anyhow::{Result, anyhow};
use directories::ProjectDirs;
use egui::{
    FontFamily, FontId, TextStyle, Theme
};
use serde::{Deserialize, Serialize};

const CONFIG_FILENAME: &'static str = "config.json";

#[derive(Clone, Deserialize, Serialize)]
pub struct Config {
    #[serde(default)]
    pub version: String,
    #[serde(default = "default_locale")]
    pub locale: String,
    // pub fps: i32,
    #[serde(default = "default_widget_opacity")]
    pub widget_opacity: f32,
    #[serde(default = "default_streamer_mode")]
    pub streamer_mode: bool,
    #[serde(default = "default_streamer_msg")]
    pub streamer_msg: String,
    #[serde(default = "default_font_sizes")]
    pub font_sizes: BTreeMap<egui::TextStyle, FontId>,
    #[serde(default = "default_theme")]
    pub theme: egui_colors::Theme,
    #[serde(default = "default_theme_mode")]
    pub theme_mode: egui::Theme,
    #[serde(default = "default_legend_text_style")]
    pub legend_text_style: egui::TextStyle,
    #[serde(default = "default_pie_chart_opacity")]
    pub pie_chart_opacity: f32,
    #[serde(default = "default_defender_exclusion")]
    pub defender_exclusion: bool,
    #[serde(default = "default_auto_showhide_ui")]
    pub auto_showhide_ui: bool,
    #[serde(default = "default_nag_versions")]
    pub nag_versions: bool,
}

fn default_locale() -> String {
    rust_i18n::locale().to_string()
}

fn default_widget_opacity() -> f32 {
    0.30
}

fn default_streamer_mode() -> bool {
    true
}

fn default_streamer_msg() -> String {
    env!("CARGO_PKG_NAME").to_string()
}

fn default_theme() -> egui_colors::Theme {
    crate::ui::themes::EGUI_THEME
}

fn default_theme_mode() -> egui::Theme {
    egui::Theme::Dark
}

fn default_font_sizes() -> BTreeMap<egui::TextStyle, FontId> {
    [
        (TextStyle::Heading, FontId::new(18.0, FontFamily::Proportional)),
        (TextStyle::Body, FontId::new(12.5, FontFamily::Proportional)),
        (TextStyle::Monospace, FontId::new(12.0, FontFamily::Monospace)),
        (TextStyle::Button, FontId::new(12.5, FontFamily::Proportional)),
        (TextStyle::Small, FontId::new(9.0, FontFamily::Proportional)),
    ]
    .into()
}

fn default_legend_text_style() -> egui::TextStyle {
    egui::TextStyle::Body
}

fn default_pie_chart_opacity() -> f32 {
    0.05
}

fn default_defender_exclusion() -> bool {
    true
}

fn default_auto_showhide_ui() -> bool {
    false
}

fn default_nag_versions() -> bool {
    true
}

impl Default for Config {
    fn default() -> Self {
        Self {
            version: String::new(),
            locale: default_locale(),
            // fps: 60,
            widget_opacity: default_widget_opacity(),
            streamer_mode: default_streamer_mode(),
            streamer_msg: default_streamer_msg(),
            theme: default_theme(),
            theme_mode: default_theme_mode(),
            font_sizes: default_font_sizes(),
            legend_text_style: default_legend_text_style(),
            pie_chart_opacity: default_pie_chart_opacity(),
            defender_exclusion: default_defender_exclusion(),
            auto_showhide_ui: default_auto_showhide_ui(),
            nag_versions: default_nag_versions(),
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