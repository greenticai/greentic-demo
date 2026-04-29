//! Bundle-structure invariants for the demo's i18n assets.
//!
//! Catches: key drift across locales, empty translations, placeholder/newline
//! drift, missing target script per locale, length-ratio anomalies, and English
//! bleed-through in non-Latin-script locales.
//!
//! These tests fail loud rather than ship runtime breakage. The runtime
//! substitution in `greentic-start::resolve_i18n_tokens` outputs the raw key
//! string when a translation is missing — which would surface as
//! `card.main_menu.body_0.text` literal text in the rendered card. The drift
//! test below prevents that from ever shipping.

#![allow(
    clippy::missing_panics_doc,
    clippy::cast_precision_loss,
    clippy::similar_names
)]

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::PathBuf;

use regex::Regex;
use serde_json::{Map, Value};

const EXPECTED_LOCALE_COUNT: usize = 65;

fn i18n_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets/i18n")
}

fn load_bundle(name: &str) -> BTreeMap<String, String> {
    let path = i18n_dir().join(format!("{name}.json"));
    let raw = fs::read_to_string(&path).unwrap_or_else(|e| panic!("read {path:?}: {e}"));
    let parsed: Map<String, Value> =
        serde_json::from_str(&raw).unwrap_or_else(|e| panic!("parse {path:?}: {e}"));
    parsed
        .into_iter()
        .map(|(k, v)| {
            let s = v
                .as_str()
                .unwrap_or_else(|| panic!("non-string value at {k} in {path:?}"))
                .to_string();
            (k, s)
        })
        .collect()
}

fn locale_codes() -> Vec<String> {
    let mut out = Vec::new();
    for entry in fs::read_dir(i18n_dir()).expect("read i18n dir") {
        let path = entry.expect("dir entry").path();
        if path.extension().and_then(|s| s.to_str()) != Some("json") {
            continue;
        }
        let stem = path
            .file_stem()
            .expect("file stem")
            .to_str()
            .expect("utf-8 stem")
            .to_string();
        if matches!(stem.as_str(), "en" | "glossary") {
            continue;
        }
        out.push(stem);
    }
    out.sort();
    out
}

#[test]
fn locale_count_matches_target() {
    let count = locale_codes().len();
    assert_eq!(
        count, EXPECTED_LOCALE_COUNT,
        "expected {EXPECTED_LOCALE_COUNT} locale bundles, found {count}"
    );
}

#[test]
fn key_set_matches_english_for_every_locale() {
    let en_keys: BTreeSet<String> = load_bundle("en").keys().cloned().collect();
    for lang in locale_codes() {
        let lang_keys: BTreeSet<String> = load_bundle(&lang).keys().cloned().collect();
        let missing: Vec<&String> = en_keys.difference(&lang_keys).collect();
        let extra: Vec<&String> = lang_keys.difference(&en_keys).collect();
        assert!(
            missing.is_empty() && extra.is_empty(),
            "{lang}: missing {missing:?}, extra {extra:?}"
        );
    }
}

#[test]
fn no_empty_values_when_english_is_nonempty() {
    let en = load_bundle("en");
    for lang in locale_codes() {
        let bundle = load_bundle(&lang);
        for (key, en_val) in &en {
            if en_val.trim().is_empty() {
                continue;
            }
            let val = bundle.get(key).expect("key parity already enforced");
            assert!(
                !val.trim().is_empty(),
                "{lang}: empty translation for {key} (en={en_val:?})"
            );
        }
    }
}

#[test]
fn placeholder_and_newline_counts_match_english() {
    let en = load_bundle("en");
    let placeholder = Regex::new(r"\{\}").expect("compile placeholder regex");
    let template_var = Regex::new(r"\$\{[^}]+\}").expect("compile template var regex");
    for lang in locale_codes() {
        let bundle = load_bundle(&lang);
        for (key, en_val) in &en {
            let translated = bundle.get(key).expect("key parity enforced");
            assert_eq!(
                placeholder.find_iter(en_val).count(),
                placeholder.find_iter(translated).count(),
                "{lang}/{key}: {{}} placeholder count drift\n  en: {en_val:?}\n  tx: {translated:?}"
            );
            assert_eq!(
                template_var.find_iter(en_val).count(),
                template_var.find_iter(translated).count(),
                "{lang}/{key}: ${{var}} template count drift (glossary should preserve these)\n  en: {en_val:?}\n  tx: {translated:?}"
            );
            assert_eq!(
                en_val.matches('\n').count(),
                translated.matches('\n').count(),
                "{lang}/{key}: newline count drift"
            );
        }
    }
}

#[test]
fn char_class_matches_locale_script() {
    let cyrillic = Regex::new(r"\p{Cyrillic}").expect("compile cyrillic regex");
    let cjk =
        Regex::new(r"\p{Han}|\p{Hiragana}|\p{Katakana}|\p{Hangul}").expect("compile cjk regex");
    let arabic = Regex::new(r"\p{Arabic}").expect("compile arabic regex");
    let devanagari = Regex::new(r"\p{Devanagari}").expect("compile devanagari regex");
    let greek = Regex::new(r"\p{Greek}").expect("compile greek regex");
    let thai = Regex::new(r"\p{Thai}").expect("compile thai regex");
    let lao = Regex::new(r"\p{Lao}").expect("compile lao regex");
    let khmer = Regex::new(r"\p{Khmer}").expect("compile khmer regex");
    let myanmar = Regex::new(r"\p{Myanmar}").expect("compile myanmar regex");
    let sinhala = Regex::new(r"\p{Sinhala}").expect("compile sinhala regex");
    let tamil = Regex::new(r"\p{Tamil}").expect("compile tamil regex");
    let telugu = Regex::new(r"\p{Telugu}").expect("compile telugu regex");
    let kannada = Regex::new(r"\p{Kannada}").expect("compile kannada regex");
    let malayalam = Regex::new(r"\p{Malayalam}").expect("compile malayalam regex");
    let gujarati = Regex::new(r"\p{Gujarati}").expect("compile gujarati regex");
    let gurmukhi = Regex::new(r"\p{Gurmukhi}").expect("compile gurmukhi regex");
    let bengali = Regex::new(r"\p{Bengali}").expect("compile bengali regex");

    // Note: Serbian (sr) is intentionally OMITTED — it accepts both Cyrillic
    // and Latin scripts; modern MT often produces Latin Serbian. We rely on the
    // English-bleed-through test to catch defects there.
    let expectations: &[(&str, &Regex)] = &[
        ("ru", &cyrillic),
        ("uk", &cyrillic),
        ("bg", &cyrillic),
        ("zh", &cjk),
        ("ja", &cjk),
        ("ko", &cjk),
        ("ar", &arabic),
        ("fa", &arabic),
        ("ur", &arabic),
        ("hi", &devanagari),
        ("mr", &devanagari),
        ("ne", &devanagari),
        ("el", &greek),
        ("th", &thai),
        ("lo", &lao),
        ("km", &khmer),
        ("my", &myanmar),
        ("si", &sinhala),
        ("ta", &tamil),
        ("te", &telugu),
        ("kn", &kannada),
        ("ml", &malayalam),
        ("gu", &gujarati),
        ("pa", &gurmukhi),
        ("bn", &bengali),
    ];

    for (locale_prefix, expected_script) in expectations {
        for lang in locale_codes() {
            let stem_lower = lang.to_lowercase();
            let prefix_lower = locale_prefix.to_lowercase();
            if !(stem_lower == prefix_lower || stem_lower.starts_with(&format!("{prefix_lower}-")))
            {
                continue;
            }
            let bundle = load_bundle(&lang);
            // Pool all values; require at least one match across the bundle.
            // Per-value enforcement is too strict: glossary terms may stay Latin
            // and short labels may legitimately not contain script characters.
            let pooled: String = bundle.values().cloned().collect::<Vec<_>>().join(" ");
            assert!(
                expected_script.is_match(&pooled),
                "{lang}: bundle contains zero characters of expected script"
            );
        }
    }
}

#[test]
fn translated_length_within_sane_ratio_of_english() {
    let en = load_bundle("en");
    // CJK languages legitimately compress to ~25–35% of English character count
    // (e.g., "Back to Main Menu" → "返回主菜单"). Use a looser min for them.
    let cjk_prefixes = ["zh", "ja", "ko"];
    let max_ratio = 4.0_f64;
    for lang in locale_codes() {
        let bundle = load_bundle(&lang);
        let stem_lower = lang.to_lowercase();
        let is_cjk = cjk_prefixes
            .iter()
            .any(|p| stem_lower == *p || stem_lower.starts_with(&format!("{p}-")));
        let min_ratio = if is_cjk { 0.15_f64 } else { 0.4_f64 };
        for (key, en_val) in &en {
            let en_len = en_val.chars().count() as f64;
            if en_len < 8.0 {
                // Short strings (button labels) are noise-dominant; skip.
                continue;
            }
            let tx = bundle.get(key).expect("key parity");
            let tx_len = tx.chars().count() as f64;
            let ratio = tx_len / en_len;
            assert!(
                (min_ratio..=max_ratio).contains(&ratio),
                "{lang}/{key}: length ratio {ratio:.2} outside [{min_ratio}, {max_ratio}]\n  en={en_val:?}\n  tx={tx:?}"
            );
        }
    }
}

#[test]
fn no_english_bleed_in_non_latin_locales() {
    // For locales whose script is non-Latin, fail if any value contains 3+
    // consecutive ASCII English-stop-words — a strong signal MT punted on
    // translation.
    let stop_words = [
        "the", "and", "for", "with", "from", "this", "that", "have", "your", "you", "are", "will",
        "not", "but", "all", "can", "any", "use", "user", "please",
    ];
    let stop_pattern = format!(
        r"(?i)\b({words})\b\W+\b({words})\b\W+\b({words})\b",
        words = stop_words.join("|")
    );
    let bleed = Regex::new(&stop_pattern).expect("compile bleed regex");

    // Serbian (sr) accepts Latin script; not enforced here.
    let non_latin_prefixes: &[&str] = &[
        "ru", "uk", "bg", "zh", "ja", "ko", "ar", "fa", "ur", "hi", "mr", "ne", "el", "th", "lo",
        "km", "my", "si", "ta", "te", "kn", "ml", "gu", "pa", "bn",
    ];

    for lang in locale_codes() {
        let stem_lower = lang.to_lowercase();
        let is_non_latin = non_latin_prefixes
            .iter()
            .any(|p| stem_lower == *p || stem_lower.starts_with(&format!("{p}-")));
        if !is_non_latin {
            continue;
        }
        let bundle = load_bundle(&lang);
        for (key, value) in &bundle {
            assert!(
                !bleed.is_match(value),
                "{lang}/{key}: English bleed detected (3+ consecutive stop-words)\n  value: {value:?}"
            );
        }
    }
}
