use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::Instant;

pub mod rendering;

/// Defines the possible application modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Mode for navigating menus and triggering actions
    Command,
    /// Mode for scrolling through output with vim-style keybinds
    Scroll,
}

/// Type for action functions that can be executed when menu items are selected
pub type ActionFn<T> = Box<dyn Fn(&mut T, Option<&str>) -> Option<String> + Send + Sync>;
pub type TickFn<T> = Box<dyn Fn(&mut T, &mut Vec<String>, f32) + Send + Sync>;

/// A trait for converting closures to ActionFn
pub trait IntoActionFn<T>: Send + Sync + 'static {
    fn into_action_fn(self) -> ActionFn<T>;
}

/// Implementation for closures that can be converted to ActionFn
impl<T, F> IntoActionFn<T> for F
where
    F: Fn(&mut T, Option<&str>) -> Option<String> + Send + Sync + 'static,
{
    fn into_action_fn(self) -> ActionFn<T> {
        Box::new(self)
    }
}

/// A trait for converting closures to TickFn
pub trait IntoTickFn<T>: Send + Sync + 'static {
    fn into_tick_fn(self) -> TickFn<T>;
}

/// Implementation for closures that can be converted to TickFn
impl<T, F> IntoTickFn<T> for F
where
    F: Fn(&mut T, &mut Vec<String>, f32) + Send + Sync + 'static,
{
    fn into_tick_fn(self) -> TickFn<T> {
        Box::new(self)
    }
}

/// A menu item that can be selected
pub struct MenuItem<T> {
    /// The key that activates this item
    pub key: char,
    /// Description of what this item does
    pub description: String,
    /// The function to run when this item is selected
    pub action: Option<ActionFn<T>>,
    /// A submenu that this item leads to, if any
    pub submenu: Option<Arc<Mutex<Menu<T>>>>,
}

impl<T> Clone for MenuItem<T> {
    fn clone(&self) -> Self {
        MenuItem {
            key: self.key,
            description: self.description.clone(),
            action: None,  // We can't clone the action function, so we set it to None
            submenu: self.submenu.clone(),
        }
    }
}

impl<T: std::fmt::Debug> fmt::Debug for MenuItem<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MenuItem")
            .field("key", &self.key)
            .field("description", &self.description)
            .field("action", &if self.action.is_some() { "Some(Action)" } else { "None" })
            .field("submenu", &self.submenu)
            .finish()
    }
}

impl<T> MenuItem<T> {
    /// Create a new menu item with an action
    pub fn new_action<F>(key: char, description: String, action: F) -> Self 
    where
        F: IntoActionFn<T>,
    {
        MenuItem {
            key,
            description,
            action: Some(action.into_action_fn()),
            submenu: None,
        }
    }

    /// Create a new menu item with a submenu
    pub fn new_submenu(key: char, description: String, submenu: Menu<T>) -> Self {
        MenuItem {
            key,
            description,
            action: None,
            submenu: Some(Arc::new(Mutex::new(submenu))),
        }
    }
}

/// A menu containing items that can be selected
#[derive(Debug)]
pub struct Menu<T> {
    /// Title of the menu
    pub title: String,
    /// Items in this menu
    pub items: Vec<MenuItem<T>>,
    /// Parent menu, if any
    pub parent: Option<Arc<Mutex<Menu<T>>>>,
}

impl<T> Default for Menu<T> {
    fn default() -> Self {
        Self {
            title: "Menu".to_string(),
            items: Vec::new(),
            parent: None,
        }
    }
}

impl<T> Menu<T> {
    /// Create a new menu with the given title
    pub fn new(title: impl Into<String>) -> Self {
        Self {
            title: title.into(),
            items: Vec::new(),
            parent: None,
        }
    }

    /// Add an item to this menu
    pub fn add_item(&mut self, item: MenuItem<T>) -> &mut Self {
        self.items.push(item);
        self
    }

    /// Add an action item to this menu
    pub fn add_action<F>(&mut self, key: char, description: impl Into<String>, action: F) -> &mut Self 
    where
        F: IntoActionFn<T>,
    {
        self.add_item(MenuItem::new_action(key, description.into(), action))
    }

    /// Add a submenu to this menu
    pub fn add_submenu(&mut self, key: char, description: impl Into<String>, mut submenu: Menu<T>) -> &mut Self {
        // We'll set the parent when navigating to the submenu
        submenu.parent = None;
        self.add_item(MenuItem::new_submenu(key, description.into(), submenu))
    }

    /// Get the item for a given key
    pub fn get_item(&self, key: char) -> Option<&MenuItem<T>> {
        self.items.iter()
            .find(|item| item.key == key)
    }
}

/// Main application that handles rendering and events
pub struct Istari<T> {
    /// The current menu being displayed
    current_menu: Arc<Mutex<Menu<T>>>,
    /// Application state shared with menu actions
    state: T,
    /// Output messages from actions
    output_messages: Vec<String>,
    /// Flag indicating if new messages were added
    new_output: bool,
    /// Last tick update time, for animations or time-based updates
    last_tick_time: Instant,
    /// Optional tick function that's called on each frame update
    tick_handler: Option<TickFn<T>>,
    /// Current application mode
    current_mode: Mode,
    /// Command input buffer
    input_buffer: String,
    /// Whether the command input should be displayed
    show_input: bool,
}

impl<T: std::fmt::Debug> Istari<T> {
    /// Create a new Istari application with the given root menu and state
    pub fn new(root_menu: Menu<T>, state: T) -> Self {
        Self {
            current_menu: Arc::new(Mutex::new(root_menu)),
            state,
            output_messages: Vec::new(),
            new_output: false,
            last_tick_time: Instant::now(),
            tick_handler: None,
            current_mode: Mode::Command, // Default to command mode
            input_buffer: String::new(),
            show_input: false,
        }
    }

    /// Set a custom tick handler
    pub fn with_tick_handler<F>(mut self, handler: F) -> Self 
    where
        F: IntoTickFn<T>,
    {
        self.tick_handler = Some(handler.into_tick_fn());
        self
    }

    /// Get a reference to the current menu
    pub fn current_menu(&self) -> Arc<Mutex<Menu<T>>> {
        self.current_menu.clone()
    }

    /// Get a reference to the output messages
    pub fn output_messages(&self) -> &[String] {
        &self.output_messages
    }
    
    /// Add an output message
    pub fn add_output(&mut self, message: String) {
        self.output_messages.push(message);
        self.new_output = true;
    }
    
    /// Check if there's new output and reset the flag
    pub fn has_new_output(&mut self) -> bool {
        let has_new = self.new_output;
        self.new_output = false;
        has_new
    }
    
    /// Handle a tick update
    /// This is called regularly to update any time-based state
    pub fn tick(&mut self) {
        let now = Instant::now();
        let delta_time = now.duration_since(self.last_tick_time).as_secs_f32();
        self.last_tick_time = now;
        
        // Call custom tick handler if one is set
        if let Some(handler) = &self.tick_handler {
            // Save the current message count to detect new messages
            let prev_msg_count = self.output_messages.len();
            
            handler(&mut self.state, &mut self.output_messages, delta_time);
            
            // If tick handler added messages, set the new_output flag
            if self.output_messages.len() > prev_msg_count {
                self.new_output = true;
            }
        }
    }

    /// Process a single character key command, potentially with parameters
    pub fn handle_key_with_params(&mut self, key: char, params: Option<String>) -> bool {
        // First find information about the matching item, keeping lock short
        let (has_submenu, has_action, idx) = {
            let menu = self.current_menu.lock().unwrap();
            let mut found_idx = None;
            let mut has_submenu = false;
            let mut has_action = false;
            
            for (idx, item) in menu.items.iter().enumerate() {
                if item.key == key {
                    has_submenu = item.submenu.is_some();
                    has_action = item.action.is_some();
                    found_idx = Some(idx);
                    break;
                }
            }
            
            (has_submenu, has_action, found_idx)
        };
        
        if let Some(idx) = idx {
            // Process the menu item - handle submenu first
            let mut submenu_to_navigate = None;
            
            if has_submenu {
                // Another lock to get the submenu
                let submenu = {
                    let menu = self.current_menu.lock().unwrap();
                    let item = &menu.items[idx];
                    item.submenu.as_ref().unwrap().clone()
                };
                
                submenu_to_navigate = Some((submenu, self.current_menu.clone()));
            }
            
            // Handle submenu navigation
            if let Some((submenu, current_menu)) = submenu_to_navigate {
                // Make sure the submenu's parent points to the current menu
                {
                    let mut submenu_guard = submenu.lock().unwrap();
                    submenu_guard.parent = Some(current_menu);
                }
                self.current_menu = submenu;
            }
            
            // Now handle action if it exists
            if has_action {
                // Execute the action
                let action_result = self.execute_action_from_idx(idx, params);
                
                // Handle action result
                if let Some(output) = action_result {
                    self.add_output(output);
                }
            }
            
            return true;
        }
        
        // Handle special keys
        if key == 'q' {
            // Only quit from root menu
            let is_root = {
                let menu = self.current_menu.lock().unwrap();
                menu.parent.is_none()
            };
            
            if is_root {
                return false; // Signal to exit the app
            } else {
                self.add_output("Use 'b' to return to previous menu, or navigate to root menu to quit".to_string());
            }
        } else if key == 'b' {
            // Back navigation
            let parent = {
                let menu = self.current_menu.lock().unwrap();
                menu.parent.clone()
            };
            
            if let Some(parent_menu) = parent {
                self.current_menu = parent_menu;
            } else {
                self.add_output("Already at root menu".to_string());
            }
        } else {
            // Unknown key
            self.add_output(format!("Unknown command: {}", key));
        }
        
        true
    }
    
    /// Execute an action with optional parameters in a way that avoids borrow conflicts
    fn execute_action_from_idx(&mut self, idx: usize, params: Option<String>) -> Option<String> {
        // The core issue is that we can't store references to the menu contents after the lock is dropped.
        // We need to extract what we need and then release the lock.
        
        // We'll use this approach:
        // 1. Get the menu lock
        // 2. Check if idx is valid and there's an action
        // 3. Extract a reference to the action closure
        // 4. Call the action directly while holding the lock, then return the result
        
        let result = {
            let menu = self.current_menu.lock().unwrap();
            
            // Check if the index is valid
            if idx >= menu.items.len() {
                return None;
            }
            
            // Get the item
            let item = &menu.items[idx];
            
            // If there's no action, return None
            if item.action.is_none() {
                return None;
            }
            
            // Get the action and call it directly
            let action = item.action.as_ref().unwrap();
            let params_ref = params.as_deref();
            action(&mut self.state, params_ref)
        };
        
        // Return the result
        result
    }
    
    /// Original handle_key method that delegates to handle_key_with_params
    pub fn handle_key(&mut self, key: char) -> bool {
        self.handle_key_with_params(key, None)
    }

    /// Run the application
    pub fn run(&mut self) -> std::io::Result<()> {
        rendering::run(self)
    }

    /// Get the current mode
    pub fn mode(&self) -> Mode {
        self.current_mode
    }
    
    /// Toggle between modes
    pub fn toggle_mode(&mut self) {
        self.current_mode = match self.current_mode {
            Mode::Command => Mode::Scroll,
            Mode::Scroll => Mode::Command,
        };
    }
    
    /// Set a specific mode
    pub fn set_mode(&mut self, mode: Mode) {
        self.current_mode = mode;
    }

    /// Get the current command input buffer
    pub fn input_buffer(&self) -> &str {
        &self.input_buffer
    }
    
    /// Add a character to the input buffer
    pub fn add_to_input_buffer(&mut self, c: char) {
        self.input_buffer.push(c);
    }
    
    /// Clear the input buffer
    pub fn clear_input_buffer(&mut self) {
        self.input_buffer.clear();
    }
    
    /// Remove the last character from the input buffer
    pub fn backspace_input_buffer(&mut self) {
        self.input_buffer.pop();
    }
    
    /// Toggle showing the input box
    pub fn toggle_show_input(&mut self) {
        self.show_input = !self.show_input;
    }
    
    /// Check if input should be shown
    pub fn show_input(&self) -> bool {
        self.show_input
    }
    
    /// Process the current input buffer as a command
    pub fn process_input_buffer(&mut self) -> bool {
        if self.input_buffer.is_empty() {
            return true;
        }
        
        // Create a binding that lives for the entire function
        let input_clone = self.input_buffer.clone();
        let input = input_clone.trim();
        
        // Split input into command and parameters
        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        let command = parts[0].to_lowercase();
        let params = parts.get(1).map(|&s| s.to_string());
        
        let mut result = true;
        
        // Handle special commands
        if command == "quit" || command == "exit" || command == "q" {
            // Quit command - only works from root menu
            let is_root = {
                let menu = self.current_menu.lock().unwrap();
                menu.parent.is_none()
            };
            
            if is_root {
                result = false; // Signal to exit the app
            } else {
                self.add_output("Use 'back' to return to previous menu, or navigate to root menu to quit".to_string());
            }
        } else if command == "back" || command == "b" {
            // Back navigation
            let parent = {
                let menu = self.current_menu.lock().unwrap();
                menu.parent.clone()
            };
            
            if let Some(parent_menu) = parent {
                self.current_menu = parent_menu;
            } else {
                self.add_output("Already at root menu".to_string());
            }
        } else if command.len() == 1 {
            // For backward compatibility with single-character commands
            if let Some(c) = command.chars().next() {
                result = self.handle_key_with_params(c, params);
            }
        } else {
            // Multi-character command processing - find matching menu item
            let (has_submenu, has_action, idx) = {
                let menu = self.current_menu.lock().unwrap();
                let mut found_idx = None;
                let mut has_submenu = false;
                let mut has_action = false;
                
                for (idx, item) in menu.items.iter().enumerate() {
                    if item.description.to_lowercase().contains(&command) {
                        has_submenu = item.submenu.is_some();
                        has_action = item.action.is_some();
                        found_idx = Some(idx);
                        break;
                    }
                }
                
                (has_submenu, has_action, found_idx)
            };
            
            if let Some(idx) = idx {
                // Process the menu item
                let mut submenu_to_navigate = None;
                
                // Handle submenu if present
                if has_submenu {
                    // Another lock to get the submenu
                    let submenu = {
                        let menu = self.current_menu.lock().unwrap();
                        let item = &menu.items[idx];
                        item.submenu.as_ref().unwrap().clone()
                    };
                    
                    submenu_to_navigate = Some((submenu, self.current_menu.clone()));
                }
                
                // Handle submenu navigation
                if let Some((submenu, current_menu)) = submenu_to_navigate {
                    // Make sure the submenu's parent points to the current menu
                    {
                        let mut submenu_guard = submenu.lock().unwrap();
                        submenu_guard.parent = Some(current_menu);
                    }
                    self.current_menu = submenu;
                }
                
                // Now handle action if it exists
                if has_action {
                    // Execute the action
                    let action_result = self.execute_action_from_idx(idx, params);
                    
                    // Handle action result
                    if let Some(output) = action_result {
                        self.add_output(output);
                    }
                }
            } else {
                // Command not found
                self.add_output(format!("Unknown command: {}", command));
            }
        }
        
        self.clear_input_buffer();
        result
    }
}
