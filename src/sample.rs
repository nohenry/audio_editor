use std::{
    fs::File,
    io,
    path::{Path, PathBuf},
    sync::Arc,
};

use eframe::egui_wgpu;
use egui::Pos2;

use crate::track::{Track, WaveUniform, WaveViewState};

pub struct Sample {
    pub name: String,
    path: PathBuf,

    pub header: wav::Header,
    pub data: wav::BitDepth,
}

impl Sample {
    pub fn load_from_file(path: impl AsRef<Path>) -> io::Result<Sample> {
        let mut file = File::open(&path)?;
        let (header, data) = wav::read(&mut file)?;

        Ok(Sample {
            name: path
                .as_ref()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap()
                .to_string(),
            path: path.as_ref().to_path_buf(),

            header,
            data,
        })
    }

    pub fn adjusted_len(&self, track: &Track) -> u64 {
        let sample_data_len = track.view_range.end as usize;
        let adjusted_len = sample_data_len.min(self.data.as_thirty_two_float().unwrap().len());

        adjusted_len as u64
    }

    pub fn display(&self, ui: &mut egui::Ui, rect: egui::Rect, track: &Track) {
        // Require at least 1/2 pixel per sample for drawing individual samples.
        let sample_threshold = 10.0;
        // Require at least 3 pixels per sample for drawing the draggable points.
        let sample_point_threshold = 1.0 / 3.0;

        // Width of viewport
        let width = rect.width();
        let sample_data = self.data.as_thirty_two_float().unwrap();

        // Samples
        let sample_data_len = track.view_range.end as usize;

        // Y-scale factor
        let scale = 100.0;

        let main_color = egui::Color32::from_rgb(181, 20, 9);
        let second_color = egui::Color32::from_rgb(227, 91, 82);
        let bg_color = egui::Color32::from_rgba_premultiplied(0, 0, 0, 0);

        let actual_len = sample_data.len();
        let adjusted_len = sample_data_len.min(actual_len);
        let samples_per_pixel = adjusted_len as f32 / width;

        // Paint circles for individual samples if zoomed in enough
        if samples_per_pixel <= sample_point_threshold {
            ui.painter().line_segment(
                [
                    Pos2::new(rect.left(), rect.center().y + 0.5),
                    Pos2::new(rect.right(), rect.center().y + 0.5),
                ],
                egui::Stroke::new(1.0, egui::Color32::BLACK),
            );
            for (i, sample) in sample_data[..sample_data_len as usize].iter().enumerate() {
                let x = i as f32 / samples_per_pixel;
                ui.painter().line_segment(
                    [
                        Pos2::new(x + rect.left() + 0.5, rect.center().y),
                        Pos2::new(x + rect.left() + 0.5, rect.center().y + *sample * scale),
                    ],
                    egui::Stroke::new(1.0, egui::Color32::BLACK),
                );

                ui.painter().circle_filled(
                    Pos2::new(x + rect.left() + 0.5, rect.center().y + *sample * scale),
                    2.0,
                    main_color,
                )
            }
        } else if samples_per_pixel <= sample_threshold {
            // Paint lines conenct each sample
            ui.painter().line_segment(
                [
                    Pos2::new(rect.left(), rect.center().y + 0.5),
                    Pos2::new(rect.right(), rect.center().y + 0.5),
                ],
                egui::Stroke::new(1.0, egui::Color32::BLACK),
            );
            let mut last = 0.0;
            for (i, sample) in sample_data[..sample_data_len as usize].iter().enumerate() {
                let x = i as f32 / samples_per_pixel;
                ui.painter().line_segment(
                    [
                        Pos2::new(x + rect.left(), rect.center().y - last * scale),
                        Pos2::new(x + rect.left(), rect.center().y - *sample * scale),
                    ],
                    egui::Stroke::new(1.0, main_color),
                );

                last = *sample;
            }
        } else {
            // Render a shader to display larger zommed-out data
            let cb = egui_wgpu::CallbackFn::new()
                .prepare(move |device, queue, _encoder, paint_callback_resources| {
                    let bind_group = {
                        let buffer: &Arc<wgpu::Buffer> = paint_callback_resources.get().unwrap();

                        let resources: &WaveViewState = paint_callback_resources.get().unwrap();

                        queue.write_buffer(
                            &resources.uniform_buffer,
                            0,
                            bytemuck::cast_slice(&[WaveUniform {
                                width,
                                samples_per_pixel,
                                yscale: scale,
                                data_len: sample_data_len.min(actual_len) as u32,
                                increment: (1.0
                                    / (adjusted_len as f32
                                        / (sample_data_len as f32 / width)
                                        / width))
                                    .round() as u32,

                                _padding: [0; 3],

                                main_color: main_color.to_normalized_gamma_f32(),
                                second_color: second_color.to_normalized_gamma_f32(),

                                bg_color: bg_color.to_normalized_gamma_f32(),
                            }]),
                        );

                        let bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
                            label: Some("wave_view_audio_buffer"),
                            entries: &[wgpu::BindGroupEntry {
                                binding: 0,
                                resource: buffer.as_entire_binding(),
                            }],
                            layout: &resources.audio_buffer_layout,
                        });

                        bind_group
                    };
                    let resources: &mut WaveViewState = paint_callback_resources.get_mut().unwrap();
                    resources.audio_buffer_bind_group = Some(bind_group);

                    Vec::new()
                })
                .paint(move |_info, render_pass, paint_callback_resources| {
                    let resources: &WaveViewState = paint_callback_resources.get().unwrap();

                    // type_name()

                    // let val = 9330782273713993017;
                    // let ti = unsafe { std::mem::transmute::<u64, TypeId>(val) };
                    // println!("{:?} {}", paint_callback_resources, type_name());

                    resources.paint(render_pass);
                });

            let callback = egui::PaintCallback {
                rect,
                callback: Arc::new(cb),
            };

            ui.painter().add(callback);
        }
    }
}
