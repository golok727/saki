use pollster::FutureExt;
use skie_draw::{
    app::{self, KeyCode, LogicalSize, SkieAppHandle, WindowAttributes},
    Half,
};
use std::collections::HashSet;

use skie_draw::{Brush, Canvas, Color, Corners, FontStyle, FontWeight, Rect, Text};

struct App {
    square: MovingSquare,
    keystate: KeyState,
}

impl App {
    fn new() -> Self {
        App {
            square: Default::default(),
            keystate: Default::default(),
        }
    }
}

#[derive(Default)]
struct KeyState {
    pressed: HashSet<KeyCode>,
}

impl KeyState {
    #[allow(unused)]
    fn is_pressed(&self, key: KeyCode) -> bool {
        self.pressed.contains(&key)
    }

    fn any_pressed(&self, keys: impl IntoIterator<Item = KeyCode>) -> bool {
        keys.into_iter().any(|key| self.pressed.contains(&key))
    }
}

#[derive(Default)]
struct MovingSquare {
    rect: Rect<f32>,
}

impl MovingSquare {
    fn draw(&self, c: &mut Canvas, _keystate: &KeyState) {
        c.draw_round_rect(
            &self.rect,
            Corners::with_all(10.0),
            Brush::filled(Color::TORCH_RED),
        );
    }

    fn update(&mut self, keystate: &KeyState, window: &app::Window) {
        let size = window.inner_size();
        let screen = Rect::xywh(0., 0., size.width as f32, size.height as f32);

        let old_pos = self.rect.origin;

        const SPEED: f32 = 1.0;
        if keystate.any_pressed([KeyCode::ArrowUp, KeyCode::KeyW]) {
            self.rect.origin.y -= SPEED;
        }

        if keystate.any_pressed([KeyCode::ArrowDown, KeyCode::KeyS]) {
            self.rect.origin.y += SPEED;
        }

        if keystate.any_pressed([KeyCode::ArrowLeft, KeyCode::KeyA]) {
            self.rect.origin.x -= SPEED;
        }

        if keystate.any_pressed([KeyCode::ArrowRight, KeyCode::KeyD]) {
            self.rect.origin.x += SPEED;
        }

        if !screen.contains(&self.rect) {
            self.rect.origin = old_pos;
        }
    }
}

impl SkieAppHandle for App {
    fn init(&mut self) -> WindowAttributes {
        WindowAttributes::default()
            .with_inner_size(LogicalSize::new(700, 500))
            .with_title("Skie")
            .with_resizable(false)
    }

    fn on_create_window(&mut self, window: &app::Window) {
        let size = window.inner_size();

        self.square.rect = Rect::xywh(
            size.width.half() as f32,
            size.height.half() as f32,
            201.0,
            201.0,
        )
        .centered();
    }

    fn update(&mut self, window: &app::Window) {
        self.square.update(&self.keystate, window);
    }

    fn draw(&mut self, cx: &mut Canvas, window: &app::Window) {
        let scale_factor = window.scale_factor();
        cx.clear_color(Color::SKIE_BLACK);

        self.square.draw(cx, &self.keystate);

        let text = Text::new("Hello, Welcome to Skie! ✨")
            .pos(101.0, 10.0)
            .font_weight(FontWeight::BOLD)
            .font_style(FontStyle::Italic)
            .size_px(33.0 * scale_factor as f32);

        cx.fill_text(&text, Color::WHITE);

        let moving = self.keystate.any_pressed([
            KeyCode::ArrowUp,
            KeyCode::ArrowDown,
            KeyCode::ArrowLeft,
            KeyCode::ArrowRight,
            KeyCode::KeyW,
            KeyCode::KeyA,
            KeyCode::KeyS,
            KeyCode::KeyD,
        ]);

        let brush = Brush::default().when_or(
            moving,
            |brush| brush.fill_color(Color::GREEN),
            |brush| brush.fill_color(Color::RED),
        );

        let height = cx.height() as f32;
        cx.draw_circle(51.0, height - 50.0, 20.0, brush);
    }

    fn on_keyup(&mut self, keycode: KeyCode) {
        self.keystate.pressed.remove(&keycode);
    }

    fn on_keydown(&mut self, keycode: KeyCode) {
        self.keystate.pressed.insert(keycode);
    }
}

pub fn run() {
    let mut app = App::new();
    app::launch(&mut app).block_on().expect("error running app");
}
