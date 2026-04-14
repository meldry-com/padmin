use js_sys::Math;

const CHARS: &str =
    "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789!@#$%^&*()_+-=[]{}|;:,.<>?";
const ALPHANUM: &str = "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789";

fn random_string(charset: &str, length: usize) -> String {
    let chars: Vec<char> = charset.chars().collect();
    (0..length)
        .map(|_| {
            let idx = (Math::random() * chars.len() as f64).floor() as usize;
            chars[idx]
        })
        .collect()
}

pub fn generate_random_password() -> String {
    random_string(CHARS, 64)
}

pub fn generate_device_id() -> String {
    random_string(ALPHANUM, 16)
}
