#version 310 es

precision highp float;
precision highp int;

struct VertexInput {
    vec2 pos;
};
struct VertexOutput {
    vec4 clip_pos;
    vec2 uv;
};
struct Uniform {
    vec2 resolution;
    float scale;
    uint data_len;
    uint increment;
    uint start;
    uint end;
    vec4 main_color;
    vec4 second_color;
    vec4 bg_color;
};
layout(std430) readonly buffer type_5_block_0Fragment { float _group_0_binding_0_fs[]; };

uniform Uniform_block_1Fragment { Uniform _group_1_binding_0_fs; };

layout(location = 0) smooth in vec2 _vs2fs_location0;
layout(location = 0) out vec4 _fs2p_location0;

float map(float value, float istart, float istop, float ostart, float ostop) {
    return (ostart + ((ostop - ostart) * ((value - istart) / (istop - istart))));
}

bool line(float x, float y, float x0_, float y0_, float x1_, float y1_, float width, float hwidth) {
    float m = ((y0_ - y1_) / (x0_ - x1_));
    float c = (y0_ - (m * x0_));
    vec2 line_1 = vec2(((y - c) / m), ((x * m) + c));
    return (((y > (line_1.y - hwidth)) && (y < (line_1.y + hwidth))) || ((x > (line_1.x - hwidth)) && (x < (line_1.x + hwidth))));
}

bool line_segment(vec2 p_1, vec2 a, vec2 b, float thickness) {
    vec2 pa = (p_1 - a);
    vec2 ba = (b - a);
    float len = length(ba);
    vec2 dir = (ba / vec2(len));
    float t = dot(pa, dir);
    vec2 n = vec2(-(dir.y), dir.x);
    float factor = max(abs(n.x), abs(n.y));
    float distThreshold = (((thickness - 1.0) + factor) * 0.5);
    float proj = dot(n, pa);
    return ((((t > 0.0) && (t < len)) && (proj <= distThreshold)) && (proj > -(distThreshold)));
}

float DistanceToLineSegment(vec2 p0_, vec2 p1_, vec2 p_2) {
    float distanceP0_ = length((p0_ - p_2));
    float distanceP1_ = length((p1_ - p_2));
    float l2_ = pow(length((p0_ - p1_)), 2.0);
    float t_1 = max(0.0, min(1.0, (dot((p_2 - p0_), (p1_ - p0_)) / l2_)));
    vec2 projection = (p0_ + (t_1 * (p1_ - p0_)));
    float distanceToProjection = length((projection - p_2));
    return min(min(distanceP0_, distanceP1_), distanceToProjection);
}

float Function(float x_1) {
    return ((sin((x_1 * 40.0)) + 1.5) / 4.0);
}

float DistanceToFunction(vec2 p_3, float xDelta) {
    float result = 0.0;
    float i = 0.0;
    vec2 q = vec2(0.0);
    result = 100.0;
    i = -3.0;
    bool loop_init = true;
    while(true) {
        if (!loop_init) {
            float _e33 = i;
            i = (_e33 + 1.0);
        }
        loop_init = false;
        float _e6 = i;
        if ((_e6 < 3.0)) {
        } else {
            break;
        }
        {
            q = p_3;
            float _e11 = i;
            float _e13 = q.x;
            q.x = (_e13 + (xDelta * _e11));
            float _e16 = q.x;
            float _e18 = q.x;
            float _e19 = Function(_e18);
            vec2 p0_1 = vec2(_e16, _e19);
            float _e22 = q.x;
            float _e25 = q.x;
            float _e27 = Function((_e25 + xDelta));
            vec2 p1_1 = vec2((_e22 + xDelta), _e27);
            float _e29 = result;
            float _e30 = DistanceToLineSegment(p0_1, p1_1, p_3);
            result = min(_e29, _e30);
        }
    }
    float _e35 = result;
    return _e35;
}

void main() {
    VertexOutput in_1 = VertexOutput(gl_FragCoord, _vs2fs_location0);
    float max = 0.0;
    float min = 0.0;
    float sq_sum = 0.0;
    float intensity = 0.0;
    uint p = 0u;
    float _e6 = _group_1_binding_0_fs.resolution.x;
    uint pixel = uint((in_1.uv.x * _e6));
    float _e14 = _group_1_binding_0_fs.resolution.y;
    int y_1 = int((in_1.uv.y * _e14));
    max = -100000.0;
    min = 100000.0;
    sq_sum = 0.0;
    uint _e26 = _group_1_binding_0_fs.end;
    uint _e29 = _group_1_binding_0_fs.start;
    float _e35 = _group_1_binding_0_fs.resolution.x;
    float samples_per_pixel = (float((_e26 - _e29)) / _e35);
    float pixels_per_sample = (1.0 / samples_per_pixel);
    uint offset = uint(roundEven((float(pixel) * samples_per_pixel)));
    if (true) {
        uint _e49 = _group_1_binding_0_fs.start;
        float lvalue = _group_0_binding_0_fs[(_e49 + offset)];
        uint _e56 = _group_1_binding_0_fs.start;
        float value_1 = _group_0_binding_0_fs[((_e56 + offset) + max(uint(samples_per_pixel), 1u))];
        float _e69 = _group_1_binding_0_fs.resolution.x;
        float _e71 = DistanceToFunction(in_1.uv, (1.0 / _e69));
        float _e78 = _group_1_binding_0_fs.resolution.y;
        intensity = smoothstep(0.0, 1.0, (3.0 - (_e71 * _e78)));
        float _e83 = intensity;
        intensity = pow(_e83, (1.0 / 2.200000047683716));
        float _e88 = intensity;
        float _e89 = intensity;
        float _e90 = intensity;
        _fs2p_location0 = vec4(_e88, _e89, _e90, 1.0);
        return;
    }
    p = 0u;
    bool loop_init_1 = true;
    while(true) {
        if (!loop_init_1) {
            uint _e113 = _group_1_binding_0_fs.increment;
            uint _e114 = p;
            p = (_e114 + _e113);
        }
        loop_init_1 = false;
        uint _e95 = p;
        if ((_e95 < uint(samples_per_pixel))) {
        } else {
            break;
        }
        {
            uint _e101 = _group_1_binding_0_fs.start;
            uint _e103 = p;
            float value_2 = _group_0_binding_0_fs[((_e101 + offset) + _e103)];
            float _e107 = max;
            if ((value_2 > _e107)) {
                max = value_2;
            } else {
                float _e109 = min;
                if ((value_2 < _e109)) {
                    min = value_2;
                }
            }
        }
    }
    float _e116 = max;
    if ((_e116 == -100000.0)) {
        max = 0.0;
    }
    float _e120 = min;
    if ((_e120 == 100000.0)) {
        min = 0.0;
    }
    float _e124 = max;
    float _e127 = _group_1_binding_0_fs.scale;
    float _e133 = _group_1_binding_0_fs.resolution.y;
    int max_coord = (int((_e124 * _e127)) + int((_e133 / 2.0)));
    float _e138 = min;
    float _e141 = _group_1_binding_0_fs.scale;
    float _e147 = _group_1_binding_0_fs.resolution.y;
    int min_coord = (int((_e138 * _e141)) + int((_e147 / 2.0)));
    if (((y_1 > max_coord) && (y_1 < (max_coord + 10)))) {
        vec4 _e159 = _group_1_binding_0_fs.main_color;
        _fs2p_location0 = _e159;
        return;
    }
    float _e163 = _group_1_binding_0_fs.resolution.y;
    if ((y_1 == int((_e163 / 2.0)))) {
        _fs2p_location0 = vec4(0.0, 0.0, 0.0, 1.0);
        return;
    }
    vec4 _e175 = _group_1_binding_0_fs.bg_color;
    _fs2p_location0 = _e175;
    return;
}

