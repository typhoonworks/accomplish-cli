use colored::*;
use rand::seq::SliceRandom;
use std::io::{self, Write};
use std::time::{Duration, Instant};
use tokio::time::sleep;

const SPINNER_CHARS: &[char] = &['⠋', '⠙', '⠹', '⠸', '⠼', '⠴', '⠦', '⠧', '⠇', '⠏'];

const WAITING_PHRASES: &[&str] = &[
    "Brewing logs",
    "Stewing updates",
    "Cooking recap",
    "Crunching entries",
    "Stirring summary",
    "Toasting tasks",
    "Wrangling notes",
    "Baking blurbs",
    "Jamming highlights",
    "Squeezing reports",
    "Glazing commits",
    "Mashing bullets",
    "Zapping thoughts",
    "Churning work",
    "Entering digest mode",
    "Reflecting deeply",
    "Pureeing patches",
    "Lassoing lists",
    "Simmering lines",
    "Zapping TL;DR",
    "Rounding up recap",
    "Tickling logs",
    "Whirling worklogs",
    "Zapping summaries",
    "Crunching entries",
    "Twirling tasks",
    "Nibbling notes",
    "Waltzing through work",
    "Spinning summaries",
    "Lassoing logs",
    "Romping through recap",
    "Teasing tasks",
    "Zapping entries",
    "Sizzling summaries",
    "Winking at work",
];

pub struct Spinner {
    start_time: Instant,
    current_phrase: String,
}

impl Spinner {
    pub fn new() -> Self {
        let mut rng = rand::thread_rng();
        let phrase = WAITING_PHRASES
            .choose(&mut rng)
            .unwrap_or(&"Processing")
            .to_string();

        Self {
            start_time: Instant::now(),
            current_phrase: phrase,
        }
    }

    pub async fn spin_with_callback<F, Fut, T>(&mut self, callback: F) -> T
    where
        F: Fn() -> Fut + Send + Sync,
        Fut: std::future::Future<Output = Option<T>> + Send,
        T: Send,
    {
        let mut spinner_index = 0;
        let mut last_check = Instant::now();
        let check_interval = Duration::from_secs(2); // Check API every 2 seconds

        loop {
            // Show spinner (ticks every 100ms for smooth animation)
            self.display_spinner(spinner_index);

            // Check if it's time to poll the API
            if last_check.elapsed() >= check_interval {
                if let Some(result) = callback().await {
                    self.clear_line();
                    return result;
                }
                last_check = Instant::now();
            }

            // Advance spinner and wait (1 second intervals for time display)
            spinner_index = (spinner_index + 1) % SPINNER_CHARS.len();
            sleep(Duration::from_millis(100)).await;
        }
    }

    fn display_spinner(&self, spinner_index: usize) {
        let elapsed = self.start_time.elapsed();
        let seconds = elapsed.as_secs();

        let spinner_char = SPINNER_CHARS[spinner_index];
        let display = format!(
            "\r{} {}... ({}s)",
            spinner_char.to_string().bright_red(),
            self.current_phrase.bright_red(),
            seconds
        );

        print!("{display}");
        io::stdout().flush().unwrap();
    }

    fn clear_line(&self) {
        print!("\r{}\r", " ".repeat(80));
        io::stdout().flush().unwrap();
    }
}

impl Default for Spinner {
    fn default() -> Self {
        Self::new()
    }
}
