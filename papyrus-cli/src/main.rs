pub fn stub_message() -> &'static str {
    "papyrus-cli: not yet implemented"
}

fn main() {
    println!("{}", stub_message());
}

#[cfg(test)]
mod tests {
    use super::stub_message;

    #[test]
    fn stub_message_is_stable() {
        assert_eq!(stub_message(), "papyrus-cli: not yet implemented");
    }
}
