use istari::{Istari, Menu};
use std::io;
use std::time::{Duration, Instant};

/// Application state with animated content
#[derive(Debug)]
struct AnimatedState {
    counter: i32,
    // Animation state
    animation_frame: usize,
    animation_frames: Vec<String>,
    last_update: Instant,
    update_interval: Duration,
    // Timer state
    timer_active: bool,
    timer_start: Option<Instant>,
    timer_duration: Duration,
}

impl AnimatedState {
    fn new() -> Self {
        Self {
            counter: 0,
            animation_frame: 0,
            animation_frames: vec![
                "Loading |".to_string(),
                "Loading /".to_string(),
                "Loading -".to_string(),
                "Loading \\".to_string(),
            ],
            last_update: Instant::now(),
            update_interval: Duration::from_millis(250),
            timer_active: false,
            timer_start: None,
            timer_duration: Duration::from_secs(10),
        }
    }

    fn update_animation(&mut self) -> Option<String> {
        let now = Instant::now();
        if now.duration_since(self.last_update) >= self.update_interval {
            self.last_update = now;
            self.animation_frame = (self.animation_frame + 1) % self.animation_frames.len();

            // If timer is active, display remaining time
            if self.timer_active {
                if let Some(start) = self.timer_start {
                    let elapsed = now.duration_since(start);
                    if elapsed >= self.timer_duration {
                        // Timer finished
                        self.timer_active = false;
                        self.timer_start = None;
                        return Some("Timer completed!".to_string());
                    } else {
                        let remaining = self.timer_duration.as_secs() - elapsed.as_secs();
                        return Some(format!(
                            "{} (Timer: {}s remaining)",
                            self.animation_frames[self.animation_frame], remaining
                        ));
                    }
                }
            }

            return Some(self.animation_frames[self.animation_frame].clone());
        }
        None
    }
}

fn main() -> io::Result<()> {
    // Create our application state
    let state = AnimatedState::new();

    // Create the root menu
    let mut root_menu = Menu::new("Animated Demo");

    // Add actions
    root_menu.add_action(
        '1',
        "Increment Counter (optional amount)",
        |state: &mut AnimatedState, params: Option<&str>| {
            let amount = params.and_then(|p| p.parse::<i32>().ok()).unwrap_or(1);
            state.counter += amount;
            Some(format!(
                "Counter incremented by {} to {}",
                amount, state.counter
            ))
        },
    );

    root_menu.add_action(
        '2',
        "Decrement Counter (optional amount)",
        |state: &mut AnimatedState, params: Option<&str>| {
            let amount = params.and_then(|p| p.parse::<i32>().ok()).unwrap_or(1);
            state.counter -= amount;
            Some(format!(
                "Counter decremented by {} to {}",
                amount, state.counter
            ))
        },
    );

    root_menu.add_action(
        't',
        "Start Timer (seconds)",
        |state: &mut AnimatedState, params: Option<&str>| {
            let seconds = params.and_then(|p| p.parse::<u64>().ok()).unwrap_or(10);
            state.timer_active = true;
            state.timer_start = Some(Instant::now());
            state.timer_duration = Duration::from_secs(seconds);
            Some(format!("Timer started! ({} seconds)", seconds))
        },
    );

    root_menu.add_action(
        'a',
        "Toggle Animation",
        |state: &mut AnimatedState, _params: Option<&str>| {
            state.timer_active = !state.timer_active;
            if state.timer_active {
                state.timer_start = None; // Don't count down, just animate
                Some("Animation started!".to_string())
            } else {
                Some("Animation stopped!".to_string())
            }
        },
    );

    // Define the animation tick handler
    let tick_handler = |state: &mut AnimatedState, messages: &mut Vec<String>, _delta: f32| {
        // Update animation and add output message if needed
        if let Some(message) = state.update_animation() {
            messages.push(message);

            // Keep only the last 10 messages to avoid cluttering the display
            if messages.len() > 10 {
                messages.remove(0);
            }
        }
    };

    // Create the Istari app with our custom tick handler
    let mut app = Istari::new(root_menu, state)
        .map_err(|e| io::Error::new(io::ErrorKind::InvalidInput, e))?
        .with_tick_handler(tick_handler);

    app.run()?;

    Ok(())
}
