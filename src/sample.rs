use std::{
    collections::{hash_map::Entry, HashMap},
    fs::File,
    io,
    num::NonZeroU64,
    path::{Path, PathBuf},
    sync::{Arc, RwLock},
    time::Duration,
};

use bytemuck::Zeroable;
use eframe::egui_wgpu;
use egui::Pos2;
use tracing::{info, warn};
use wgpu::util::DeviceExt;

use crate::{
    id::{get_id_mgr, Id},
    state::State,
    track::Track,
    wave_view::{WaveComputeUniform, WaveUniform, WaveViewState},
};

pub struct WaveViewSampleState {
    compute_uniform_buffer: wgpu::Buffer,
    compute_uniform_bind_group: wgpu::BindGroup,

    draw_uniform_buffer: wgpu::Buffer,
    draw_uniform_bind_group: wgpu::BindGroup,

    _audio_buffer: wgpu::Buffer,
    audio_bind_group: wgpu::BindGroup,

    compute_output_buffer: wgpu::Buffer,
    compute_output_bind_group: wgpu::BindGroup,

    wave_state: Arc<WaveViewState>,
}

impl WaveViewSampleState {
    pub fn uniform_bind_group(&self) -> &wgpu::BindGroup {
        &self.draw_uniform_bind_group
    }

    pub fn audio_bind_group(&self) -> &wgpu::BindGroup {
        &self.audio_bind_group
    }

    pub fn paint<'rp>(&'rp self, rpass: &mut wgpu::RenderPass<'rp>) {
        rpass.set_pipeline(&self.wave_state.draw_pipeline);
        rpass.set_vertex_buffer(0, self.wave_state.vertex_buffer.slice(..));

        rpass.set_bind_group(0, self.audio_bind_group(), &[]);
        rpass.set_bind_group(1, self.uniform_bind_group(), &[]);
        rpass.set_bind_group(2, &self.compute_output_bind_group, &[]);

        rpass.draw(0..4, 0..1);
    }
}

pub struct Sample {
    pub id: Id,

    pub name: String,
    _path: PathBuf,

    pub header: wav::Header,
    pub data: wav::BitDepth,

    pub sample_rate: f64,

    wgpu_state: Arc<WaveViewSampleState>,
}

impl Sample {
    pub fn load_from_file(
        path: impl AsRef<Path>,
        name: Option<impl ToString>,
        app_state: &Arc<RwLock<State>>,
    ) -> io::Result<Sample> {
        let mut file = File::open(&path)?;
        let (header, data) = wav::read(&mut file)?;

        let audio_len = data.as_thirty_two_float().unwrap().len();
        let app_state = app_state.read().unwrap();
        let audio_buffer =
            app_state
                .wgpu_ctx
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("wave_view_aduio_buffer"),
                    contents: bytemuck::cast_slice(data.as_thirty_two_float().unwrap()),
                    usage: wgpu::BufferUsages::STORAGE,
                });

        let audio_bind_group =
            app_state
                .wgpu_ctx
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("wave_view_audio_buffer"),
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: audio_buffer.as_entire_binding(),
                    }],
                    layout: &app_state.wave_view_state.audio_buffer_layout,
                });

        let draw_uniform_buffer =
            app_state
                .wgpu_ctx
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("wave_form_uniform_buffer"),
                    contents: bytemuck::cast_slice(&[WaveUniform::zeroed()]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let draw_uniform_bind_group =
            app_state
                .wgpu_ctx
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("wave_view_uniform_buffer_bind_group"),
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: draw_uniform_buffer.as_entire_binding(),
                    }],
                    layout: &app_state.wave_view_state.draw_uniform_layout,
                });

        let compute_uniform_buffer =
            app_state
                .wgpu_ctx
                .device
                .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                    label: Some("wave_form_compute_uniform_buffer"),
                    contents: bytemuck::cast_slice(&[WaveUniform::zeroed()]),
                    usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
                });

        let compute_uniform_bind_group =
            app_state
                .wgpu_ctx
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("wave_view_compute_uniform_buffer_bind_group"),
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: compute_uniform_buffer.as_entire_binding(),
                    }],
                    layout: &app_state.wave_view_state.compute_uniform_layout,
                });

        let compute_output_buffer =
            app_state
                .wgpu_ctx
                .device
                .create_buffer(&wgpu::BufferDescriptor {
                    label: Some("wave_form_compute_output_buffer"),
                    mapped_at_creation: false,
                    size: audio_len as u64 / 4 * 4,
                    usage: wgpu::BufferUsages::STORAGE,
                });

        let compute_output_bind_group =
            app_state
                .wgpu_ctx
                .device
                .create_bind_group(&wgpu::BindGroupDescriptor {
                    label: Some("wave_view_compute_output_bind_group"),
                    entries: &[wgpu::BindGroupEntry {
                        binding: 0,
                        resource: compute_output_buffer.as_entire_binding(),
                    }],
                    layout: &app_state.wave_view_state.compute_output_buffer_layout,
                });

        Ok(Sample {
            id: get_id_mgr().gen_id(),
            name: name.map(|n| n.to_string()).unwrap_or_else(|| {
                path.as_ref()
                    .file_name()
                    .unwrap()
                    .to_str()
                    .unwrap()
                    .to_string()
            }),
            _path: path.as_ref().to_path_buf(),

            header,
            data,

            sample_rate: header.sampling_rate as f64 / 1000.0 / 1000.0,

            wgpu_state: Arc::new(WaveViewSampleState {
                compute_uniform_buffer,
                compute_uniform_bind_group,

                draw_uniform_buffer,
                draw_uniform_bind_group,

                _audio_buffer: audio_buffer,
                audio_bind_group,

                compute_output_buffer,
                compute_output_bind_group,

                wave_state: app_state.wave_view_state.clone(),
            }),
        })
    }

    pub fn adjusted_len(&self, track: &Track) -> u64 {
        let sample_data_len = track.view_range.end as usize;
        let adjusted_len = sample_data_len.min(self.len());

        adjusted_len as u64
    }

    pub fn len(&self) -> usize {
        match &self.data {
            wav::BitDepth::ThirtyTwoFloat(data) => data.len(),
            _ => 0,
        }
    }

    pub fn len_time(&self) -> Duration {
        let secs = self.len() as f64 / self.header.sampling_rate as f64;
        Duration::from_secs_f64(secs)
    }

    pub fn view_updated(&self, ui: &mut egui::Ui, rect: egui::Rect, track: &Track, index: usize) {
        info!("Updating View...");
        let Some(range) = track.get_clip_sample_width(index) else {
            return;
        };

        let state = track.app_state.read().unwrap();

        state.wgpu_ctx.queue.write_buffer(
            &self.wgpu_state.compute_uniform_buffer,
            0,
            bytemuck::cast_slice(&[WaveComputeUniform {
                width: rect.width(),
                height: rect.height(),
                increment: 1,
                // increment: (1.0
                //     / (adjusted_len as f32 / (sample_data_len as f32 / width) / width))
                //     .round() as u32,
                start: range.min as _,
                end: range.max as _,
                _padding: [0; 3],
            }]),
        );

        let mut encoder = state
            .wgpu_ctx
            .device
            .create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let mut cpass = encoder.begin_compute_pass(&wgpu::ComputePassDescriptor::default());

            cpass.set_pipeline(&self.wgpu_state.wave_state.compute_pipeline);
            cpass.set_bind_group(0, &self.wgpu_state.audio_bind_group, &[]);
            cpass.set_bind_group(1, &self.wgpu_state.compute_uniform_bind_group, &[]);
            cpass.set_bind_group(2, &self.wgpu_state.compute_output_bind_group, &[]);

            cpass.dispatch_workgroups(rect.width() as _, 1, 1);
        }

        state.wgpu_ctx.queue.submit(Some(encoder.finish()));
    }

    pub fn display(
        &self,
        ui: &mut egui::Ui,
        rect: egui::Rect,
        track: &Track,
        index: usize,
        // sample_offset: usize,
        // cutoff_offset: usize,
    ) -> usize {
        // Require at least 1/2 pixel per sample for drawing individual samples.
        let _sample_threshold = 10.0;
        // Require at least 3 pixels per sample for drawing the draggable points.
        let sample_point_threshold = 1.0 / 3.0;

        // Width of viewport
        let width = rect.width();
        let height = rect.height();
        let sample_data = self.data.as_thirty_two_float().unwrap();

        // Samples
        // let sample_data_len = track.view_range.end - sample_offset;

        // Y-scale factor
        let scale = height / 2.0 - 10.0;
        // println!("{}", scale);

        let main_color = egui::Color32::from_rgb(181, 20, 9);

        let second_color = egui::Color32::from_rgb(227, 91, 82);
        let bg_color = egui::Color32::from_rgba_premultiplied(0, 0, 0, 0);

        let actual_len = sample_data.len();
        // let adjusted_len = sample_data_len.min(actual_len);

        let Some(range) = track.get_clip_sample_width(index) else {
            return actual_len;
        };

        let samples_per_pixel = range.len() as f32 / width;
        // Paint circles for individual samples if zoomed in enough
        if samples_per_pixel <= sample_point_threshold {
            ui.painter().line_segment(
                [
                    Pos2::new(rect.left(), rect.center().y + 0.5),
                    Pos2::new(rect.right(), rect.center().y + 0.5),
                ],
                egui::Stroke::new(1.0, egui::Color32::BLACK),
            );
            for (i, sample) in sample_data[range.min as usize..range.max as usize]
                .iter()
                .enumerate()
            {
                let x = i as f32 / samples_per_pixel + rect.left();

                ui.painter().line_segment(
                    [
                        Pos2::new(x + 0.5, rect.center().y),
                        Pos2::new(x + 0.5, rect.center().y - *sample * scale),
                    ],
                    egui::Stroke::new(1.0, egui::Color32::BLACK),
                );

                ui.painter().circle_filled(
                    Pos2::new(x + 0.5, rect.center().y - *sample * scale),
                    2.0,
                    main_color,
                )
            }
        // } else if samples_per_pixel <= sample_threshold {
        //     //Paint lines conenct each sample
        //     ui.painter().line_segment(
        //         [
        //             Pos2::new(rect.left(), rect.center().y + 0.5),
        //             Pos2::new(rect.right(), rect.center().y + 0.5),
        //         ],
        //         egui::Stroke::new(1.0, egui::Color32::BLACK),
        //     );
        //     let mut last = 0.0;
        //     for (i, sample) in sample_data[range.min as usize..range.max as usize]
        //         .iter()
        //         .enumerate()
        //     {
        //         let x = i as f32 / samples_per_pixel;
        //         ui.painter().line_segment(
        //             [
        //                 Pos2::new(x + rect.left(), rect.center().y - last * scale),
        //                 Pos2::new(x + rect.left(), rect.center().y - *sample * scale),
        //             ],
        //             egui::Stroke::new(1.0, main_color),
        //         );

        //         last = *sample;
        //     }
        } else {
            let wave_state = self.wgpu_state.clone();
            let id = self.id;
            // let start = track.view_range.start as u32;

            // Render a shader to display larger zommed-out data
            let cb = egui_wgpu::CallbackFn::new()
                .prepare(move |_device, queue, _encoder, paint_callback_resources| {
                    let uniform = wave_state.as_ref();
                    let uniform = &uniform.draw_uniform_buffer;

                    queue.write_buffer(
                        uniform,
                        0,
                        bytemuck::cast_slice(&[WaveUniform {
                            width,
                            height,
                            yscale: scale,

                            start: range.min as u32,
                            end: range.max as u32,

                            _padding: [0; 3],

                            main_color: main_color.to_normalized_gamma_f32(),
                            second_color: second_color.to_normalized_gamma_f32(),

                            bg_color: bg_color.to_normalized_gamma_f32(),
                        }]),
                    );

                    let map: &mut HashMap<Id, Arc<WaveViewSampleState>> =
                        paint_callback_resources.get_mut().unwrap();

                    match map.entry(id) {
                        Entry::Occupied(_) => (),
                        Entry::Vacant(entry) => {
                            entry.insert(wave_state.clone());
                        }
                    }

                    Vec::new()
                })
                .paint(move |_info, render_pass, paint_callback_resources| {
                    let resources: &HashMap<Id, Arc<WaveViewSampleState>> =
                        paint_callback_resources.get().unwrap();

                    let id = id;

                    if let Some(sample) = resources.get(&id) {
                        sample.paint(render_pass);
                    }
                });

            let callback = egui::PaintCallback {
                rect,
                callback: Arc::new(cb),
            };

            ui.painter().add(callback);
        }

        actual_len
    }
}
