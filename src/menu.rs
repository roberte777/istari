use crate::error::{IstariError, RESERVED_KEYS};
use crate::types::{ActionType, IntoActionFn};
use std::fmt;
use std::sync::{Arc, Mutex};

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

    /// Validate menu structure to ensure no duplicate or reserved keys
    pub fn validate_menu(menu: &Menu<T>) -> Result<(), IstariError> {
        let mut seen_keys = std::collections::HashSet::new();

        // Check for duplicate and reserved keys in this menu
        for item in &menu.items {
            // Check if key is reserved
            if RESERVED_KEYS.contains(&item.key.as_str()) {
                return Err(IstariError::ReservedCommand(
                    item.key.clone(),
                    menu.title.clone(),
                ));
            }

            // Check if key is a duplicate
            if !seen_keys.insert(item.key.clone()) {
                return Err(IstariError::DuplicateCommand(
                    item.key.clone(),
                    menu.title.clone(),
                ));
            }

            // Recursively validate submenu if it exists
            if let Some(submenu) = &item.submenu {
                Self::validate_menu(&submenu.lock().unwrap())?;
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Debug)]
    pub struct TestState {
        pub counter: i32,
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
        let result = Menu::validate_menu(&menu);

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
        let result = Menu::validate_menu(&menu);

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
        let result = Menu::validate_menu(&root_menu);

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
        let result = Menu::validate_menu(&root_menu);

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
