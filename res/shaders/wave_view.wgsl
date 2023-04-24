
struct VertexInput {
    @location(0) pos: vec2<f32>,
}

struct VertexOutput {
    @builtin(position) clip_pos: vec4<f32>,
    @location(0) uv: vec2<f32>,
}

struct Uniform {
    resolution: vec2<f32>,
    // samples_per_pixel: f32,
    scale: f32,
    data_len: u32,
    increment: u32,
    start: u32,
    end: u32,

    main_color: vec4<f32>,
    second_color: vec4<f32>,

    bg_color: vec4<f32>,
}

fn map(value: f32, istart: f32, istop: f32, ostart: f32, ostop: f32) -> f32 {
    return ostart + (ostop - ostart) * ((value - istart) / (istop - istart));
}

fn DistanceToLineSegment(p0: vec2<f32>, p1: vec2<f32>, p: vec2<f32>) -> f32
{
    let distanceP0 = length(p0 - p);
    let distanceP1 = length(p1 - p);
    
    let l2 =pow(length(p0 - p1), 2.0);
    let t = max(0.0, min(1.0, dot(p - p0, p1 - p0) / l2));
    let projection = p0 + t * (p1 - p0); 
    let distanceToProjection = length(projection - p);
    
    return min(min(distanceP0, distanceP1), distanceToProjection);
}

fn Function(x: f32) -> f32
{
    let f = map(x, 0.0, 1.0, f32(config.start), f32(config.end));
    let fl = u32(floor(f));
    let fr = fract(f);
    return (mix(audio_buf[fl], audio_buf[fl + 1u], fr) * config.scale / config.resolution.y + 0.5) ;
}

fn DistanceToFunction(p: vec2<f32>, xDelta: f32,) -> f32
{
    var result = 100.0;
    
    for (var i = -3.0; i < 3.0; i += 1.0)
    {
        var q = p;
        q.x += xDelta * i;
        
        let p0 = vec2(q.x, Function(q.x));
    	let p1 = vec2(q.x + xDelta, Function(q.x + xDelta));
        result = min(result, DistanceToLineSegment(p0, p1, p));
    }

    return result;
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


    // if true {
    //     return vec4<f32>(1.0, 0.0, 0.0, 1.0);
    // }

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
    var max = -100000.0;
    var min = 100000.0;
    var sq_sum = 0.0;

    let skip = 5u;
    let samples_per_pixel = f32((config.end - config.start)) / config.resolution.x;
    let pixels_per_sample = 1.0 / samples_per_pixel;
    let offset = u32(round(f32(pixel) * samples_per_pixel));

    if samples_per_pixel < sample_threshold {

        let distanceToPlot = DistanceToFunction(in.uv, (config.resolution.y / config.resolution.x) / config.resolution.x);
        var intensity = smoothstep(0., 1., 1. - distanceToPlot * config.resolution.y);
        intensity = pow(intensity,1./2.2);

        // if distanceToPlot < 0.01 {
        //     return vec4<f32>(config.main_color.xyz, intensity);
        // }

        if y == i32(config.resolution.y / 2.0) {
            return mix(vec4<f32>(0.0, 0.0, 0.0, 1.0), config.main_color, intensity);
        }
        return mix(config.bg_color, config.main_color, intensity);
    } else {
        var count = 0u;
        for (var p = 0u; p < u32(samples_per_pixel); p += config.increment) {
            let value = audio_buf[config.start + offset + p];
            if value > max {
                max = value;
            } else if value < min {
                min = value;
            }

            sq_sum += value * value;
            count += 1u;
        }

        let rms = sqrt(sq_sum / f32(count) / 2.0);

        if max <= -90000.0 {
            max = 0.0;
        }
        if min >= 90000.0 {
            min = 0.0;
        }

        // Convert into coordinate space
        let max_coord = i32(max * config.scale) + i32(config.resolution.y / 2.0);
        let min_coord = i32(min * config.scale) + i32(config.resolution.y / 2.0);

        let max_rms = i32(rms * config.scale) + i32(config.resolution.y / 2.0);
        let min_rms = i32(rms * -config.scale) + i32(config.resolution.y / 2.0);

        if y < max_rms && y > min_rms {
            return config.second_color;
        } else if y <= max_coord && y >= min_coord {
            return config.main_color;
        }
    }
    if y == i32(config.resolution.y / 2.0) {
        return vec4<f32>(0.0, 0.0, 0.0, 1.0);
    }
    return config.bg_color;
}
 
