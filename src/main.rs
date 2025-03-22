use istari::{Istari, Menu};
use std::io;

fn main() -> io::Result<()> {
    // Create a simple state for our demo
    let state = AppState { counter: 0 };

    // Create a root menu
    let mut root_menu = Menu::new("Command Input Demo");

    // Add some actions with parameter support
    root_menu.add_action(
        '1',
        "Increment Counter (optional amount)",
        Box::new(|state: &mut AppState, params: Option<&str>| {
            let amount = params.and_then(|p| p.parse::<i32>().ok()).unwrap_or(1);
            state.counter += amount;
            Some(format!(
                "Counter incremented by {} to {}",
                amount, state.counter
            ))
        }),
    );

    root_menu.add_action(
        '2',
        "Decrement Counter (optional amount)",
        Box::new(|state: &mut AppState, params: Option<&str>| {
            let amount = params.and_then(|p| p.parse::<i32>().ok()).unwrap_or(1);
            state.counter -= amount;
            Some(format!(
                "Counter decremented by {} to {}",
                amount, state.counter
            ))
        }),
    );

    // Create a settings submenu
    let mut settings = Menu::new("Settings");
    settings.add_action(
        'r',
        "Reset Counter (optionally to value)",
        Box::new(|state: &mut AppState, params: Option<&str>| {
            let value = params.and_then(|p| p.parse::<i32>().ok()).unwrap_or(0);
            state.counter = value;
            Some(format!("Counter reset to {}", value))
        }),
    );

    // Add the submenu
    root_menu.add_submenu('s', "Settings Menu", settings);

    // Create and run our application
    let mut app = Istari::new(root_menu, state)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?;
    app.run()
}

/// Simple state for our demo app
#[derive(Debug)]
struct AppState {
    counter: i32,
}
