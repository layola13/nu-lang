// Utility Functions

pub fn to_snake_case(s: &str) -> String {
    let mut result = String::new();
    for (i, ch) in s.chars().enumerate() {
        if ch.is_uppercase() && i > 0 {
            result.push('_');
        }
        result.push(ch.to_lowercase().next().unwrap());
    }
    result
}

pub fn is_public_ident(s: &str) -> bool {
    s.chars().next().map_or(false, |c| c.is_uppercase())
}