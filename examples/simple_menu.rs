use istari::{Istari, Menu};
use std::io;

/// This example demonstrates a simple counter application with multiple menus
/// and the new mode system:
///
/// - COMMAND MODE: The default mode where menu actions can be triggered
///   - Press 'i' to enter Scroll Mode
///   - Use menu keys (1, 2, s, etc.) to trigger actions
///   - Commands can take parameters: "1 5" will increment by 5
///   - Ctrl+Q to quit from any screen
///
/// - SCROLL MODE: For navigating output with vim-style keybinds
///   - Press Esc to return to Command Mode
///   - Use j/k for line up/down
///   - Use u/d for page up/down
///   - Use gg/G for top/bottom
///   - Ctrl+A to toggle auto-scroll
///
/// The mode is clearly indicated in the UI title bar.
#[derive(Debug)]
struct AppState {
    counter: i32,
}

fn main() -> io::Result<()> {
    // Create our application state
    let state = AppState { counter: 0 };

    // Create the root menu
    let mut root_menu = Menu::new("Main Menu");

    // Add some simple actions that return output strings
    root_menu.add_action(
        "inc",
        "Increment Counter (optional amount)",
        |state: &mut AppState, params: Option<&str>| {
            let amount = params.and_then(|p| p.parse::<i32>().ok()).unwrap_or(1);
            state.counter += amount;
            Some(format!(
                "Counter incremented by {} to {}",
                amount, state.counter
            ))
        },
    );

    root_menu.add_action(
        "dec",
        "Decrement Counter (optional amount)",
        |state: &mut AppState, params: Option<&str>| {
            let amount = params.and_then(|p| p.parse::<i32>().ok()).unwrap_or(1);
            state.counter -= amount;
            Some(format!(
                "Counter decremented by {} to {}",
                amount, state.counter
            ))
        },
    );

    // Create a submenu
    let mut submenu = Menu::new("Settings");
    submenu.add_action(
        'r',
        "Reset Counter (optionally to value)",
        |state: &mut AppState, params: Option<&str>| {
            let value = params.and_then(|p| p.parse::<i32>().ok()).unwrap_or(0);
            state.counter = value;
            Some(format!("Counter reset to {}", value))
        },
    );

    submenu.add_action(
        'd',
        "Double Counter (optional multiplier)",
        |state: &mut AppState, params: Option<&str>| {
            let multiplier = params.and_then(|p| p.parse::<i32>().ok()).unwrap_or(2);
            state.counter *= multiplier;
            Some(format!(
                "Counter multiplied by {} to {}",
                multiplier, state.counter
            ))
        },
    );

    // Add a silent action that doesn't produce output
    submenu.add_action(
        's',
        "Silent Update (optional amount)",
        |state: &mut AppState, params: Option<&str>| {
            let amount = params.and_then(|p| p.parse::<i32>().ok()).unwrap_or(5);
            state.counter += amount;
            // Return None for no output
            None
        },
    );

    // Add an action that produces a lot of output to demonstrate scrolling
    submenu.add_action(
        'l',
        "Generate Log Output (optional lines)",
        |state: &mut AppState, params: Option<&str>| {
            let lines = params.and_then(|p| p.parse::<usize>().ok()).unwrap_or(50);
            let mut output = String::new();
            for i in 1..=lines {
                output.push_str(&format!(
                    "Log line {}: Counter is currently {}\n",
                    i, state.counter
                ));
            }
            Some(output)
        },
    );

    // Add the submenu to the root menu
    root_menu.add_submenu('s', "Settings", submenu);

    // Create and run our application
    let mut app = Istari::new(root_menu, state)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    app.run()
}
