mod text;
mod tui;

pub use text::run as run_text;
pub use tui::run as run_tui;

/// Main run function that delegates to the appropriate renderer
pub fn run<T: std::fmt::Debug>(app: &mut crate::Istari<T>) -> std::io::Result<()> {
    use crate::RenderMode;

    match app.render_mode() {
        RenderMode::TUI => run_tui(app),
        RenderMode::Text => run_text(app),
    }
}
