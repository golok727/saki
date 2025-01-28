use skie_draw::{
    app::{self, LogicalSize, SkieAppHandle, WindowAttributes},
    Canvas, Color, Half, Size,
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
        let size: LogicalSize<f32> = window.inner_size().to_logical(window.scale_factor());
        let size = Size::new(size.width, size.height);

        cx.clear_color(Color::THAMAR_BLACK);

        cx.save();

        cx.scale(scale_factor, scale_factor);
        let rect = Rect::xywh(0.0, 0.0, 200.0, 200.0);

        let mut brush = Brush::default();
        brush.fill_color(Color::TORCH_RED);
        cx.draw_rect(&rect, &brush);

        let center = rect.center();
        brush.fill_color(Color::BLUE);
        cx.draw_circle(center.x, center.y, 10.0, &brush);

        // draw rotated square
        cx.save();
        cx.translate(center.x, center.y);
        cx.scale(0.5, 0.5);
        cx.rotate(60f32.to_radians());
        brush.fill_color(Color::WHITE);
        cx.draw_rect(&Rect::xywh(0.0, 0.0, 200.0, 200.0), &brush);
        cx.restore();

        cx.draw_rect(&Rect::xywh(0.0, 0.0, 50.0, 50.0), &brush);

        cx.save();
        cx.translate(size.width.half(), size.height.half());
        brush.fill_color(Color::WHITE);
        cx.draw_rect(&Rect::xywh(0.0, 0.0, 200.0, 200.0).centered(), &brush);
        cx.restore();

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
