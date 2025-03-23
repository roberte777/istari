use crate::error::IstariError;
use crate::menu::Menu;
use crate::types::ActionType;
use std::sync::{Arc, Mutex};

/// Manages menu navigation and action execution
pub struct MenuManager<T> {
    /// The current menu being displayed
    current_menu: Arc<Mutex<Menu<T>>>,
}

impl<T: std::fmt::Debug> MenuManager<T> {
    /// Create a new menu manager with the given root menu
    pub fn new(root_menu: Menu<T>) -> Result<Self, IstariError> {
        // Validate the menu structure
        Menu::validate_menu(&root_menu)?;

        Ok(Self {
            current_menu: Arc::new(Mutex::new(root_menu)),
        })
    }

    /// Get a reference to the current menu
    pub fn current_menu(&self) -> Arc<Mutex<Menu<T>>> {
        self.current_menu.clone()
    }

    /// Navigate to a submenu by key
    pub fn navigate_to_submenu(&mut self, key: &str) -> bool {
        // First find the menu item with the given key
        let (has_submenu, idx) = {
            let menu = self.current_menu.lock().unwrap();
            let mut found_idx = None;
            let mut has_submenu = false;

            for (idx, item) in menu.items.iter().enumerate() {
                if item.key.to_lowercase() == key.to_lowercase() {
                    has_submenu = item.submenu.is_some();
                    found_idx = Some(idx);
                    break;
                }
            }

            (has_submenu, found_idx)
        };

        if let Some(idx) = idx {
            if has_submenu {
                // Get the submenu
                let submenu = {
                    let menu = self.current_menu.lock().unwrap();
                    let item = &menu.items[idx];
                    item.submenu.as_ref().unwrap().clone()
                };

                // Set the parent of the submenu to the current menu
                {
                    let mut submenu_guard = submenu.lock().unwrap();
                    submenu_guard.parent = Some(self.current_menu.clone());
                }

                // Update the current menu
                self.current_menu = submenu;
                return true;
            }
        }

        false
    }

    /// Navigate back to the parent menu
    pub fn navigate_back(&mut self) -> bool {
        let parent = {
            let menu = self.current_menu.lock().unwrap();
            menu.parent.clone()
        };

        if let Some(parent_menu) = parent {
            self.current_menu = parent_menu;
            true
        } else {
            false
        }
    }

    /// Check if the current menu is the root menu
    pub fn is_at_root(&self) -> bool {
        let menu = self.current_menu.lock().unwrap();
        menu.parent.is_none()
    }

    /// Find a menu item by key
    fn find_item_idx(&self, key: &str) -> Option<usize> {
        let menu = self.current_menu.lock().unwrap();

        for (idx, item) in menu.items.iter().enumerate() {
            if item.key.to_lowercase() == key.to_lowercase() {
                return Some(idx);
            }
        }

        None
    }

    /// Check if a menu item has an action
    pub fn has_action(&self, key: &str) -> bool {
        if let Some(idx) = self.find_item_idx(key) {
            let menu = self.current_menu.lock().unwrap();
            let item = &menu.items[idx];
            item.action.is_some()
        } else {
            false
        }
    }

    /// Check if a menu item has a submenu
    pub fn has_submenu(&self, key: &str) -> bool {
        if let Some(idx) = self.find_item_idx(key) {
            let menu = self.current_menu.lock().unwrap();
            let item = &menu.items[idx];
            item.submenu.is_some()
        } else {
            false
        }
    }

    /// Execute an action for a menu item by key
    pub fn execute_action(
        &mut self,
        key: &str,
        state: &mut T,
        params: Option<&str>,
        runtime: &tokio::runtime::Runtime,
    ) -> Option<String> {
        let idx = self.find_item_idx(key)?;

        // Execute the action
        let menu = self.current_menu.lock().unwrap();
        let item = &menu.items[idx];

        // If there's no action, return None
        let action = item.action.as_ref()?;

        // Call the action
        match action {
            ActionType::Sync(sync_fn) => sync_fn(state, params),
            ActionType::Async(async_fn) => runtime.block_on(async {
                let future = async_fn(state, params);
                future.await
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::menu::MenuItem;

    #[derive(Debug)]
    struct TestState {
        counter: i32,
    }

    #[test]
    fn test_menu_navigation() {
        // Create a submenu
        let mut root_menu: Menu<TestState> = Menu::new("Root".to_string());
        root_menu.add_item(MenuItem::new_submenu(
            "s",
            "Submenu".to_string(),
            Menu::<TestState>::new("Submenu".to_string()),
        ));

        // Create the menu manager
        let mut manager = MenuManager::new(root_menu).unwrap();

        // Check root status
        assert!(manager.is_at_root());

        // Navigate to submenu
        assert!(manager.navigate_to_submenu("s"));
        assert!(!manager.is_at_root());

        // Navigate back
        assert!(manager.navigate_back());
        assert!(manager.is_at_root());

        // Try to navigate back from root
        assert!(!manager.navigate_back());

        // Navigate to a non-existent submenu
        assert!(!manager.navigate_to_submenu("x"));
    }

    #[test]
    fn test_action_execution() {
        let mut state = TestState { counter: 0 };

        // Create a menu with an action
        let mut menu = Menu::new("Test".to_string());
        menu.add_item(MenuItem::new_action(
            "a",
            "Increment".to_string(),
            |state: &mut TestState, _: Option<&str>| {
                state.counter += 1;
                Some(format!("Counter: {}", state.counter))
            },
        ));

        // Create the menu manager
        let mut manager = MenuManager::new(menu).unwrap();

        // Check if item has action
        assert!(manager.has_action("a"));
        assert!(!manager.has_submenu("a"));

        // Execute the action
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let result = manager.execute_action("a", &mut state, None, &runtime);
        assert_eq!(result, Some("Counter: 1".to_string()));
        assert_eq!(state.counter, 1);

        // Execute with parameters
        let result = manager.execute_action("a", &mut state, Some("param"), &runtime);
        assert_eq!(result, Some("Counter: 2".to_string()));
        assert_eq!(state.counter, 2);

        // Execute non-existent action
        let result = manager.execute_action("x", &mut state, None, &runtime);
        assert_eq!(result, None);
    }
}
