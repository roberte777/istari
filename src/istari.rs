use crate::error::IstariError;
use crate::menu::Menu;
use crate::menu_manager::MenuManager;
use crate::types::{IntoTickFn, Mode, TickFn};
use std::sync::{Arc, Mutex};
use std::time::Instant;
use tokio;

/// Defines the user interface mode used by the application
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UIMode {
    /// Full terminal UI with ratatui
    TUI,
    /// Simple text-based interface
    Text,
}

/// Manages command history with navigation capabilities
#[derive(Debug, Clone)]
pub struct CommandHistory {
    /// Command history
    entries: Vec<String>,
    /// Current position in command history (None means not browsing history)
    position: Option<usize>,
    /// Maximum number of commands to keep in history
    max_size: usize,
}

impl CommandHistory {
    /// Create a new command history
    pub fn new(max_size: usize) -> Self {
        Self {
            entries: Vec::new(),
            position: None,
            max_size,
        }
    }

    /// Add a command to history
    pub fn add(&mut self, command: String) {
        if command.is_empty() {
            return;
        }

        // Don't add duplicate of the last command
        if !self.entries.is_empty() && self.entries.last().unwrap() == &command {
            return;
        }

        self.entries.push(command);

        // Trim history if it exceeds the maximum size
        if self.entries.len() > self.max_size {
            self.entries.remove(0);
        }

        // Reset navigation position
        self.position = None;
    }

    /// Navigate up in history (to older commands)
    pub fn up(&mut self) -> Option<&String> {
        if self.entries.is_empty() {
            return None;
        }

        // If not browsing, start from the end
        if self.position.is_none() {
            self.position = Some(self.entries.len() - 1);
        } else if let Some(pos) = self.position {
            // Move up if possible
            if pos > 0 {
                self.position = Some(pos - 1);
            }
        }

        // Return the current entry
        self.position.and_then(|pos| self.entries.get(pos))
    }

    /// Navigate down in history (to newer commands)
    pub fn down(&mut self) -> Option<&String> {
        if self.position.is_none() {
            return None;
        }

        let pos = self.position.unwrap();
        if pos < self.entries.len() - 1 {
            // Move to newer command
            self.position = Some(pos + 1);
            self.position.and_then(|pos| self.entries.get(pos))
        } else {
            // At the end of history, exit browsing mode
            self.position = None;
            None
        }
    }

    /// Exit history browsing mode
    pub fn exit_browsing(&mut self) {
        self.position = None;
    }
}

/// Manages output messages with notification capabilities
#[derive(Debug, Clone)]
pub struct OutputBuffer {
    /// Output messages
    messages: Vec<String>,
    /// Flag indicating if new messages were added
    new_output: bool,
}

impl OutputBuffer {
    /// Create a new output buffer
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            new_output: false,
        }
    }

    /// Add an output message
    pub fn add(&mut self, message: String) {
        self.messages.push(message);
        self.new_output = true;
    }

    /// Get all messages
    pub fn messages(&self) -> &[String] {
        &self.messages
    }

    /// Check if there's new output and reset the flag
    pub fn has_new_output(&mut self) -> bool {
        let has_new = self.new_output;
        self.new_output = false;
        has_new
    }

    /// Clear all messages
    pub fn clear(&mut self) {
        self.messages.clear();
        self.new_output = false;
    }
}

/// Main application that handles rendering and events
pub struct Istari<T> {
    /// Menu navigation and management
    menu_manager: MenuManager<T>,
    /// Application state shared with menu actions
    state: T,
    /// Output management
    output: OutputBuffer,
    /// Last tick update time, for animations or time-based updates
    last_tick_time: Instant,
    /// Optional tick function that's called on each frame update
    tick_handler: Option<TickFn<T>>,
    /// Current application mode
    current_mode: Mode,
    /// Command input buffer
    input_buffer: String,
    /// Command history management
    command_history: CommandHistory,
    /// Whether the command input should be displayed
    show_input: bool,
    /// Tokio runtime for executing async actions
    runtime: tokio::runtime::Runtime,
    /// User interface mode (TUI or Text)
    ui_mode: UIMode,
}

impl<T: std::fmt::Debug> Istari<T> {
    /// Create a new Istari application with the given root menu and state
    pub fn new(root_menu: Menu<T>, state: T) -> Result<Self, IstariError> {
        Ok(Self {
            menu_manager: MenuManager::new(root_menu)?,
            state,
            output: OutputBuffer::new(),
            last_tick_time: Instant::now(),
            tick_handler: None,
            current_mode: Mode::Command, // Default to command mode
            input_buffer: String::new(),
            command_history: CommandHistory::new(100),
            show_input: false,
            runtime: tokio::runtime::Runtime::new().unwrap(),
            ui_mode: UIMode::TUI, // Default to TUI mode
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

    /// Set the user interface mode
    pub fn with_ui_mode(mut self, mode: UIMode) -> Self {
        self.ui_mode = mode;
        self
    }

    /// Set the maximum number of commands to keep in history
    pub fn with_max_history_size(mut self, size: usize) -> Self {
        self.command_history = CommandHistory::new(size);
        self
    }

    /// Get the current UI mode
    pub fn ui_mode(&self) -> UIMode {
        self.ui_mode
    }

    /// Get a reference to the current menu
    pub fn current_menu(&self) -> Arc<Mutex<Menu<T>>> {
        self.menu_manager.current_menu()
    }

    /// Get a reference to the output messages
    pub fn output_messages(&self) -> &[String] {
        self.output.messages()
    }

    /// Add an output message
    pub fn add_output(&mut self, message: String) {
        self.output.add(message);
    }

    /// Check if there's new output and reset the flag
    pub fn has_new_output(&mut self) -> bool {
        self.output.has_new_output()
    }

    /// Clear all output messages
    pub fn clear_output_messages(&mut self) {
        self.output.clear();
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
            let prev_msg_count = self.output.messages().len();
            let mut output_messages = self.output.messages.clone();

            handler(&mut self.state, &mut output_messages, delta_time);

            // Check if tick handler added messages
            if output_messages.len() > prev_msg_count {
                // Update with new messages
                self.output.messages = output_messages;
                self.output.new_output = true;
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

        // Check for special keys first
        if key_string == "q" {
            // Only quit from root menu
            if self.menu_manager.is_at_root() {
                return false; // Signal to exit the app
            } else {
                self.add_output(
                    "Use 'b' to return to previous menu, or navigate to root menu to quit"
                        .to_string(),
                );
                return true;
            }
        } else if key_string == "b" {
            // Back navigation
            if !self.menu_manager.navigate_back() {
                self.add_output("Already at root menu".to_string());
            }
            return true;
        }

        // Check if the key corresponds to a menu item with a submenu
        if self.menu_manager.has_submenu(&key_string) {
            self.menu_manager.navigate_to_submenu(&key_string);
            return true;
        }

        // Check if the key corresponds to a menu item with an action
        if self.menu_manager.has_action(&key_string) {
            let params_ref = params.as_deref();
            if let Some(result) = self.menu_manager.execute_action(
                &key_string,
                &mut self.state,
                params_ref,
                &self.runtime,
            ) {
                self.add_output(result);
            }
            return true;
        }

        // If we get here, the key wasn't recognized
        self.add_output(format!("Unknown command: {}", key_string));
        true
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

        // Add command to history
        if !input.is_empty() {
            self.command_history.add(input.to_string());
        }

        // Split input into command and parameters
        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        let command = parts[0].to_lowercase();
        let params = parts.get(1).map(|&s| s.to_string());

        // Delegate to handle_key_with_params
        let result = self.handle_key_with_params(command, params);

        self.clear_input_buffer();
        result
    }

    /// Navigate up in command history
    pub fn history_up(&mut self) {
        if let Some(cmd) = self.command_history.up() {
            self.input_buffer = cmd.clone();
        }
    }

    /// Navigate down in command history
    pub fn history_down(&mut self) {
        if let Some(cmd) = self.command_history.down() {
            self.input_buffer = cmd.clone();
        } else {
            // At the end of history or exited browsing mode
            self.input_buffer.clear();
        }
    }

    /// Exit history browsing mode
    pub fn exit_history_browsing(&mut self) {
        self.command_history.exit_browsing();
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

    #[test]
    fn test_command_history() {
        let mut history = CommandHistory::new(3);

        // Add commands
        history.add("cmd1".to_string());
        history.add("cmd2".to_string());
        history.add("cmd3".to_string());

        // Test navigation
        assert_eq!(history.up().unwrap(), "cmd3");
        assert_eq!(history.up().unwrap(), "cmd2");
        assert_eq!(history.up().unwrap(), "cmd1");
        assert_eq!(history.up().unwrap(), "cmd1"); // Can't go past beginning

        assert_eq!(history.down().unwrap(), "cmd2");
        assert_eq!(history.down().unwrap(), "cmd3");
        assert_eq!(history.down(), None); // Exit browsing

        // Test max size
        history.add("cmd4".to_string());
        assert_eq!(history.entries.len(), 3);
        assert_eq!(history.entries[0], "cmd2"); // cmd1 was removed
    }

    #[test]
    fn test_output_buffer() {
        let mut buffer = OutputBuffer::new();

        assert!(buffer.messages().is_empty());
        buffer.add("Test".to_string());
        assert_eq!(buffer.messages().len(), 1);

        assert!(buffer.has_new_output());
        assert!(!buffer.has_new_output());

        buffer.clear();
        assert!(buffer.messages().is_empty());
        assert!(!buffer.has_new_output());
    }
}
