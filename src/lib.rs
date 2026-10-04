pub fn create_greeting(name: Option<&str>) -> String {
    match name {
        Some(n) => format!("hello world from Rust CLI, {}!", n),
        None => String::from("hello world from Rust CLI!"),
    }
}

pub fn process_input(input: &str) -> String {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        create_greeting(None)
    } else {
        format!("Processed: {}", trimmed)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_greeting_without_name() {
        assert_eq!(create_greeting(None), "hello world from Rust CLI!");
    }

    #[test]
    fn test_greeting_with_name() {
        assert_eq!(
            create_greeting(Some("Alice")),
            "hello world from Rust CLI, Alice!"
        );
    }

    #[test]
    fn test_process_empty_input() {
        assert_eq!(process_input(""), "hello world from Rust CLI!");
    }

    #[test]
    fn test_process_whitespace_input() {
        assert_eq!(process_input("   "), "hello world from Rust CLI!");
    }

    #[test]
    fn test_process_normal_input() {
        assert_eq!(process_input("Hello there"), "Processed: Hello there");
    }

    #[test]
    fn test_process_input_with_spaces() {
        assert_eq!(process_input("  Hello there  "), "Processed: Hello there");
    }
}
