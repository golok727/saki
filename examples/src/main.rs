use std::fmt::Write;

mod skie_draw_app;
mod skie_draw_canvas;

struct ExampleDescriptor {
    name: &'static str,
    runner: fn(),
}

static EXAMPLES: &[ExampleDescriptor] = &[
    ExampleDescriptor {
        name: "skie_draw_app",
        runner: skie_draw_app::run,
    },
    ExampleDescriptor {
        name: "skie_draw_canvas",
        runner: skie_draw_canvas::run,
    },
];

fn main() {
    let example_name = std::env::args().nth(1);

    if let Some(example_name) = example_name {
        if let Some(example) = EXAMPLES.iter().find(|example| example.name == example_name) {
            println!("Running `{}`", example.name);
            (example.runner)();
        } else {
            eprintln!("Example not found: {example_name}");
        }
    } else {
        let mut error = String::new();
        writeln!(&mut error, "Usage: skie_examples <example_name>").unwrap();
        writeln!(&mut error).unwrap();
        writeln!(&mut error, "Examples\n---------").unwrap();
        for example in EXAMPLES {
            writeln!(&mut error, "- {}", example.name).unwrap();
        }

        eprintln!("{error}");
    };
}
