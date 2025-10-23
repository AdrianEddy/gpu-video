
pub fn select_custom_option<'a>(options: &'a std::collections::HashMap<String, String>, keys: &[&str]) -> Option<&'a str> {
    keys.iter().find_map(|key| options.get(*key).map(|value| value.as_str()))
}
