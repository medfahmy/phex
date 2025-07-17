struct VertexOutput {
    @builtin(position) position: vec4f,
    @location(0) color: vec4f,
};

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> VertexOutput {
    let positions = array<vec2f, 3>(
        vec2f(0.0, 0.5),
        vec2f(-0.5, -0.5),
        vec2f(0.5, -0.5),
    );

    let colors = array<vec3f, 3>(
        vec3f(1.0, 0.0, 0.0),
        vec3f(0.0, 1.0, 0.0),
        vec3f(0.0, 0.0, 1.0),
    );

    var out: VertexOutput;
    out.position = vec4f(positions[vertex_index], 0.0, 1.0);
    out.color = vec4f(colors[vertex_index], 1.0);
    return out;
}

@fragment
fn fs_main(out: VertexOutput) -> @location(0) vec4f {
    return out.color;
}
