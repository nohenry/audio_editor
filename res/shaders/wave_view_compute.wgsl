
struct Uniform {
    resolution: vec2<f32>,

    increment: u32,
    start: u32,
    end: u32,
}

struct OutPoint {
    min: f32,
    max: f32,
    rms: f32,
}

@group(0) @binding(0)
var<storage, read> audio_buf: array<f32>;

@group(1) @binding(0)
var<uniform> config: Uniform;

@group(2) @binding(0)
var<storage, read_write> out_points: array<OutPoint>;


@compute
@workgroup_size(64)
fn compute_main(@builtin(global_invocation_id) global_id: vec3<u32>) {
    let pixel = global_id.x;

    var out_point: OutPoint;
    out_point.max = -100000.0;
    out_point.min = 100000.0;
    out_point.rms = 0.0;

    let samples_per_pixel = f32((config.end - config.start)) / config.resolution.x;
    let offset = u32(round(f32(pixel) * samples_per_pixel));

    // var count = 0u;
    for (var p = 0u; p < u32(samples_per_pixel); p += config.increment) {
        let value = audio_buf[config.start + offset + p];
        if value > out_point.max {
            out_point.max = value;
        } else if value < out_point.min {
            out_point.min = value;
        }

        // out_point.rms += value * value;
        // count += 1u;
    }

    // let rms = sqrt(out_point.rms / f32(count) / 2.0);

    if out_point.max <= -90000.0 {
        out_point.max = 0.0;
    }
    if out_point.min >= 90000.0 {
        out_point.min = 0.0;
    }

    out_point.max = max(0.0, out_point.max);
    out_point.min = min(0.0, out_point.min);

    out_points[pixel] = out_point;
}