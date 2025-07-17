struct Base {
    color: vec4f,
    offset: vec2f,
};

struct Extra {
    scale: vec2f,
};

@group(0) @binding(0) var<uniform> base: Base;
@group(0) @binding(1) var<uniform> extra: Extra;

@vertex
fn vs_main(@builtin(vertex_index) vertex_index: u32) -> @builtin(position) vec4f {
    let positions = array(
        vec2f(0.0, 0.5),
        vec2f(-0.5, -0.5),
        vec2f(0.5, -0.5),
    );

    return vec4f(positions[vertex_index] * extra.scale + base.offset, 0.0, 1.0);
}

@fragment
fn fs_main() -> @location(0) vec4f {
    return base.color;
}
