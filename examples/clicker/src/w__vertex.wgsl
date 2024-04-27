struct VertexInput {
    @location(0) pos: vec2<f32>,
    @location(1) @interpolate(flat) id: u32,
};

struct VertexOutput {
    @builtin(position) position: vec4<f32>,
    @location(0) pos: vec4<f32>,
    @location(1) @interpolate(flat) id: u32,
};

struct General {
    resolution: vec2<u32>,
    resized: vec2<u32>,
}

struct Widget {
    @location(0) limits: vec4<f32>,
    @location(1) ty: u32,
    @location(2) ty2: vec3<u32>,
};


@group(0) @binding(0)
var<uniform> gen: General;


// Widgets buffer
@group(1) @binding(0)
var<storage,read_write> widget: array<Widget>;
// ID buffer
@group(2) @binding(0)
var<storage,read_write> ids: array<u32>;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(in.pos, 0.0, 1.0);
    out.pos = vec4<f32>(in.pos, 0.0, 1.0);
    out.id = in.id;
    return out;
}