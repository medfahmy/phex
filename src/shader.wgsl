struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) tex_coords: vec2f,
};

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
) -> VertexOutput {
    let vertices = array(
        vec2f(0.0, 0.0),
        vec2f(1.0, 0.0),
        vec2f(0.0, 1.0),
        vec2f(0.0, 1.0),
        vec2f(1.0, 0.0),
        vec2f(1.0, 1.0),
    );

    var out: VertexOutput;
    let xy = vertices[vertex_index];
    out.position = vec4f(xy, 0.0, 1.0);
    out.tex_coords = xy;
    return out;
}

@group(0) @binding(0) var texture_sampler: sampler;
@group(0) @binding(1) var texture: texture_2d<f32>;

@fragment
fn fs_main(out: VertexOutput) -> @location(0) vec4f {
    return textureSample(texture, texture_sampler, out.tex_coords);
}
