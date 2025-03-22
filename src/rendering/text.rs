use std::io::{self, Write};

/// Run the application with text-based interface
pub fn run<T: std::fmt::Debug>(app: &mut crate::Istari<T>) -> io::Result<()> {
    println!("Starting Istari in text mode");
    println!("Type 'quit' or 'exit' to exit the application");
    println!("Type 'back' to navigate to previous menu");
    println!();

    loop {
        // Display the current menu
        print_menu(app);

        // Get user input
        print!("> ");
        io::stdout().flush()?;

        let mut input = String::new();
        io::stdin().read_line(&mut input)?;
        let input = input.trim();

        // Exit if the user types quit/exit and we're at the root menu
        if input == "quit" || input == "exit" || input == "q" {
            // Get a reference to the menu and check if it's root
            let current_menu = app.current_menu();
            let is_root = {
                let menu = current_menu.lock().unwrap();
                menu.parent.is_none()
            };

            if is_root {
                break;
            } else {
                println!("Can only exit from root menu. Use 'back' to navigate to previous menu.");
                continue;
            }
        }

        // Handle back navigation
        if input == "back" || input == "b" {
            // Get a reference to the menu and check if it has a parent
            let current_menu = app.current_menu();
            let has_parent = {
                let menu = current_menu.lock().unwrap();
                menu.parent.is_some()
            };

            if has_parent {
                // Navigate back
                let success = app.handle_key("b");
                if !success {
                    // If handle_key returns false, it means we should exit
                    break;
                }
            } else {
                println!("Already at root menu");
            }
            continue;
        }

        // Process other commands
        let parts: Vec<&str> = input.splitn(2, ' ').collect();
        let command = parts[0].to_lowercase();
        let params = parts.get(1).map(|&s| s.to_string());

        // Look for matching menu item
        let should_continue = app.handle_key_with_params(command, params);
        if !should_continue {
            break;
        }

        // Show output messages
        print_output(app);

        // Simulate a tick for any automatic updates
        app.tick();

        println!("\n");
    }

    println!("Exiting Istari");
    Ok(())
}

/// Print the current menu
fn print_menu<T: std::fmt::Debug>(app: &crate::Istari<T>) {
    let current_menu = app.current_menu();
    let menu = current_menu.lock().unwrap();

    println!("==== {} ====", menu.title);

    for item in &menu.items {
        println!("[{}] {}", item.key, item.description);
    }

    // Show back or quit option
    if menu.parent.is_some() {
        println!("[b] Back");
    } else {
        println!("[q] Quit");
    }

    println!();
}

/// Print output messages
fn print_output<T: std::fmt::Debug>(app: &mut crate::Istari<T>) {
    let messages = app.output_messages().to_vec(); // Clone the messages
    if !messages.is_empty() {
        println!("\n----- Output -----");
        for msg in messages {
            println!("{}", msg);
        }
        println!("-----------------");
        app.clear_output_messages(); // Clear messages after printing
    }
}
