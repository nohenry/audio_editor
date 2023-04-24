use bytemuck::{Pod, Zeroable};
use eframe::egui_wgpu::RenderState;
use wgpu::util::DeviceExt;

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
    pub height: f32,
    // pub samples_per_pixel: f32,
    pub yscale: f32,
    pub data_len: u32,
    pub increment: u32,
    pub start: u32,
    pub end: u32,

    pub _padding: [u32; 1],

    pub main_color: [f32; 4],
    pub second_color: [f32; 4],

    pub bg_color: [f32; 4],
}

pub struct WaveViewState {
    pub pipeline: wgpu::RenderPipeline,

    pub vertex_buffer: wgpu::Buffer,

    pub uniform_layout: wgpu::BindGroupLayout,
    pub audio_buffer_layout: wgpu::BindGroupLayout,
}

impl WaveViewState {
    pub fn new(render_state: &RenderState) -> WaveViewState {
        let device = &render_state.device;

        let shader = device.create_shader_module(wgpu::ShaderModuleDescriptor {
            label: Some("wave_view_shader_module"),
            source: wgpu::ShaderSource::Wgsl(include_str!("../res/shaders/wave_view.wgsl").into()),
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
                    visibility: wgpu::ShaderStages::FRAGMENT,
                }],
            });

        let uniform_layout = device.create_bind_group_layout(&wgpu::BindGroupLayoutDescriptor {
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
            bind_group_layouts: &[&audio_buffer_layout, &uniform_layout],
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
            pipeline,
            vertex_buffer,

            uniform_layout,
            audio_buffer_layout,
        }
    }
}
