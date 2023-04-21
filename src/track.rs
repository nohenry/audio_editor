use std::{
    ops::Range,
    sync::{Arc, RwLock},
    time::Duration,
};

use egui::Pos2;

use crate::{sample::Sample, state::State};

pub struct Track {
    pub name: String,
    pub samples: Vec<Arc<Sample>>,
    pub view_range: Range<isize>,

    pub app_state: Arc<RwLock<State>>,
}

const TRACK_HEIGHT: f32 = 200.0;

impl Track {
    pub fn new(
        name: impl Into<String>,
        samples: Vec<Arc<Sample>>,
        app_state: Arc<RwLock<State>>,
    ) -> Track {
        Track {
            name: name.into(),
            samples,
            app_state,
            view_range: 0..5000000,
        }
    }

    pub fn sample_at_time(&self, duration: &Duration) -> Option<&Arc<Sample>> {
        let mut offset = 0;
        self.samples.iter().find(|&x| {
            if duration.as_millis() > offset
                && duration.as_millis() < offset + x.len_time().as_millis()
            {
                return true;
            } else {
                offset += x.len_time().as_millis();
                return false;
            }
        })
    }

    pub fn sample_at_index(&self, index: usize) -> Option<&Arc<Sample>> {
        let mut offset = 0;
        println!("{}", index);
        self.samples.iter().find(|&x| {
            println!("off: {}", offset);
            if index >= offset && index < offset + x.len() {
                true
            } else {
                offset += x.len();
                false
            }
        })
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let frame = egui::containers::Frame {
            shadow: eframe::epaint::Shadow {
                extrusion: 4.0,
                color: egui::Color32::from_rgba_premultiplied(0, 0, 0, 50),
            },
            fill: egui::Color32::from_rgb(50, 50, 50),
            ..Default::default()
        };

        frame.begin(ui);
        frame
            .show(ui, |ui| {
                ui.horizontal(|ui| {
                    egui::containers::Frame::none()
                        .inner_margin(egui::Margin {
                            left: 10.0,
                            ..Default::default()
                        })
                        .show(ui, |ui| {
                            egui::Resize::default()
                                .id_source(&self.name)
                                .default_width(100.0)
                                .min_height(TRACK_HEIGHT)
                                .max_size(egui::vec2(f32::INFINITY, TRACK_HEIGHT))
                                .with_stroke(false)
                                .show(ui, |ui| {
                                    ui.label(&self.name);
                                });
                        });

                    ui.separator();

                    let (rect, _) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), ui.available_height()),
                        egui::Sense::drag(),
                    );
                    // Change the preview zoom on scroll
                    let scoll_delta = ui.ctx().input(|input| input.scroll_delta);
                    // Scroll faster when zoomed out more. This makes zoom feel more consistent
                    self.view_range.end = self.view_range.end
                        + (scoll_delta.y * self.view_range.len() as f32 / 1000.0) as isize;

                    // } else {
                    //     let data: Vec<_> = sample_data[..sample_data_len as usize]
                    //         .chunks(samples_per_pixel as usize)
                    //         .map(|sample| {
                    //             let (min, max) =
                    //                 sample.iter().fold((f32::MAX, f32::MIN), |(min, max), l| {
                    //                     (min.min(*l), max.max(*l))
                    //                 });

                    //             let sum_sqrd = sample.iter().map(|f| *f * *f).sum::<f32>();
                    //             let rms = (sum_sqrd / sample.len() as f32).sqrt();

                    //             (min, max, rms)
                    //         })
                    //         .collect();

                    //     for (i, avg) in data.into_iter().enumerate() {
                    //         // Max value
                    //         ui.painter().line_segment(
                    //             [
                    //                 Pos2::new(i as f32 + rect.left() + 0.5, rect.center().y),
                    //                 Pos2::new(
                    //                     i as f32 + rect.left() + 0.5,
                    //                     rect.center().y - avg.1 * scale,
                    //                 ),
                    //             ],
                    //             egui::Stroke::new(1.0, main_color),
                    //         );

                    //         // Min value
                    //         ui.painter().line_segment(
                    //             [
                    //                 Pos2::new(i as f32 + rect.left() + 0.5, rect.center().y),
                    //                 Pos2::new(
                    //                     i as f32 + rect.left() + 0.5,
                    //                     rect.center().y + avg.0.abs() * scale,
                    //                 ),
                    //             ],
                    //             egui::Stroke::new(1.0, main_color),
                    //         );

                    //         // RMS
                    //         ui.painter().line_segment(
                    //             [
                    //                 Pos2::new(
                    //                     i as f32 + rect.left() + 0.5,
                    //                     rect.center().y + avg.2.abs() / 2.0 * scale,
                    //                 ),
                    //                 Pos2::new(
                    //                     i as f32 + rect.left() + 0.5,
                    //                     rect.center().y - avg.2.abs() / 2.0 * scale,
                    //                 ),
                    //             ],
                    //             egui::Stroke::new(1.0, second_color),
                    //         );
                    //     }
                    // }

                    let width = rect.width();

                    ui.allocate_ui_at_rect(rect, |ui| {
                        let sample_data_len = self.view_range.end as usize;
                        let samples_per_pixel = sample_data_len as f32 / width;

                        let mut offset = 0;
                        let mut time_offset = 0.0;

                        let track_width = ui.available_width();

                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                        for sample in &self.samples {
                            if ((offset + sample.len()) as isize) < self.view_range.start {
                                continue;
                            } else if offset as isize > self.view_range.end {
                                break;
                            }

                            // Samples
                            let pixels_per_millis =
                                sample.header.sampling_rate as f32 / samples_per_pixel / 1000.0;

                            let sample_width = (sample.adjusted_len(self)) as f32
                                / (self.view_range.len() as f32 / track_width);

                            let sample_response = ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                                let res = ui.allocate_ui_with_layout(
                                    egui::vec2(sample_width, 20.0),
                                    egui::Layout::left_to_right(egui::Align::Min),
                                    |ui| {
                                        egui::Frame::none()
                                            .outer_margin(egui::Margin {
                                                bottom: 0.0,
                                                left: 0.0,
                                                right: 0.0,
                                                top: 5.0,
                                            })
                                            .inner_margin(2.0)
                                            .fill(egui::Color32::from_black_alpha(75))
                                            .rounding(egui::Rounding {
                                                ne: 5.0,
                                                nw: 5.0,
                                                se: 0.0,
                                                sw: 0.0,
                                            })
                                            .show(ui, |ui| {
                                                ui.set_min_width(ui.available_width());
                                                ui.label(&sample.name);
                                            });
                                    },
                                );

                                let frame_response = egui::Frame::none()
                                    .fill(egui::Color32::from_black_alpha(50))
                                    .outer_margin(egui::Margin {
                                        bottom: 5.0,
                                        left: 0.0,
                                        right: 0.0,
                                        top: 0.0,
                                    })
                                    .rounding(egui::Rounding {
                                        ne: 0.0,
                                        nw: 0.0,
                                        se: 5.0,
                                        sw: 5.0,
                                    })
                                    // .stroke(egui::Stroke::new(1.5, egui::Color32::from_gray(10)))
                                    // .shadow(egui::epaint::Shadow::small_dark())
                                    .show(ui, |ui| {
                                        let mut new_rect = ui.max_rect();
                                        new_rect
                                            .set_width(sample_width.max(res.response.rect.width()));
                                        ui.allocate_rect(new_rect, egui::Sense::drag());

                                        sample.display(ui, new_rect, self, offset);
                                        offset += sample.len();

                                        new_rect
                                    });
                                frame_response
                            });

                            let rect = sample_response.inner.inner;
                            let response = sample_response.inner.response;

                            // Display playback cursor
                            let state = self.app_state.read().unwrap();
                            if state.playing {
                                if let Some(duration) = state.duration_played() {
                                    let millis = duration.as_millis() as f32;
                                    if (millis as f64) > time_offset * 1000.0 {
                                        let x = (millis * pixels_per_millis + rect.left()).round()
                                            + 0.5;

                                        ui.painter().line_segment(
                                            [Pos2::new(x, rect.bottom()), Pos2::new(x, rect.top())],
                                            egui::Stroke::new(1.0, egui::Color32::GREEN),
                                        );
                                    }
                                }
                            }

                            // Display mouse cursor
                            if let Some(pos) = response.hover_pos() {
                                let x = pos.x + 0.5;

                                ui.painter().line_segment(
                                    [Pos2::new(x, rect.bottom()), Pos2::new(x, rect.top())],
                                    egui::Stroke::new(1.0, egui::Color32::RED),
                                );
                            }

                            time_offset += sample.len_time().as_secs_f64();
                        }
                    });
                })
            })
            .response
    }
}

use std::any::Any;

trait NamedAny: Any {
    fn type_name(&self) -> &'static str;
}

impl<T: Any> NamedAny for T {
    fn type_name(&self) -> &'static str {
        std::any::type_name::<T>()
    }
}
