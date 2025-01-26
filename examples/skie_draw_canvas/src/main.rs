use std::{fs, path::Path};

use pollster::FutureExt;
use skie_draw::{
    gpu::{TexelCopyBufferInfo, TexelCopyTextureInfo},
    Brush, Canvas, Color, Corners, Extent3d, GpuContext, GpuTexture, GpuTextureDescriptor,
    GpuTextureDimension, GpuTextureFormat, GpuTextureUsages, GpuTextureViewDescriptor, Half, Rect,
    Size,
};

fn main() {
    let gpu = GpuContext::new()
        .block_on()
        .expect("Error creating gpu context");

    let mut canvas = Canvas::create(Size::new(1024, 1024)).build(gpu.clone());

    let size = canvas.screen().map(|v| *v as f32);

    let rect = Rect::xywh(size.width.half(), size.height.half(), 500.0, 500.0).centered();

    let mut brush = Brush::default();
    brush.fill_color(Color::TORCH_RED);
    brush.stroke_color(Color::WHITE);
    brush.stroke_width(5);

    canvas.draw_round_rect(&rect, &Corners::with_all(10.0), &brush);

    // TODO: auto flush
    canvas.paint();

    // TODO: screenshot
    let output_texture = create_render_texture(&gpu, canvas.width(), canvas.height());
    let view = output_texture.create_view(&GpuTextureViewDescriptor::default());

    canvas.finish(&view, Color::THAMAR_BLACK);

    save_to_file("render.png", &gpu, &output_texture);
}

fn save_to_file(file_name: &str, gpu: &GpuContext, texture: &GpuTexture) {
    let buffer_size = (texture.width() * texture.height() * 4) as u64; // 4 bytes per pixel (RGBA8)
                                                                       //
    let output_buffer = gpu.device.create_buffer(&skie_draw::gpu::BufferDescriptor {
        label: Some("Output Buffer"),
        size: buffer_size,
        usage: skie_draw::gpu::BufferUsages::MAP_READ | skie_draw::gpu::BufferUsages::COPY_DST,
        mapped_at_creation: false,
    });

    let mut encoder = gpu.create_command_encoder(Some("Command Encoder"));

    encoder.copy_texture_to_buffer(
        TexelCopyTextureInfo {
            texture,
            mip_level: 0,
            origin: skie_draw::gpu::Origin3d::ZERO,
            aspect: skie_draw::gpu::TextureAspect::All,
        },
        TexelCopyBufferInfo {
            buffer: &output_buffer,
            layout: skie_draw::gpu::TexelCopyBufferLayout {
                offset: 0,
                bytes_per_row: Some(texture.width() * 4),
                rows_per_image: Some(texture.height()),
            },
        },
        Extent3d {
            width: texture.width(),
            height: texture.height(),
            depth_or_array_layers: 1,
        },
    );

    // Submit the commands
    gpu.queue.submit(Some(encoder.finish()));

    let buffer_slice = output_buffer.slice(..);
    buffer_slice.map_async(skie_draw::gpu::MapMode::Read, |_| {});
    gpu.device.poll(skie_draw::gpu::Maintain::Wait);
    let data = buffer_slice.get_mapped_range();

    let image_buffer = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(
        texture.width(),
        texture.height(),
        data.to_vec(),
    )
    .expect("Failed to create image buffer");

    let out_dir = Path::new("output");
    fs::create_dir_all(out_dir).expect("Error creaitng output dir");

    let out_path = out_dir.join(file_name);
    image_buffer
        .save(out_path.clone())
        .expect("Failed to save image");

    println!("Saved to {}", out_path.to_string_lossy());
}

fn create_render_texture(gpu: &GpuContext, width: u32, height: u32) -> GpuTexture {
    gpu.create_texture(&GpuTextureDescriptor {
        label: Some("framebuffer"),
        size: Extent3d {
            width,
            height,
            depth_or_array_layers: 1,
        },
        mip_level_count: 1,
        sample_count: 1,
        dimension: GpuTextureDimension::D2,
        format: GpuTextureFormat::Rgba8Unorm,
        usage: GpuTextureUsages::RENDER_ATTACHMENT | GpuTextureUsages::COPY_SRC,
        view_formats: &[],
    })
}
