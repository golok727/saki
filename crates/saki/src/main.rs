use std::io::Write;

use ski::{app::App, window::WindowSpecification};

/*
TODO
 - [] Texture Atlas
 - [] More Primitives
 - [] Text System
 - [] Dom
 - [] Layout system
*/
fn main() {
    println!("Radhe Shyam!");

    init_stdout_logger();

    log::info!("Welcome to ski!");

    let mut app = App::new();
    app.run(|app| {
        let window_specs = WindowSpecification {
            width: 1875,
            height: 1023,
            ..Default::default()
        };

        app.open_window(window_specs.clone(), move |cx| {
            cx.set_timeout(
                move |cx| {
                    cx.window.set_bg_color(1.0, 1.0, 1.0);
                },
                std::time::Duration::from_secs(2),
            );

            cx.set_timeout(
                move |cx| cx.window.set_bg_color(1.0, 1.0, 0.0),
                std::time::Duration::from_secs(4),
            );

            cx.set_timeout(
                move |cx| cx.window.set_bg_color(0.01, 0.01, 0.01),
                std::time::Duration::from_secs(6),
            );

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
