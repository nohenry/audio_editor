
struct VertexInput {
    @location(0) pos: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct Uniform {
    resolution: vec2<f32>,
    samples_per_pixel: f32,
    scale: f32,
    data_len: u32,
    increment: u32,

    main_color: vec4<f32>,
    second_color: vec4<f32>,

    bg_color: vec4<f32>,
}

@group(0) @binding(0)
var<storage, read> audio_buf: array<f32>;

@group(1) @binding(0)
var<uniform> config: Uniform;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.clip_pos = vec4<f32>(in.pos * 2.0 - 1.0, 0.0, 1.0);
    out.uv = in.pos;

    return out;
}

const sample_threshold = 10.0;
// Require at least 3 pixels per sample for drawing the draggable points.
const sample_point_threshold = 0.333333333;

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {

    let pixel = u32(in.uv.x * config.resolution.x);
    let y = i32(in.uv.y * config.resolution.y);


    // if config.samples_per_pixel <= sample_point_threshold {
    //     let xl = f32(pixel - 1u) * config.samples_per_pixel;
    //     let index = f32(pixel) * config.samples_per_pixel;

    //     let r = config.width / f32(config.data_len);
    //     let p = u32(index * r);

    //     let coord = audio_buf[u32(index)] * 100.0 + 100.0;
        
    //     var point = pixel;
    //     for (var i = pixel; i < config.data_len + pixel; i += 1u) {
    //         let xl = f32(i) * config.samples_per_pixel;
    //         let index = f32(i + 1u) * config.samples_per_pixel;
    //         let c1 = audio_buf[u32(xl)] * 100.0 + 100.0;
    //         let c2 = audio_buf[u32(index)] * 100.0 + 100.0;
    //         if u32(xl) != u32(index) && c1 == coord {
    //             point = i;
    //             break;
    //         }
    //     }
    // } else {
        
    // Calculate max, min and rms values
    var max = -1000000.0;
    var min= 1000000.0;
    var sq_sum = 0.0;

    let offset = u32(f32(pixel) * config.samples_per_pixel);
    // let increment = u32(round(1.0 / (f32(config.data_len) / config.samples_per_pixel / config.width)));
    for (var p = 0u; p < u32(config.samples_per_pixel); p += config.increment) {
        let value = audio_buf[offset + p];
        if value > max {
            max = value;
        } else if value < min {
            min = value;
        }

        sq_sum += value * value;
    }

    let rms = sqrt(sq_sum / (config.samples_per_pixel / f32(config.increment)));

    // Convert into coordinate space
    let max_coord = i32(max * config.scale) + i32(config.resolution.y / 2.0);
    let min_coord = i32(min * config.scale) + i32(config.resolution.y / 2.0);

    let max_rms = i32(rms * config.scale) + i32(config.resolution.y / 2.0);
    let min_rms = i32(rms * -config.scale) + i32(config.resolution.y / 2.0);

    if y < max_rms && y > min_rms {
        return config.second_color;
    } else if y < max_coord && y > min_coord {
        return config.main_color;
    }

    if y == i32(config.resolution.y / 2.0) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    return config.bg_color;
}
 