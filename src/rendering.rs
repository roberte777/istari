use crate::{Istari, Mode};
use ratatui::{
    Terminal,
    backend::CrosstermBackend,
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Block, Borders, List, ListItem, Paragraph},
};
use std::io;
use std::time::{Duration, Instant};

pub struct Renderer {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    output_scroll: usize,       // Track scroll position for output
    auto_scroll: bool,          // Whether to auto-scroll to bottom on new output
    last_content_height: usize, // Track the last content height to detect changes
}

impl Renderer {
    /// Create a new renderer
    pub fn new() -> io::Result<Self> {
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)?;
        Ok(Self {
            terminal,
            output_scroll: 0,
            auto_scroll: true,
            last_content_height: 0,
        })
    }

    /// Initialize the terminal
    pub fn init(&mut self) -> io::Result<()> {
        crossterm::terminal::enable_raw_mode()?;
        crossterm::execute!(
            io::stdout(),
            crossterm::terminal::EnterAlternateScreen,
            crossterm::event::EnableMouseCapture
        )?;
        self.terminal.clear()?;
        Ok(())
    }

    /// Restore the terminal
    pub fn cleanup(&mut self) -> io::Result<()> {
        crossterm::terminal::disable_raw_mode()?;
        crossterm::execute!(
            io::stdout(),
            crossterm::terminal::LeaveAlternateScreen,
            crossterm::event::DisableMouseCapture
        )?;
        self.terminal.show_cursor()?;
        Ok(())
    }

    /// Render the current menu
    pub fn render<T: std::fmt::Debug>(&mut self, app: &mut Istari<T>) -> io::Result<()> {
        let menu = app.current_menu();

        // Check for new output and update auto-scroll before rendering
        let has_new_output = app.has_new_output();

        // Show or hide cursor based on mode
        if app.mode() == Mode::Command {
            self.terminal.show_cursor()?;
        } else {
            self.terminal.hide_cursor()?;
        }

        self.terminal.draw(|f| {
            let area = f.area();

            // First split the screen vertically into main content and footer
            let vertical_split = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Min(5),     // Main content area
                    Constraint::Length(4),  // Footer (help text + command input)
                ])
                .split(area);

            // Split the footer vertically with command input above help text
            let footer_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),  // Command input
                    Constraint::Length(1),  // Help text
                ])
                .split(vertical_split[1]);

            // Split the main content horizontally for menu and output
            let horizontal_split = Layout::default()
                .direction(Direction::Horizontal)
                .constraints([
                    Constraint::Percentage(50),  // Menu side
                    Constraint::Percentage(50),  // Output side
                ])
                .split(vertical_split[0]);

            // Split the menu side vertically
            let menu_chunks = Layout::default()
                .direction(Direction::Vertical)
                .constraints([
                    Constraint::Length(3),  // Title
                    Constraint::Min(0),     // Menu items
                ])
                .split(horizontal_split[0]);

            // Output takes the entire right side of the main content
            let output_chunk = horizontal_split[1];

            let menu = menu.lock().unwrap();

            // Render title
            let title_text = Text::styled(
                menu.title.clone(),
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD)
            );

            // Add mode indicator to title
            let mode_name = match app.mode() {
                Mode::Command => "COMMAND MODE",
                Mode::Scroll => "SCROLL MODE",
            };
            let mode_style = match app.mode() {
                Mode::Command => Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
                Mode::Scroll => Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD),
            };

            let title = Paragraph::new(title_text)
                .block(Block::default().borders(Borders::ALL).title(format!("Istari - {}", 
                    Span::styled(mode_name, mode_style))));
            f.render_widget(title, menu_chunks[0]);

            // Render menu items
            let mut items = Vec::new();
            for item in &menu.items {
                let key_style = Style::default().fg(Color::Yellow);
                let desc_style = Style::default().fg(Color::White);
                let item_line = Line::from(vec![
                    Span::styled(format!("[{}] ", item.key), key_style),
                    Span::styled(&item.description, desc_style),
                ]);
                items.push(ListItem::new(item_line));
            }

            // Add back/quit option if not at root
            if menu.parent.is_some() {
                items.push(ListItem::new(Line::from(vec![
                    Span::styled("[b] ", Style::default().fg(Color::Yellow)),
                    Span::styled("Back", Style::default().fg(Color::White)),
                ])));
            } else {
                items.push(ListItem::new(Line::from(vec![
                    Span::styled("[q] ", Style::default().fg(Color::Yellow)),
                    Span::styled("Quit", Style::default().fg(Color::White)),
                ])));
            }

            let items_list = List::new(items)
                .block(Block::default().borders(Borders::ALL).title("Menu Items"));
            f.render_widget(items_list, menu_chunks[1]);

            // Render command input box when in Command mode
            if app.mode() == Mode::Command {
                let input_text = app.input_buffer();
                let input_widget = Paragraph::new(input_text)
                    .style(Style::default().fg(Color::White))
                    .block(Block::default().borders(Borders::ALL).title("Command Input - Command [param] - Press Enter to execute"));
                f.render_widget(input_widget, footer_chunks[0]);

                // Show cursor at input position
                let cursor_x = input_text.len() as u16;
                f.set_cursor_position(
                    ratatui::layout::Position::new(
                        footer_chunks[0].x + cursor_x + 1, // +1 for border
                        footer_chunks[0].y + 1             // +1 for border
                    )
                );
            }

            // Render help text based on current mode
            let help_text = match app.mode() {
                Mode::Command => {
                    Paragraph::new("Type commands with optional parameters | Tab to switch mode | Ctrl+Q to quit")
                        .style(Style::default().fg(Color::Gray))
                },
                Mode::Scroll => {
                    Paragraph::new("SCROLL MODE: Tab to exit | j/k Scroll | u/d Page | g/G Top/Bottom | Ctrl+A Toggle auto-scroll")
                        .style(Style::default().fg(Color::Yellow))
                }
            };
            f.render_widget(help_text, footer_chunks[1]);

            // Render output area on the right side
            let output_messages = app.output_messages();
            let output_text = if output_messages.is_empty() {
                Text::styled(
                    "No output yet. Run commands to see their output here.",
                    Style::default().fg(Color::Gray)
                )
            } else {
                let messages: Vec<Line> = output_messages
                    .iter()
                    .map(|msg| Line::from(msg.as_str()))
                    .collect();
                Text::from(messages)
            };

            // Calculate max scroll position based on content height
            let output_area_height = output_chunk.height as usize - 2; // Adjusting for borders
            let content_height = output_messages.len();
            let max_scroll = content_height.saturating_sub(output_area_height);

            // Check if content height changed
            let content_changed = content_height != self.last_content_height;
            self.last_content_height = content_height;

            // Auto-scroll to bottom if there's new output and auto-scroll is enabled
            if self.auto_scroll && (has_new_output || content_changed) {
                self.output_scroll = max_scroll;
            }

            // Ensure scroll position is valid
            self.output_scroll = self.output_scroll.min(max_scroll);

            // Show auto-scroll status in title
            let scroll_status = if self.auto_scroll {
                "Auto-scroll ON"
            } else {
                "Auto-scroll OFF"
            };

            // Render output content
            let output_widget = Paragraph::new(output_text)
                .block(Block::default().borders(Borders::ALL).title(format!("Output [{}] [{}/{}]", scroll_status, self.output_scroll, max_scroll)))
                .scroll((self.output_scroll as u16, 0))
                .wrap(ratatui::widgets::Wrap { trim: true });

            f.render_widget(output_widget, output_chunk);
        })?;
        Ok(())
    }
}

/// Run the application
pub fn run<T: std::fmt::Debug>(app: &mut Istari<T>) -> io::Result<()> {
    let mut renderer = Renderer::new()?;
    renderer.init()?;

    let result = event_loop(app, &mut renderer);

    renderer.cleanup()?;

    result
}

/// Main event loop
fn event_loop<T: std::fmt::Debug>(app: &mut Istari<T>, renderer: &mut Renderer) -> io::Result<()> {
    // Define the tick rate (how often to redraw)
    let tick_rate = Duration::from_millis(100);
    let mut last_tick = Instant::now();

    loop {
        // Render the current state
        renderer.render(app)?;

        // Calculate how long to wait before the next tick
        let timeout = tick_rate
            .checked_sub(last_tick.elapsed())
            .unwrap_or_else(|| Duration::from_secs(0));

        // Poll for events with the calculated timeout
        if crossterm::event::poll(timeout)? {
            if let crossterm::event::Event::Key(key) = crossterm::event::read()? {
                match key.code {
                    // Handle Escape key to just clear input in command mode
                    crossterm::event::KeyCode::Esc => {
                        if app.mode() == Mode::Command {
                            // Clear input when in command mode
                            app.clear_input_buffer();
                        }
                    }
                    // Handle Tab key to switch between modes
                    crossterm::event::KeyCode::Tab => {
                        app.toggle_mode();
                    }
                    // Handle Ctrl+Q to quit from anywhere
                    crossterm::event::KeyCode::Char('q')
                        if key
                            .modifiers
                            .contains(crossterm::event::KeyModifiers::CONTROL) =>
                    {
                        break;
                    }
                    // Handle Enter key to process command input
                    crossterm::event::KeyCode::Enter => {
                        if app.mode() == Mode::Command {
                            if !app.process_input_buffer() {
                                break;
                            }
                        }
                    }
                    // Handle backspace to delete from input buffer
                    crossterm::event::KeyCode::Backspace => {
                        if app.mode() == Mode::Command {
                            app.backspace_input_buffer();
                        }
                    }
                    crossterm::event::KeyCode::Char(c) => {
                        match app.mode() {
                            Mode::Command => {
                                // Add character to input buffer
                                app.add_to_input_buffer(c);
                            }
                            Mode::Scroll => {
                                // Handle vim-style navigation in scroll mode
                                match c {
                                    // vim-style scrolling: j = down, k = up
                                    'j' => {
                                        renderer.output_scroll += 1;
                                        // Re-enable auto-scroll if we scroll to the bottom manually
                                        let output_messages = app.output_messages();
                                        let output_area_height =
                                            renderer.terminal.size()?.height as usize - 2;
                                        let content_height = output_messages.len();
                                        let max_scroll =
                                            content_height.saturating_sub(output_area_height);
                                        if renderer.output_scroll >= max_scroll.saturating_sub(1) {
                                            renderer.auto_scroll = true;
                                        }
                                    }
                                    'k' => {
                                        if renderer.output_scroll > 0 {
                                            renderer.output_scroll -= 1;
                                            // Disable auto-scroll when manually scrolling up
                                            renderer.auto_scroll = false;
                                        }
                                    }
                                    // vim-style page scrolling: u = half page up, d = half page down
                                    'u' => {
                                        // Half-page up
                                        let page_size =
                                            (renderer.terminal.size()?.height as usize - 2) / 2;
                                        renderer.output_scroll =
                                            renderer.output_scroll.saturating_sub(page_size);
                                        // Disable auto-scroll when manually scrolling up
                                        renderer.auto_scroll = false;
                                    }
                                    'd' => {
                                        // Half-page down
                                        let page_size =
                                            (renderer.terminal.size()?.height as usize - 2) / 2;
                                        renderer.output_scroll += page_size;
                                        // Check if we're at the bottom and re-enable auto-scroll
                                        let output_messages = app.output_messages();
                                        let output_area_height =
                                            renderer.terminal.size()?.height as usize - 2;
                                        let content_height = output_messages.len();
                                        let max_scroll =
                                            content_height.saturating_sub(output_area_height);
                                        if renderer.output_scroll >= max_scroll.saturating_sub(1) {
                                            renderer.auto_scroll = true;
                                        }
                                    }
                                    // vim-style to bottom: G
                                    'G' => {
                                        // Same as End key
                                        renderer.output_scroll = usize::MAX; // Will be clamped in render
                                        // Enable auto-scroll when going to bottom
                                        renderer.auto_scroll = true;
                                    }
                                    // vim-style to top: gg (need to track state)
                                    'g' => {
                                        // We got "g" - go to top
                                        renderer.output_scroll = 0;
                                        // Disable auto-scroll when going to top
                                        renderer.auto_scroll = false;
                                    }
                                    'a' if key
                                        .modifiers
                                        .contains(crossterm::event::KeyModifiers::CONTROL) =>
                                    {
                                        // Toggle auto-scroll with Ctrl+A
                                        renderer.auto_scroll = !renderer.auto_scroll;
                                        // If enabling auto-scroll, jump to bottom immediately
                                        if renderer.auto_scroll {
                                            renderer.output_scroll = usize::MAX; // Will be clamped in render
                                        }
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    // Keep the arrow key navigation for output scrolling in any mode
                    crossterm::event::KeyCode::Up => {
                        if renderer.output_scroll > 0 {
                            renderer.output_scroll -= 1;
                            // Disable auto-scroll when manually scrolling up
                            renderer.auto_scroll = false;
                        }
                    }
                    crossterm::event::KeyCode::Down => {
                        // We'll check the max scroll in render()
                        renderer.output_scroll += 1;
                        // Re-enable auto-scroll if we scroll to the bottom manually
                        let output_messages = app.output_messages();
                        let output_area_height = renderer.terminal.size()?.height as usize - 2; // Adjusting for borders
                        let content_height = output_messages.len();
                        let max_scroll = content_height.saturating_sub(output_area_height);
                        if renderer.output_scroll >= max_scroll.saturating_sub(1) {
                            renderer.auto_scroll = true;
                        }
                    }
                    // Page Up/Down for faster scrolling
                    crossterm::event::KeyCode::PageUp => {
                        renderer.output_scroll = renderer.output_scroll.saturating_sub(10);
                        // Disable auto-scroll when manually scrolling up
                        renderer.auto_scroll = false;
                    }
                    crossterm::event::KeyCode::PageDown => {
                        renderer.output_scroll += 10;
                        // Upper bound will be enforced during rendering
                        // Check if we're at the bottom and re-enable auto-scroll
                        let output_messages = app.output_messages();
                        let output_area_height = renderer.terminal.size()?.height as usize - 2; // Adjusting for borders
                        let content_height = output_messages.len();
                        let max_scroll = content_height.saturating_sub(output_area_height);
                        if renderer.output_scroll >= max_scroll.saturating_sub(1) {
                            renderer.auto_scroll = true;
                        }
                    }
                    // Home/End keys for jumping to top/bottom
                    crossterm::event::KeyCode::Home => {
                        renderer.output_scroll = 0;
                        // Disable auto-scroll when going to top
                        renderer.auto_scroll = false;
                    }
                    crossterm::event::KeyCode::End => {
                        // Set to a large value, will be clamped in render
                        renderer.output_scroll = usize::MAX;
                        // Enable auto-scroll when going to bottom
                        renderer.auto_scroll = true;
                    }
                    _ => {}
                }
            }
        } else {
            // Reset the 'g' sequence if we time out waiting for the second 'g'
        }

        // Check if it's time for a tick update
        if last_tick.elapsed() >= tick_rate {
            last_tick = Instant::now();
            // Call the app's tick method to update time-based state
            app.tick();
        }
    }

    Ok(())
}
