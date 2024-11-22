struct Globals {
    color: vec4f,
    proj: mat3x3<f32>, 
}; 

@group(0) @binding(0) var<uniform> globals: Globals;


struct QuadOut {
    @builtin(position) position: vec4f,
    @location(0) color: vec4f,
};

@vertex fn vs(@builtin(vertex_index) vertexIndex: u32) -> QuadOut {
    let pos = array(
        vec2f(-0.5, -0.5),  // left
        vec2f(0.0, 0.5),  // top
        vec2f(0.5, -0.5)
        );
        
    var out: QuadOut; 
    out.position = vec4f(vec3(pos[vertexIndex], 1.0), 1.0); 

    
    out.color = globals.color; 
    return out;
}
@fragment fn fs(in: QuadOut)-> @location(0) vec4f {
    return in.color; 
}
