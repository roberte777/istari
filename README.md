# Istari

A simple terminal UI menu system powered by ratatui.

## Features

- Create hierarchical menus with keybindings
- Associate actions with menu items
- Automatic back/quit navigation
- State management for your application
- Streamlined API for creating menus and submenus
- Split-view UI with menu on the left and output on the right
- Action output display for informational feedback

## Usage

Add to your Cargo.toml:

```toml
[dependencies]
istari = "0.1.0"
```

### Basic Example

```rust
use istari::{ActionFn, Istari, Menu};
use std::io;

// Your application state
struct AppState {
    counter: i32,
}

fn main() -> io::Result<()> {
    // Create application state
    let state = AppState { counter: 0 };

    // Create root menu
    let mut root_menu = Menu::new("Main Menu");
    
    // Add a simple action that returns output
    root_menu.add_action('1', "Increment Counter", Box::new(|state: &mut AppState, params: Option<&str>| {
        // Parse optional parameter as amount or default to 1
        let amount = params.and_then(|p| p.parse::<i32>().ok()).unwrap_or(1);
        state.counter += amount;
        // Return Some with a message to show in output view
        Some(format!("Counter incremented by {} to {}", amount, state.counter))
    }));
    
    // Create a submenu
    let mut submenu = Menu::new("Settings");
    submenu.add_action('r', "Reset Counter", Box::new(|state: &mut AppState, params: Option<&str>| {
        // Parse optional parameter as value or default to 0
        let value = params.and_then(|p| p.parse::<i32>().ok()).unwrap_or(0);
        state.counter = value;
        Some(format!("Counter reset to {}", value))
    }));
    
    // Add submenu to root
    root_menu.add_submenu('s', "Settings", submenu);
    
    // Create and run application
    let mut app = Istari::new(root_menu, state);
    app.run()?;
    
    Ok(())
}
```

### Command Parameters

Commands can now accept parameters:

1. For single-character menu items:
   - Type the character followed by the parameter: `1 5` (increments by 5)
   
2. For text commands:
   - Type the command followed by the parameter: `increment 10`

Parameters are passed to action functions as an `Option<&str>`:
- When a parameter is provided, the action receives `Some("parameter_value")`
- When no parameter is provided, the action receives `None`

For example:
```rust
// Action that uses an optional parameter
let action = Box::new(|state: &mut AppState, params: Option<&str>| {
    // Parse parameter or use default value
    let amount = params.and_then(|p| p.parse::<i32>().ok()).unwrap_or(1);
    state.counter += amount;
    Some(format!("Counter incremented by {}", amount))
});
```

### Action Output

Actions can return `Option<String>` to display informational messages:

- Return `Some(message)` to display a message in the output pane
- Return `None` for actions that don't need to display output

### Navigation

- Use the key shown in brackets to select a menu item
- In submenus, press `b` to go back to the parent menu
- In the root menu, press `q` to quit the application

### Running Examples

```
cargo run --example simple_menu   # Basic example
cargo run --example advanced_menu # More complex example with output history
```

## License

MIT 