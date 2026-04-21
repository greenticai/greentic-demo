pub fn t(_locale: &str, key: &str) -> String {
    key.to_string()
}

pub fn all_keys() -> Vec<String> {
    vec![
        "qa.install.title".to_string(),
        "qa.install.description".to_string(),
        "qa.update.title".to_string(),
        "qa.update.description".to_string(),
        "qa.remove.title".to_string(),
        "qa.remove.description".to_string(),
        "qa.field.query_hint.label".to_string(),
        "qa.field.query_hint.help".to_string(),
        "qa.field.confirm_remove.label".to_string(),
        "qa.field.confirm_remove.help".to_string(),
        "qa.error.remove_confirmation".to_string(),
    ]
}
