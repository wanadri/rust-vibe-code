/// Returns a greeting for `name`.
pub fn greet(name: &str) -> String {
    format!("Hello, {name}! Welcome to Rust Vibe Code.")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn greets_by_name() {
        assert_eq!(greet("Wan"), "Hello, Wan! Welcome to Rust Vibe Code.");
    }
}
