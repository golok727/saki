use skie_draw::{
    app::{self, LogicalSize, SkieAppHandle, WindowAttributes},
    paint::PathBrush,
    vec2, Canvas, Color, Half, LineCap, Path, Size,
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
            Brush::filled(Color::WHITE).feathering(10.0),
        );
        cx.restore();

        Man::draw(cx);

        cx.restore();
    }
}

struct Man;

impl Man {
    fn draw(cx: &mut Canvas) {
        let mut path = Path::builder();

        let height = 100.0;
        let spine_start = vec2(300.0, 200.0);
        let spine_end = spine_start + vec2(0.0, height);

        let head_size: f32 = f32::min(height * 0.20, 30.0);

        let arm_length = height - head_size;
        let left_arm_spread = 20.0;
        let right_arm_spread = 25.0;
        let shoulder = spine_start + vec2(0.0, head_size + 10.0);

        let leg_length = height * 0.75;
        let left_leg_spread = 20.0;
        let right_leg_spread = 20.0;

        // Cape
        let cape_pin = spine_start + vec2(0.0, head_size);
        let cape_spread = 100.0;
        let cape_left_offset = 0.0;
        let cape_right_offset = 20.0;
        let cape_height = 100.0;

        let cape_left_end = cape_pin + vec2(-cape_spread + cape_left_offset, cape_height);
        let cape_right_end = cape_pin + vec2(cape_spread + cape_right_offset, cape_height);

        path.begin(cape_pin);
        path.line_to(cape_left_end);
        path.cubic_to(
            cape_left_end + vec2(100.0, 30.0),
            cape_right_end - vec2(100.0, 10.0),
            cape_right_end,
        );
        let cape = path.close();

        // Spine
        path.begin(spine_start);
        path.line_to(spine_end);
        let body = path.end(false);

        // Left leg
        path.begin(spine_end);
        path.line_to(spine_end + vec2(-left_leg_spread, leg_length));
        let left_leg = path.end(false);

        // Right leg
        path.begin(spine_end);
        path.line_to(spine_end + vec2(right_leg_spread, leg_length));
        let right_leg = path.end(false);

        path.begin(shoulder);
        path.line_to(shoulder + vec2(-left_arm_spread, arm_length));
        let left_arm = path.end(false);

        path.begin(shoulder);
        path.line_to(shoulder + vec2(right_arm_spread, arm_length));
        let right_arm = path.end(false);

        // Head
        let head = path.circle(spine_start, head_size);

        let mut brush = PathBrush::default();
        let common_style = Brush::default()
            .line_width(7)
            .stroke_color(Color::BLACK)
            .line_cap(LineCap::Round);

        brush.set(body, common_style.clone());
        brush.set(left_leg, common_style.clone());
        brush.set(right_leg, common_style.clone());
        brush.set(left_arm, common_style.clone());
        brush.set(right_arm, common_style.clone());
        brush.set(head, common_style.fill_color(Color::LIGHT_YELLOW));

        let cape_style = Brush::filled(Color::ORANGE)
            .stroke_color(Color::TORCH_RED)
            .line_width(5)
            .line_join(skie_draw::LineJoin::Round)
            .line_cap(LineCap::Round);

        brush.set(cape, cape_style);

        cx.draw_path(path, brush);
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
