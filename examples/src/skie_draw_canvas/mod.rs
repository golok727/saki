use std::{fs, path::Path};

use pollster::FutureExt;
use skie_draw::{vec2, Brush, Canvas, Color, Corners, GpuContext, Half, Rect, Size, Text};

pub fn run() {
    let gpu = GpuContext::new()
        .block_on()
        .expect("Error creating gpu context");

    let mut canvas = Canvas::create(Size::new(1024, 1024)).build(gpu.clone());
    let mut surface = canvas.create_offscreen_target();

    let size = canvas.screen().map(|v| *v as f32);

    let rect = Rect::xywh(size.width.half(), size.height.half(), 500.0, 500.0).centered();

    let mut brush = Brush::default();
    brush.fill_color(Color::TORCH_RED);
    brush.stroke_color(Color::WHITE);
    brush.stroke_width(5);

    canvas.draw_round_rect(&rect, &Corners::with_all(10.0), &brush);

    brush.reset();
    brush.fill_color(Color::WHITE);
    let center = rect.center();
    canvas.draw_circle(center.x, center.y, 200.0, &brush);

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
