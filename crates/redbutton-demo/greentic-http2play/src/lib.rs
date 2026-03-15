#[cfg(target_arch = "wasm32")]
use std::collections::BTreeMap;

#[cfg(target_arch = "wasm32")]
use greentic_interfaces_guest::component_v0_6::node;
#[cfg(target_arch = "wasm32")]
use greentic_interfaces_guest::http_client;
#[cfg(target_arch = "wasm32")]
use greentic_types::cbor::canonical;
#[cfg(target_arch = "wasm32")]
use greentic_types::schemas::common::schema_ir::{AdditionalProperties, SchemaIr};
#[cfg(target_arch = "wasm32")]
use greentic_types::schemas::component::v0_6_0::{ComponentInfo, I18nText};

pub mod i18n;
pub mod i18n_bundle;
pub mod qa;

const COMPONENT_NAME: &str = "greentic-http2play";
const COMPONENT_ORG: &str = "ai.greentic";
const COMPONENT_VERSION: &str = "0.1.0";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct PlaybackRequest {
    pub audio_url: String,
    #[serde(default)]
    pub route: Option<String>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct PlaybackPlan {
    pub ok: bool,
    pub audio_url: String,
    pub player_hint: String,
    pub downloaded_bytes: usize,
    pub played: bool,
    pub message: String,
}

pub fn validate_audio_url(audio_url: &str) -> Result<url::Url, String> {
    if audio_url.trim().is_empty() {
        return Err("audio_url must not be empty".to_string());
    }
    url::Url::parse(audio_url).map_err(|error| format!("invalid audio_url: {error}"))
}

pub fn player_hint_for_platform(platform: &str) -> &'static str {
    match platform {
        "macos" => "afplay",
        "linux" => "ffplay -autoexit",
        "windows" => "powershell -c (New-Object Media.SoundPlayer $args[0]).PlaySync()",
        _ => "open",
    }
}

#[cfg(target_arch = "wasm32")]
#[used]
#[unsafe(link_section = ".greentic.wasi")]
static WASI_TARGET_MARKER: [u8; 13] = *b"wasm32-wasip2";

#[cfg(target_arch = "wasm32")]
struct Component;

#[cfg(target_arch = "wasm32")]
impl node::Guest for Component {
    fn describe() -> node::ComponentDescriptor {
        let input_schema_cbor = input_schema_cbor();
        let output_schema_cbor = output_schema_cbor();
        node::ComponentDescriptor {
            name: COMPONENT_NAME.to_string(),
            version: COMPONENT_VERSION.to_string(),
            summary: Some("Prepare and fetch remote audio for local playback".to_string()),
            capabilities: Vec::new(),
            ops: vec![
                node::Op {
                    name: "play-url".to_string(),
                    summary: Some("Fetch audio and return playback guidance".to_string()),
                    input: node::IoSchema {
                        schema: node::SchemaSource::InlineCbor(input_schema_cbor.clone()),
                        content_type: "application/cbor".to_string(),
                        schema_version: None,
                    },
                    output: node::IoSchema {
                        schema: node::SchemaSource::InlineCbor(output_schema_cbor.clone()),
                        content_type: "application/cbor".to_string(),
                        schema_version: None,
                    },
                    examples: Vec::new(),
                },
                node::Op {
                    name: "qa-spec".to_string(),
                    summary: Some("Return QA spec (CBOR) for a requested mode".to_string()),
                    input: node::IoSchema {
                        schema: node::SchemaSource::InlineCbor(input_schema_cbor.clone()),
                        content_type: "application/cbor".to_string(),
                        schema_version: None,
                    },
                    output: node::IoSchema {
                        schema: node::SchemaSource::InlineCbor(output_schema_cbor.clone()),
                        content_type: "application/cbor".to_string(),
                        schema_version: None,
                    },
                    examples: Vec::new(),
                },
                node::Op {
                    name: "apply-answers".to_string(),
                    summary: Some(
                        "Apply QA answers and optionally return config override".to_string(),
                    ),
                    input: node::IoSchema {
                        schema: node::SchemaSource::InlineCbor(input_schema_cbor.clone()),
                        content_type: "application/cbor".to_string(),
                        schema_version: None,
                    },
                    output: node::IoSchema {
                        schema: node::SchemaSource::InlineCbor(output_schema_cbor.clone()),
                        content_type: "application/cbor".to_string(),
                        schema_version: None,
                    },
                    examples: Vec::new(),
                },
                node::Op {
                    name: "i18n-keys".to_string(),
                    summary: Some("Return i18n keys referenced by QA/setup".to_string()),
                    input: node::IoSchema {
                        schema: node::SchemaSource::InlineCbor(input_schema_cbor),
                        content_type: "application/cbor".to_string(),
                        schema_version: None,
                    },
                    output: node::IoSchema {
                        schema: node::SchemaSource::InlineCbor(output_schema_cbor),
                        content_type: "application/cbor".to_string(),
                        schema_version: None,
                    },
                    examples: Vec::new(),
                },
            ],
            schemas: Vec::new(),
            setup: None,
        }
    }

    fn invoke(
        operation: String,
        envelope: node::InvocationEnvelope,
    ) -> Result<node::InvocationResult, node::NodeError> {
        Ok(node::InvocationResult {
            ok: true,
            output_cbor: run_component_cbor(&operation, envelope.payload_cbor),
            output_metadata_cbor: None,
        })
    }
}

#[cfg(target_arch = "wasm32")]
greentic_interfaces_guest::export_component_v060!(Component);

pub fn describe_payload() -> String {
    serde_json::json!({
        "component": {
            "name": COMPONENT_NAME,
            "org": COMPONENT_ORG,
            "version": COMPONENT_VERSION,
            "world": "greentic:component/component@0.6.0",
            "self_describing": true
        }
    })
    .to_string()
}

pub fn handle_message(operation: &str, input: &str) -> String {
    format!("{COMPONENT_NAME}::{operation} => {}", input.trim())
}

#[cfg(target_arch = "wasm32")]
fn encode_cbor<T: serde::Serialize>(value: &T) -> Vec<u8> {
    canonical::to_canonical_cbor_allow_floats(value).expect("encode cbor")
}

#[cfg(target_arch = "wasm32")]
fn parse_payload(input: &[u8]) -> serde_json::Value {
    if let Ok(value) = canonical::from_cbor(input) {
        return value;
    }
    serde_json::from_slice(input).unwrap_or_else(|_| serde_json::json!({}))
}

#[cfg(target_arch = "wasm32")]
fn normalized_mode(payload: &serde_json::Value) -> qa::NormalizedMode {
    let mode = payload
        .get("mode")
        .and_then(|v| v.as_str())
        .or_else(|| payload.get("operation").and_then(|v| v.as_str()))
        .unwrap_or("setup");
    qa::normalize_mode(mode).unwrap_or(qa::NormalizedMode::Setup)
}

#[cfg(target_arch = "wasm32")]
fn input_schema() -> SchemaIr {
    SchemaIr::Object {
        properties: BTreeMap::new(),
        required: Vec::new(),
        additional: AdditionalProperties::Allow,
    }
}

#[cfg(target_arch = "wasm32")]
fn output_schema() -> SchemaIr {
    SchemaIr::Object {
        properties: BTreeMap::new(),
        required: Vec::new(),
        additional: AdditionalProperties::Allow,
    }
}

#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
fn config_schema() -> SchemaIr {
    SchemaIr::Object {
        properties: BTreeMap::new(),
        required: Vec::new(),
        additional: AdditionalProperties::Forbid,
    }
}

#[cfg(target_arch = "wasm32")]
#[allow(dead_code)]
fn component_info() -> ComponentInfo {
    ComponentInfo {
        id: format!("{COMPONENT_ORG}.{COMPONENT_NAME}"),
        version: COMPONENT_VERSION.to_string(),
        role: "tool".to_string(),
        display_name: Some(I18nText::new(
            "component.display_name",
            Some(COMPONENT_NAME.to_string()),
        )),
    }
}

#[cfg(target_arch = "wasm32")]
fn input_schema_cbor() -> Vec<u8> {
    encode_cbor(&input_schema())
}

#[cfg(target_arch = "wasm32")]
fn output_schema_cbor() -> Vec<u8> {
    encode_cbor(&output_schema())
}

#[cfg(target_arch = "wasm32")]
fn run_component_cbor(operation: &str, input: Vec<u8>) -> Vec<u8> {
    let value = parse_payload(&input);
    let output = match operation {
        "qa-spec" => qa::qa_spec_json(normalized_mode(&value)),
        "apply-answers" => qa::apply_answers(normalized_mode(&value), &value),
        "i18n-keys" => serde_json::Value::Array(
            qa::i18n_keys()
                .into_iter()
                .map(serde_json::Value::String)
                .collect(),
        ),
        "play-url" => execute_playback(&value),
        _ => execute_playback(&value),
    };
    encode_cbor(&output)
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn execute_playback(value: &serde_json::Value) -> serde_json::Value {
    let input: PlaybackRequest = match serde_json::from_value(value.clone()) {
        Ok(input) => input,
        Err(error) => {
            return serde_json::json!({ "ok": false, "message": format!("invalid input: {error}") });
        }
    };
    let parsed = match validate_audio_url(&input.audio_url) {
        Ok(parsed) => parsed,
        Err(error) => return serde_json::json!({ "ok": false, "message": error }),
    };

    #[cfg(not(target_arch = "wasm32"))]
    {
        serde_json::to_value(PlaybackPlan {
            ok: true,
            audio_url: parsed.to_string(),
            player_hint: player_hint_for_platform(std::env::consts::OS).to_string(),
            downloaded_bytes: 0,
            played: false,
            message: "Host tests only: playback is planned, not executed.".to_string(),
        })
        .unwrap_or_else(|_| serde_json::json!({ "ok": false, "message": "serialization failed" }))
    }

    #[cfg(target_arch = "wasm32")]
    {
        match http_client::send(
            &http_client::Request {
                method: "GET".to_string(),
                url: parsed.to_string(),
                headers: Vec::new(),
                body: None,
            },
            None,
        ) {
            Ok(response) => serde_json::to_value(PlaybackPlan {
                ok: (200..300).contains(&response.status),
                audio_url: parsed.to_string(),
                player_hint: player_hint_for_platform("macos").to_string(),
                downloaded_bytes: response.body.as_ref().map_or(0, Vec::len),
                played: false,
                message: "Audio was fetched. Direct speaker playback is not available from this WASI component; use the player hint or a host bridge.".to_string(),
            })
            .unwrap_or_else(|_| serde_json::json!({ "ok": false, "message": "serialization failed" })),
            Err(error) => serde_json::json!({
                "ok": false,
                "audio_url": parsed.to_string(),
                "played": false,
                "message": format!("failed to fetch audio: {error:?}")
            }),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn missing_audio_url_is_rejected() {
        assert_eq!(
            validate_audio_url("").expect_err("invalid"),
            "audio_url must not be empty"
        );
    }

    #[test]
    fn invalid_audio_url_is_rejected() {
        assert!(
            validate_audio_url("not-a-url")
                .expect_err("invalid")
                .contains("invalid audio_url")
        );
    }

    #[test]
    fn player_hint_varies_by_platform() {
        assert_eq!(player_hint_for_platform("macos"), "afplay");
        assert_eq!(player_hint_for_platform("linux"), "ffplay -autoexit");
    }
}
