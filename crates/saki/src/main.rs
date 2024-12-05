use std::io::Write;

use skie::{
    app::App,
    math::{unit::px, Rect},
    window::WindowSpecification,
};

/*
TODO
 - [] Color correction
 - [] More Primitives
 - [] Text System
 - [] Dom
 - [] Layout system
*/
fn main() {
    println!("Radhe Shyam!");

    init_stdout_logger();

    log::info!("Welcome to saki!");

    let mut app = App::new();
    app.run(move |app| {
        let window_specs = WindowSpecification {
            width: 1875,
            height: 1023,
            ..Default::default()
        };

        app.open_window(window_specs.clone(), move |cx| {
            let mut args = std::env::args();
            args.next(); // program

            {
                let file = args.next().filter(|f| {
                    std::fs::metadata(f)
                        .map(|data| data.is_file())
                        .unwrap_or(false)
                });

                if let Some(file) = file {
                    // TODO: Add Assets system to preload assets and pass in the asset handle ?
                    cx.load_image_from_file(
                        Rect {
                            x: px(350),
                            y: px(100),
                            width: px(500),
                            height: px(500),
                        },
                        file,
                    );
                } else {
                    log::error!("Unable to load file");
                }
            }

            {
                let file = args.next().filter(|f| {
                    std::fs::metadata(f)
                        .map(|data| data.is_file())
                        .unwrap_or(false)
                });

                if let Some(file) = file {
                    cx.load_image_from_file(
                        Rect {
                            x: px(800),
                            y: px(600),
                            width: px(300),
                            height: px(300),
                        },
                        file,
                    );
                } else {
                    log::error!("Unable to load file");
                }
            }

            cx.window.set_bg_color(0.01, 0.01, 0.01);
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
