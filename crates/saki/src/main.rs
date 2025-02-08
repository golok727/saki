use std::io::Write;

use skie::{window::WindowSpecification, Color};

fn main() {
    println!("Radhe Shyam!");
    init_stdout_logger();
    let app = skie::app::App::new();

    log::info!("Welcome to saki!");

    app.run(|app| {
        let window_specs = WindowSpecification {
            width: 1280,
            height: 720,
            background: Color::THAMAR_BLACK,
            ..Default::default()
        };

        app.open_window(window_specs.clone(), move |window, app| {
            window.set_timeout(
                app,
                |window, _| {
                    window.set_bg_color(Color::YELLOW);
                },
                std::time::Duration::from_secs(3),
            );

            window.set_timeout(
                app,
                |window, _| {
                    window.set_bg_color(Color::KHAKI);
                },
                std::time::Duration::from_secs(5),
            );

            window.set_timeout(
                app,
                |window, _| {
                    window.set_bg_color(Color::from_rgb(0x181818));
                },
                std::time::Duration::from_secs(7),
            );

            window.set_bg_color(Color::from_rgb(0x181818));
        });
    });
}

pub fn create_checker_texture(width: usize, height: usize, tile_size: usize) -> Vec<u8> {
    let mut texture_data = vec![0u8; width * height * 4];

    for y in 0..height {
        for x in 0..width {
            let tile_x = x / tile_size;
            let tile_y = y / tile_size;
            let is_black = (tile_x + tile_y) % 2 == 0;

            let offset = (y * width + x) * 4;
            if is_black {
                texture_data[offset] = 0; // Red
                texture_data[offset + 1] = 0; // Green
                texture_data[offset + 2] = 0; // Blue
                texture_data[offset + 3] = 255; // Alpha
            } else {
                texture_data[offset] = 255; // Red
                texture_data[offset + 1] = 255; // Green
                texture_data[offset + 2] = 255; // Blue
                texture_data[offset + 3] = 255; // Alpha
            }
        }
    }
    texture_data
}

fn init_stdout_logger() {
    env_logger::Builder::new()
        .parse_default_env()
        .format(|buf, record| {
            use env_logger::fmt::style::{AnsiColor, Style};

            // Subtle style for the whole date part, dimmed color
            let dimmed = Style::new().fg_color(Some(AnsiColor::BrightBlack.into()));

            // Apply the dimmed style to the date part
            write!(buf, "{dimmed}[{dimmed:#}")?;
            write!(
                buf,
                "{dimmed}{}{dimmed:#} ",
                chrono::Local::now().format("%Y-%m-%dT%H:%M:%S%:z")
            )?;

            let level_style = buf.default_level_style(record.level());
            write!(buf, "{level_style}{:<5}{level_style:#}", record.level())?;

            if let Some(path) = record.module_path() {
                write!(buf, "  {dimmed}{path}{dimmed:#}")?;
            }

            write!(buf, "{dimmed}]{dimmed:#}")?;
            writeln!(buf, " {}", record.args())
        })
        .init();
}
