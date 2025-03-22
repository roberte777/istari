use futures::future::BoxFuture;
use std::future::Future;

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
pub type AsyncActionFn<T> =
    Box<dyn Fn(&mut T, Option<&str>) -> BoxFuture<'static, Option<String>> + Send + Sync>;

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
