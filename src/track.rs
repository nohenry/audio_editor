use std::{any::TypeId, ops::Range, sync::Arc};

use bytemuck::{Pod, Zeroable};
use eframe::egui_wgpu::{self, RenderState, Renderer};
use egui::{
    mutex::{Mutex, RwLock},
    Pos2,
};
use wgpu::util::DeviceExt;

use crate::{main, sample::Sample, state::State};

pub struct Track {
    pub name: String,
    pub sample: Arc<Sample>,
    pub view_range: Range<isize>,

    pub app_state: Arc<RwLock<State>>,

    pub audio_buffer: Arc<wgpu::Buffer>,
}

const TRACK_HEIGHT: f32 = 200.0;

impl Track {
    pub fn new(
        name: impl Into<String>,
        sample: Arc<Sample>,
        app_state: Arc<RwLock<State>>,
    ) -> Track {
        let buffer = {
            let state = app_state.read();
            let buffer =
                state
                    .wgpu_ctx
                    .device
                    .create_buffer_init(&wgpu::util::BufferInitDescriptor {
                        label: Some("wave_view_aduio_buffer"),
                        // contents: bytemuck::cast_slice(&sample_data),
                        contents: bytemuck::cast_slice(&sample.data.as_thirty_two_float().unwrap()),
                        usage: wgpu::BufferUsages::STORAGE,
                    });

            let buffer = Arc::new(buffer);
            state
                .wgpu_ctx
                .renderer
                .write()
                .paint_callback_resources
                .insert(buffer.clone());

            buffer
        };

        Track {
            name: name.into(),
            sample,
            app_state,
            view_range: 0..50000,
            audio_buffer: buffer,
        }
    }

    pub fn ui(&mut self, ui: &mut egui::Ui) -> egui::Response {
        let frame = egui::containers::Frame {
            // inner_margin: egui::Margin::same(10.0),
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

                    let (rect, response) = ui.allocate_exact_size(
                        egui::vec2(ui.available_width(), ui.available_height()),
                        egui::Sense::drag(),
                    );
                    // Change the preview zoom on scroll
                    let scoll_delta = ui.ctx().input(|input| input.scroll_delta);
                    self.view_range.end = self.view_range.end
                        + (scoll_delta.y
                            * (10000.0
                                * (self.view_range.len() as f32
                                    / self.sample.data.as_thirty_two_float().unwrap().len()
                                        as f32))) as isize;

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
                    println!("{:?}", rect);

                    // Samples
                    let sample_data_len = self.view_range.end as usize;
                    let samples_per_pixel = sample_data_len as f32 / width;
                    let pixels_per_millis =
                        self.sample.header.sampling_rate as f32 / samples_per_pixel / 1000.0;

                    let sample_response = ui.allocate_ui_at_rect(rect, |ui| {
                        let sample_width = self.sample.adjusted_len(self) as f32
                            / (self.view_range.len() as f32 / ui.available_width()) ;

                        ui.vertical(|ui| {
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
                                            ui.label(&self.sample.name);
                                        });
                                },
                            );

                            // println!("{:?}", ui.max_rect());
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
                                .show(ui, |ui| {
                                    let mut new_rect = rect;
                                    new_rect.min.y += res.response.rect.height();
                                    new_rect.set_width(sample_width.max(res.response.rect.width()));
                                    ui.allocate_rect(new_rect, egui::Sense::drag());

                                    self.sample.display(ui, new_rect, self);

                                    new_rect
                                });
                            frame_response
                        })
                        .inner
                    });

                    // ui.painter().debug_rect(ui.max_rect(), egui::Color32::KHAKI, "this rect");
                    // ui.painter().debug_rect(ui.available_rect_before_wrap(), egui::Color32::LIGHT_BLUE, "this rect");
                    let rect = sample_response.inner.inner;
                    let response = sample_response.inner.response;

                    let state = self.app_state.read();
                    if state.playing {
                        if let (Some(start_time), Some(current_time)) =
                            (&state.play_time, &state.current_time)
                        {
                            if let Some(duration) = current_time.duration_since(start_time) {
                                let millis = duration.as_millis() as f32;
                                let x = (millis * pixels_per_millis + rect.left()).round() + 0.5;

                                ui.painter().line_segment(
                                    [Pos2::new(x, rect.bottom()), Pos2::new(x, rect.top())],
                                    egui::Stroke::new(1.0, egui::Color32::GREEN),
                                );
                            }
                        }
                    }

                    if let Some(pos) = response.hover_pos() {
                        let x = pos.x + 0.5;

                        ui.painter().line_segment(
                            [Pos2::new(x, rect.bottom()), Pos2::new(x, rect.top())],
                            egui::Stroke::new(1.0, egui::Color32::RED),
                        );
                    }

                    // ui.set_min_width(ui.available_width());
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

const VERTICIES: [f32; 8] = [
    1.0, 0.0, // top right
    0.0, 0.0, // top left
    1.0, 1.0, // bottom right
    0.0, 1.0, // bottom left
];

#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct WaveUniform {
    pub width: f32,
    pub samples_per_pixel: f32,
    pub yscale: f32,
    pub data_len: u32,
    pub increment: u32,

    pub _padding: [u32; 3],

    pub main_color: [f32; 4],
    pub second_color: [f32; 4],

    pub bg_color: [f32; 4],
}

pub struct WaveViewState {
    renderer: Arc<RwLock<Renderer>>,
    pipeline: wgpu::RenderPipeline,

    vertex_buffer: wgpu::Buffer,

    pub uniform_buffer: wgpu::Buffer,
    pub uniform_bind_group: wgpu::BindGroup,

    pub audio_buffer_layout: wgpu::BindGroupLayout,
    pub audio_buffer_bind_group: Option<wgpu::BindGroup>,
}

impl WaveViewState {
    pub fn run(&self, device: &wgpu::Device, queue: &wgpu::Queue, texture: &wgpu::Texture) {
        let mut encoder = device.create_command_encoder(&wgpu::CommandEncoderDescriptor::default());

        {
            let view = texture.create_view(&wgpu::TextureViewDescriptor::default());

            let mut rpass = encoder.begin_render_pass(&wgpu::RenderPassDescriptor {
                label: Some("wave_view_render_pass"),
                depth_stencil_attachment: None,
                color_attachments: &[Some(wgpu::RenderPassColorAttachment {
                    ops: wgpu::Operations {
                        // load: wgpu::LoadOp::Clear(wgpu::Color::WHITE),
                        load: wgpu::LoadOp::Load,
                        store: true,
                    },
                    resolve_target: None,
                    view: &view,
                })],
            });

            rpass.set_pipeline(&self.pipeline);
            rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

            rpass.draw(0..4, 0..1);
        }

        queue.submit(Some(encoder.finish()));
    }

    pub fn paint<'rp>(&'rp self, rpass: &mut wgpu::RenderPass<'rp>) {
        rpass.set_pipeline(&self.pipeline);
        rpass.set_vertex_buffer(0, self.vertex_buffer.slice(..));

        rpass.set_bind_group(0, self.audio_buffer_bind_group.as_ref().unwrap(), &[]);
        rpass.set_bind_group(1, &self.uniform_bind_group, &[]);
        let render_state = self.renderer.read();

        // rpass.set_bind_group(2, &render_state. , offsets)

        rpass.draw(0..4, 0..1);
    }
}

pub fn init_wave_view_wgpu(render_state: &RenderState) -> WaveViewState {
    let device = &render_state.device;

    let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
        label: Some("wave_view_shader_module"),
        source: wgpu::ShaderSource::Wgsl(include_str!("../res/shaders/wave_view.wgsl").into()),
    });

    // let audio_buffer = device.create_buffer(&wgpu::)
    let audio_buffer_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("wave_view_audio_buffer_layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            count: None,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Storage { read_only: true },
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            visibility: wgpu::ShaderStages::FRAGMENT,
        }],
    });

    let uniform_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("wave_form_uniform_buffer"),
        contents: bytemuck::cast_slice(&[WaveUniform::zeroed()]),
        usage: wgpu::BufferUsages::UNIFORM | wgpu::BufferUsages::COPY_DST,
    });

    let uniform_buffer_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
        label: Some("wave_view_uniform_buffer_layout"),
        entries: &[wgpu::BindGroupLayoutEntry {
            binding: 0,
            count: None,
            ty: wgpu::BindingType::Buffer {
                ty: wgpu::BufferBindingType::Uniform,
                has_dynamic_offset: false,
                min_binding_size: None,
            },
            visibility: wgpu::ShaderStages::FRAGMENT,
        }],
    });

    let uniform_bind_group = device.create_bind_group(&wgpu::BindGroupDescriptor {
        label: Some("wave_view_uniform_bind_group"),
        layout: &uniform_buffer_layout,
        entries: &[wgpu::BindGroupEntry {
            binding: 0,
            resource: uniform_buffer.as_entire_binding(),
        }],
    });

    let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
        label: Some("wave_view_vertex_buffer"),
        contents: &bytemuck::cast_slice(&VERTICIES),
        usage: wgpu::BufferUsages::VERTEX,
    });

    let vertex_buffer_lauout = wgpu::VertexBufferLayout {
        step_mode: wgpu::VertexStepMode::Vertex,
        attributes: &[wgpu::VertexAttribute {
            offset: 0,
            shader_location: 0,
            format: wgpu::VertexFormat::Float32x2,
        }],
        array_stride: 2 * std::mem::size_of::<f32>() as u64,
    };

    let pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
        label: Some("wave_view_pipeline_layout"),
        bind_group_layouts: &[&audio_buffer_layout, &uniform_buffer_layout],
        push_constant_ranges: &[],
    });

    let pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
        label: Some("wave_view_pipeline"),
        layout: Some(&pipeline_layout),
        depth_stencil: None,
        fragment: Some(wgpu::FragmentState {
            module: &shader,
            entry_point: "fs_main",
            targets: &[Some(wgpu::ColorTargetState {
                format: render_state.target_format,
                blend: Some(wgpu::BlendState::PREMULTIPLIED_ALPHA_BLENDING),
                write_mask: wgpu::ColorWrites::ALL,
            })],
        }),
        multisample: wgpu::MultisampleState::default(),
        multiview: None,
        primitive: wgpu::PrimitiveState {
            conservative: false,
            cull_mode: None,
            front_face: wgpu::FrontFace::Ccw,
            polygon_mode: wgpu::PolygonMode::Fill,
            strip_index_format: None,
            topology: wgpu::PrimitiveTopology::TriangleStrip,
            unclipped_depth: false,
        },
        vertex: wgpu::VertexState {
            module: &shader,
            entry_point: "vs_main",
            buffers: &[vertex_buffer_lauout],
        },
    });

    WaveViewState {
        renderer: render_state.renderer.clone(),
        pipeline,
        vertex_buffer,

        uniform_buffer,
        uniform_bind_group,

        audio_buffer_layout,
        audio_buffer_bind_group: None,
    }
}
