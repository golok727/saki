use std::io::Write;

use skie::{
    math::Rect,
    px,
    window::{WindowContext, WindowSpecification},
    Color, Pixels,
};

/*
TODO
 - [] Color correction
 - [] Clamp rounding
 - [] Text System
 - [] Dom
 - [] Layout system
*/

fn main() {
    println!("Radhe Shyam!");
    init_stdout_logger();
    let app = skie::app::App::new();

    log::info!("Welcome to saki!");

    app.run(|cx| {
        let window_specs = WindowSpecification {
            width: 1875,
            height: 1023,
            ..Default::default()
        };

        cx.open_window(window_specs.clone(), |cx| {
            cx.set_timeout(
                |cx| {
                    cx.window.set_bg_color(Color::YELLOW);
                },
                std::time::Duration::from_secs(3),
            );

            cx.set_timeout(
                |cx| {
                    cx.window.set_bg_color(Color::from_rgb(0x181818));
                },
                std::time::Duration::from_secs(5),
            );

            cx.window.set_bg_color(Color::from_rgb(0x181818));

            load_images_from_args(cx);
        });
    });
}

fn load_images_from_args(cx: &mut WindowContext) {
    // TODO: Add Assets system to preload assets and pass in the asset handle
    let mut args = std::env::args();
    args.next(); // Skip the program name

    // Helper function to load an image from a file argument
    fn load_image(cx: &mut WindowContext, file: String, rect: Rect<Pixels>) {
        let file_clone = file.clone();
        if let Some(file) = Some(file).filter(|f| {
            std::fs::metadata(f)
                .map(|data| data.is_file())
                .unwrap_or(false)
        }) {
            cx.spawn(|cx| async move {
                let idx = cx.load_image_from_file(rect, file).await;
                if let Ok(idx) = idx {
                    if idx == 0 {
                        cx.with(|cx| {
                            let obj = cx.get_object_mut(idx).unwrap().as_image_mut().unwrap();
                            obj.bbox.origin.x = obj.bbox.origin.x - px(100);
                            obj.bbox.origin.y = obj.bbox.origin.x + px(100);
                            cx.window.refresh();
                        });
                    }
                }
            })
            .detach();
        } else {
            log::error!("Unable to load file {}", file_clone);
        }
    }

    // Define the positions and sizes for the images
    let rects = [
        Rect::xywh(px(350), px(100), px(500), px(500)),
        Rect::xywh(px(800), px(600), px(300), px(300)),
    ];

    // Attempt to load up to two images
    for rect in rects.iter() {
        let file = args.next();
        if let Some(file) = file {
            load_image(cx, file, rect.clone());
        } else {
            break;
        }
    }
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
