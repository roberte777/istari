mod text;
mod tui;

use crate::Istari;
use std::io;

/// Dispatch to the right renderer based on the application's render mode
pub fn run<T: std::fmt::Debug>(app: &mut Istari<T>) -> io::Result<()> {
    match app.render_mode() {
        crate::RenderMode::TUI => tui::run(app),
        crate::RenderMode::Text => text::run(app),
    }
}

/// Common trait that all renderers must implement
pub trait Renderer {
    /// Initialize the renderer
    fn init(&mut self) -> io::Result<()>;

    /// Clean up and restore terminal state
    fn cleanup(&mut self) -> io::Result<()>;

    /// Render a frame of the application
    fn render_frame<T: std::fmt::Debug>(&mut self, app: &mut Istari<T>) -> io::Result<()>;

    /// Run the main event loop
    fn run_event_loop<T: std::fmt::Debug>(&mut self, app: &mut Istari<T>) -> io::Result<()>;
}

/// Direction for scrolling operations
pub enum ScrollDirection {
    Up,
    Down,
    PageUp,
    PageDown,
    Top,
    Bottom,
}

/// State for scroll position in output window
pub struct ScrollState {
    /// Current scroll position (0 = top)
    pub position: usize,
    /// Whether to auto-scroll to bottom on new output
    pub auto_scroll: bool,
}

impl ScrollState {
    /// Create a new scroll state with auto-scroll enabled
    pub fn new() -> Self {
        Self {
            position: 0,
            auto_scroll: true,
        }
    }

    /// Toggle auto-scroll
    pub fn toggle_auto_scroll(&mut self) {
        self.auto_scroll = !self.auto_scroll;
    }

    /// Scroll in the specified direction
    pub fn scroll(
        &mut self,
        direction: ScrollDirection,
        content_height: usize,
        view_height: usize,
    ) {
        // Calculate max scroll position
        let max_scroll = content_height.saturating_sub(view_height);

        match direction {
            ScrollDirection::Up => {
                self.position = self.position.saturating_sub(1);
            }
            ScrollDirection::Down => {
                self.position = (self.position + 1).min(max_scroll);
            }
            ScrollDirection::PageUp => {
                self.position = self.position.saturating_sub(view_height);
            }
            ScrollDirection::PageDown => {
                self.position = (self.position + view_height).min(max_scroll);
            }
            ScrollDirection::Top => {
                self.position = 0;
            }
            ScrollDirection::Bottom => {
                self.position = max_scroll;
            }
        }
    }

    /// Update scroll position if auto-scroll is enabled and there's new content
    pub fn update_auto_scroll(
        &mut self,
        content_height: usize,
        view_height: usize,
        has_new_content: bool,
    ) {
        if self.auto_scroll && has_new_content {
            let max_scroll = content_height.saturating_sub(view_height);
            self.position = max_scroll;
        }
    }
}
