# Istari

A modern terminal UI menu framework powered by [ratatui](https://github.com/ratatui-org/ratatui),
enabling you to quickly build interactive terminal applications with beautiful menus and rich UI.

## ✨ Features

- **Hierarchical Menus** - Create nested menu structures with intuitive navigation
- **Flexible Input** - Support for both keybindings and command-based interaction
- **Async Support** - Run background tasks while keeping your UI responsive
- **Mode System** - Switch between command and scroll modes for different interaction styles
- **Split-View UI** - Menu on the left, action output on the right
- **State Management** - Associate actions with your application state
- **Parameter Support** - Pass parameters to menu actions
- **Dual Rendering** - Choose between rich TUI or plain text output

## 🚀 Quick Start

```toml
[dependencies]
istari = "0.1.0"
```

```rust
use istari::{Istari, Menu};

// Your application state
struct AppState { counter: i32 }

fn main() {
    // Create state
    let state = AppState { counter: 0 };

    // Build menu
    let mut menu = Menu::new("Main Menu");
    
    // Add action with parameter support
    menu.add_action("inc", "Increment Counter", |state: &mut AppState, params: Option<&str>| {
        let amount = params.and_then(|p| p.parse::<i32>().ok()).unwrap_or(1);
        state.counter += amount;
        Some(format!("Counter: {}", state.counter))
    });
    
    // Create and run application
    Istari::new(menu, state).run()
}
```

## 🧩 Advanced Features

### Async Actions

```rust
menu.add_action("fetch", "Fetch Data", |state, params| {
    async move {
        // Perform async operations...
        Some("Data fetched successfully!".to_string())
    }
});
```

### Interactive Modes

- **Command Mode** - Execute menu actions (default)
- **Scroll Mode** - Navigate output with vim-style keybindings (j/k, u/d, gg/G)

### Parameter Passing

```
// In the terminal:
inc 5      // Pass "5" to the "inc" action
```

### Rendering Modes

Istari supports two rendering modes to fit different use cases:

```rust
// Rich TUI mode (default) - interactive menus with split-view UI
let app = Istari::new(menu, state);

// Plain text mode - simpler output for scripts or CI environments
let app = Istari::new(menu, state).with_render_mode(RenderMode::Text);
```

- **TUI Mode**: Full-featured interactive UI with colors, borders, and styled text
- **Text Mode**: Plain text output ideal for scripts, CI/CD pipelines, or testing

## 📚 Examples

Run the included examples to see Istari in action:

```bash
# Basic menu with counter
cargo run --example simple_menu

# Advanced features demo
cargo run --example advanced_menu

# Async operation demo
cargo run --example async_menu

# Animation and custom rendering
cargo run --example animated_demo
```

## 📄 License

MIT 