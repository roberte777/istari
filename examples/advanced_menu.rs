use istari::{Istari, Menu};
use std::io;

/// Advanced application state with multiple fields
#[derive(Debug)]
struct AdvancedAppState {
    counter: i32,
    name: String,
    settings: AppSettings,
    history: Vec<String>,
}

/// Settings for the application
#[derive(Debug)]
struct AppSettings {
    theme: String,
    notifications: bool,
    auto_save: bool,
}

fn main() -> io::Result<()> {
    // Create our application state
    let state = AdvancedAppState {
        counter: 0,
        name: "User".to_string(),
        settings: AppSettings {
            theme: "Default".to_string(),
            notifications: true,
            auto_save: false,
        },
        history: Vec::new(),
    };

    // Create the root menu
    let mut root_menu = Menu::new("Advanced Demo");

    // Counter submenu
    let mut counter_menu = Menu::new("Counter Operations");
    
    counter_menu.add_action('i', "Increment (optional amount)", Box::new(|state: &mut AdvancedAppState, params: Option<&str>| {
        let amount = params.and_then(|p| p.parse::<i32>().ok()).unwrap_or(1);
        state.counter += amount;
        state.history.push(format!("Incremented counter by {} to {}", amount, state.counter));
        Some(format!("Counter incremented by {} to {}", amount, state.counter))
    }));
    
    counter_menu.add_action('d', "Decrement (optional amount)", Box::new(|state: &mut AdvancedAppState, params: Option<&str>| {
        let amount = params.and_then(|p| p.parse::<i32>().ok()).unwrap_or(1);
        state.counter -= amount;
        state.history.push(format!("Decremented counter by {} to {}", amount, state.counter));
        Some(format!("Counter decremented by {} to {}", amount, state.counter))
    }));
    
    counter_menu.add_action('r', "Reset (optional value)", Box::new(|state: &mut AdvancedAppState, params: Option<&str>| {
        let new_value = params.and_then(|p| p.parse::<i32>().ok()).unwrap_or(0);
        let old_value = state.counter;
        state.counter = new_value;
        state.history.push(format!("Reset counter from {} to {}", old_value, new_value));
        Some(format!("Counter has been reset to {}", new_value))
    }));
    
    counter_menu.add_action('m', "Multiply (optional factor)", Box::new(|state: &mut AdvancedAppState, params: Option<&str>| {
        let factor = params.and_then(|p| p.parse::<i32>().ok()).unwrap_or(2);
        state.counter *= factor;
        state.history.push(format!("Multiplied counter by {} to {}", factor, state.counter));
        Some(format!("Counter multiplied by {} to {}", factor, state.counter))
    }));

    // Settings submenu
    let mut settings_menu = Menu::new("Settings");
    
    settings_menu.add_action('t', "Change Theme", Box::new(|state: &mut AdvancedAppState, params: Option<&str>| {
        if let Some(theme_name) = params {
            state.settings.theme = theme_name.to_string();
        } else {
            // Toggle between themes when no parameter is provided
            state.settings.theme = match state.settings.theme.as_str() {
                "Default" => "Dark".to_string(),
                "Dark" => "Light".to_string(),
                _ => "Default".to_string(),
            };
        }
        Some(format!("Theme changed to {}", state.settings.theme))
    }));
    
    settings_menu.add_action('n', "Toggle Notifications", Box::new(|state: &mut AdvancedAppState, _params: Option<&str>| {
        state.settings.notifications = !state.settings.notifications;
        Some(format!("Notifications are now {}", 
            if state.settings.notifications { "enabled" } else { "disabled" }))
    }));
    
    settings_menu.add_action('a', "Toggle Auto-save", Box::new(|state: &mut AdvancedAppState, _params: Option<&str>| {
        state.settings.auto_save = !state.settings.auto_save;
        Some(format!("Auto-save is now {}", 
            if state.settings.auto_save { "enabled" } else { "disabled" }))
    }));

    // User submenu
    let mut user_menu = Menu::new("User");
    
    user_menu.add_action('r', "Rename User", Box::new(|state: &mut AdvancedAppState, params: Option<&str>| {
        let old_name = state.name.clone();
        
        if let Some(new_name) = params {
            if !new_name.trim().is_empty() {
                state.name = new_name.to_string();
                Some(format!("Name changed from {} to {}", old_name, state.name))
            } else {
                Some("Name cannot be empty".to_string())
            }
        } else {
            // In a real app, this would prompt for input
            let names = ["Alice", "Bob", "Charlie", "Diana", "Ethan", "Fiona"];
            let random_index = (std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() % names.len() as u128) as usize;
            
            state.name = names[random_index].to_string();
            
            Some(format!("Name changed from {} to {}", old_name, state.name))
        }
    }));
    
    // History viewer
    user_menu.add_action('h', "View History", Box::new(|state: &mut AdvancedAppState, params: Option<&str>| {
        if state.history.is_empty() {
            Some("No history available yet.".to_string())
        } else {
            let limit = params.and_then(|p| p.parse::<usize>().ok())
                .unwrap_or(state.history.len());
            
            let start = if state.history.len() > limit {
                state.history.len() - limit
            } else {
                0
            };
            
            let history = state.history[start..].iter()
                .enumerate()
                .map(|(i, action)| format!("{}. {}", i + start + 1, action))
                .collect::<Vec<_>>()
                .join("\n");
            
            Some(format!("Action History (last {} items):\n{}", 
                         state.history.len().min(limit), history))
        }
    }));
    
    user_menu.add_action('c', "Clear History", Box::new(|state: &mut AdvancedAppState, _params: Option<&str>| {
        let count = state.history.len();
        state.history.clear();
        Some(format!("Cleared {} history items", count))
    }));

    // Status action in root menu
    root_menu.add_action('s', "Show Status", Box::new(|state: &mut AdvancedAppState, _params: Option<&str>| {
        Some(format!(
            "Current Status:\n\
             - User: {}\n\
             - Counter: {}\n\
             - Theme: {}\n\
             - Notifications: {}\n\
             - Auto-save: {}\n\
             - History Items: {}",
            state.name,
            state.counter,
            state.settings.theme,
            if state.settings.notifications { "Enabled" } else { "Disabled" },
            if state.settings.auto_save { "Enabled" } else { "Disabled" },
            state.history.len()
        ))
    }));

    // Add submenus to root
    root_menu.add_submenu('c', "Counter", counter_menu);
    root_menu.add_submenu('u', "User", user_menu);
    root_menu.add_submenu('o', "Options", settings_menu);

    // Create and run the application
    let mut app = Istari::new(root_menu, state);
    app.run()?;

    Ok(())
} 