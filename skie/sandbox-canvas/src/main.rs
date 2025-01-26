use skie_draw::{
    app::{self, LogicalSize, SkieAppHandle, WindowAttributes},
    Color,
};

use skie_draw::{Brush, Rect};

#[derive(Default)]
struct SandboxApp;

impl SkieAppHandle for SandboxApp {
    fn init(&mut self) -> WindowAttributes {
        WindowAttributes::default()
            .with_inner_size(LogicalSize::new(700, 500))
            .with_title("SandBox")
            .with_resizable(false)
    }

    fn update(&mut self, _window: &app::Window) {}

    fn draw(&mut self, cx: &mut app::DrawingContext, window: &app::Window) {
        let scale_factor = window.scale_factor();
        cx.set_clear_color(Color::THAMAR_BLACK);

        let mut rect = Rect::xywh(0.0, 0.0, 200.0, 200.0);
        rect.size = rect.size.map(|v| *v * scale_factor as f32);
        cx.translate(10.0, 10.0);

        let mut brush = Brush::default();
        brush.fill_color(Color::TORCH_RED);
        cx.draw_rect(&rect, &brush);
        cx.paint();
    }
}

async fn async_main() {
    app::launch(&mut SandboxApp)
        .await
        .expect("error running app");
}

fn main() {
    pollster::block_on(async_main());
}
