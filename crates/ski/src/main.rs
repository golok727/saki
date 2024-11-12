use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;
use std::sync::Arc;

use ski_draw::{app::App, Renderer, SurfaceRenderTarget, SurfaceRenderTargetSpecs};

fn main() {
    println!("Radhe Shyam!");

    init_stdout_logger();

    log::info!("Welcome to ski!");

    let mut app = App::new();

    app.run(|app| {
        let gpu_arc = app.gpu();
        let gpu = &gpu_arc;

        let window = app.window_handle();
        let size = window.inner_size();

        let specs = &(SurfaceRenderTargetSpecs {
            width: size.width,
            height: size.height,
        });

        let surface_target = {
            let screen = Arc::clone(window);
            SurfaceRenderTarget::new(specs, gpu, screen)
        };

        let renderer = Rc::new(RefCell::new(Renderer::new(
            Arc::clone(gpu_arc),
            surface_target,
        )));

        let ren = Rc::clone(&renderer);

        app.on_next_frame(move |_| {
            let mut renderer = ren.borrow_mut();
            renderer.render();
        })
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
