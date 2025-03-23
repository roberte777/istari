use crate::Istari;
use crossterm::{
    event::{self, Event, KeyCode, KeyEvent, KeyModifiers},
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::io::{self, Write, stdout};
use std::time::{Duration, Instant};

/// Simple text renderer for Istari application
pub struct Renderer {}

impl Renderer {
    /// Create a new text renderer
    pub fn new() -> io::Result<Self> {
        Ok(Self {})
    }

    /// Print the menu items
    fn print_menu<T: std::fmt::Debug>(&self, app: &Istari<T>) -> io::Result<()> {
        let menu = app.current_menu();
        let menu = menu.lock().unwrap();

        // Print the title
        println!("\n== {} ==", menu.title);

        // Print menu items
        for item in &menu.items {
            println!("[{}] {}", item.key, item.description);
        }

        // Add back/quit option if not at root
        if menu.parent.is_some() {
            println!("[b] Back");
        } else {
            println!("[q] Quit");
        }

        // Print a separator after the menu
        println!("----------------------------------------");

        Ok(())
    }

    /// Print the output messages
    fn print_output<T: std::fmt::Debug>(&self, app: &Istari<T>) -> io::Result<()> {
        let output_messages = app.output_messages();
        if !output_messages.is_empty() {
            // Only print the last message
            if let Some(last_msg) = output_messages.last() {
                println!("Output:");
                println!("  {}", last_msg);
                println!("----------------------------------------");
            }
        }
        Ok(())
    }
}

/// Run the application in Text mode
pub fn run<T: std::fmt::Debug>(app: &mut crate::Istari<T>) -> io::Result<()> {
    let renderer = Renderer::new()?;

    // Print welcome message
    println!("Welcome to Istari (Text Mode)");
    println!("Type commands and press Enter to execute");
    println!("Use Up/Down arrows for command history");
    println!("Type 'b' to go back, 'q' to quit");
    println!("----------------------------------------");

    event_loop(app, &renderer)
}

/// Main event loop for text mode
fn event_loop<T: std::fmt::Debug>(
    app: &mut crate::Istari<T>,
    renderer: &Renderer,
) -> io::Result<()> {
    // Define the tick rate
    let tick_rate = Duration::from_millis(100);
    let mut last_tick = Instant::now();

    // Enable raw mode to handle arrow keys
    enable_raw_mode()?;

    // Main loop
    loop {
        // Print the current menu
        disable_raw_mode()?;
        renderer.print_menu(app)?;
        renderer.print_output(app)?;
        enable_raw_mode()?;

        // Print command prompt
        disable_raw_mode()?;
        print!("> ");
        stdout().flush()?;
        enable_raw_mode()?;

        // Command input loop - allows for up/down arrow navigation
        let mut input = String::new();
        let mut cursor_pos = 0;

        loop {
            // Check if it's time for a tick update
            if last_tick.elapsed() >= tick_rate {
                app.tick();
                last_tick = Instant::now();
            }

            // Poll for events with a timeout
            if event::poll(Duration::from_millis(100))? {
                match event::read()? {
                    Event::Key(KeyEvent {
                        code, modifiers, ..
                    }) => {
                        match code {
                            // Exit application with Ctrl+Q
                            KeyCode::Char('q') if modifiers.contains(KeyModifiers::CONTROL) => {
                                disable_raw_mode()?;
                                println!("\nExiting...");
                                return Ok(());
                            }

                            // Enter key - process command
                            KeyCode::Enter => {
                                // Update input buffer from our local input
                                app.clear_input_buffer();
                                for c in input.chars() {
                                    app.add_to_input_buffer(c);
                                }

                                // Process the input
                                disable_raw_mode()?;
                                println!(); // New line after input
                                let should_continue = app.process_input_buffer();
                                if !should_continue {
                                    println!("Exiting...");
                                    return Ok(());
                                }
                                break;
                            }

                            // Backspace - delete last character
                            KeyCode::Backspace => {
                                if cursor_pos > 0 {
                                    input.remove(cursor_pos - 1);
                                    cursor_pos -= 1;

                                    // Redraw the input line
                                    disable_raw_mode()?;
                                    print!("\r> {}", input);
                                    print!("{}", " ".repeat(10)); // Clear any trailing characters
                                    print!("\r> {}", input);
                                    stdout().flush()?;
                                }
                            }

                            // Up arrow - previous command in history
                            KeyCode::Up => {
                                app.history_up();
                                input = app.input_buffer().to_string();
                                cursor_pos = input.len();

                                // Redraw the input line
                                disable_raw_mode()?;
                                print!("\r> {}", input);
                                print!("{}", " ".repeat(10)); // Clear any trailing characters
                                print!("\r> {}", input);
                                stdout().flush()?;
                                enable_raw_mode()?;
                            }

                            // Down arrow - next command in history
                            KeyCode::Down => {
                                app.history_down();
                                input = app.input_buffer().to_string();
                                cursor_pos = input.len();

                                // Redraw the input line
                                disable_raw_mode()?;
                                print!("\r> {}", input);
                                print!("{}", " ".repeat(10)); // Clear any trailing characters
                                print!("\r> {}", input);
                                stdout().flush()?;
                                enable_raw_mode()?;
                            }

                            // Normal character input
                            KeyCode::Char(c) => {
                                input.insert(cursor_pos, c);
                                cursor_pos += 1;

                                // Redraw the input line
                                disable_raw_mode()?;
                                print!("\r> {}", input);
                                stdout().flush()?;
                                enable_raw_mode()?;
                            }

                            _ => {}
                        }
                    }
                    _ => {}
                }
            }
        }
    }
}
