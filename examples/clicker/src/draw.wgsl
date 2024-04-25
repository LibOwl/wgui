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


@group(0) @binding(0)
var<uniform> gen: General;


// Widgets buffer


// TEXTURE
// @group(1) @binding(0)
// var id_texture: texture_storage_2d<r32uint, write>;

// @group(1) @binding(1)
// var id_sampler: sampler;

@group(1) @binding(0)
var<storage,read_write> id_buffer: array<u32>;

@vertex
fn vs_main(in: VertexInput) -> VertexOutput {
    var out: VertexOutput;
    out.position = vec4<f32>(in.pos, 0.0, 1.0);
    out.pos = vec4<f32>(in.pos, 0.0, 1.0);
    out.id = in.id;
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    if (pow(abs(in.pos.x), 2.0) + pow(abs(in.pos.y), 2.0) > 0.25) {
        discard;
    }

    var coords: vec2<u32> = vec2<u32>(u32((in.pos.x+1.0)*0.5*f32(gen.resolution.x)), u32((-in.pos.y+1.0)*0.5*f32(gen.resolution.y)));
    if (gen.resized.x%2 == 1) {
        id_buffer[coords.y * gen.resolution.x + coords.x] = u32(4294967295); // DEBUG
    }

    return vec4<f32>(1.0, 1.0, 1.0, 1.0);
}