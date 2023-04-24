use std::{
    ops::Range,
    sync::{Arc, RwLock},
    time::Duration,
};

use egui::Pos2;

use crate::{
    sample::Sample,
    state::State,
    util::{PixelRange, SampleRange},
};

pub struct Track {
    pub name: String,
    pub samples: Vec<Arc<Sample>>,
    pub view_range: Range<u64>,
    pub cached_times: Vec<u64>,

    pub app_state: Arc<RwLock<State>>,
}

const TRACK_HEIGHT: f32 = 200.0;

impl Track {
    pub fn new(
        name: impl Into<String>,
        samples: Vec<Arc<Sample>>,
        app_state: Arc<RwLock<State>>,
    ) -> Track {
        let sample_times = samples
            .iter()
            .scan(0, |state, sample| {
                let micros = sample.len_time().as_micros() as u64;
                let old_state = *state;
                *state += micros;

                Some(old_state)
            })
            .collect();

        Track {
            name: name.into(),
            samples,
            app_state,
            cached_times: sample_times,

            view_range: Duration::from_secs(0).as_micros() as u64
                ..Duration::from_secs(20).as_micros() as u64,
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

    pub fn sample_at(&self, index: usize) -> &Arc<Sample> {
        &self.samples[index]
    }

    pub fn sample_at_sample_index(
        &self,
        index: usize,
        target_rate: f64,
    ) -> Option<(&Arc<Sample>, usize)> {
        let mut offset = 0;
        let ind = self.samples.iter().position(|x| {
            let factor = target_rate / x.header.sampling_rate as f64;
            if index >= offset && index < offset + (x.len() as f64 * factor) as usize {
                true
            } else {
                offset += (x.len() as f64 * factor) as usize;
                false
            }
        });

        ind.map(|i| (&self.samples[i], i))
    }

    pub fn time_rel_left(&self, absolute_time: u64) -> Option<u64> {
        absolute_time.checked_sub(self.view_range.start)
    }

    pub fn calculate_times(&mut self) {
        self.cached_times.clear();
        self.cached_times
            .extend(self.samples.iter().scan(0, |state, sample| {
                let micros = sample.len_time().as_micros() as u64;
                let old_state = *state;
                *state += micros;

                Some(old_state)
            }));
    }

    /// Get the bounds of the sample in number of sample points while respecting the view boundries
    ///
    /// `sample_index` should be the index of a sample that this track contains
    ///
    /// If the sample is outside of the view range, `None` is returned
    pub fn get_clip_sample_width(&self, sample_index: usize) -> Option<SampleRange> {
        let sample = self.sample_at(sample_index);
        let start_time = self.cached_times[sample_index];
        let end_time = start_time + sample.len_time().as_micros() as u64;

        if end_time < self.view_range.start || start_time > self.view_range.end {
            return None;
        }

        match (
            self.view_range.contains(&start_time),
            self.view_range.contains(&end_time),
        ) {
            (false, false) => Some(SampleRange {
                min: ((self.view_range.start - start_time) as f64 * sample.sample_rate).floor()
                    as u64,
                max: ((self.view_range.end - start_time) as f64 * sample.sample_rate).ceil() as u64,
            }),
            (true, false) => Some(SampleRange {
                min: 0,
                max: ((self.view_range.end - start_time) as f64 * sample.sample_rate) as u64,
            }),
            (false, true) => Some(SampleRange {
                min: ((self.view_range.start - start_time) as f64 * sample.sample_rate) as u64,
                max: sample.len() as u64,
            }),
            (true, true) => Some(SampleRange {
                min: 0,
                max: sample.len() as u64,
            }),
        }
    }

    /// Get the pixel position (horizontally) in the given width and view range of a duration
    ///
    /// `duration` is the time relative to the beginning of the track
    /// `width` is the width of the timeline
    ///
    /// If the duration does not fall inside the view range, `None` is return
    pub fn get_pixel_from_duration(&self, duration: &Duration, width: f32) -> Option<f32> {
        let micros = duration.as_micros() as u64;

        if self.view_range.contains(&micros) {
            Some(width / (self.view_range.end - self.view_range.start) as f32 * micros as f32)
        } else {
            None
        }
    }

    /// Get the pixel position (horizontally) in the given width and view range of a duration
    ///
    /// `duration` is the time relative to the beginning of the sample
    /// `width` is the width of the timeline
    ///
    /// If the duration does not fall inside the sample, `None` is return
    pub fn get_pixel_from_duration_sample(
        &self,
        sample_index: usize,
        duration: &Duration,
        width: f32,
    ) -> Option<f32> {
        let sample = self.sample_at(sample_index);
        let start_time = self.cached_times[sample_index];
        let end_time = start_time + sample.len_time().as_micros() as u64;

        let micros = duration.as_micros() as u64;

        let pixels_per_micro = width / (self.view_range.end - self.view_range.start) as f32;

        if micros < self.view_range.start
            || micros > self.view_range.end
            || micros < start_time
            || micros > end_time
        {
            return None;
        }

        match (
            self.view_range.contains(&start_time),
            self.view_range.contains(&end_time),
        ) {
            (false, false) => Some(
                pixels_per_micro * micros as f32
                    - pixels_per_micro * (self.view_range.start - start_time) as f32,
            ),
            (true, false) => Some(
                pixels_per_micro * micros as f32
                    - pixels_per_micro * (start_time - self.view_range.start) as f32,
            ),
            (false, true) => Some(
                pixels_per_micro * micros as f32
                    - pixels_per_micro * (self.view_range.start - start_time) as f32,
            ),
            (true, true) => {
                Some(pixels_per_micro * micros as f32 - pixels_per_micro * (start_time) as f32)
            }
        }
    }

    /// Get the pixel range the provided sample occupies on the timeline
    ///
    /// `sample_index` should be the index of a sample that this track contains
    ///
    /// If the sample is outside of the view range, `None` is returned
    pub fn get_clip_pixel_width(&self, width: f32, sample_index: usize) -> Option<PixelRange> {
        let sample = self.sample_at(sample_index);
        let start_time = self.cached_times[sample_index];
        let end_time = start_time + sample.len_time().as_micros() as u64;

        let pixels_per_micro = width / (self.view_range.end - self.view_range.start) as f32;

        if end_time < self.view_range.start || start_time > self.view_range.end {
            return None;
        }

        match (
            self.view_range.contains(&start_time),
            self.view_range.contains(&end_time),
        ) {
            (false, false) => Some(PixelRange {
                min: 0.0,
                max: width,
            }),
            (true, false) => Some(PixelRange {
                min: pixels_per_micro * (start_time - self.view_range.start) as f32,
                max: width,
            }),
            (false, true) => Some(PixelRange {
                min: 0.0,
                max: pixels_per_micro * (end_time - self.view_range.start) as f32,
            }),
            (true, true) => Some(PixelRange {
                min: pixels_per_micro * start_time as f32,
                max: pixels_per_micro * end_time as f32,
            }),
        }
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

                    let (rect, bg_response) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), ui.available_height()),
                        egui::Sense::drag(),
                    );
                    // Change the preview zoom on scroll
                    let (scoll_delta, bg_pos) = ui
                        .ctx()
                        .input(|input| (input.scroll_delta, input.pointer.hover_pos()));

                    if let Some(bg_pos) = bg_pos {
                        if rect.contains(bg_pos) {
                            let dist = (bg_pos.x - rect.min.x) / rect.width();

                            // Scroll faster when zoomed out more. This makes zoom feel more consistent
                            let delta = scoll_delta.y
                                * (self.view_range.end - self.view_range.start) as f32
                                / 5000.0;

                            if delta.abs() > 0.0 {
                                self.view_range.start =
                                    (self.view_range.start as f32 + delta * dist)
                                        .round()
                                        .max(0.0) as u64;

                                self.view_range.end =
                                    (self.view_range.end as f32 - delta * (1.0 - dist)).round()
                                        as u64;
                            }
                        }
                    }

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
                        let mut offset = 0;
                        let mut time_offset = 0.0;

                        ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                        for (index, sample) in self.samples.iter().enumerate() {
                            let Some(pixel_range) = self.get_clip_pixel_width(width, index) else {
                                continue;
                            };

                            let sample_response = ui.vertical(|ui| {
                                ui.spacing_mut().item_spacing = egui::vec2(0.0, 0.0);

                                let res = ui.allocate_ui_with_layout(
                                    egui::vec2(pixel_range.len().round(), 20.0),
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

                                        new_rect.set_width(
                                            pixel_range.len().max(res.response.rect.width()),
                                        );
                                        ui.allocate_rect(new_rect, egui::Sense::drag());

                                        sample.display(ui, new_rect, self, index);
                                        offset += sample.len() as usize;

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
                                    if self.sample_at_time(&duration).is_some() {
                                        if let Some(pixel) = self
                                            .get_pixel_from_duration_sample(index, &duration, width)
                                        {
                                            ui.painter().line_segment(
                                                [
                                                    Pos2::new(
                                                        pixel.round() + rect.left() + 0.5,
                                                        rect.bottom(),
                                                    ),
                                                    Pos2::new(
                                                        pixel.round() + rect.left() + 0.5,
                                                        rect.top(),
                                                    ),
                                                ],
                                                egui::Stroke::new(1.0, egui::Color32::GREEN),
                                            );
                                        }
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
