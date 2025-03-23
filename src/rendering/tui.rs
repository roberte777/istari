use crate::rendering::{ScrollDirection, ScrollState, UIController};
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

pub struct TuiController {
    terminal: Terminal<CrosstermBackend<io::Stdout>>,
    scroll_state: ScrollState,
    last_content_height: usize, // Track the last content height to detect changes
}

impl TuiController {
    /// Create a new TUI controller
    pub fn new() -> io::Result<Self> {
        let backend = CrosstermBackend::new(io::stdout());
        let terminal = Terminal::new(backend)?;
        Ok(Self {
            terminal,
            scroll_state: ScrollState::new(),
            last_content_height: 0,
        })
    }
}

impl UIController for TuiController {
    /// Initialize the terminal
    fn init(&mut self) -> io::Result<()> {
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
    fn cleanup(&mut self) -> io::Result<()> {
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
    fn render_frame<T: std::fmt::Debug>(&mut self, app: &mut Istari<T>) -> io::Result<()> {
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

            // Check if content height changed
            let content_changed = content_height != self.last_content_height;
            self.last_content_height = content_height;

            // Auto-scroll to bottom if there's new output and auto-scroll is enabled
            self.scroll_state.update_auto_scroll(
                content_height,
                output_area_height,
                has_new_output || content_changed
            );

            // Show auto-scroll status in title
            let scroll_status = if self.scroll_state.auto_scroll {
                "Auto-scroll ON"
            } else {
                "Auto-scroll OFF"
            };

            // Calculate max_scroll for display
            let max_scroll = content_height.saturating_sub(output_area_height);

            // Render output content
            let output_widget = Paragraph::new(output_text)
                .block(Block::default().borders(Borders::ALL).title(format!("Output [{}] [{}/{}]", 
                    scroll_status, self.scroll_state.position, max_scroll)))
                .scroll((self.scroll_state.position as u16, 0))
                .wrap(ratatui::widgets::Wrap { trim: true });

            f.render_widget(output_widget, output_chunk);
        })?;
        Ok(())
    }

    /// Run the application event loop
    fn run_event_loop<T: std::fmt::Debug>(&mut self, app: &mut Istari<T>) -> io::Result<()> {
        // Define the tick rate (how often to redraw)
        let tick_rate = Duration::from_millis(100);
        let mut last_tick = Instant::now();

        loop {
            // Render the current state
            self.render_frame(app)?;

            // Check if we should perform a tick update
            let timeout = tick_rate
                .checked_sub(last_tick.elapsed())
                .unwrap_or_else(|| Duration::from_secs(0));

            // Poll for events with a timeout
            if crossterm::event::poll(timeout)? {
                match crossterm::event::read()? {
                    crossterm::event::Event::Key(key) => {
                        // Process key events based on current mode
                        match app.mode() {
                            crate::Mode::Command => {
                                // Handle different key events in command mode
                                match key.code {
                                    // Exit the application
                                    crossterm::event::KeyCode::Char('q')
                                        if key
                                            .modifiers
                                            .contains(crossterm::event::KeyModifiers::CONTROL) =>
                                    {
                                        return Ok(());
                                    }

                                    // Toggle mode
                                    crossterm::event::KeyCode::Tab => {
                                        app.toggle_mode();
                                    }

                                    // Toggle input display
                                    crossterm::event::KeyCode::Char('i')
                                        if key
                                            .modifiers
                                            .contains(crossterm::event::KeyModifiers::CONTROL) =>
                                    {
                                        app.toggle_show_input();
                                    }

                                    // Process input when Enter is pressed
                                    crossterm::event::KeyCode::Enter => {
                                        if !app.input_buffer().is_empty()
                                            && !app.process_input_buffer()
                                        {
                                            return Ok(());
                                        }
                                    }

                                    // Backspace to delete last character
                                    crossterm::event::KeyCode::Backspace => {
                                        app.exit_history_browsing();
                                        app.backspace_input_buffer();
                                    }

                                    // Up arrow key for history navigation
                                    crossterm::event::KeyCode::Up => {
                                        app.history_up();
                                    }

                                    // Down arrow key for history navigation
                                    crossterm::event::KeyCode::Down => {
                                        app.history_down();
                                    }

                                    // Any other key press exits history browsing
                                    crossterm::event::KeyCode::Char(c) => {
                                        app.exit_history_browsing();
                                        app.add_to_input_buffer(c);
                                    }

                                    // Handle single-key commands directly
                                    _ => {
                                        // Exit history browsing for any other key
                                        app.exit_history_browsing();

                                        // Convert keycode to string representation
                                        if let crossterm::event::KeyCode::Char(c) = key.code {
                                            if app.input_buffer().is_empty()
                                                && !app.handle_key(c.to_string())
                                            {
                                                return Ok(());
                                            }
                                        }
                                    }
                                }
                            }

                            crate::Mode::Scroll => {
                                // Handle different key events in scroll mode
                                match key.code {
                                    // Exit the application
                                    crossterm::event::KeyCode::Char('q')
                                        if key
                                            .modifiers
                                            .contains(crossterm::event::KeyModifiers::CONTROL) =>
                                    {
                                        return Ok(());
                                    }

                                    // Toggle mode
                                    crossterm::event::KeyCode::Tab => {
                                        app.toggle_mode();
                                    }

                                    // Toggle auto-scroll
                                    crossterm::event::KeyCode::Char('a')
                                        if key
                                            .modifiers
                                            .contains(crossterm::event::KeyModifiers::CONTROL) =>
                                    {
                                        self.scroll_state.toggle_auto_scroll();
                                    }

                                    // Scroll down
                                    crossterm::event::KeyCode::Char('j')
                                    | crossterm::event::KeyCode::Down => {
                                        self.scroll_state.scroll(
                                            ScrollDirection::Down,
                                            app.output_messages().len(),
                                            10, // Approximate view height
                                        );
                                    }

                                    // Scroll up
                                    crossterm::event::KeyCode::Char('k')
                                    | crossterm::event::KeyCode::Up => {
                                        self.scroll_state.scroll(
                                            ScrollDirection::Up,
                                            app.output_messages().len(),
                                            10, // Approximate view height
                                        );
                                    }

                                    // Page down
                                    crossterm::event::KeyCode::Char('d')
                                    | crossterm::event::KeyCode::PageDown => {
                                        self.scroll_state.scroll(
                                            ScrollDirection::PageDown,
                                            app.output_messages().len(),
                                            10, // Approximate view height
                                        );
                                    }

                                    // Page up
                                    crossterm::event::KeyCode::Char('u')
                                    | crossterm::event::KeyCode::PageUp => {
                                        self.scroll_state.scroll(
                                            ScrollDirection::PageUp,
                                            app.output_messages().len(),
                                            10, // Approximate view height
                                        );
                                    }

                                    // Go to top
                                    crossterm::event::KeyCode::Char('g')
                                    | crossterm::event::KeyCode::Home => {
                                        self.scroll_state.scroll(
                                            ScrollDirection::Top,
                                            app.output_messages().len(),
                                            10, // Approximate view height
                                        );
                                    }

                                    // Go to bottom
                                    crossterm::event::KeyCode::Char('G')
                                    | crossterm::event::KeyCode::End => {
                                        self.scroll_state.scroll(
                                            ScrollDirection::Bottom,
                                            app.output_messages().len(),
                                            10, // Approximate view height
                                        );
                                    }

                                    _ => {}
                                }
                            }
                        }
                    }
                    crossterm::event::Event::Mouse(_) => {
                        // Mouse events could be handled here if needed
                    }
                    crossterm::event::Event::Resize(_, _) => {
                        // Resize events are automatically handled by the Terminal
                    }
                    _ => {}
                }
            }

            // Check if it's time for a tick update
            if last_tick.elapsed() >= tick_rate {
                app.tick();
                last_tick = Instant::now();
            }
        }
    }
}

/// Run the application in TUI mode
pub fn run<T: std::fmt::Debug>(app: &mut crate::Istari<T>) -> io::Result<()> {
    let mut controller = TuiController::new()?;
    controller.init()?;

    let result = controller.run_event_loop(app);

    controller.cleanup()?;

    result
}
