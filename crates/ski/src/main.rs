use std::cell::RefCell;
use std::io::Write;
use std::rc::Rc;
use std::sync::Arc;

use ski_draw::{
    app::App, gpu::surface::GpuSurfaceSpecification, window::WindowSpecification, Renderer,
};

fn main() {
    println!("Radhe Shyam!");

    init_stdout_logger();

    log::info!("Welcome to ski!");

    let mut app = App::new();

    app.run(|app| {
        let window_specs = WindowSpecification {
            width: 1280,
            height: 720,
            ..Default::default()
        };

        app.open_window(window_specs.clone(), move |cx| {
            let gpu_arc = cx.app.gpu();

            let gpu = &gpu_arc;

            let winit_window = cx.window.winit_handle();
            let size = winit_window.inner_size();

            let specs = &(GpuSurfaceSpecification {
                width: size.width,
                height: size.height,
            });

            let surface = Rc::new(gpu.create_surface(Arc::clone(winit_window), specs).unwrap());

            let renderer = Rc::new(RefCell::new(Renderer::new(
                Arc::clone(gpu_arc),
                size.width,
                size.height,
            )));

            let ren = Rc::clone(&renderer);

            {
                let surface = Rc::clone(&surface);
                cx.app.on_next_frame(move |_| {
                    let mut renderer = ren.borrow_mut();
                    let surface_texture = surface.surface.get_current_texture().unwrap();

                    renderer.render_to_texture(&surface_texture.texture);
                    surface_texture.present();
                });
            }
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
