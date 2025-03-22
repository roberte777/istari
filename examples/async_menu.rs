use futures::future::BoxFuture;
use istari::{Istari, Menu};
use std::io;
use std::time::Duration;

/// Simple application state with a counter
#[derive(Debug)]
struct AppState {
    counter: i32,
    async_counter: i32,
    last_operation: String,
}

fn main() -> io::Result<()> {
    // Create our application state
    let state = AppState {
        counter: 0,
        async_counter: 0,
        last_operation: "None".to_string(),
    };

    // Create the root menu
    let mut root_menu = Menu::new("Async/Sync Demo");

    // Add synchronous actions
    root_menu.add_action(
        'i',
        "Synchronously Increment Counter",
        |state: &mut AppState, params: Option<&str>| {
            let amount = params.and_then(|p| p.parse::<i32>().ok()).unwrap_or(1);
            state.counter += amount;
            state.last_operation = format!("Sync increment by {}", amount);
            Some(format!(
                "Counter synchronously incremented by {} to {}",
                amount, state.counter
            ))
        },
    );

    root_menu.add_action(
        'd',
        "Synchronously Decrement Counter",
        |state: &mut AppState, params: Option<&str>| {
            let amount = params.and_then(|p| p.parse::<i32>().ok()).unwrap_or(1);
            state.counter -= amount;
            state.last_operation = format!("Sync decrement by {}", amount);
            Some(format!(
                "Counter synchronously decremented by {} to {}",
                amount, state.counter
            ))
        },
    );

    // Add asynchronous actions
    root_menu.add_action(
        'a',
        "Asynchronously Increment Counter (with delay)",
        |state: &mut AppState, params: Option<&str>| {
            let amount = params.and_then(|p| p.parse::<i32>().ok()).unwrap_or(1);
            let delay_ms = 1000; // 1 second delay to simulate async work
            
            // Mutate state here before the async block
            state.async_counter += amount;
            state.last_operation = format!("Async increment by {} after {}ms", amount, delay_ms);
            
            // Capture values by value, not by reference
            let new_counter = state.async_counter;
            
            Box::pin(async move {
                // Simulate some async work
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                
                Some(format!(
                    "Async counter incremented by {} to {} (after {}ms delay)",
                    amount, new_counter, delay_ms
                ))
            })
        },
    );

    root_menu.add_action(
        's',
        "Asynchronously Decrement Counter (with delay)",
        |state: &mut AppState, params: Option<&str>| {
            let amount = params.and_then(|p| p.parse::<i32>().ok()).unwrap_or(1);
            let delay_ms = 500; // 0.5 second delay
            
            // Mutate state here before the async block
            state.async_counter -= amount;
            state.last_operation = format!("Async decrement by {} after {}ms", amount, delay_ms);
            
            // Capture values by value, not by reference
            let new_counter = state.async_counter;
            
            Box::pin(async move {
                // Simulate some async work
                tokio::time::sleep(Duration::from_millis(delay_ms)).await;
                
                Some(format!(
                    "Async counter decremented by {} to {} (after {}ms delay)",
                    amount, new_counter, delay_ms
                ))
            })
        },
    );

    // Status action
    root_menu.add_action(
        'v',
        "View Current State",
        |state: &mut AppState, _params: Option<&str>| {
            Some(format!(
                "Current State:\n- Sync Counter: {}\n- Async Counter: {}\n- Last Operation: {}",
                state.counter, state.async_counter, state.last_operation
            ))
        },
    );

    // Create and run the application
    let mut app = Istari::new(root_menu, state);
    app.run()?;

    Ok(())
} 