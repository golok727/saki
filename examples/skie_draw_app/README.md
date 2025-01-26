### Example: Using `skie_draw::app`

This example demonstrates how to use `skie_draw::app` to easily create a window with `winit` and leverage `skie_draw::Canvas` for drawing. The `skie_draw::app` module provides an abstraction over `skie_draw`, simplifying the setup process.

If you'd like to see a more minimal, barebones setup, check out the `examples/skie_draw_canvas` directory.

#### Enabling the `application` Feature

To use `skie_draw::app`, you need to enable the `application` feature in your project. Add the following to your `Cargo.toml`:

```toml
[dependencies]
skie-draw = { git = "https://github.com/golok727/saki.git", features = ["application"] }
```

#### Note:

The `skie_draw::app` module also re-exports `winit` for your convenience.
