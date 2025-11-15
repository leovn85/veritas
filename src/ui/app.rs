use std::fs::File;
use std::io::Read;
use std::io::Write;

use anyhow::Result;
use anyhow::anyhow;
use directories::ProjectDirs;
use edio11::{Overlay, WindowMessage, WindowProcessOptions, input::InputResult};
use egui::CollapsingHeader;
use egui::Key;
use egui::KeyboardShortcut;
use egui::Label;
use egui::Memory;
use egui::Modifiers;
use egui::RichText;
use egui::ScrollArea;
use egui::Stroke;
use egui::TextEdit;
use egui::Ui;
use egui::UiBuilder;
use egui::{
    CentralPanel, Color32, Context, Frame, Slider, Window,
    epaint::text::{FontInsert, InsertFontFamily},
};
use egui_colors::Colorix;
use egui_commonmark::CommonMarkCache;
use egui_commonmark::CommonMarkViewer;
use egui_inbox::UiInbox;
use egui_notify::Toasts;
use serde::Deserialize;
use serde::Serialize;
use windows::Win32::{
    Foundation::{LPARAM, WPARAM},
    UI::{Input::KeyboardAndMouse::VK_MENU, WindowsAndMessaging::WM_KEYDOWN},
};

use crate::CHANGELOG;
use crate::LOCALES;
use crate::RUNTIME;
use crate::entry::InitErrorInfo;
use crate::battle::BattleContext;
use crate::export::BattleDataExporter;
use crate::ui::themes;
use crate::updater::Status;
use crate::updater::Update;
use crate::updater::Updater;

use super::config::Config;

#[derive(Default, PartialEq, Serialize, Deserialize)]
pub enum GraphUnit {
    #[default]
    Turn,
    ActionValue,
}

#[derive(Clone)]
pub enum ExportNotification {
    Success,
    Error { message: String },
}

#[derive(Serialize, Deserialize)]
pub struct AppState {
    pub show_menu: bool,
    pub show_changelog: bool,
    pub show_help: bool,
    pub show_settings: bool,
    pub show_console: bool,
    pub show_damage_distribution: bool,
    pub show_damage_bars: bool,
    pub show_real_time_damage: bool,
    pub show_enemy_stats: bool,
    pub show_battle_metrics: bool,
    pub should_hide: bool,
    pub graph_x_unit: GraphUnit,
    #[serde(skip)]
    pub use_custom_color: bool,
    #[serde(skip)]
    pub update_bttn_enabled: bool,
    #[serde(skip)]
    pub show_version_mismatch_popup: bool,
    #[serde(skip)]
    pub center_updater_window: bool,
    show_character_legend: bool,
    pub auto_save_battle_data: bool,
    pub show_export_window: bool,
    pub show_updater_window: bool,
    pub custom_export_path: Option<String>,
    pub auto_create_date_folders: bool,
}

pub struct App {
    pub state: AppState,
    pub config: Config,
    pub notifs: Toasts,
    pub colorix: Colorix,
    pub update_inbox: UiInbox<Option<Update>>,
    pub export_inbox: UiInbox<ExportNotification>,
    pub update: Option<Update>,
    beta_channel: bool,
    skip_version_mismatch_popup: bool,
    reopen_changelog: bool,
    init_err: Option<InitErrorInfo>,
    is_state_loaded: bool,
    updater_hint: Option<String>,
    updater_window_last_size: Option<egui::Vec2>,
}

pub const HIDE_UI: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::H);
pub const SHOW_MENU_SHORTCUT: KeyboardShortcut = KeyboardShortcut::new(Modifiers::COMMAND, Key::M);

impl Overlay for App {
    fn update(&mut self, ctx: &egui::Context) {
        if self.state.show_changelog {
            let changelog = parse_changelog::parse(CHANGELOG).unwrap();

            if let Some(release) = changelog.get(env!("CARGO_PKG_VERSION")) {
                Window::new(t!("Changelog"))
                    .id("changelog_window".into())
                    .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                    .collapsible(false)
                    .resizable(true)
                    .frame(Frame::window(&ctx.style()).inner_margin(5.))
                    .show(ctx, |ui| {
                        ScrollArea::new([false, true]).show(ui, |ui| {
                            ui.vertical_centered(|ui| {
                                ui.heading(release.title);

                                let mut cache = CommonMarkCache::default();
                                CommonMarkViewer::new().show(ui, &mut cache, release.notes);

                                ui.add_space(5.);

                                if ui.button(t!("Close")).clicked() {
                                    self.state.show_changelog = false;
                                    self.config.version = env!("CARGO_PKG_VERSION").to_string();
                                }
                            });
                        });
                    });
            }
        }

        if self.state.show_help {
            Window::new(format!("{} {}", egui_phosphor::bold::QUESTION, t!("Help")))
            .id("help_window".into())
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .collapsible(false)
            .resizable(true)
            .frame(Frame::window(&ctx.style()).inner_margin(5.))
            .show(ctx, |ui| {
                ScrollArea::new([false, true]).show(ui, |ui| {
                    ui.vertical_centered(|ui| {
                        let markup = indoc::indoc!("
                            # [Shortcuts](https://github.com/hessiser/veritas/wiki/Home/#shortcuts)
                            - `Ctrl` + `M` to toggle menu
                            - `Ctrl` + `H` to hide the UI
                            - `Ctrl` + `+` to zoom in
                            - `Ctrl` + `-` to zoom out
                            - `Ctrl` + `0` to reset zoom

                            # [FAQ](https://github.com/hessiser/veritas/wiki/Home/#troubleshooting)
                            - **How do I reset my graphs?**

                            Double-click the graph to reset. Alternatively, can delete `persistence` in `appdata/local/veritas/data` and restart.

                            - **The game is not processing keyboard/mouse inputs.**

                            If your mouse is hovering over the overlay, it will consume all mouse inputs. If the overlay is taking keyboard inputs, it will consume all keyboard inputs as well. Either move your mouse away or click around the overlay, or use the hide UI shortcut.

                            - **`[Error] Client is damaged, please reinstall the client.` on official servers.**

                            Follow instructions [here](https://github.com/hessiser/veritas/wiki/Home/#method-1-recommended-for-official-servers).
                        ");
                        let mut cache = CommonMarkCache::default();
                        CommonMarkViewer::new().show(ui, &mut cache, markup);

                        ui.add_space(5.);

                        if ui.button(t!("Close")).clicked() {
                            self.state.show_help = false;
                            self.config.version = env!("CARGO_PKG_VERSION").to_string();
                        }
                    });
                });
            });
        }

        if self.state.show_version_mismatch_popup {
            // hacky fix
            if self.state.show_changelog {
                self.state.show_changelog = false;
                self.reopen_changelog = true;
            }
            Window::new(RichText::new(format!("{} Version mismatch detected", egui_phosphor::bold::WARNING)).size(24.0))
                .id("version_mismatch_popup".into())
                .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                .default_width(440.0)
                .min_width(320.0)
                .frame(Frame::window(&ctx.style()).inner_margin(14.0))
                .collapsible(false)
                .resizable(true)
            .show(ctx, |ui| self.show_version_mismatch_popup(ui));
        }

        if self.config.streamer_mode {
            egui::TopBottomPanel::bottom("statusbar")
                .resizable(true)
                .show(ctx, |ui| {
                    ui.style_mut().override_text_style = Some(egui::TextStyle::Body);
                    let label = Label::new(RichText::new(&self.config.streamer_msg).strong())
                        .selectable(false);
                    ui.add(label);
                    ui.allocate_space(ui.available_size())
                });
        }

        if !self.state.should_hide {
            if self.state.show_menu {
                CentralPanel::default()
                    .frame(Frame {
                        fill: Color32::BLACK.gamma_multiply(0.25),
                        ..Default::default()
                    })
                    .show(ctx, |_ui: &mut egui::Ui| {
                        Window::new(t!("Menu"))
                            .id("menu_window".into())
                            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
                            .collapsible(false)
                            .show(ctx, |ui| {
                                // Settings
                                egui::Frame::default().inner_margin(5.0).show(ui, |ui| {
                                    egui::MenuBar::new().ui(ui, |ui| {
                                        ui.toggle_value(
                                            &mut self.state.show_settings,
                                            RichText::new(format!(
                                                "{} {}",
                                                egui_phosphor::bold::GEAR,
                                                t!("Settings")
                                            )),
                                        );

                                        ui.toggle_value(
                                            &mut self.state.show_export_window,
                                            RichText::new(format!(
                                                "{} Export",
                                                egui_phosphor::bold::DOWNLOAD_SIMPLE,
                                            )),
                                        );

                                        ui.toggle_value(
                                            &mut self.state.show_updater_window,
                                            RichText::new(format!(
                                                "{} Updates",
                                                egui_phosphor::bold::DOWNLOAD,
                                            )),
                                        );

                                        if ui
                                            .button(RichText::new(format!(
                                                "{} {}",
                                                egui_phosphor::bold::ARROW_COUNTER_CLOCKWISE,
                                                t!("Reset")
                                            )))
                                            .clicked()
                                        {
                                            ctx.memory_mut(|writer| *writer = Memory::default());
                                        }

                                        if ui
                                            .button(RichText::new(format!(
                                                "{} {}",
                                                egui_phosphor::bold::QUESTION,
                                                t!("Help")
                                            )))
                                            .clicked()
                                        {
                                            self.state.show_help = !self.state.show_help;
                                        }

                                        // ui.menu_button(RichText::new(format!(
                                        //         "{} {}",
                                        //         egui_phosphor::bold::COMMAND,
                                        //         t!("Shortcuts")
                                        //     )).strong(), |ui| {
                                        //         let button = Button::new(RichText::new(t!("Show menu"))).shortcut_text(ctx.format_shortcut(&SHOW_MENU_SHORTCUT));
                                        //         if ui.add(button).changed() {

                                        //         };
                                        //     });
                                    });
                                });

                                ui.separator();

                                let mut show_settings = self.state.show_settings;
                                if show_settings {
                                    egui::Window::new(format!("{} {}", egui_phosphor::bold::GEAR, t!("Settings")))
                                        .id("settings_window".into())
                                        .open(&mut show_settings)
                                        .show(ctx, |ui| {
                                            self.show_settings(ui);
                                        });
                                    self.state.show_settings = show_settings;
                                }

                                let mut show_export_window = self.state.show_export_window;
                                if show_export_window {
                                    Window::new(format!("{} Export Battle Data", egui_phosphor::bold::DOWNLOAD_SIMPLE))
                                        .id("export_window".into())
                                        .open(&mut show_export_window)
                                        .show(ctx, |ui| {
                                            self.show_export_window(ui);
                                        });
                                    self.state.show_export_window = show_export_window;
                                }

                                let mut show_updater_window = self.state.show_updater_window;
                                if show_updater_window {
                                    let mut updater_window = Window::new(format!(
                                        "{} Updates",
                                        egui_phosphor::bold::DOWNLOAD
                                    ))
                                    .id("updater_window".into())
                                    .open(&mut show_updater_window);

                                    if self.state.center_updater_window {
                                        let center = ctx.input(|input| input.screen_rect.center());
                                        if let Some(size) = self.updater_window_last_size {
                                            let top_left = center - size * 0.5;
                                            updater_window = updater_window.current_pos(top_left);
                                            self.state.center_updater_window = false;
                                        } else {
                                            updater_window = updater_window
                                                .pivot(egui::Align2::CENTER_CENTER)
                                                .current_pos(center);
                                        }
                                    }

                                    if let Some(response) = updater_window.show(ctx, |ui| {
                                        self.show_updater_window(ui);
                                    }) {
                                        self.updater_window_last_size = Some(response.response.rect.size());
                                    }
                                    self.state.show_updater_window = show_updater_window;
                                    if !self.state.show_updater_window {
                                        self.updater_hint = None;
                                    }
                                }

                                ui.vertical_centered(|ui| {
                                    ui.add_space(5.);
                                    ui.checkbox(&mut self.state.show_console, t!("Show Logs"));
                                    ui.checkbox(
                                        &mut self.state.show_character_legend,
                                        t!("Show Character Legend"),
                                    );
                                    ui.checkbox(
                                        &mut self.state.show_damage_distribution,
                                        t!("Show Damage Distribution"),
                                    );
                                    ui.checkbox(
                                        &mut self.state.show_damage_bars,
                                        t!("Show Damage Bars"),
                                    );
                                    ui.checkbox(
                                        &mut self.state.show_real_time_damage,
                                        t!("Show Real-Time Damage"),
                                    );
                                    ui.checkbox(
                                        &mut self.state.show_enemy_stats,
                                        t!("Show Enemy Stats"),
                                    );

                                    ui.checkbox(
                                        &mut self.state.show_battle_metrics,
                                        t!("Show Battle Metrics"),
                                    );

                                    ui.add_space(5.);

                                    ui.separator();
                                    if ui.button(t!("Close")).clicked() {
                                        self.state.show_menu = false;
                                    }
                                });
                            });
                    });
            }

            if self.state.show_console {
                egui::Window::new(t!("Log"))
                    .id("log_window".into())
                    .resizable(true)
                    .default_height(300.0)
                    .default_width(400.0)
                    .min_width(200.0)
                    .min_height(100.0)
                    .show(ctx, |ui| {
                        let available = ui.available_size();
                        ui.set_min_size(available);
                        ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
                            egui_logger::logger_ui().show(ui);
                        });
                    });
            }

            let opacity = self.config.widget_opacity.clamp(0.0, 1.0);
            let color = ctx.style().visuals.extreme_bg_color.gamma_multiply(opacity);
            let window_frame = egui::Frame::new()
                .fill(color)
                .stroke(Stroke::new(0.5, Color32::WHITE))
                .inner_margin(8.0)
                .corner_radius(10.0);

            let transparent_frame = egui::Frame::new().inner_margin(8.0).corner_radius(10.0);

            let damage_distribution_window_title = if self.state.show_menu {
                t!("Damage Distribution").into_owned()
            } else {
                String::new()
            };
            if self.state.show_damage_distribution {
                egui::containers::Window::new(damage_distribution_window_title)
                    .id("damage_distribution_window".into())
                    .frame(if self.state.show_menu {
                        window_frame
                    } else {
                        transparent_frame
                    })
                    .collapsible(false)
                    .resizable(true)
                    .min_width(200.0)
                    .min_height(200.0)
                    .show(ctx, |ui| {
                        self.show_damage_distribution_widget(ui);
                    });
            }

            if self.state.show_character_legend {
                egui::containers::Window::new(t!("Character Legend"))
                    .id("character_legend_window".into())
                    .frame(window_frame)
                    .resizable(true)
                    .min_width(200.0)
                    .min_height(200.0)
                    .show(ctx, |ui| {
                        self.show_character_legend(ui);
                    });
            }

            if self.state.show_damage_bars {
                egui::containers::Window::new(t!("Character Damage"))
                    .id("damage_by_character_window".into())
                    .frame(window_frame)
                    .resizable(true)
                    .min_width(200.0)
                    .min_height(200.0)
                    .show(ctx, |ui| {
                        self.show_damage_bar_widget(ui);
                    });
            }

            if self.state.show_real_time_damage {
                egui::containers::Window::new(t!("Real-Time Damage"))
                    .id("realt_time_damage_window".into())
                    .frame(window_frame)
                    .resizable(true)
                    .min_width(200.0)
                    .min_height(200.0)
                    .show(ctx, |ui| {
                        self.show_real_time_damage_graph_widget(ui);
                    });
            }

            if self.state.show_battle_metrics {
                egui::containers::Window::new(t!("Battle Metrics"))
                    .id("action_value_metrics_window".into())
                    .frame(window_frame)
                    .resizable(true)
                    .min_width(200.0)
                    .min_height(150.0)
                    .show(ctx, |ui| {
                        self.show_av_metrics_widget(ui);
                    });
            }

            if self.state.show_enemy_stats {
                egui::containers::Window::new(t!("Enemy Stats"))
                    .id("enemy_stats_window".into())
                    .frame(window_frame)
                    .resizable(true)
                    .min_width(200.0)
                    .min_height(150.0)
                    .show(ctx, |ui| {
                        self.show_enemy_stats_widget(ui);
                    });
            }
        }

        // This is a weird quirk of immediate mode where we must initialize our state a frame later
        if !self.is_state_loaded {
            let keep_popup = self.state.show_version_mismatch_popup;
            self.is_state_loaded = !self.is_state_loaded;
            self.state = AppState::load().unwrap_or_else(|e| {
                log::error!("{e}");
                AppState::default()
            });
            if keep_popup {
                self.state.show_version_mismatch_popup = true;
            }
            if env!("CARGO_PKG_VERSION") != self.config.version {
                self.state.show_changelog = true
            }
        }

        if ctx.input_mut(|i| i.consume_shortcut(&HIDE_UI)) {
            self.state.should_hide = !self.state.should_hide;
        }

        if let Some(Some(update)) = self.update_inbox.read(ctx).last() {
            if let Some(new_version) = &update.new_version {
                match &update.status {
                    Some(status) => {
                        match status {
                            Status::Failed(e) => {
                                self.notifs.error(t!("Update failed: %{error}", error = e))
                            }
                            Status::Succeeded => self.notifs.success(t!("Update succeeded")),
                        };
                    }
                    None => {
                        self.notifs
                            .info(t!(
                                "Version %{version} is available! Click here to open settings and update.", version = new_version
                            ))
                            .closable(true)
                            .show_progress_bar(true)
                            .duration(Some(std::time::Duration::from_secs_f32(20.0)));
                    }
                }
            }
            self.state.update_bttn_enabled = true;
            self.update = Some(update);
        }

        if let Some(export_notification) = self.export_inbox.read(ctx).last() {
            match export_notification {
                ExportNotification::Success => {
                    self.notifs.success("Battle data auto-exported successfully!");
                }
                ExportNotification::Error { message } => {
                    self.notifs.error(format!("Auto-export failed: {}", message));
                }
            }
        }

        if self.update.is_some() {
            // let message = format!("Version {} is available! Click here to open settings and update.",
            //     self.state.update_available.as_ref().unwrap());

            if let Some(screen_rect) = ctx.input(|i| i.pointer.hover_pos()) {
                if ctx.input(|i| i.pointer.primary_clicked()) {
                    let notification_area = egui::Rect::from_min_max(
                        egui::pos2(
                            ctx.screen_rect().right() - 200.0,
                            ctx.screen_rect().top() * self.notifs.len() as f32,
                        ),
                        egui::pos2(
                            ctx.screen_rect().right(),
                            (ctx.screen_rect().top() + 50.0) * self.notifs.len() as f32,
                        ),
                    );

                    if notification_area.contains(screen_rect) {
                        self.state.show_menu = true;
                        self.state.show_settings = true;
                        self.notifs.dismiss_all_toasts();
                    }
                }
            }
        }

        if let Some(state) = BattleContext::get_instance().state.take() {
            match state {
                crate::battle::BattleState::Started => {
                    if self.config.auto_showhide_ui {
                        self.state.should_hide = false;
                    }
                }
                crate::battle::BattleState::Ended => {
                    if self.config.auto_showhide_ui {
                        self.state.should_hide = true;
                    }
                    
                    if self.state.auto_save_battle_data {
                        
                        let export_data = BattleContext::take_prepared_export_data();
                        let csv_data = BattleContext::take_prepared_csv_data();
                        
                        match (export_data, csv_data) {
                            (Some(export_data), Some(csv_data)) => {
                                
                                let custom_path = self.state.custom_export_path.clone();
                                let auto_create_date_folders = self.state.auto_create_date_folders;
                                let export_sender = self.export_inbox.sender();
                                
                                RUNTIME.spawn(async move {
                                    use std::time::{SystemTime, UNIX_EPOCH};
                                    
                                    let timestamp = SystemTime::now()
                                        .duration_since(UNIX_EPOCH)
                                        .unwrap_or_default()
                                        .as_secs();
                                        
                                    let json_filename = format!("veritas_battledata_{}.json", timestamp);
                                    let json_result = export_json_data(&export_data, &json_filename, custom_path.as_deref(), auto_create_date_folders);
                                    
                                    let csv_filename = format!("veritas_battledata_{}.csv", timestamp);
                                    let csv_result = export_csv_data(&csv_data, &csv_filename, custom_path.as_deref(), auto_create_date_folders);
                                    
                                    match (json_result, csv_result) {
                                        (Ok(json_path), Ok(csv_path)) => {
                                            log::info!("Auto-exported JSON to: {}", json_path);
                                            log::info!("Auto-exported CSV to: {}", csv_path);
                                            let _ = export_sender.send(ExportNotification::Success);
                                        }
                                        (Err(e), _) | (_, Err(e)) => {
                                            log::error!("Failed to auto-export: {}", e);
                                            let _ = export_sender.send(ExportNotification::Error { 
                                                message: e.to_string() 
                                            });
                                        }
                                    }
                                });
                            }
                            _ => {
                                log::warn!("No prepared export data found");
                                self.notifs.error("Auto-export failed: No battle data available");
                            }
                        }
                    }
                }
            }
        }

        self.notifs.show(ctx);
    }

    fn window_process(
        &mut self,
        input: &InputResult,
        input_events: &Vec<egui::Event>,
    ) -> Option<WindowProcessOptions> {
        // Refactor later
        match input {
            InputResult::Key => {
                for e in input_events {
                    match e {
                        egui::Event::Key {
                            key,
                            physical_key: _,
                            pressed,
                            repeat: _,
                            modifiers,
                        } => {
                            if modifiers.matches_exact(SHOW_MENU_SHORTCUT.modifiers)
                                && *key == SHOW_MENU_SHORTCUT.logical_key
                                && *pressed
                            {
                                self.state.show_menu = !self.state.show_menu;

                                return Some(WindowProcessOptions {
                                    // Simulate alt to get cursor
                                    window_message: Some(WindowMessage {
                                        msg: WM_KEYDOWN,
                                        wparam: WPARAM(VK_MENU.0 as _),
                                        lparam: LPARAM(0),
                                    }),
                                    ..Default::default()
                                });
                            }
                        }
                        _ => {}
                    }
                }
            }
            _ => {}
        };

        if !self.state.should_hide && self.state.show_menu {
            Some(WindowProcessOptions {
                should_capture_all_input: true,
                ..Default::default()
            })
        } else {
            Some(WindowProcessOptions::default())
        }
    }

    fn save(&mut self, _storage: &mut egui::Memory) {
        self.save_persist(_storage)
            .unwrap_or_else(|e| log::error!("{e}"));
        self.state.save().unwrap_or_else(|e| log::error!("{e}"));

        self.config.theme = *self.colorix.theme();
        if self.colorix.dark_mode() {
            self.config.theme_mode = egui::Theme::Dark;
        } else {
            self.config.theme_mode = egui::Theme::Light;
        }

        self.config.save().unwrap_or_else(|e| log::error!("{e}"));
    }
}

const PERSISTENCE_FILENAME: &'static str = "persistence";
const STATE_FILENAME: &'static str = "state";

impl Default for AppState {
    fn default() -> Self {
        Self {
            show_menu: false,
            show_changelog: false,
            show_help: false,
            show_settings: false,
            show_console: false,
            show_damage_distribution: false,
            show_damage_bars: false,
            show_real_time_damage: false,
            show_enemy_stats: false,
            show_battle_metrics: false,
            should_hide: false,
            graph_x_unit: GraphUnit::default(),
            use_custom_color: false,
            update_bttn_enabled: false,
            show_version_mismatch_popup: false,
            center_updater_window: false,
            show_character_legend: false,
            auto_save_battle_data: false,
            show_export_window: false,
            show_updater_window: false,
            custom_export_path: None,
            auto_create_date_folders: true,
        }
    }
}

impl AppState {
    fn load() -> Result<Self> {
        match ProjectDirs::from("", "", env!("CARGO_PKG_NAME")) {
            Some(proj_dirs) => {
                let data_local_dir = proj_dirs.data_local_dir();
                let state_path = data_local_dir.join(STATE_FILENAME);

                if state_path.exists() {
                    let mut file = File::open(&state_path)?;
                    let mut buffer = String::new();
                    file.read_to_string(&mut buffer)?;
                    Ok(ron::from_str(&buffer)?)
                } else {
                    Ok(Self::default())
                }
            }
            None => Err(anyhow!("Failed to load/create data project dirs.")),
        }
    }

    fn save(&mut self) -> Result<()> {
        match ProjectDirs::from("", "", env!("CARGO_PKG_NAME")) {
            Some(proj_dirs) => {
                let data_local_dir = proj_dirs.data_local_dir();
                let state_path = data_local_dir.join(STATE_FILENAME);

                if !state_path.exists() {
                    std::fs::create_dir_all(data_local_dir)?;
                }
                let mut file = File::create(state_path)?;
                file.write(ron::to_string(&self)?.as_bytes())?;
                file.flush()?;
                Ok(())
            }
            None => Err(anyhow!("Failed to load/create data project dirs.")),
        }
    }
}

impl App {
    fn load_persist(ctx: &Context) -> Result<()> {
        match ProjectDirs::from("", "", env!("CARGO_PKG_NAME")) {
            Some(proj_dirs) => {
                let data_local_dir = proj_dirs.data_local_dir();
                let persist_path = data_local_dir.join(PERSISTENCE_FILENAME);

                if persist_path.exists() {
                    let mut file = File::open(&persist_path)?;
                    let mut buffer = String::new();
                    file.read_to_string(&mut buffer)?;
                    let memory: Memory = ron::from_str(&buffer)?;
                    ctx.memory_mut(|writer| {
                        *writer = memory;
                    });
                }

                Ok(())
            }
            None => Err(anyhow!("Failed to load/create data project dirs.")),
        }
    }

    fn save_persist(&mut self, _storage: &mut egui::Memory) -> Result<()> {
        match ProjectDirs::from("", "", env!("CARGO_PKG_NAME")) {
            Some(proj_dirs) => {
                let data_local_dir = proj_dirs.data_local_dir();
                let persist_path = data_local_dir.join(PERSISTENCE_FILENAME);

                if !persist_path.exists() {
                    std::fs::create_dir_all(data_local_dir)?;
                }
                let mut file = File::create(persist_path)?;
                file.write(ron::to_string(_storage)?.as_bytes())?;
                file.flush()?;
                Ok(())
            }
            None => Err(anyhow!("Failed to load/create data project dirs.")),
        }
    }

    pub fn new(ctx: Context) -> Self {
        if App::load_persist(&ctx).is_err() {
            log::error!("Failed to load persistence.");
        }

        let path = r"StarRail_Data\StreamingAssets\MiHoYoSDKRes\HttpServerResources\font\zh-cn.ttf";
        match std::fs::read(path) {
            Ok(font) => {
                // Start with the default fonts (we will be adding to them rather than replacing them).
                ctx.add_font(FontInsert::new(
                    "game_font",
                    egui::FontData::from_owned(font),
                    vec![
                        InsertFontFamily {
                            family: egui::FontFamily::Proportional,
                            priority: egui::epaint::text::FontPriority::Highest,
                        },
                        InsertFontFamily {
                            family: egui::FontFamily::Monospace,
                            priority: egui::epaint::text::FontPriority::Lowest,
                        },
                    ],
                ));
            }
            Err(e) => log::warn!(
                "{} : Could not locate {}. Defaulting to default font.",
                e,
                path
            ),
        }

        let path2 = r"StarRail_Data\StreamingAssets\MiHoYoSDKRes\HttpServerResources\font\ja-jp.ttf";
        match std::fs::read(path2) {
            Ok(font) => {
                ctx.add_font(FontInsert::new(
                    "game_font_jp",
                    egui::FontData::from_owned(font),
                    vec![
                        InsertFontFamily {
                            family: egui::FontFamily::Proportional,
                            priority: egui::epaint::text::FontPriority::Lowest,
                        },
                        InsertFontFamily {
                            family: egui::FontFamily::Monospace,
                            priority: egui::epaint::text::FontPriority::Lowest,
                        },
                    ],
                ));
            }
            Err(e) => log::warn!(
                "{} : Could not locate {}.",
                e,
                path2
            ),
        }

        let mut fonts = egui::FontDefinitions::default();
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Regular);
        egui_phosphor::add_to_fonts(&mut fonts, egui_phosphor::Variant::Bold);

        ctx.set_fonts(fonts);

        let config = Config::new(&ctx).unwrap_or_else(|e| {
            log::error!("{e}");
            Config::default()
        });

        let beta_channel = Updater::beta_channel_enabled();

        let mut app = Self {
            colorix: Colorix::global(&ctx, config.theme),
            config,
            notifs: Toasts::default(),
            state: AppState::default(),
            update_inbox: UiInbox::new(),
            export_inbox: UiInbox::new(),
            update: None,
            beta_channel,
            skip_version_mismatch_popup: false,
            reopen_changelog: false,
            init_err: None,
            is_state_loaded: false,
            updater_hint: None,
            updater_window_last_size: None,
        };

        rust_i18n::set_locale(&app.config.locale);
        match app.config.theme_mode {
            egui::Theme::Dark => {
                app.colorix
                    .set_dark(&mut Ui::new(ctx.clone(), "".into(), UiBuilder::new()))
            }
            egui::Theme::Light => {
                app.colorix
                    .set_light(&mut Ui::new(ctx.clone(), "".into(), UiBuilder::new()))
            }
        }

        let init_err = crate::entry::take_init_error();
        if app.config.nag_versions
            && matches!(init_err, Some(InitErrorInfo::ObfuscationMismatch { .. }))
        {
            app.state.show_version_mismatch_popup = true;
        }
        app.init_err = init_err;

        app.queue_update_check();

        app
    }

    fn show_settings(&mut self, ui: &mut Ui) {
        egui::MenuBar::new().ui(ui, |ui| {
            let style = ui.ctx().style();
            let font_id = &style.text_styles[&egui::TextStyle::Button];
            let font_size = font_id.size;
            self.colorix.light_dark_toggle_button(ui, font_size);

            ui.separator();

            ui.menu_button(
                RichText::new(format!("{} {}", egui_phosphor::bold::GLOBE, t!("Language"))),
                |ui| {
                    for locale_code in rust_i18n::available_locales!() {
                        if let Some(locale) = LOCALES.get(locale_code) {
                            if ui.button(*locale).clicked() {
                                self.config.locale = locale_code.to_owned();
                                rust_i18n::set_locale(locale_code);
                                ui.close();
                            }
                        }
                    }
                },
            );

            ui.toggle_value(
                &mut self.config.streamer_mode,
                RichText::new(format!(
                    "{} {}",
                    egui_phosphor::bold::VIDEO_CAMERA,
                    t!("Streamer Mode")
                )),
            );
        });

        ui.separator();

        ui.with_layout(egui::Layout::top_down_justified(egui::Align::LEFT), |ui| {
            ui.add_space(5.);

            CollapsingHeader::new(t!("Theme"))
                .id_salt("theme_header")
                .show(ui, |ui| {
                    if self.state.use_custom_color {
                        self.colorix.twelve_from_custom(ui);
                    };

                    ui.horizontal(|ui| {
                        self.colorix.custom_picker(ui);
                        ui.toggle_value(
                            &mut self.state.use_custom_color,
                            t!("Custom color. Click here to enable."),
                        );
                    });

                    if self.colorix.dark_mode() {
                        self.colorix.themes_dropdown(
                            ui,
                            Some((themes::THEME_NAMES.to_vec(), themes::THEMES.to_vec())),
                            true,
                        );
                    } else {
                        self.colorix.themes_dropdown(ui, None, false);
                    }

                    self.colorix.ui_combo_12(ui, false);
                });

            ui.horizontal(|ui| {
                ui.horizontal(|ui| {
                    let all_text_styles = ui.style().text_styles();
                    for style in all_text_styles {
                        ui.selectable_value(
                            &mut self.config.legend_text_style,
                            style.clone(),
                            style.to_string(),
                        );
                    }
                });
                ui.label(t!("Legend Text Style"));
            });

            ui.horizontal(|ui| {
                ui.add(egui::Slider::new(
                    &mut self.config.pie_chart_opacity,
                    0.0..=1.0,
                ));
                ui.label(t!("Pie Chart Opacity"));
            });

            ui.add(
                Slider::new(&mut self.config.widget_opacity, 0.0..=1.0).text(t!("Window Opacity")),
            );

            CollapsingHeader::new(t!("Fonts"))
                .id_salt("fonts_header  ")
                .show(ui, |ui| {
                    for (style, id) in &mut self.config.font_sizes {
                        let label = format!("{:?}", style);
                        ui.add(Slider::new(&mut id.size, 8.0..=48.0).text(label));
                    }

                    let font_sizes = self.config.font_sizes.clone();
                    ui.ctx().all_styles_mut(move |style| {
                        style.text_styles = font_sizes.clone();
                    });
                });

            ui.checkbox(
                &mut self.config.auto_showhide_ui,
                t!("Auto(show/hide) UI on battle (start/end)."),
            );

            if ui
                .checkbox(
                    &mut self.config.nag_versions,
                    "Show version mismatch help when startup fails",
                )
                .changed()
            {
                if let Err(e) = self.config.save() {
                    log::error!("{e}");
                }
            }

            // TODO:
            // Change using a grid like so:

            // ui.label("Text style:");
            // ui.horizontal(|ui| {
            //     let all_text_styles = ui.style().text_styles();
            //     for style in all_text_styles {
            //         ui.selectable_value(&mut config.text_style, style.clone(), style.to_string());
            //     }
            // });
            // ui.end_row();

            // if ui
            //     .add(
            //         Slider::new(
            //             &mut self.settings.fps,
            //             10..=120,
            //         )
            //         .text(t!("FPS")),
            //     )
            //     .changed()
            // {
            //     self.config.set_fps(self.settings.fps);
            //     unsafe {
            //         Application_set_targetFrameRate(
            //             self.settings.fps,
            //         )
            //     };
            // }

            ui.add(
                TextEdit::singleline(&mut self.config.streamer_msg).hint_text(RichText::new(
                    format!(
                        "{} {}",
                        t!("Streamer Message. Can also use Phosphor Icons!"),
                        egui_phosphor::bold::RAINBOW
                    ),
                )),
            );
        });
    }

    fn show_export_window(&mut self, ui: &mut Ui) {
        ui.group(|ui| {
            ui.label(RichText::new(format!("{} Export Current/Last Played Battle", egui_phosphor::regular::UPLOAD)).strong());

            ui.add_space(4.0);

            ui.horizontal(|ui| {
                if ui.button(format!("{} Export JSON", egui_phosphor::bold::FILE_TEXT))
                    .clicked() 
                {
                    match self.export_battle_data("json") {
                        Ok(filepath) => {
                            self.notifs.success("JSON exported successfully!");
                            log::info!("JSON file exported to: {}", filepath);
                        }
                        Err(e) => {
                            self.notifs.error(format!("Failed to export JSON: {}", e));
                            log::error!("Failed to export JSON: {}", e);
                        }
                    }
                }

                if ui.button(format!("{} Export CSV", egui_phosphor::bold::FILE_CSV))
                    .clicked() 
                {
                    match self.export_battle_data("csv") {
                        Ok(filepath) => {
                            self.notifs.success("CSV exported successfully!");
                            log::info!("CSV file exported to: {}", filepath);
                        }
                        Err(e) => {
                            self.notifs.error(format!("Failed to export CSV: {}", e));
                            log::error!("Failed to export CSV: {}", e);
                        }
                    }
                }
            });
            
            ui.add_space(8.0);

            CollapsingHeader::new(format!("{} Format Information", egui_phosphor::regular::INFO))
                .id_salt("format_info_header")
                .default_open(false)
                .show(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!("{}", egui_phosphor::regular::FILE_TEXT));
                        ui.label("JSON format: Compatible with");
                        ui.hyperlink_to("Firefly Analysis", "https://sranalysis.kain.id.vn/");
                        ui.label("for detailed battle analysis");
                    });
                    
                    ui.horizontal_wrapped(|ui| {
                        ui.label(format!("{}", egui_phosphor::regular::FILE_CSV));
                        ui.label("CSV format: Spreadsheet-friendly data for creating custom charts and graphs");
                    });
                });

            ui.add_space(8.0);
            
            if ui.button(format!("{} Open Export Folder", egui_phosphor::bold::FOLDER_OPEN))
                .clicked() 
            {
                match BattleDataExporter::get_export_directory_path() {
                    Ok(dir_path) => {
                        self.open_folder(&dir_path);
                    }
                    Err(e) => {
                        self.notifs.error(format!("Failed to get export directory: {}", e));
                        log::error!("Failed to get export directory: {}", e);
                    }
                }
            }
            
            ui.add_space(8.0);
            
            ui.label("Export Folder Location:");
            if let Some(custom_path) = self.state.custom_export_path.clone() {
                ui.horizontal(|ui| {
                    ui.monospace(&custom_path);
                    if ui.button(format!("{} Change", egui_phosphor::regular::FOLDER_OPEN)).clicked() {
                        if let Some(path) = rfd::FileDialog::new().pick_folder() {
                            let path_str = path.to_string_lossy().to_string();
                            self.state.custom_export_path = Some(path_str);
                            self.state.auto_create_date_folders = false;
                        }
                    }
                    if ui.button(format!("{} Reset to Default", egui_phosphor::regular::ARROW_COUNTER_CLOCKWISE)).clicked() {
                        self.state.custom_export_path = None;
                        self.state.auto_create_date_folders = true;
                    }
                });
            } else {
                if let Ok(dir_path) = BattleDataExporter::get_export_directory_path() {
                    ui.horizontal(|ui| {
                        ui.monospace(&dir_path);
                        if ui.button(format!("{} Change", egui_phosphor::regular::FOLDER_OPEN)).clicked() {
                            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                let path_str = path.to_string_lossy().to_string();
                                self.state.custom_export_path = Some(path_str);
                                self.state.auto_create_date_folders = false;
                            }
                        }
                    });
                }
            }
        });
        
        ui.add_space(12.0);
        
        ui.group(|ui| {
            ui.label(RichText::new(format!("{} Settings", egui_phosphor::regular::GEAR)).strong());
            
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.state.auto_save_battle_data, "Auto-export data after battle ends");
                ui.add(egui::widgets::Label::new(egui::RichText::new(egui_phosphor::regular::INFO).size(16.0))
                    .sense(egui::Sense::hover()))
                    .on_hover_text("Automatically exports the most recent battle's data in both JSON and CSV formats immediately after the battle ends");
            });
            
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.state.auto_create_date_folders, "Auto-create date folders");
                ui.add(egui::widgets::Label::new(egui::RichText::new(egui_phosphor::regular::INFO).size(16.0))
                    .sense(egui::Sense::hover()))
                    .on_hover_text("Automatically organize exported data files into date-based folders (YYYY-MM-DD)");
            });
        });
    }

    fn show_updater_window(&mut self, ui: &mut Ui) {
        if let Some(hint) = self.updater_hint.as_deref() {
            Frame::group(ui.style())
                .inner_margin(10.0)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.label(
                            RichText::new(format!("{} Listen...", egui_phosphor::regular::LIGHTBULB))
                                .strong(),
                        );
                    });
                    ui.add(Label::new(hint).wrap());
                });

            ui.add_space(12.0);
        }

        ui.group(|ui| {
            ui.label(RichText::new(format!("{} Version Information", egui_phosphor::regular::INFO)).strong());
            
            let current_version = env!("CARGO_PKG_VERSION");
            if let Some(new_update) = &self.update {
                if let Some(new_version) = &new_update.new_version {
                    ui.colored_label(Color32::GREEN, t!("Version %{version} is available!", version = new_version));
                    ui.horizontal(|ui| {
                        ui.label(format!(
                            "{} ➡ {}",
                            current_version,
                            new_version
                        ));
                    });
                    
                    ui.add_space(8.0);
                    
                    if ui
                        .add_enabled(self.state.update_bttn_enabled, egui::Button::new(format!("{} Update Now", egui_phosphor::bold::DOWNLOAD)))
                        .clicked()
                    {
                        self.updater_hint = None;
                        let defender_exclusion = self.config.defender_exclusion;
                        let new_version = new_version.clone();
                        let sender = self.update_inbox.sender();
                        self.state.update_bttn_enabled = false;
                        self.notifs.success(t!("Update in progress"));
                        RUNTIME.spawn(async move {
                            let status = if let Err(e) = Updater::new(env!("CARGO_PKG_VERSION"))
                                .download_update(defender_exclusion)
                                .await
                            {
                                Some(Status::Failed(e))
                            }
                            else {
                                Some(Status::Succeeded)
                            };

                            if sender.send(Some(Update { new_version: Some(new_version.to_string()), status})).is_err() {
                                let e = anyhow!("Failed to send update to inbox");
                                log::error!("{e}");
                            }

                        });
                    }
                } else {
                    ui.horizontal(|ui| {
                        ui.label("Current version:");
                        ui.monospace(current_version);
                    });
                    ui.colored_label(Color32::LIGHT_GREEN, "You have the latest version!");
                }
            } else {
                ui.horizontal(|ui| {
                    ui.label("Current version:");
                    ui.monospace(current_version);
                });
                ui.label("Checking for updates...");
            }
        });
        
        ui.add_space(12.0);
        
        ui.group(|ui| {
            ui.label(RichText::new(format!("{} Settings", egui_phosphor::regular::GEAR)).strong());
            let prev_beta = self.beta_channel;
            ui.horizontal(|ui| {
                let changed = ui
                    .checkbox(&mut self.beta_channel, "Check beta updates (pre-release)")
                    .changed();

                ui.add(
                    egui::widgets::Label::new(
                        egui::RichText::new(egui_phosphor::regular::INFO).size(16.0),
                    )
                    .sense(egui::Sense::hover()),
                )
                .on_hover_text(
                    "Only enable this if you're running on a beta client, installing a DLL meant for the newest beta client on release client (current official version of the game) might break things",
                );

                if changed && !self.set_beta_flag(self.beta_channel) {
                    self.beta_channel = prev_beta;
                }
            });
            ui.horizontal(|ui| {
                ui.checkbox(&mut self.config.defender_exclusion, t!("Add Defender Exclusion during update"));
                ui.add(egui::widgets::Label::new(egui::RichText::new(egui_phosphor::regular::INFO).size(16.0))
                    .sense(egui::Sense::hover()))
                    .on_hover_text(t!(indoc::indoc!("
                        If enabled, the updater will temporarily add the new DLL file to Windows Defender exclusions during update to avoid false positives.
                        This is recommended to be enabled (if disabled, Windows Defender may cause the update to fail) however you can disable it if you prefer. The exclusion is removed after the update is finished.
                    ")));
            });
        });
    }

    fn show_version_mismatch_popup(&mut self, ui: &mut Ui) {
        const POPUP_MIN_WIDTH: f32 = 320.0;
        const POPUP_PREFERRED_WIDTH: f32 = 460.0;

        ui.scope(|ui| {
            {
                let spacing = ui.spacing_mut();
                spacing.item_spacing = egui::vec2(0.0, 12.0);
                spacing.button_padding = egui::vec2(16.0, 8.0);
            }

            let available_width = ui.available_width();
            let min_width = available_width.min(POPUP_MIN_WIDTH);
            ui.set_min_width(min_width);
            ui.set_max_width(POPUP_PREFERRED_WIDTH);

            ui.add(
                Label::new(
                    RichText::new(format!(
                        "{} Veritas couldn't start because the version you have was built for a different version of the game",
                        egui_phosphor::regular::SMILEY_SAD
                    ))
                    .size(16.0)
                    .strong(),
                )
                .wrap(),
            );

            if let Some(info) = &self.init_err {
                let message = match info {
                    InitErrorInfo::Other { message } => message,
                    InitErrorInfo::ObfuscationMismatch { message, .. } => message,
                };

                if !message.is_empty() {
                    CollapsingHeader::new("Error details")
                        .show_unindented(ui, |ui| {
                            ui.add(Label::new(message).wrap());
                        });
                }
            }

            ui.add(
                Label::new(
                    RichText::new(format!(
                        "{} But don't worry, we can fix it!",
                        egui_phosphor::regular::SMILEY
                    ))
                    .size(16.0)
                    .strong(),
                )
                .wrap(),
            );

            ui.separator();
            
            ui.add(
                Label::new(
                    RichText::new("Pick the client you are currently playing on")
                        .size(15.0)
                        .strong(),
                )
                .wrap(),
            );

            ui.horizontal(|ui| {
                {
                    let spacing = ui.spacing_mut();
                    spacing.item_spacing.x = 16.0;
                }
                let button_width = ((ui.available_width() - 16.0).max(0.0)) / 2.0;

                if ui
                    .add_sized([button_width, 36.0], egui::Button::new(RichText::new("I'm on live client").strong()))
                    .clicked()
                {
                    self.pick_build(false);
                }

                if ui
                    .add_sized([button_width, 36.0], egui::Button::new(RichText::new("I'm on beta client").strong()))
                    .clicked()
                {
                    self.pick_build(true);
                }
            });

            ui.add_space(6.0);

            Frame::group(ui.style()).inner_margin(8.0).show(ui, |ui| {
                CollapsingHeader::new(
                    RichText::new(format!(
                        "{} {}",
                        egui_phosphor::regular::QUESTION,
                        "How do I check?"
                    ))
                    .strong(),
                )
                .show_unindented(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = 6.0;
                    ui.add(Label::new(RichText::new("Look at the bottom-left corner of the screen when you have the game open").size(14.0).strong()).wrap());
                    ui.add(Label::new("If the text has OSBETA or CNBETA, you are on the beta client and need the beta version").wrap());
                    ui.add(Label::new("If the text has OSPROD or CNPROD, you are on the live client and need the live version").wrap());
                });
            });
            ui.add(Label::new("We'll point the updater at the right download so you can install a version that works with your game version").wrap());

            ui.checkbox(&mut self.skip_version_mismatch_popup, "Don't show this again");

            if ui.button("I'll handle it later").clicked() {
                self.close_version_mismatch_popup();
            }
        });
    }

    fn pick_build(&mut self, beta: bool) {
        if self.set_beta_flag(beta) {
            self.state.show_menu = true;
            self.state.show_updater_window = true;
            self.state.center_updater_window = true;
            let channel = if beta { "beta" } else { "live" };

            self.updater_hint = Some(
                "Click Update Now so your version matches the correct version for the client you're running".to_owned(),
            );

            self.notifs.info(format!(
                "Updates window opened on the {channel} channel. Click Update Now to download the version that matches your client"
            ));
            self.close_version_mismatch_popup();
        }
    }

    fn close_version_mismatch_popup(&mut self) {
        if self.skip_version_mismatch_popup && self.config.nag_versions {
            self.config.nag_versions = false;
            if let Err(e) = self.config.save() {
                log::error!("{e}");
            }
        }
        self.state.show_version_mismatch_popup = false;
        self.skip_version_mismatch_popup = false;
        if self.reopen_changelog {
            self.state.show_changelog = true;
            self.reopen_changelog = false;
        }
        self.init_err = None;
    }

    fn set_beta_flag(&mut self, enabled: bool) -> bool {
        if let Err(err) = Updater::set_beta_channel(enabled) {
            log::error!("failed to update beta toggle: {err}");
            self.notifs.error("Failed to switch update channel. See logs for details.");
            return false;
        }

        self.beta_channel = enabled;
        self.update = None;
        self.state.update_bttn_enabled = true;
        self.queue_update_check();
        true
    }

    fn queue_update_check(&self) {
        let sender = self.update_inbox.sender();
        RUNTIME.spawn(async move {
            match Updater::new(env!("CARGO_PKG_VERSION")).check_update().await {
                Ok(new_ver) => {
                    if sender
                        .send(Some(Update {
                            new_version: new_ver,
                            status: None,
                        }))
                        .is_err()
                    {
                        log::error!("Failed to send update to inbox");
                    }
                }
                Err(e) => {
                    log::error!("Update check failed: {e}");
                    if sender
                        .send(Some(Update {
                            new_version: None,
                            status: Some(Status::Failed(e)),
                        }))
                        .is_err()
                    {
                        log::error!("Failed to send update-failure to inbox");
                    }
                }
            }
        });
    }
    
    fn export_battle_data(&self, format: &str) -> Result<String, Box<dyn std::error::Error>> {
        let battle_context = BattleContext::get_instance();
        let exporter = BattleDataExporter::new();
        let custom_path = self.state.custom_export_path.as_deref();
        
        match format {
            "json" => exporter.export_to_file_with_custom_path(
                &battle_context, 
                None, 
                custom_path, 
                self.state.auto_create_date_folders
            ),
            "csv" => exporter.export_to_csv_with_custom_path(
                &battle_context, 
                None, 
                custom_path, 
                self.state.auto_create_date_folders
            ),
            _ => Err("Unsupported format".into())
        }
    }
    
    fn open_folder(&mut self, path: &str) {
        #[cfg(target_os = "windows")]
        {
            if let Err(e) = std::process::Command::new("explorer")
                .arg(path)
                .spawn()
            {
                self.notifs.error(format!("Failed to open folder: {}", e));
                log::error!("Failed to open folder: {}", e);
            }
        }
        #[cfg(not(target_os = "windows"))]
        {
            if let Err(e) = std::process::Command::new("xdg-open")
                .arg(path)
                .spawn()
            {
                self.notifs.error(format!("Failed to open folder: {}", e));
                log::error!("Failed to open folder: {}", e);
            }
        }
    }
}

fn export_json_data(
    export_data: &crate::export::ExportBattleData, 
    filename: &str,
    custom_path: Option<&str>, 
    auto_create_date_folders: bool
) -> Result<String, Box<dyn std::error::Error>> {
    use crate::export::BattleDataExporter;
    
    let json = serde_json::to_string_pretty(export_data)?;
    
    let export_dir = BattleDataExporter::get_export_directory_with_custom_path(custom_path, auto_create_date_folders)?;
    let full_path = export_dir.join(filename);
    
    std::fs::write(&full_path, &json)?;
    Ok(full_path.to_string_lossy().to_string())
}

fn export_csv_data(
    csv_data: &[crate::export::ComprehensiveData], 
    filename: &str,
    custom_path: Option<&str>, 
    auto_create_date_folders: bool
) -> Result<String, Box<dyn std::error::Error>> {
    use crate::export::BattleDataExporter;
    
    let export_dir = BattleDataExporter::get_export_directory_with_custom_path(custom_path, auto_create_date_folders)?;
    let full_path = export_dir.join(filename);
    
    let mut wtr = csv::Writer::from_path(&full_path)?;
    for record in csv_data {
        wtr.serialize(record)?;
    }
    wtr.flush()?;
    
    Ok(full_path.to_string_lossy().to_string())
}
