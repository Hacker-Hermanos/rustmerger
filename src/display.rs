use std::io::{self, Stdout, Write}; // Importing necessary modules from the standard library
use std::time::Instant; // Importing Instant for tracking elapsed time

// Struct to manage status display on the terminal
pub struct StatusDisplay {
    stdout: Stdout,          // Standard output handle
    last_line_length: usize, // Length of the last printed line
    terminal_width: usize,   // Width of the terminal
    start_time: Instant,     // Start time to track elapsed time
}

impl StatusDisplay {
    // Function to create a new StatusDisplay instance
    pub fn new() -> io::Result<Self> {
        let stdout = io::stdout(); // Get the standard output handle
        let terminal_width = terminal_size::terminal_size() // Get the terminal size
            .map(|(w, _)| w.0 as usize) // Extract the width and convert to usize
            .unwrap_or(80); // Default to 80 if terminal size is not available
        let start_time = Instant::now(); // Record the current time as start time

        Ok(Self {
            stdout,              // Initialize stdout
            last_line_length: 0, // Initialize last line length to 0
            terminal_width,      // Initialize terminal width
            start_time,          // Initialize start time
        })
    }

    // Function to update the status message on the terminal
    pub fn update_status(&mut self, message: &str) -> io::Result<()> {
        // Clear the previous line
        write!(self.stdout, "\r")?; // Move cursor to the beginning of the line
        for _ in 0..self.last_line_length {
            write!(self.stdout, " ")?; // Overwrite the previous line with spaces
        }
        write!(self.stdout, "\r")?; // Move cursor to the beginning of the line again

        // Write the new message
        write!(self.stdout, "{}", message)?; // Print the new message
        self.stdout.flush()?; // Flush the output to ensure it is displayed

        // Update the last line length
        self.last_line_length = message.len(); // Store the length of the new message

        Ok(())
    }

    // Function to update the progress bar on the terminal
    pub fn update_progress(
        &mut self,
        current: usize,
        total: usize,
        message: &str,
    ) -> io::Result<()> {
        let percentage = (current as f64 / total as f64 * 100.0) as usize; // Calculate the progress percentage
        let bar_width = 30; // Width of the progress bar
        let filled = (bar_width as f64 * (current as f64 / total as f64)) as usize; // Calculate the filled portion of the bar

        // Create the progress bar string
        let bar: String = format!(
            "[{}{}] {}/{} ({}%) {}",
            "=".repeat(filled),             // Filled portion of the bar
            " ".repeat(bar_width - filled), // Empty portion of the bar
            current,                        // Current progress
            total,                          // Total progress
            percentage,                     // Progress percentage
            message                         // Additional message
        );

        self.update_status(&self.truncate_message(&bar)) // Update the status with the progress bar
    }

    // Function to truncate the message if it exceeds the terminal width
    fn truncate_message(&self, message: &str) -> String {
        if message.len() > self.terminal_width {
            format!("{}...", &message[..self.terminal_width - 3]) // Truncate and add ellipsis
        } else {
            message.to_string() // Return the original message if it fits
        }
    }

    // Function to finish the status display
    pub fn finish(&mut self) -> io::Result<()> {
        writeln!(self.stdout)?; // Print a newline to finish the status display
        self.stdout.flush() // Flush the output to ensure it is displayed
    }

    // Function to log the elapsed time since the start
    pub fn log_elapsed_time(&self) {
        let elapsed = self.start_time.elapsed(); // Calculate the elapsed time
        println!("Elapsed time: {:.2?}", elapsed); // Print the elapsed time
    }
}
