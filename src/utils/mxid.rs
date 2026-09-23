use std::sync::OnceLock;

use crate::utils::storage;

fn mxid_regex() -> &'static regex_lite::Regex {
    static RE: OnceLock<regex_lite::Regex> = OnceLock::new();
    RE.get_or_init(|| regex_lite::Regex::new(r"^@[^@:]+:[^@:]+$").expect("static mxid regex"))
}

pub fn is_mxid(id: &str) -> bool {
    mxid_regex().is_match(id)
}

pub fn return_mxid(input: &str) -> String {
    if is_mxid(input) {
        return input.to_string();
    }

    let home_server = storage::get_item("home_server").unwrap_or_default();

    let localpart = input.strip_prefix('@').unwrap_or(input);

    format!("@{localpart}:{home_server}")
}
