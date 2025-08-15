use crate::ui::app::GraphUnit;
use egui::{Stroke, TextStyle, Ui};
use egui_plot::{Bar, BarChart, Legend, Line, Plot, PlotPoints, Polygon};

use crate::{battle::BattleContext, models::misc::Avatar};

use super::{app::App, helpers};

pub struct PieSegment {
    pub points: Vec<[f64; 2]>,
    pub value: f64,
}

impl App {
    pub fn show_damage_distribution_widget(&mut self, ui: &mut Ui) {
        let available = ui.available_size();

        Plot::new("damage_pie")
            .legend(
                Legend::default()
                    .position(egui_plot::Corner::RightTop)
                    .text_style(TextStyle::Small),
            )
            .height(available.y)
            .width(available.x)
            .data_aspect(1.0)
            .clamp_grid(true)
            .show_grid(false)
            .show_background(false)
            .show_axes([false; 2])
            .allow_drag(false)
            .allow_zoom(false)
            .allow_scroll(false)
            .show(ui, |plot_ui: &mut egui_plot::PlotUi<'_>| {
                let battle_context = BattleContext::get_instance();

                let total_damage = battle_context.total_damage as f64;
                if total_damage > 0.0 {
                    let segments = create_pie_segments(
                        &battle_context.real_time_damages,
                        &battle_context.avatar_lineup,
                    );
                    for (avatar, segment, i) in segments {
                        let color = helpers::get_character_color(i);
                        let percentage = segment.value / total_damage * 100.0;

                        let plot_points = PlotPoints::new(segment.points);
                        let polygon = Polygon::new("Damage Pie", plot_points)
                            .stroke(Stroke::new(1.5, color))
                            .id(avatar.name.clone())
                            .name(format!(
                                "{}: {:.1}%, {} DMG, {:.0} DpAV",
                                avatar.name,
                                percentage,
                                helpers::format_damage(segment.value),
                                segment.value / battle_context.action_value
                            ));

                        plot_ui.polygon(polygon);
                    }
                }
            });
    }

    pub fn show_damage_bar_widget(&mut self, ui: &mut Ui) {
        let battle_context = BattleContext::get_instance();
        let available = ui.available_size();
        Plot::new("damage_bars")
            .legend(Legend::default())
            .height(available.y)
            .width(available.x)
            .allow_drag(false)
            .allow_zoom(false)
            .allow_scroll(false)
            .show_background(false)
            .y_axis_formatter(|y, _| helpers::format_damage(y.value))
            .x_axis_formatter(|x, _| {
                let index = x.value.floor() as usize;
                battle_context
                    .avatar_lineup
                    .get(index)
                    .map(|avatar| avatar.name.clone())
                    .unwrap_or_default()
            })
            .show(ui, |plot_ui| {
                let bars_data = create_bar_data(
                    &battle_context.real_time_damages,
                    &battle_context.avatar_lineup,
                );
                let bars: Vec<Bar> = bars_data
                    .iter()
                    .enumerate()
                    .map(|(pos, (avatar, value, color_idx))| {
                        Bar::new(pos as f64, *value)
                            .name(&avatar.name)
                            .fill(helpers::get_character_color(*color_idx))
                            .width(0.7)
                    })
                    .collect();
    
                plot_ui.bar_chart(BarChart::new("", bars).id("bar_chart"));
            });
    }

    pub fn show_turn_damage_plot(&mut self, ui: &mut Ui) {
        let battle_context = BattleContext::get_instance();
        let available = ui.available_size();
        Plot::new("turn_damage_plot")
            .legend(
                Legend::default()
                    .position(egui_plot::Corner::RightTop)
                    .text_style(TextStyle::Small),
            )
            .height(available.y)
            .width(available.x)
            .include_y(0.0)
            .x_axis_label(t!("Turn"))
            .y_axis_label(t!("Damage"))
            .y_axis_formatter(|y, _| helpers::format_damage(y.value))
            .show(ui, |plot_ui| {
                for (i, avatar) in battle_context.avatar_lineup.iter().enumerate() {
                    let color = helpers::get_character_color(i);
                    let points = battle_context
                        .turn_history
                        .iter()
                        .enumerate()
                        .map(|(turn_idx, turn)| {
                            [turn_idx as f64 + 1.0, turn.avatars_turn_damage[i]]
                        })
                        .collect::<Vec<[f64; 2]>>();

                    if !points.is_empty() {
                        plot_ui.line(
                            Line::new(&avatar.name, PlotPoints::from(points))
                                .color(color)
                                .width(2.0),
                        );
                    }
                }
            });
    }

    pub fn show_av_damage_plot(&mut self, ui: &mut Ui) {
        let battle_context = BattleContext::get_instance();
        let available = ui.available_size();
        Plot::new("av_damage_plot")
            .legend(
                Legend::default()
                    .position(egui_plot::Corner::RightTop)
                    .text_style(TextStyle::Small),
            )
            .height(available.y)
            .width(available.x)
            .include_y(0.0)
            .x_axis_label(t!("Action Value"))
            .y_axis_label(t!("Damage"))
            .y_axis_formatter(|y, _| helpers::format_damage(y.value))
            .show(ui, |plot_ui| {
                for (i, avatar) in battle_context.avatar_lineup.iter().enumerate() {
                    let color = helpers::get_character_color(i);
                    let points = battle_context
                        .av_history
                        .iter()
                        .map(|turn| [turn.action_value, turn.avatars_turn_damage[i]])
                        .collect::<Vec<[f64; 2]>>();

                    if !points.is_empty() {
                        plot_ui.line(
                            Line::new(&avatar.name, PlotPoints::from(points))
                                .color(color)
                                .width(2.0),
                        );
                    }
                }
            });
    }

    pub fn show_real_time_damage_graph_widget(&mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            ui.horizontal(|ui| {
                ui.radio_value(&mut self.state.graph_x_unit, GraphUnit::Turn, t!("Turn"));
                ui.radio_value(
                    &mut self.state.graph_x_unit,
                    GraphUnit::ActionValue,
                    t!("Action Value"),
                );
            });
            ui.add_space(8.0);

            match self.state.graph_x_unit {
                GraphUnit::Turn => self.show_turn_damage_plot(ui),
                GraphUnit::ActionValue => self.show_av_damage_plot(ui),
            }
        });
    }

    pub fn show_av_metrics_widget(&mut self, ui: &mut Ui) {
        let battle_context = BattleContext::get_instance();

        egui::CollapsingHeader::new(format!(
            "{}: {:.2}",
            t!("Total Damage"),
            battle_context.total_damage
        ))
        .id_salt("total_damage_header")
        .show(ui, |ui| {
            ui.vertical(|ui| {
                for (i, avatar) in battle_context.avatar_lineup.iter().enumerate() {
                    ui.horizontal(|ui| {
                        ui.label(format!("{}", avatar.name));

                        ui.label(format!(
                            "{:.2}",
                            battle_context.real_time_damages[i],
                        ));

                    });
                }
            });
        });

        let current_action_value =
            battle_context.action_value - battle_context.last_wave_action_value;

        egui::CollapsingHeader::new(format!("{}: {:.2}", t!("AV"), current_action_value))
            .id_salt("action_value_header")
            .show(ui, |ui| {
                ui.label(format!(
                    "{}: {:.2}",
                    t!("Total Elapsed AV"),
                    battle_context.action_value
                ));
                ui.vertical(|ui| {
                    for (i, avatar) in battle_context.avatar_lineup.iter().enumerate() {
                        ui.horizontal(|ui| {
                            ui.label(format!("{}", avatar.name,));

                            ui.label(format!(
                                "{:.2}",
                                battle_context.battle_avatars[i].battle_stats.av
                                    + current_action_value,
                            ));
                        });
                    }
                });
            });

        let dpav = if battle_context.action_value > 0.0 {
            battle_context.total_damage / battle_context.action_value
        } else {
            battle_context.total_damage
        };
        egui::CollapsingHeader::new(format!("{}: {:.2}", t!("DPAV"), dpav))
            .id_salt("dpav_header")
            .show(ui, |ui| {
                ui.vertical(|ui| {
                    for (i, avatar) in battle_context.avatar_lineup.iter().enumerate() {
                        let dpav = if battle_context.action_value > 0.0 {
                            battle_context.real_time_damages[i] / battle_context.action_value
                        } else {
                            battle_context.real_time_damages[i]
                        };
                        ui.horizontal(|ui| {
                            ui.label(format!("{}", avatar.name));
                            ui.label(format!("{:.2}", dpav));
                        });
                    }
                });
            });

    }

    pub fn show_enemy_stats_widget(&mut self, ui: &mut Ui) {
        let battle_context = BattleContext::get_instance();
        let enemy_lineup = battle_context.enemy_lineup.clone();

        ui.vertical(|ui| {
            for enemy in &enemy_lineup {
                if let Some(i) = battle_context
                    .battle_enemies
                    .iter()
                    .enumerate()
                    .find(|(_, x)| x.entity == *enemy)
                    .map(|(i, _)| i)
                {
                    ui.horizontal(|ui| {
                        ui.label(format!("{}: ", &battle_context.enemies[i].name));
                        ui.label(format!(
                            "{:.2} {}",
                            battle_context.battle_enemies[i].battle_stats.hp,
                            t!("HP")
                        ));
                    });
                }
            }
        });
    }
}

fn create_bar_data(
    real_time_damages: &Vec<f64>,
    avatars: &Vec<Avatar>,
) -> Vec<(Avatar, f64, usize)> {
    let mut bar_data = Vec::new();
    for (i, avatar) in avatars.iter().enumerate() {
        bar_data.push((avatar.clone(), real_time_damages[i], i));
    }
    bar_data
}

fn create_pie_segments(
    real_time_damages: &Vec<f64>,
    avatars: &Vec<Avatar>,
) -> Vec<(Avatar, PieSegment, usize)> {
    let total_damage = real_time_damages.into_iter().sum::<f64>();
    let mut segments = Vec::new();
    let mut start_angle = -std::f64::consts::FRAC_PI_2;

    for (i, avatar) in avatars.iter().enumerate() {
        let damage = real_time_damages[i];
        let fraction = damage as f64 / total_damage;
        let angle = fraction * std::f64::consts::TAU;
        let end_angle = start_angle + angle;

        segments.push((
            avatar.clone(),
            PieSegment {
                points: create_pie_slice(start_angle, end_angle),
                value: damage as f64,
            },
            i,
        ));

        start_angle = end_angle;
    }

    segments
}

fn create_pie_slice(start_angle: f64, end_angle: f64) -> Vec<[f64; 2]> {
    let center = [0.0, 0.0];
    let radius = 0.8;
    let mut points = vec![center];

    let steps = 50;
    let p = (end_angle - start_angle) / (steps as f64);
    for i in 0..=steps {
        let angle = start_angle + p * i as f64;
        let (sin, cos) = angle.sin_cos();
        points.push([cos * radius, sin * radius]);
    }
    points.push(center);

    points
}
