use istari::{Istari, Menu, UIMode};
use std::error::Error;

/// This example demonstrates the simple text-based rendering mode.
///
/// Unlike the default TUI mode that uses ratatui for a rich terminal UI,
/// the text mode provides a simpler interface:
/// - The current menu is displayed as text
/// - Commands are entered via standard input
/// - Command output is printed after each command
/// - No special terminal handling is required
///
/// This mode is useful for:
/// - Simpler interfaces without TUI dependencies
/// - Terminal environments where TUI libraries don't work well
/// - Scripting or automation scenarios
/// - Applications where a minimal interface is desired
#[derive(Debug)]
struct AppState {
    counter: i32,
}

fn main() -> Result<(), Box<dyn Error>> {
    // Create a simple counter state
    let state = AppState { counter: 0 };

    // Create a root menu
    let mut root_menu = Menu::new("Text Mode Demo");

    // Add some items to the menu
    root_menu.add_action(
        "inc",
        "Increment counter (optional amount)",
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
        "Decrement counter",
        |state: &mut AppState, _params: Option<&str>| {
            state.counter -= 1;
            Some(format!("Counter decremented to {}", state.counter))
        },
    );

    root_menu.add_action(
        "set",
        "Set counter to specific value",
        |state: &mut AppState, params: Option<&str>| {
            if let Some(param) = params {
                if let Ok(value) = param.parse::<i32>() {
                    state.counter = value;
                    Some(format!("Counter set to {}", state.counter))
                } else {
                    Some("Invalid parameter. Please provide a number.".to_string())
                }
            } else {
                Some("Missing parameter. Usage: set <number>".to_string())
            }
        },
    );

    // Add a submenu
    let mut submenu = Menu::new("Advanced Operations");

    submenu.add_action(
        "double",
        "Double the counter",
        |state: &mut AppState, _params: Option<&str>| {
            state.counter *= 2;
            Some(format!("Counter doubled to {}", state.counter))
        },
    );

    submenu.add_action(
        "reset",
        "Reset counter to zero",
        |state: &mut AppState, _params: Option<&str>| {
            state.counter = 0;
            Some("Counter reset to 0".to_string())
        },
    );

    // Add the submenu to the root menu
    root_menu.add_submenu("adv", "Advanced operations", submenu);

    // Create the application with TEXT renderer mode
    // Note: The default mode is RenderMode::TUI if not specified
    let mut app = Istari::new(root_menu, state)?.with_ui_mode(UIMode::Text);

    // Run the application
    app.run()?;

    Ok(())
}
