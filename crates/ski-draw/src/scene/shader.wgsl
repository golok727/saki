struct Globals {
    proj: mat4x4<f32>, 
}; 

@group(0) @binding(0) var<uniform> globals: Globals;


struct SceneVertex {
    @location(0) position: vec2f,
    @location(1) color: vec4f,
};

struct VsOut {
    @builtin(position) position: vec4f,
    @location(0) color: vec4f,
};

@vertex fn vs(in: SceneVertex) -> VsOut {
    var out: VsOut; 
    let proj = transpose(globals.proj);
    out.position = proj * vec4f(in.position, 1.0, 1.0); 
    out.color = in.color; 
    return out; 
}

@fragment fn fs(in: VsOut)-> @location(0) vec4f {
    return in.color; 
}

