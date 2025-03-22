use istari::{Istari, Menu, Mode};
use std::io;

/// This example specifically demonstrates the mode system in Istari.
///
/// The application has two distinct modes:
///
/// 1. COMMAND MODE (default)
///    - This is where you can trigger menu actions
///    - Menu keys work (e.g., 1, 2, 3 for actions)
///    - Press 'i' to switch to Scroll Mode
///    - Keybinds shown in menu items work
///
/// 2. SCROLL MODE
///    - For navigating output with vim-style keybinds
///    - Press Esc to return to Command Mode
///    - Navigation:
///      * j/k: Move up/down by line
///      * u/d: Move up/down by half page
///      * gg/G: Jump to top/bottom
///      * Ctrl+A: Toggle auto-scroll
///
/// The current mode is clearly displayed in the UI title bar.

/// Sample application state
#[derive(Debug)]
struct ModeTestState {
    counter: i32,
    log_entries: Vec<String>,
}

impl ModeTestState {
    fn new() -> Self {
        Self {
            counter: 0,
            log_entries: Vec::new(),
        }
    }

    fn add_log(&mut self, message: &str) {
        self.log_entries
            .push(format!("[{}] {}", self.log_entries.len() + 1, message));
    }
}

fn main() -> io::Result<()> {
    // Create our application state
    let mut state = ModeTestState::new();

    // Add some initial log entries
    for i in 1..=5 {
        state.add_log(&format!("Initial log entry {}", i));
    }

    // Create the root menu
    let mut root_menu = Menu::new("Mode System Demo");

    // Add simple counter actions
    root_menu.add_action(
        '1',
        "Increment Counter (optional amount)",
        |state: &mut ModeTestState, params: Option<&str>| {
            let amount = params.and_then(|p| p.parse::<i32>().ok()).unwrap_or(1);
            state.counter += amount;
            state.add_log(&format!(
                "Incremented counter by {} to {}",
                amount, state.counter
            ));
            Some(format!(
                "Counter incremented by {} to {}",
                amount, state.counter
            ))
        },
    );

    root_menu.add_action(
        '2',
        "Decrement Counter (optional amount)",
        |state: &mut ModeTestState, params: Option<&str>| {
            let amount = params.and_then(|p| p.parse::<i32>().ok()).unwrap_or(1);
            state.counter -= amount;
            state.add_log(&format!(
                "Decremented counter by {} to {}",
                amount, state.counter
            ));
            Some(format!(
                "Counter decremented by {} to {}",
                amount, state.counter
            ))
        },
    );

    // Add an action that demonstrates the need for scrolling
    root_menu.add_action(
        '3',
        "Generate Many Log Lines (optional count)",
        |state: &mut ModeTestState, params: Option<&str>| {
            let count = params.and_then(|p| p.parse::<usize>().ok()).unwrap_or(50);
            let mut output = String::new();
            output.push_str(&format!("Generated {} log lines:\n", count));

            for i in 1..=count {
                let log_entry = format!("Log entry {} - Counter value: {}", i, state.counter);
                state.add_log(&log_entry);
                output.push_str(&format!("{}\n", log_entry));
            }

            Some(output)
        },
    );

    // Add an action to show all logs (demonstrating the need for scrolling)
    root_menu.add_action(
        'l',
        "Show All Logs",
        |state: &mut ModeTestState, _params: Option<&str>| {
            if state.log_entries.is_empty() {
                return Some("No logs available.".to_string());
            }

            let logs = state.log_entries.join("\n");
            Some(format!("Complete Log History:\n\n{}", logs))
        },
    );

    // Create a mode info submenu to explain the mode system
    let mut mode_info = Menu::new("Mode Information");

    mode_info.add_action(
        'c',
        "About Command Mode",
        |_: &mut ModeTestState, _params: Option<&str>| {
            Some(
                "COMMAND MODE\n\n\
              This is the default mode where you can use the menu keys to trigger actions.\n\
              - Menu keybindings work in this mode\n\
              - Press 'i' to switch to Scroll Mode\n\
              - Press 'q' to quit from the root menu\n\
              - Press 'b' to go back from submenus\n\
              - Ctrl+Q works to quit from anywhere"
                    .to_string(),
            )
        },
    );

    mode_info.add_action(
        's',
        "About Scroll Mode",
        |_: &mut ModeTestState, _params: Option<&str>| {
            Some(
                "SCROLL MODE\n\n\
              This mode allows you to navigate output content with vim-style keybindings.\n\
              - Press Esc to return to Command Mode\n\
              - j/k: Move down/up by line\n\
              - u/d: Move up/down by half page\n\
              - gg: Jump to top (press g twice)\n\
              - G: Jump to bottom\n\
              - Ctrl+A: Toggle auto-scroll\n\
              - Menu keys don't work in this mode"
                    .to_string(),
            )
        },
    );

    mode_info.add_action(
        't',
        "Try Scrolling (Generate Text)",
        |_: &mut ModeTestState, params: Option<&str>| {
            let line_count = params.and_then(|p| p.parse::<usize>().ok()).unwrap_or(50);
            let mut output = String::new();

            // Generate enough text to require scrolling
            output.push_str("SCROLLING DEMO\n\n");
            output.push_str("This text is intentionally long to demonstrate scrolling.\n");
            output.push_str("Press 'i' to enter scroll mode after viewing this text.\n\n");

            for i in 1..=line_count {
                output.push_str(&format!("Line {} - Use j/k to scroll up and down\n", i));
            }

            output.push_str("\nEnd of scrolling demo text.\n");
            output.push_str("Press 'g' twice to jump to the top, or 'G' to jump to the bottom.\n");

            Some(output)
        },
    );

    // Add the info submenu to the root menu
    root_menu.add_submenu('i', "Mode Information", mode_info);

    // Create and run the application
    let mut app = Istari::new(root_menu, state);

    // Run the application
    app.run()?;

    Ok(())
}
