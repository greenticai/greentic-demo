//! Render-smoke: every `{{i18n:KEY}}` token in every card resolves to a
//! non-empty value across a sample of locales.
//!
//! Catches the runtime "raw key string" fallback at
//! `greentic-start/src/http_ingress/messaging.rs:422` — when a card requests
//! a token that's missing from the locale bundle, runtime emits the raw key
//! string. This test fails loud before runtime would.

#![allow(clippy::missing_panics_doc, clippy::similar_names)]

use std::collections::BTreeMap;
use std::fs;
use std::path::PathBuf;

use regex::Regex;
use serde_json::{Map, Value};

const SAMPLE_LOCALES: &[&str] = &["en", "fr", "ja", "ar", "hi", "id"];

fn crate_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn load_bundle(name: &str) -> BTreeMap<String, String> {
    let path = crate_root()
        .join("assets/i18n")
        .join(format!("{name}.json"));
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    let parsed: Map<String, Value> =
        serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse {path:?}: {e}"));
    parsed
        .into_iter()
        .map(|(k, v)| {
            (
                k,
                v.as_str().expect("string-valued bundle entry").to_string(),
            )
        })
        .collect()
}

fn extract_tokens(card_json: &Value) -> Vec<String> {
    let token_re = Regex::new(r"\{\{i18n:([^}]+)\}\}").expect("compile token regex");
    let mut out = Vec::new();
    fn walk(node: &Value, out: &mut Vec<String>, re: &Regex) {
        match node {
            Value::String(s) => {
                for cap in re.captures_iter(s) {
                    out.push(cap[1].to_string());
                }
            }
            Value::Array(arr) => arr.iter().for_each(|v| walk(v, out, re)),
            Value::Object(obj) => obj.values().for_each(|v| walk(v, out, re)),
            _ => {}
        }
    }
    walk(card_json, &mut out, &token_re);
    out
}

#[test]
fn every_card_token_resolves_in_sampled_locales() {
    let cards_dir = crate_root().join("assets/cards");
    let mut all_tokens: Vec<String> = Vec::new();

    for entry in fs::read_dir(&cards_dir).expect("read cards dir") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let raw = fs::read_to_string(&path).expect("read card");
        let card: Value = serde_json::from_str(&raw).expect("parse card");
        let tokens = extract_tokens(&card);
        assert!(
            !tokens.is_empty(),
            "{path:?}: card contains zero {{i18n:...}} tokens — was tokenization skipped?"
        );
        all_tokens.extend(tokens);
    }

    for locale in SAMPLE_LOCALES {
        let bundle = load_bundle(locale);
        for token in &all_tokens {
            let resolved = bundle.get(token).unwrap_or_else(|| {
                panic!("{locale}: token `{token}` has no entry in bundle (would render as raw key)")
            });
            assert!(
                !resolved.trim().is_empty(),
                "{locale}: token `{token}` resolves to empty string"
            );
            assert_ne!(
                resolved, token,
                "{locale}: token `{token}` resolves to its own key"
            );
        }
    }
}
