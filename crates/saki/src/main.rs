use std::io::Write;

use skie::{
    math::{
        unit::{px, Pixels},
        Rect,
    },
    window::{WindowContext, WindowSpecification},
};

/*
TODO
 - [] Color correction
 - [] More Primitives
 - [] Text System - [] Dom
 - [] Layout system
*/

fn load_images_from_args(cx: &mut WindowContext) {
    // TODO: Add Assets system to preload assets and pass in the asset handle
    let mut args = std::env::args();
    args.next(); // Skip the program name

    // Helper function to load an image from a file argument
    fn load_image(cx: &mut WindowContext, file: Option<String>, rect: Rect<Pixels>) {
        if let Some(file) = file.filter(|f| {
            std::fs::metadata(f)
                .map(|data| data.is_file())
                .unwrap_or(false)
        }) {
            cx.load_image_from_file(rect, file);
        } else {
            log::error!("Unable to load file");
        }
    }

    // Define the positions and sizes for the images
    let rects = [
        Rect {
            x: px(350),
            y: px(100),
            width: px(500),
            height: px(500),
        },
        Rect {
            x: px(800),
            y: px(600),
            width: px(300),
            height: px(300),
        },
    ];

    // Attempt to load up to two images
    for rect in rects.iter() {
        let file = args.next();
        load_image(cx, file, rect.clone());
    }

    cx.window.set_bg_color(0.01, 0.01, 0.01);
}

fn main() {
    println!("Radhe Shyam!");
    init_stdout_logger();
    let app = skie::app::App::new();

    log::info!("Welcome to saki!");

    app.run(|cx| {
        log::info!("Hello From init fn");
        let window_specs = WindowSpecification {
            width: 1875,
            height: 1023,
            ..Default::default()
        };

        cx.open_window(window_specs.clone(), |cx| {
            cx.set_timeout(
                |cx| {
                    cx.window.set_bg_color(1.0, 1.0, 0.0);
                },
                std::time::Duration::from_secs(3),
            );

            cx.set_timeout(
                |cx| {
                    cx.window.set_bg_color(0.01, 0.01, 0.01);
                },
                std::time::Duration::from_secs(5),
            );

            cx.window.set_bg_color(0.01, 0.01, 0.01);

            load_images_from_args(cx);
        });
    });
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
