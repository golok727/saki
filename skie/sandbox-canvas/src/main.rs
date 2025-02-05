use skie_draw::{
    app::{self, LogicalSize, SkieAppHandle, WindowAttributes},
    Canvas, Color, Half, Path2D, Size, StrokeCap,
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
        let size = window.inner_size();
        let size = Size::new(size.width as f32, size.height as f32).scale(1.0 / scale_factor);

        cx.clear_color(Color::THAMAR_BLACK);

        cx.save();
        cx.scale(scale_factor, scale_factor);

        let rect = Rect::xywh(0.0, 0.0, 200.0, 200.0);

        cx.draw_rect(
            &Rect::from_origin_size(Default::default(), size),
            Brush::filled(Color::KHAKI),
        );

        cx.draw_rect(&rect, Brush::filled(Color::TORCH_RED));

        let center = rect.center();
        cx.draw_circle(center.x, center.y, 10.0, Brush::filled(Color::BLUE));

        // draw rotated square
        cx.save();
        cx.translate(center.x, center.y);
        cx.scale(0.5, 0.5);
        cx.rotate(60f32.to_radians());
        cx.draw_rect(
            &Rect::xywh(0.0, 0.0, 200.0, 200.0),
            Brush::filled(Color::WHITE),
        );
        cx.restore();

        cx.draw_rect(&Rect::xywh(0.0, 0.0, 50.0, 50.0), Brush::filled(Color::RED));

        cx.save();
        cx.translate(size.width.half(), size.height.half());
        cx.draw_rect(
            &Rect::xywh(0.0, 0.0, 200.0, 200.0).centered(),
            Brush::filled(Color::WHITE),
        );
        cx.restore();

        let mut path = Path2D::new();
        path.move_to((100.0, 100.0).into());
        path.line_to((300.0, 200.0).into());
        path.line_to((90.0, 300.0).into());
        path.line_to((70.0, 300.0).into());
        path.close();

        let brush = Brush::filled(Color::ORANGE)
            .fill_color(Color::ORANGE)
            .stroke_color(Color::THAMAR_BLACK)
            .stroke_width(7)
            .stroke_join(skie_draw::StrokeJoin::Round)
            .stroke_cap(StrokeCap::Round);

        cx.draw_path(path, brush);

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
