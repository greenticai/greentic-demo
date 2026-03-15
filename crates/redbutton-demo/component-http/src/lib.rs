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

const COMPONENT_NAME: &str = "component-http";
const COMPONENT_ORG: &str = "ai.greentic";
const COMPONENT_VERSION: &str = "0.1.0";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct HttpRequestInput {
    pub url: String,
    #[serde(default)]
    pub method: Option<String>,
    #[serde(default)]
    pub headers: serde_json::Map<String, serde_json::Value>,
    #[serde(default)]
    pub body: serde_json::Value,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub ignore_missing_url: bool,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq)]
pub struct HttpResponseOutput {
    pub ok: bool,
    pub status: u16,
    pub url: String,
    pub body: serde_json::Value,
    #[serde(default)]
    pub error: Option<String>,
}

type PreparedRequest = (String, Vec<(String, String)>, Vec<u8>);

pub fn prepare_request(input: &HttpRequestInput) -> Result<PreparedRequest, String> {
    if input.url.trim().is_empty() {
        if input.ignore_missing_url {
            return Ok((String::new(), Vec::new(), Vec::new()));
        }
        return Err("url must not be empty".to_string());
    }
    let parsed = url::Url::parse(&input.url).map_err(|error| format!("invalid url: {error}"))?;
    if let Some(timeout_ms) = input.timeout_ms
        && timeout_ms == 0
    {
        return Err("timeout_ms must be greater than zero".to_string());
    }
    let method = input.method.as_deref().unwrap_or("POST").to_uppercase();
    let mut headers = Vec::with_capacity(input.headers.len() + 1);
    for (key, value) in &input.headers {
        let rendered = value
            .as_str()
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| value.to_string());
        headers.push((key.to_ascii_lowercase(), rendered));
    }
    if !headers.iter().any(|(key, _)| key == "content-type") {
        headers.push(("content-type".to_string(), "application/json".to_string()));
    }
    let body = serde_json::to_vec(&input.body).map_err(|error| format!("invalid body: {error}"))?;
    Ok((format!("{method} {}", parsed), headers, body))
}

pub fn finalize_response(url: &str, status: u16, body: serde_json::Value) -> HttpResponseOutput {
    HttpResponseOutput {
        ok: (200..300).contains(&status),
        status,
        url: url.to_string(),
        body,
        error: if (200..300).contains(&status) {
            None
        } else {
            Some(format!("request failed with status {status}"))
        },
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
            summary: Some("Call an external HTTP endpoint with a JSON payload".to_string()),
            capabilities: Vec::new(),
            ops: vec![
                node::Op {
                    name: "post-json".to_string(),
                    summary: Some("Call an HTTP endpoint".to_string()),
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
        "post-json" => execute_http(&value),
        _ => execute_http(&value),
    };
    encode_cbor(&output)
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn execute_http(value: &serde_json::Value) -> serde_json::Value {
    let input: HttpRequestInput = match serde_json::from_value(value.clone()) {
        Ok(input) => input,
        Err(error) => {
            return serde_json::json!({ "ok": false, "status": 500, "error": format!("invalid input: {error}") });
        }
    };

    #[cfg(not(target_arch = "wasm32"))]
    {
        match prepare_request(&input) {
            Ok((url, _, _)) if url.is_empty() => serde_json::json!({
                "ok": true,
                "status": 204,
                "url": "",
                "body": null
            }),
            Ok((url, _, _)) => serde_json::to_value(finalize_response(&url, 200, serde_json::json!({"mocked": true})))
                .unwrap_or_else(|_| serde_json::json!({ "ok": false, "status": 500, "error": "serialization failed" })),
            Err(error) => serde_json::json!({ "ok": false, "status": 500, "error": error }),
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        let (request_line, headers, body) = match prepare_request(&input) {
            Ok(prepared) => prepared,
            Err(error) => {
                return serde_json::json!({ "ok": false, "status": 500, "error": error });
            }
        };
        if request_line.is_empty() {
            return serde_json::json!({ "ok": true, "status": 204, "url": "", "body": null });
        }
        let (method, url) = request_line
            .split_once(' ')
            .expect("request line should contain method and url");
        match http_client::send(
            &http_client::Request {
                method: method.to_string(),
                url: url.to_string(),
                headers,
                body: Some(body),
            },
            None,
        ) {
            Ok(response) => {
                let response_body = parse_json_or_string(response.body.as_deref());
                serde_json::to_value(finalize_response(url, response.status, response_body))
                    .unwrap_or_else(|_| serde_json::json!({ "ok": false, "status": 500, "error": "serialization failed" }))
            }
            Err(error) => serde_json::json!({
                "ok": false,
                "status": 500,
                "url": url,
                "body": null,
                "error": format!("connection failure: {error:?}")
            }),
        }
    }
}

#[cfg(target_arch = "wasm32")]
fn parse_json_or_string(body: Option<&[u8]>) -> serde_json::Value {
    let Some(body) = body else {
        return serde_json::Value::Null;
    };
    serde_json::from_slice(body)
        .unwrap_or_else(|_| serde_json::Value::String(String::from_utf8_lossy(body).into_owned()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_url_is_rejected() {
        let input = HttpRequestInput {
            url: "nope".to_string(),
            ..HttpRequestInput::default()
        };
        assert!(
            prepare_request(&input)
                .expect_err("invalid")
                .contains("invalid url")
        );
    }

    #[test]
    fn zero_timeout_is_rejected() {
        let input = HttpRequestInput {
            url: "https://example.com".to_string(),
            timeout_ms: Some(0),
            ..HttpRequestInput::default()
        };
        assert_eq!(
            prepare_request(&input).expect_err("timeout"),
            "timeout_ms must be greater than zero"
        );
    }

    #[test]
    fn success_response_is_marked_ok() {
        let output = finalize_response("https://example.com", 201, serde_json::json!({"ok": true}));
        assert!(output.ok);
        assert_eq!(output.status, 201);
    }

    #[test]
    fn non_2xx_response_is_reported() {
        let output = finalize_response(
            "https://example.com",
            503,
            serde_json::json!({"error": "down"}),
        );
        assert!(!output.ok);
        assert_eq!(
            output.error.as_deref(),
            Some("request failed with status 503")
        );
    }

    #[test]
    fn missing_url_can_be_ignored() {
        let input = HttpRequestInput {
            ignore_missing_url: true,
            ..HttpRequestInput::default()
        };
        assert_eq!(prepare_request(&input).expect("prepared").0, "");
    }
}
