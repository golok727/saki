struct Globals {
    proj: mat4x4<f32>, 
    view: mat4x4<f32>, 
}; 

@group(0) @binding(0) var<uniform> globals: Globals;


struct QuadOut {
    @builtin(position) position: vec4f,
    @location(0) color: vec4f,
};

@vertex fn vs(@builtin(vertex_index) vertexIndex: u32) -> QuadOut {
    let pos = array(
            vec2f(-0.5, 0.5), 
            vec2f(-0.5, -0.5), 
            vec2f(0.5, -0.5), 
           
            vec2f(0.5, -0.5), 
            vec2f(0.5, 0.5), 
            vec2f(-0.5, 0.5), 
        );

    
    var out: QuadOut; 
    let proj = transpose(globals.proj);
    let view = transpose(globals.view); 
    out.position = proj * view * vec4f(pos[vertexIndex], 1.0, 1.0); 
    out.color = vec4f(1.0, 0.3 , 0.4, 1.0);
    return out; 
}
@fragment fn fs(in: QuadOut)-> @location(0) vec4f {
    return in.color; 
}
