use std::{fs, path::Path};

use pollster::FutureExt;
use skie_draw::{gpu, vec2, Brush, Canvas, Color, Corners, GpuContext, Half, Rect, Text};

pub fn run() {
    let gpu = GpuContext::new()
        .block_on()
        .expect("Error creating gpu context");

    let mut canvas = Canvas::create()
        .width(1024)
        .height(1024)
        .msaa_samples(4)
        .add_surface_usage(gpu::TextureUsages::COPY_SRC)
        .build(gpu.clone());

    let mut surface = canvas.create_offscreen_target();

    let size = canvas.size().map(|v| *v as f32);

    let rect = Rect::xywh(size.width.half(), size.height.half(), 500.0, 500.0).centered();

    canvas.draw_round_rect(
        &rect,
        Corners::with_all(10.0),
        Brush::filled(Color::TORCH_RED)
            .stroke_color(Color::WHITE)
            .line_width(5),
    );

    let center = rect.center();
    canvas.draw_circle(center.x, center.y, 200.0, Brush::filled(Color::WHITE));

    // Aligns wont work now :)
    let pos = center - vec2(170.0, 50.0);
    let text = Text::new("✨ Hello ✨").pos(pos.x, pos.y).size_px(64.0);
    canvas.fill_text(&text, Color::BLACK);

    canvas.clear_color(Color::THAMAR_BLACK);
    canvas.render(&mut surface).expect("error painting");

    let snapshot = canvas
        .snapshot_sync(&surface)
        .expect("error taking snapshot");

    let image_buffer = image::ImageBuffer::<image::Rgba<u8>, _>::from_raw(
        snapshot.size.width,
        snapshot.size.height,
        snapshot.data,
    )
    .expect("Failed to create image buffer");

    let out_dir = Path::new("output");
    fs::create_dir_all(out_dir).expect("Error creaitng output dir");

    let out_path = out_dir.join("render.png");

    image_buffer
        .save(out_path.clone())
        .expect("Failed to save image");

    println!("Saved to {}", out_path.to_string_lossy());
}
