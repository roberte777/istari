use crate::error::IstariError;
use crate::menu::Menu;
use crate::types::{ActionType, IntoTickFn, Mode, TickFn};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio;

/// Defines the rendering method used by the application
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum RenderMode {
    /// Full terminal UI with ratatui
    TUI,
    /// Simple text-based interface
    Text,
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
    /// Rendering mode (TUI or Text)
    render_mode: RenderMode,
    /// Command history
    command_history: Vec<String>,
    /// Current position in command history (None means not browsing history)
    history_position: Option<usize>,
    /// Maximum number of commands to keep in history
    max_history_size: usize,
}

impl<T: std::fmt::Debug> Istari<T> {
    /// Create a new Istari application with the given root menu and state
    pub fn new(root_menu: Menu<T>, state: T) -> Result<Self, IstariError> {
        // Validate the menu structure
        Menu::validate_menu(&root_menu)?;

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
            render_mode: RenderMode::TUI, // Default to TUI mode
            command_history: Vec::new(),
            history_position: None,
            max_history_size: 100,
        })
    }

    /// Set a custom tick handler
    pub fn with_tick_handler<F>(mut self, handler: F) -> Self
    where
        F: IntoTickFn<T>,
    {
        self.tick_handler = Some(handler.into_tick_fn());
        self
    }

    /// Set the rendering mode
    pub fn with_render_mode(mut self, mode: RenderMode) -> Self {
        self.render_mode = mode;
        self
    }

    /// Set the maximum number of commands to keep in history
    pub fn with_max_history_size(mut self, size: usize) -> Self {
        self.max_history_size = size;
        self
    }

    /// Get the current rendering mode
    pub fn render_mode(&self) -> RenderMode {
        self.render_mode
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

    /// Clear all output messages
    pub fn clear_output_messages(&mut self) {
        self.output_messages.clear();
        self.new_output = false;
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
    pub fn handle_key_with_params(
        &mut self,
        key: impl Into<String>,
        params: Option<String>,
    ) -> bool {
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
            item.action.as_ref()?;

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
        crate::rendering::run(self)
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

        // Add command to history only if it's not empty and different from the last entry
        if !input.is_empty() {
            if self.command_history.is_empty() || self.command_history.last().unwrap() != input {
                self.command_history.push(input.to_string());

                // Trim history if it exceeds the maximum size
                if self.command_history.len() > self.max_history_size {
                    self.command_history.remove(0);
                }
            }
        }

        // Reset history position
        self.history_position = None;

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

    /// Navigate up in command history
    pub fn history_up(&mut self) {
        // If not currently browsing history, save current input and start from the end
        if self.history_position.is_none() {
            if self.command_history.is_empty() {
                return;
            }
            self.history_position = Some(self.command_history.len() - 1);
        } else {
            // Move up in history (to older commands)
            if let Some(pos) = self.history_position {
                if pos > 0 {
                    self.history_position = Some(pos - 1);
                }
            }
        }

        // Update input buffer with the selected history item
        if let Some(pos) = self.history_position {
            if let Some(cmd) = self.command_history.get(pos) {
                self.input_buffer = cmd.clone();
            }
        }
    }

    /// Navigate down in command history
    pub fn history_down(&mut self) {
        // Only act if we're browsing history
        if let Some(pos) = self.history_position {
            // Move down in history (to newer commands)
            if pos < self.command_history.len() - 1 {
                self.history_position = Some(pos + 1);
                if let Some(cmd) = self.command_history.get(pos + 1) {
                    self.input_buffer = cmd.clone();
                }
            } else {
                // We've reached the end of history, return to empty input
                self.history_position = None;
                self.input_buffer.clear();
            }
        }
    }

    /// Exit history browsing mode
    pub fn exit_history_browsing(&mut self) {
        self.history_position = None;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::Menu;

    #[derive(Debug)]
    pub struct TestState {
        pub counter: i32,
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
        let mut app = Istari::new(menu, state).unwrap().with_tick_handler(
            |state: &mut TestState, messages: &mut Vec<String>, _delta: f32| {
                state.counter += 1;
                messages.push(format!("Tick: {}", state.counter));
            },
        );

        // Simulate a tick
        app.tick();
        assert_eq!(app.output_messages().len(), 1);
        assert_eq!(app.output_messages()[0], "Tick: 1");
    }
}
