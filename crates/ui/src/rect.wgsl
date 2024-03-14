struct VertexInput {
    @location(0) position: vec2f,
    @location(1) tex_coords: vec2f,
}

struct VertexOutput {
    @builtin(position) clip_position: vec4f,
    @location(0) tex_coords: vec2f,
    @location(1) color: vec4f,
}

struct CameraUniform {
    projection: mat4x4f,
}

struct InstanceInput {
    @location(5) position: vec2f,
    @location(6) scale: vec2f,
    @location(7) color: vec4f,
}

@group(0) @binding(0)
var<uniform> camera: CameraUniform;

@vertex
fn vs_main(model: VertexInput, instance: InstanceInput) -> VertexOutput {
    var out: VertexOutput;
    out.color = instance.color;
    out.tex_coords = model.tex_coords;
    out.clip_position = camera.projection * vec4f(model.position * instance.scale + instance.position, 0.0, 1.0);
    return out;
}

@fragment
fn fs_main(in: VertexOutput) -> @location(0) vec4f {
    return in.color;
}