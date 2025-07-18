struct Vertex {
    position: vec2f,
};

struct VSOutput {
    @builtin(position) position: vec4f,
    @location(0) color: vec4f,
};

struct Base {
    color: vec4f,
    offset: vec2f,
};

struct Extra {
    scale: vec2f,
};

@group(0) @binding(0) var<storage, read> bases: array<Base>;
@group(0) @binding(1) var<storage, read> extras: array<Extra>;
@group(0) @binding(2) var<storage, read> vertices: array<Vertex>;

@vertex
fn vs_main(
    @builtin(vertex_index) vertex_index: u32,
    @builtin(instance_index) instance_index: u32,
) -> VSOutput {
    var out: VSOutput;
    out.position = vec4f(
        vertices[vertex_index].position * extras[instance_index].scale + bases[instance_index].offset, 
        0.0,
        1.0,
    );
    out.color = bases[instance_index].color;
    return out;
}

@fragment
fn fs_main(out: VSOutput) -> @location(0) vec4f {
    return out.color;
}
