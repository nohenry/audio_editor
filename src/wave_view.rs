use bytemuck::{Pod, Zeroable};
use eframe::egui_wgpu::RenderState;
use wgpu::util::DeviceExt;

/// These are the verticies used for TRIANGLE_STRIP display to the screen
const VERTICIES: [f32; 8] = [
    1.0, 0.0, // top right
    0.0, 0.0, // top left
    1.0, 1.0, // bottom right
    0.0, 1.0, // bottom left
];

/// The structure that represents a wave view uniform sent to the drawing frag shader
/// 
/// `width` and `height` are the dimensions of the view in pixels
/// `yscale` is the scale of the amplitude of the wave
/// `start` and `end` are the range (in samples) to index into the audio buffer
/// `main_color` is the color of the waves
/// `second_color` is the color of rms
/// `bg_color` is the background color
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct WaveUniform {
    pub width: f32,
    pub height: f32,
    pub yscale: f32,
    pub start: u32,
    pub end: u32,

    pub _padding: [u32; 3],

    pub main_color: [f32; 4],
    pub second_color: [f32; 4],

    pub bg_color: [f32; 4],
}

/// This represents the uniform for the min/max/rms compute shader
/// Fields are similar to those in `WaveUniform`
#[repr(C)]
#[derive(Clone, Copy, Pod, Zeroable)]
pub struct WaveComputeUniform {
    pub width: f32,
    pub height: f32,

    pub increment: u32,
    pub start: u32,
    pub end: u32,

    pub _padding: [u32; 3],
}

/// This is the state which contains common things for rendering
pub struct WaveViewState {
    pub draw_pipeline: wgpu::RenderPipeline,
    pub compute_pipeline: wgpu::ComputePipeline,

    pub vertex_buffer: wgpu::Buffer,

    pub draw_uniform_layout: wgpu::BindGroupLayout,
    pub audio_buffer_layout: wgpu::BindGroupLayout,
    pub compute_uniform_layout: wgpu::BindGroupLayout,
    pub compute_output_buffer_layout: wgpu::BindGroupLayout,
}

impl WaveViewState {
    pub fn new(render_state: &RenderState) -> WaveViewState {
        let device = &render_state.device;

        let draw_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wave_view_shader_module"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../res/shaders/wave_view.wgsl").into()),
        });

        let compute_shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wave_view_compute_shader_module"),
            source: wgpu::ShaderSource::Wgsl(
                include_str!("../res/shaders/wave_view_compute.wgsl").into(),
            ),
        });

        let audio_buffer_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("wave_view_audio_buffer_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    count: None,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: true },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                }],
            });

        let compute_output_buffer_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("wave_view_compute_output_buffer_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    count: None,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Storage { read_only: false },
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                }],
            });

        let draw_uniform_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("wave_view_draw_uniform_layout"),
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

        let compute_uniform_layout =
            device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
                label: Some("wave_view_compute_uniform_layout"),
                entries: &[wgpu::BindGroupLayoutEntry {
                    binding: 0,
                    count: None,
                    ty: wgpu::BindingType::Buffer {
                        ty: wgpu::BufferBindingType::Uniform,
                        has_dynamic_offset: false,
                        min_binding_size: None,
                    },
                    visibility: wgpu::ShaderStages::FRAGMENT | wgpu::ShaderStages::COMPUTE,
                }],
            });

        let vertex_buffer = device.create_buffer_init(&wgpu::util::BufferInitDescriptor {
            label: Some("wave_view_vertex_buffer"),
            contents: bytemuck::cast_slice(&VERTICIES),
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

        let compute_pipeline_layout =
            device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
                label: Some("wave_view_compute_pipeline_layout"),
                bind_group_layouts: &[
                    &audio_buffer_layout,
                    &compute_uniform_layout,
                    &compute_output_buffer_layout,
                ],
                push_constant_ranges: &[],
            });

        let compute_pipeline = device.create_compute_pipeline(&wgpu::ComputePipelineDescriptor {
            label: Some("wave_view_compute_pipeline"),
            layout: Some(&compute_pipeline_layout),
            module: &compute_shader,
            entry_point: "compute_main",
        });

        let draw_pipeline_layout = device.create_pipeline_layout(&wgpu::PipelineLayoutDescriptor {
            label: Some("wave_view_pipeline_layout"),
            bind_group_layouts: &[
                &audio_buffer_layout,
                &draw_uniform_layout,
                &compute_output_buffer_layout,
            ],
            push_constant_ranges: &[],
        });

        let draw_pipeline = device.create_render_pipeline(&wgpu::RenderPipelineDescriptor {
            label: Some("wave_view_pipeline"),
            layout: Some(&draw_pipeline_layout),
            depth_stencil: None,
            fragment: Some(wgpu::FragmentState {
                module: &draw_shader,
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
                module: &draw_shader,
                entry_point: "vs_main",
                buffers: &[vertex_buffer_lauout],
            },
        });

        WaveViewState {
            draw_pipeline,
            compute_pipeline,

            vertex_buffer,

            draw_uniform_layout,
            audio_buffer_layout,
            compute_uniform_layout,
            compute_output_buffer_layout,
        }
    }
}
