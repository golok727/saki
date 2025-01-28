use skie_draw::{
    app::{self, LogicalSize, SkieAppHandle, WindowAttributes},
    Canvas, Color,
};

use skie_draw::{Brush, Rect};

#[derive(Default)]
struct SandboxApp;

impl SkieAppHandle for SandboxApp {
    fn init(&mut self) -> WindowAttributes {
        WindowAttributes::default()
            .with_inner_size(LogicalSize::new(700, 500))
            .with_title("Sandbox App")
    }

    fn update(&mut self, _window: &app::Window) {}

    fn draw(&mut self, cx: &mut Canvas, window: &app::Window) {
        let scale_factor = window.scale_factor() as f32;
        cx.clear_color(Color::THAMAR_BLACK);
        // fixme scale is getting reset
        cx.save();

        cx.scale(scale_factor, scale_factor);
        let mut rect = Rect::xywh(10.0, 10.0, 100.0, 100.0);
        rect.size = rect.size.map(|v| *v * scale_factor);

        let mut brush = Brush::default();
        brush.fill_color(Color::TORCH_RED);
        cx.draw_rect(&rect, &brush);

        cx.translate(100.0, 100.0);
        cx.rotate(45f32.to_radians());
        brush.fill_color(Color::WHITE);
        cx.draw_rect(&Rect::xywh(0.0, 0.0, 100.0, 100.0), &brush);

        cx.restore();
    }
}

async fn run() {
    app::launch(&mut SandboxApp)
        .await
        .expect("error running app");
}

fn main() {
    pollster::block_on(run());
}
