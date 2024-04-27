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
    @location(1) ty: vec4<u32>,
};


@group(0) @binding(0)
var<uniform> gen: General;

// Widgets buffer
@group(1) @binding(0)
var<storage,read_write> widget: array<Widget>;
// ID buffer
@group(2) @binding(0)
var<storage,read_write> ids: array<u32>;

fn elliptic_button(in: VertexOutput) -> vec4<f32> {
    var x = in.pos.x;
    var y = in.pos.y;
    var h = (widget[in.id].limits[0]+widget[in.id].limits[1]) * 0.5;
    var k = (widget[in.id].limits[2]+widget[in.id].limits[3]) * 0.5;
    var rad1 = abs(widget[in.id].limits[1]-widget[in.id].limits[0])/2.0;
    var rad2 = abs(widget[in.id].limits[3]-widget[in.id].limits[2])/2.0;
    var a = max(rad1, rad2);
    var b = min(rad1, rad2);
    
    var point_in = (x-h)*(x-h)/(a*a) + (y-k)*(y-k)/(b*b);
    if (point_in > 1.0) {
        discard;
    }
    return vec4<f32>(point_in, 0.0, 0.0, 1.0);
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4<f32> {
    var coords: vec2<u32> = vec2<u32>(u32((in.pos.x+1.0)*0.5*f32(gen.resolution.x)), u32((-in.pos.y+1.0)*0.5*f32(gen.resolution.y)));
    var color: vec4<f32>;
    // switch widget[in.id].ty {
    //     case 0u: {
            color = elliptic_button(in); // Elliptic mask AND button function
    //         break;
    //     }
    //     case default: {
    //         color = vec4<f32>(0.0, 1.0, 0.0, 1.0);
    //         break;
    //     }
    // }
    // DEBUG // DO IN ANOTHER SHADER
    if (gen.resized.x%2 == 1) {
        ids[coords.y * gen.resolution.x + coords.x] = in.id + 1; 
    }
    // -----------------------------
    return color;
}