use std::collections::BTreeMap;

use greentic_types::cbor::canonical;
use greentic_types::i18n_text::I18nText;
use greentic_types::schemas::component::v0_6_0::{ComponentQaSpec, QaMode, Question, QuestionKind};
use serde_json::{Value as JsonValue, json};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NormalizedMode {
    Setup,
    Update,
    Remove,
}

impl NormalizedMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Setup => "setup",
            Self::Update => "update",
            Self::Remove => "remove",
        }
    }
}

pub fn normalize_mode(raw: &str) -> Option<NormalizedMode> {
    match raw {
        "default" | "setup" | "install" => Some(NormalizedMode::Setup),
        "update" | "upgrade" => Some(NormalizedMode::Update),
        "remove" => Some(NormalizedMode::Remove),
        _ => None,
    }
}

pub fn qa_spec_cbor(mode: NormalizedMode) -> Vec<u8> {
    canonical::to_canonical_cbor_allow_floats(&qa_spec(mode)).unwrap_or_default()
}

pub fn qa_spec_json(mode: NormalizedMode) -> JsonValue {
    serde_json::to_value(qa_spec(mode)).unwrap_or_else(|_| json!({}))
}

pub fn qa_spec(mode: NormalizedMode) -> ComponentQaSpec {
    let (title_key, description_key, questions) = match mode {
        NormalizedMode::Setup => (
            "qa.install.title",
            Some("qa.install.description"),
            vec![question(
                "query_hint",
                "qa.field.query_hint.label",
                "qa.field.query_hint.help",
                false,
            )],
        ),
        NormalizedMode::Update => (
            "qa.update.title",
            Some("qa.update.description"),
            vec![question(
                "query_hint",
                "qa.field.query_hint.label",
                "qa.field.query_hint.help",
                false,
            )],
        ),
        NormalizedMode::Remove => (
            "qa.remove.title",
            Some("qa.remove.description"),
            vec![question(
                "confirm_remove",
                "qa.field.confirm_remove.label",
                "qa.field.confirm_remove.help",
                true,
            )],
        ),
    };

    ComponentQaSpec {
        mode: match mode {
            NormalizedMode::Setup => QaMode::Setup,
            NormalizedMode::Update => QaMode::Update,
            NormalizedMode::Remove => QaMode::Remove,
        },
        title: I18nText::new(title_key, None),
        description: description_key.map(|key| I18nText::new(key, None)),
        questions,
        defaults: BTreeMap::new(),
    }
}

fn question(id: &str, label_key: &str, help_key: &str, required: bool) -> Question {
    Question {
        id: id.to_string(),
        label: I18nText::new(label_key, None),
        help: Some(I18nText::new(help_key, None)),
        error: None,
        kind: QuestionKind::Text,
        required,
        default: None,
        skip_if: None,
    }
}

pub fn i18n_keys() -> Vec<String> {
    crate::i18n::all_keys()
}

pub fn apply_answers(mode: NormalizedMode, payload: &JsonValue) -> JsonValue {
    let answers = payload.get("answers").cloned().unwrap_or_else(|| json!({}));
    let current_config = payload
        .get("current_config")
        .cloned()
        .unwrap_or_else(|| json!({}));

    let mut errors = Vec::new();
    if mode == NormalizedMode::Remove
        && answers
            .get("confirm_remove")
            .and_then(|v| v.as_str())
            .map(|v| v != "true")
            .unwrap_or(true)
    {
        errors.push(json!({
            "key": "qa.error.remove_confirmation",
            "msg_key": "qa.error.remove_confirmation",
            "fields": ["confirm_remove"]
        }));
    }

    if !errors.is_empty() {
        return json!({
            "ok": false,
            "warnings": [],
            "errors": errors,
            "meta": { "mode": mode.as_str(), "version": "v1" }
        });
    }

    let mut config = match current_config {
        JsonValue::Object(map) => map,
        _ => serde_json::Map::new(),
    };
    if let JsonValue::Object(map) = answers {
        for (key, value) in map {
            config.insert(key, value);
        }
    }

    json!({
        "ok": true,
        "config": config,
        "warnings": [],
        "errors": [],
        "meta": { "mode": mode.as_str(), "version": "v1" }
    })
}
