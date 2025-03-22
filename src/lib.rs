use std::fmt;
use std::sync::{Arc, Mutex};
use std::time::Instant;
use std::future::Future;
use futures::future::BoxFuture;
use tokio;
use thiserror::Error;

pub mod rendering;

/// Error types for Istari
#[derive(Error, Debug)]
pub enum IstariError {
    #[error("Duplicate command key '{0}' in menu '{1}'")]
    DuplicateCommand(String, String),
    
    #[error("Reserved command key '{0}' in menu '{1}'")]
    ReservedCommand(String, String),
}

/// Reserved command keys that cannot be used in menus
const RESERVED_KEYS: [&str; 2] = ["q", "b"];

/// Defines the possible application modes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mode {
    /// Mode for navigating menus and triggering actions
    Command,
    /// Mode for scrolling through output with vim-style keybinds
    Scroll,
}

/// Marker structs to differentiate between sync and async functions
pub struct SyncFnMarker;
pub struct AsyncFnMarker;

/// Type for synchronous action functions that can be executed when menu items are selected
pub type ActionFn<T> = Box<dyn Fn(&mut T, Option<&str>) -> Option<String> + Send + Sync>;

/// Type for asynchronous action functions that can be executed when menu items are selected
pub type AsyncActionFn<T> = Box<dyn Fn(&mut T, Option<&str>) -> BoxFuture<'static, Option<String>> + Send + Sync>;

/// Represents either a synchronous or asynchronous action function
pub enum ActionType<T> {
    /// A synchronous action function
    Sync(ActionFn<T>),
    /// An asynchronous action function
    Async(AsyncActionFn<T>),
}

pub type TickFn<T> = Box<dyn Fn(&mut T, &mut Vec<String>, f32) + Send + Sync>;

/// A trait for converting closures to ActionFn
pub trait IntoActionFn<T, Marker>: Send + Sync + 'static {
    fn into_action_fn(self) -> ActionType<T>;
}

/// Implementation for synchronous closures that can be converted to ActionFn
impl<T, F> IntoActionFn<T, SyncFnMarker> for F
where
    F: Fn(&mut T, Option<&str>) -> Option<String> + Send + Sync + 'static,
{
    fn into_action_fn(self) -> ActionType<T> {
        ActionType::Sync(Box::new(self))
    }
}

/// Implementation for asynchronous closures that can be converted to ActionFn
impl<T, F, Fut> IntoActionFn<T, AsyncFnMarker> for F
where
    F: Fn(&mut T, Option<&str>) -> Fut + Send + Sync + 'static,
    Fut: Future<Output = Option<String>> + Send + 'static,
{
    fn into_action_fn(self) -> ActionType<T> {
        ActionType::Async(Box::new(move |state, params| {
            // Clone self to ensure the future doesn't reference the original closure
            let fut = self(state, params);
            // Convert the future to a BoxFuture
            Box::pin(fut)
        }))
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
    pub key: String,
    /// Description of what this item does
    pub description: String,
    /// The function to run when this item is selected
    pub action: Option<ActionType<T>>,
    /// A submenu that this item leads to, if any
    pub submenu: Option<Arc<Mutex<Menu<T>>>>,
}

impl<T> Clone for MenuItem<T> {
    fn clone(&self) -> Self {
        MenuItem {
            key: self.key.clone(),
            description: self.description.clone(),
            action: None, // We can't clone the action function, so we set it to None
            submenu: self.submenu.clone(),
        }
    }
}

impl<T: std::fmt::Debug> fmt::Debug for MenuItem<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("MenuItem")
            .field("key", &self.key)
            .field("description", &self.description)
            .field(
                "action",
                &if self.action.is_some() {
                    "Some(Action)"
                } else {
                    "None"
                },
            )
            .field("submenu", &self.submenu)
            .finish()
    }
}

impl<T> MenuItem<T> {
    /// Create a new menu item with a synchronous action
    pub fn new_action<F, Marker>(key: impl Into<String>, description: String, action: F) -> Self
    where
        F: IntoActionFn<T, Marker>,
    {
        MenuItem {
            key: key.into(),
            description,
            action: Some(action.into_action_fn()),
            submenu: None,
        }
    }

    /// Create a new menu item with a submenu
    pub fn new_submenu(key: impl Into<String>, description: String, submenu: Menu<T>) -> Self {
        MenuItem {
            key: key.into(),
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

    /// Add a synchronous action item to this menu
    pub fn add_action<F, Marker>(
        &mut self,
        key: impl Into<String>,
        description: impl Into<String>,
        action: F,
    ) -> &mut Self
    where
        F: IntoActionFn<T, Marker>,
    {
        self.add_item(MenuItem::new_action(key, description.into(), action))
    }

    /// Add a submenu to this menu
    pub fn add_submenu(
        &mut self,
        key: impl Into<String>,
        description: impl Into<String>,
        mut submenu: Menu<T>,
    ) -> &mut Self {
        // We'll set the parent when navigating to the submenu
        submenu.parent = None;
        self.add_item(MenuItem::new_submenu(key, description.into(), submenu))
    }

    /// Get the item for a given key
    pub fn get_item(&self, key: &str) -> Option<&MenuItem<T>> {
        self.items.iter().find(|item| item.key == key)
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
    /// Tokio runtime for executing async actions
    runtime: tokio::runtime::Runtime,
}

impl<T: std::fmt::Debug> Istari<T> {
    /// Create a new Istari application with the given root menu and state
    pub fn new(root_menu: Menu<T>, state: T) -> Result<Self, IstariError> {
        // Validate the menu structure
        Self::validate_menu(&root_menu)?;

        Ok(Self {
            current_menu: Arc::new(Mutex::new(root_menu)),
            state,
            output_messages: Vec::new(),
            new_output: false,
            last_tick_time: Instant::now(),
            tick_handler: None,
            current_mode: Mode::Command, // Default to command mode
            input_buffer: String::new(),
            show_input: false,
            runtime: tokio::runtime::Runtime::new().unwrap(),
        })
    }

    /// Validate menu structure to ensure no duplicate or reserved keys
    fn validate_menu(menu: &Menu<T>) -> Result<(), IstariError> {
        let mut seen_keys = std::collections::HashSet::new();
        
        // Check for duplicate and reserved keys in this menu
        for item in &menu.items {
            // Check if key is reserved
            if RESERVED_KEYS.contains(&item.key.as_str()) {
                return Err(IstariError::ReservedCommand(item.key.clone(), menu.title.clone()));
            }
            
            // Check if key is a duplicate
            if !seen_keys.insert(item.key.clone()) {
                return Err(IstariError::DuplicateCommand(item.key.clone(), menu.title.clone()));
            }
            
            // Recursively validate submenu if it exists
            if let Some(submenu) = &item.submenu {
                Self::validate_menu(&submenu.lock().unwrap())?;
            }
        }
        
        Ok(())
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
    pub fn handle_key_with_params(&mut self, key: impl Into<String>, params: Option<String>) -> bool {
        let key_string = key.into();
        
        // First find information about the matching item, keeping lock short
        let (has_submenu, has_action, idx) = {
            let menu = self.current_menu.lock().unwrap();
            let mut found_idx = None;
            let mut has_submenu = false;
            let mut has_action = false;

            for (idx, item) in menu.items.iter().enumerate() {
                if item.key == key_string {
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
        if key_string == "q" {
            // Only quit from root menu
            let is_root = {
                let menu = self.current_menu.lock().unwrap();
                menu.parent.is_none()
            };

            if is_root {
                return false; // Signal to exit the app
            } else {
                self.add_output(
                    "Use 'b' to return to previous menu, or navigate to root menu to quit"
                        .to_string(),
                );
            }
        } else if key_string == "b" {
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
            self.add_output(format!("Unknown command: {}", key_string));
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

            match action {
                ActionType::Sync(sync_fn) => sync_fn(&mut self.state, params_ref),
                ActionType::Async(async_fn) => {
                    // Use the shared runtime instead of creating a new one
                    self.runtime.block_on(async {
                        let future = async_fn(&mut self.state, params_ref);
                        future.await
                    })
                }
            }
        };

        // Return the result
        result
    }

    /// Original handle_key method that delegates to handle_key_with_params
    pub fn handle_key(&mut self, key: impl Into<String>) -> bool {
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
                self.add_output(
                    "Use 'back' to return to previous menu, or navigate to root menu to quit"
                        .to_string(),
                );
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
        } else {
            // Try to match on the command key
            let (has_submenu, has_action, idx) = {
                let menu = self.current_menu.lock().unwrap();
                let mut found_idx = None;
                let mut has_submenu = false;
                let mut has_action = false;

                for (idx, item) in menu.items.iter().enumerate() {
                    if item.key.to_lowercase() == command {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;
    use std::time::Duration;

    #[derive(Debug)]
    struct TestState {
        counter: i32,
    }

    #[test]
    fn test_menu_creation() {
        let menu: Menu<TestState> = Menu::new("Test Menu".to_string());
        assert_eq!(menu.title, "Test Menu");
        assert!(menu.items.is_empty());
        assert!(menu.parent.is_none());
    }

    #[test]
    fn test_menu_item_creation() {
        let item = MenuItem::new_action(
            "1".to_string(),
            "Test Action".to_string(),
            |state: &mut TestState, _params: Option<&str>| {
                state.counter += 1;
                Some("Action executed".to_string())
            },
        );
        assert_eq!(item.key, "1");
        assert_eq!(item.description, "Test Action");
        assert!(item.action.is_some());
        assert!(item.submenu.is_none());
    }

    #[test]
    fn test_menu_validation_duplicate_keys() {
        let mut menu: Menu<TestState> = Menu::new("Test Menu".to_string());
        
        // Add first action
        menu.add_action(
            "1".to_string(),
            "First Action".to_string(),
            |_state: &mut TestState, _params: Option<&str>| Some("First".to_string()),
        );
        
        // Add duplicate key
        menu.add_action(
            "1".to_string(),
            "Duplicate Action".to_string(),
            |_state: &mut TestState, _params: Option<&str>| Some("Second".to_string()),
        );
        
        // Try to create Istari with invalid menu
        let state = TestState { counter: 0 };
        let result = Istari::new(menu, state);
        
        assert!(result.is_err());
        if let Err(IstariError::DuplicateCommand(key, menu_title)) = result {
            assert_eq!(key, "1");
            assert_eq!(menu_title, "Test Menu");
        } else {
            panic!("Expected DuplicateCommand error");
        }
    }

    #[test]
    fn test_menu_validation_reserved_keys() {
        let mut menu: Menu<TestState> = Menu::new("Test Menu".to_string());
        
        // Add action with reserved key 'q'
        menu.add_action(
            "q".to_string(),
            "Reserved Key Action".to_string(),
            |_state: &mut TestState, _params: Option<&str>| Some("Reserved".to_string()),
        );
        
        // Try to create Istari with invalid menu
        let state = TestState { counter: 0 };
        let result = Istari::new(menu, state);
        
        assert!(result.is_err());
        if let Err(IstariError::ReservedCommand(key, menu_title)) = result {
            assert_eq!(key, "q");
            assert_eq!(menu_title, "Test Menu");
        } else {
            panic!("Expected ReservedCommand error");
        }
    }

    #[test]
    fn test_menu_validation_nested_duplicate_keys() {
        let mut root_menu: Menu<TestState> = Menu::new("Root Menu".to_string());
        let mut submenu: Menu<TestState> = Menu::new("Submenu".to_string());
        
        // Add duplicate keys in submenu
        submenu.add_action(
            "1".to_string(),
            "First Action".to_string(),
            |_state: &mut TestState, _params: Option<&str>| Some("First".to_string()),
        );
        
        submenu.add_action(
            "1".to_string(),
            "Duplicate Action".to_string(),
            |_state: &mut TestState, _params: Option<&str>| Some("Second".to_string()),
        );
        
        root_menu.add_submenu("s".to_string(), "Go to Submenu".to_string(), submenu);
        
        // Try to create Istari with invalid menu
        let state = TestState { counter: 0 };
        let result = Istari::new(root_menu, state);
        
        assert!(result.is_err());
        if let Err(IstariError::DuplicateCommand(key, menu_title)) = result {
            assert_eq!(key, "1");
            assert_eq!(menu_title, "Submenu");
        } else {
            panic!("Expected DuplicateCommand error");
        }
    }

    #[test]
    fn test_menu_validation_nested_reserved_keys() {
        let mut root_menu: Menu<TestState> = Menu::new("Root Menu".to_string());
        let mut submenu: Menu<TestState> = Menu::new("Submenu".to_string());
        
        // Add action with reserved key in submenu
        submenu.add_action(
            "q".to_string(),
            "Reserved Key Action".to_string(),
            |_state: &mut TestState, _params: Option<&str>| Some("Reserved".to_string()),
        );
        
        root_menu.add_submenu("s".to_string(), "Go to Submenu".to_string(), submenu);
        
        // Try to create Istari with invalid menu
        let state = TestState { counter: 0 };
        let result = Istari::new(root_menu, state);
        
        assert!(result.is_err());
        if let Err(IstariError::ReservedCommand(key, menu_title)) = result {
            assert_eq!(key, "q");
            assert_eq!(menu_title, "Submenu");
        } else {
            panic!("Expected ReservedCommand error");
        }
    }

    #[test]
    fn test_menu_navigation() {
        let mut root_menu: Menu<TestState> = Menu::new("Root".to_string());
        let mut submenu: Menu<TestState> = Menu::new("Submenu".to_string());
        
        submenu.add_action(
            "1".to_string(),
            "Submenu Action".to_string(),
            |state: &mut TestState, _params: Option<&str>| {
                state.counter += 1;
                Some("Submenu action executed".to_string())
            },
        );
        
        root_menu.add_submenu("s".to_string(), "Go to Submenu".to_string(), submenu);
        
        let item = root_menu.get_item("s").unwrap();
        assert!(item.submenu.is_some());
        
        let submenu = item.submenu.as_ref().unwrap().lock().unwrap();
        assert_eq!(submenu.title, "Submenu");
        assert_eq!(submenu.items.len(), 1);
    }

    #[test]
    fn test_istari_creation() {
        let state = TestState { counter: 0 };
        let menu: Menu<TestState> = Menu::new("Test Menu".to_string());
        
        let result = Istari::new(menu, state);
        assert!(result.is_ok());
        
        let app = result.unwrap();
        assert_eq!(app.mode(), Mode::Command);
        assert!(app.output_messages().is_empty());
        assert!(!app.show_input());
    }

    #[test]
    fn test_mode_toggling() {
        let state = TestState { counter: 0 };
        let menu: Menu<TestState> = Menu::new("Test Menu".to_string());
        let mut app = Istari::new(menu, state).unwrap();
        
        assert_eq!(app.mode(), Mode::Command);
        app.toggle_mode();
        assert_eq!(app.mode(), Mode::Scroll);
        app.toggle_mode();
        assert_eq!(app.mode(), Mode::Command);
        
        app.set_mode(Mode::Scroll);
        assert_eq!(app.mode(), Mode::Scroll);
    }

    #[test]
    fn test_input_buffer() {
        let state = TestState { counter: 0 };
        let menu: Menu<TestState> = Menu::new("Test Menu".to_string());
        let mut app = Istari::new(menu, state).unwrap();
        
        assert!(app.input_buffer().is_empty());
        app.add_to_input_buffer('t');
        app.add_to_input_buffer('e');
        app.add_to_input_buffer('s');
        app.add_to_input_buffer('t');
        assert_eq!(app.input_buffer(), "test");
        
        app.backspace_input_buffer();
        assert_eq!(app.input_buffer(), "tes");
        
        app.clear_input_buffer();
        assert!(app.input_buffer().is_empty());
    }

    #[test]
    fn test_output_messages() {
        let state = TestState { counter: 0 };
        let menu: Menu<TestState> = Menu::new("Test Menu".to_string());
        let mut app = Istari::new(menu, state).unwrap();
        
        assert!(app.output_messages().is_empty());
        app.add_output("Test message".to_string());
        assert_eq!(app.output_messages().len(), 1);
        assert_eq!(app.output_messages()[0], "Test message");
        
        assert!(app.has_new_output());
        assert!(!app.has_new_output());
    }

    #[test]
    fn test_tick_handler() {
        let state = TestState { counter: 0 };
        let menu: Menu<TestState> = Menu::new("Test Menu".to_string());
        let mut app = Istari::new(menu, state)
            .unwrap()
            .with_tick_handler(|state: &mut TestState, messages: &mut Vec<String>, _delta: f32| {
                state.counter += 1;
                messages.push(format!("Tick: {}", state.counter));
            });
        
        // Simulate a tick
        app.tick();
        assert_eq!(app.output_messages().len(), 1);
        assert_eq!(app.output_messages()[0], "Tick: 1");
    }

    #[test]
    fn test_menu_item_clone() {
        let item = MenuItem::new_action(
            "1".to_string(),
            "Test Action".to_string(),
            |_state: &mut TestState, _params: Option<&str>| Some("Action".to_string()),
        );
        
        let cloned = item.clone();
        assert_eq!(cloned.key, item.key);
        assert_eq!(cloned.description, item.description);
        assert!(cloned.action.is_none()); // Action should be None in clone
        assert!(cloned.submenu.is_none());
    }

    #[test]
    fn test_menu_debug() {
        let mut menu: Menu<TestState> = Menu::new("Test Menu".to_string());
        menu.add_action(
            "1".to_string(),
            "Test Action".to_string(),
            |_state: &mut TestState, _params: Option<&str>| Some("Action".to_string()),
        );
        
        let debug_string = format!("{:?}", menu);
        assert!(debug_string.contains("Test Menu"));
        assert!(debug_string.contains("Test Action"));
    }
}
