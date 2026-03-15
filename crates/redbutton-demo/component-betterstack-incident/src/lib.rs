#[cfg(target_arch = "wasm32")]
use std::collections::BTreeMap;

#[cfg(target_arch = "wasm32")]
use greentic_interfaces_guest::component_v0_6::node;
#[cfg(target_arch = "wasm32")]
use greentic_interfaces_guest::{http_client, secrets_store};
#[cfg(target_arch = "wasm32")]
use greentic_types::cbor::canonical;
#[cfg(target_arch = "wasm32")]
use greentic_types::schemas::common::schema_ir::{AdditionalProperties, SchemaIr};
#[cfg(target_arch = "wasm32")]
use greentic_types::schemas::component::v0_6_0::{ComponentInfo, I18nText};

pub mod i18n;
pub mod i18n_bundle;
pub mod qa;

const COMPONENT_NAME: &str = "component-betterstack-incident";
const COMPONENT_ORG: &str = "ai.greentic";
const COMPONENT_VERSION: &str = "0.1.0";
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
const BETTERSTACK_INCIDENTS_URL: &str = "https://uptime.betterstack.com/api/v3/incidents";
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
const BETTERSTACK_TOKEN_SECRET: &str = "betterstack_token";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
struct IncidentRequest {
    #[serde(default)]
    team_name: Option<String>,
    #[serde(default)]
    requester_email: Option<String>,
    #[serde(default)]
    name: Option<String>,
    #[serde(default)]
    summary: Option<String>,
    #[serde(default)]
    description: Option<String>,
    #[serde(default)]
    call: Option<bool>,
    #[serde(default)]
    sms: Option<bool>,
    #[serde(default)]
    email: Option<bool>,
    #[serde(default)]
    critical_alert: Option<bool>,
    #[serde(default)]
    team_wait: Option<i64>,
    #[serde(default)]
    policy_id: Option<String>,
    #[serde(default)]
    metadata: Option<serde_json::Value>,
    #[serde(default)]
    event: Option<serde_json::Value>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
struct IncidentOutput {
    ok: bool,
    status: u16,
    incident_id: Option<String>,
    response: serde_json::Value,
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
            summary: Some(
                "Create Better Stack incidents from webhook-triggered events".to_string(),
            ),
            capabilities: Vec::new(),
            ops: vec![
                node::Op {
                    name: "create-incident".to_string(),
                    summary: Some(
                        "POST a Better Stack incident using the configured secret token"
                            .to_string(),
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
        let output = run_component_cbor(&operation, envelope.payload_cbor);
        Ok(node::InvocationResult {
            ok: true,
            output_cbor: output,
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
    let mut properties = BTreeMap::new();
    for key in [
        "team_name",
        "requester_email",
        "name",
        "summary",
        "description",
        "policy_id",
    ] {
        properties.insert(
            key.to_string(),
            SchemaIr::String {
                min_len: Some(0),
                max_len: None,
                regex: None,
                format: None,
            },
        );
    }
    for key in ["call", "sms", "email", "critical_alert"] {
        properties.insert(key.to_string(), SchemaIr::Bool);
    }
    properties.insert(
        "team_wait".to_string(),
        SchemaIr::Int {
            min: None,
            max: None,
        },
    );
    properties.insert(
        "metadata".to_string(),
        SchemaIr::Object {
            properties: BTreeMap::new(),
            required: Vec::new(),
            additional: AdditionalProperties::Allow,
        },
    );
    properties.insert(
        "event".to_string(),
        SchemaIr::Object {
            properties: BTreeMap::new(),
            required: Vec::new(),
            additional: AdditionalProperties::Allow,
        },
    );

    SchemaIr::Object {
        properties,
        required: Vec::new(),
        additional: AdditionalProperties::Forbid,
    }
}

#[cfg(target_arch = "wasm32")]
fn output_schema() -> SchemaIr {
    SchemaIr::Object {
        properties: BTreeMap::from([
            ("ok".to_string(), SchemaIr::Bool),
            (
                "status".to_string(),
                SchemaIr::Int {
                    min: None,
                    max: None,
                },
            ),
            (
                "incident_id".to_string(),
                SchemaIr::String {
                    min_len: Some(0),
                    max_len: None,
                    regex: None,
                    format: None,
                },
            ),
            (
                "response".to_string(),
                SchemaIr::Object {
                    properties: BTreeMap::new(),
                    required: Vec::new(),
                    additional: AdditionalProperties::Allow,
                },
            ),
        ]),
        required: vec![
            "ok".to_string(),
            "status".to_string(),
            "response".to_string(),
        ],
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
#[allow(dead_code)]
fn component_info_cbor() -> Vec<u8> {
    encode_cbor(&component_info())
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
#[allow(dead_code)]
fn config_schema_cbor() -> Vec<u8> {
    encode_cbor(&config_schema())
}

#[cfg(target_arch = "wasm32")]
fn run_component_cbor(operation: &str, input: Vec<u8>) -> Vec<u8> {
    let value = parse_payload(&input);
    let output = match operation {
        "qa-spec" => {
            let mode = normalized_mode(&value);
            qa::qa_spec_json(mode)
        }
        "apply-answers" => {
            let mode = normalized_mode(&value);
            qa::apply_answers(mode, &value)
        }
        "i18n-keys" => serde_json::Value::Array(
            qa::i18n_keys()
                .into_iter()
                .map(serde_json::Value::String)
                .collect(),
        ),
        "create-incident" => create_incident_output(&value),
        _ => create_incident_output(&value),
    };

    encode_cbor(&output)
}

#[cfg(target_arch = "wasm32")]
fn create_incident_output(value: &serde_json::Value) -> serde_json::Value {
    match create_incident(value) {
        Ok(output) => serde_json::to_value(output).unwrap_or_else(|_| {
            serde_json::json!({
                "ok": false,
                "status": 500,
                "response": { "error": "failed to serialize Better Stack response" }
            })
        }),
        Err(error) => serde_json::json!({
            "ok": false,
            "status": 500,
            "response": { "error": error }
        }),
    }
}

#[cfg(target_arch = "wasm32")]
fn create_incident(value: &serde_json::Value) -> Result<IncidentOutput, String> {
    let request: IncidentRequest =
        serde_json::from_value(value.clone()).map_err(|err| format!("invalid input: {err}"))?;
    let token = load_betterstack_token()?;
    let body = build_incident_body(&request);
    let body_bytes = serde_json::to_vec(&body).map_err(|err| format!("serialize body: {err}"))?;
    let response = http_client::send(
        &http_client::Request {
            method: "POST".to_string(),
            url: BETTERSTACK_INCIDENTS_URL.to_string(),
            headers: vec![
                ("authorization".to_string(), format!("Bearer {token}")),
                ("content-type".to_string(), "application/json".to_string()),
            ],
            body: Some(body_bytes),
        },
        None,
    )
    .map_err(|err| format!("host http error: {err:?}"))?;

    let response_json = parse_response_json(response.body.as_deref());
    let incident_id = response_json
        .get("data")
        .and_then(|data| data.get("id"))
        .and_then(|id| id.as_str())
        .map(ToOwned::to_owned);

    Ok(IncidentOutput {
        ok: (200..300).contains(&response.status),
        status: response.status,
        incident_id,
        response: response_json,
    })
}

#[cfg(target_arch = "wasm32")]
fn load_betterstack_token() -> Result<String, String> {
    let secret = secrets_store::get(BETTERSTACK_TOKEN_SECRET)
        .map_err(|err| format!("secret lookup failed: {err:?}"))?
        .ok_or_else(|| format!("missing required secret `{BETTERSTACK_TOKEN_SECRET}`"))?;
    String::from_utf8(secret).map_err(|err| format!("secret is not utf-8: {err}"))
}

#[cfg(target_arch = "wasm32")]
#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn build_incident_body(request: &IncidentRequest) -> serde_json::Value {
    let mut body = serde_json::Map::new();
    insert_opt_string(&mut body, "team_name", request.team_name.clone());
    insert_opt_string(
        &mut body,
        "requester_email",
        request.requester_email.clone(),
    );
    insert_opt_string(
        &mut body,
        "name",
        request
            .name
            .clone()
            .or_else(|| Some("redbutton-demo webhook incident".to_string())),
    );
    insert_opt_string(
        &mut body,
        "summary",
        request
            .summary
            .clone()
            .or_else(|| Some("A webhook event triggered redbutton-demo.".to_string())),
    );
    insert_opt_string(
        &mut body,
        "description",
        request.description.clone().or_else(|| {
            request.event.as_ref().map(|event| {
                format!(
                    "A global webhook event triggered this incident.\n\n{}",
                    serde_json::to_string_pretty(event).unwrap_or_else(|_| event.to_string())
                )
            })
        }),
    );
    insert_opt_bool(&mut body, "call", request.call);
    insert_opt_bool(&mut body, "sms", request.sms);
    insert_opt_bool(&mut body, "email", request.email);
    insert_opt_bool(&mut body, "critical_alert", request.critical_alert);
    insert_opt_i64(&mut body, "team_wait", request.team_wait);
    insert_opt_string(&mut body, "policy_id", request.policy_id.clone());
    if let Some(metadata) = request.metadata.clone() {
        body.insert("metadata".to_string(), metadata);
    }
    serde_json::Value::Object(body)
}

#[cfg(not(target_arch = "wasm32"))]
#[allow(dead_code)]
fn build_incident_body(request: &IncidentRequest) -> serde_json::Value {
    let mut body = serde_json::Map::new();
    insert_opt_string(
        &mut body,
        "name",
        request
            .name
            .clone()
            .or_else(|| Some("redbutton-demo webhook incident".to_string())),
    );
    insert_opt_string(
        &mut body,
        "summary",
        request
            .summary
            .clone()
            .or_else(|| Some("A webhook event triggered redbutton-demo.".to_string())),
    );
    serde_json::Value::Object(body)
}

#[cfg(target_arch = "wasm32")]
fn parse_response_json(body: Option<&[u8]>) -> serde_json::Value {
    let Some(bytes) = body else {
        return serde_json::Value::Null;
    };
    serde_json::from_slice(bytes)
        .unwrap_or_else(|_| serde_json::Value::String(String::from_utf8_lossy(bytes).into_owned()))
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn insert_opt_string(
    map: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    value: Option<String>,
) {
    if let Some(value) = value {
        map.insert(key.to_string(), serde_json::Value::String(value));
    }
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn insert_opt_bool(
    map: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    value: Option<bool>,
) {
    if let Some(value) = value {
        map.insert(key.to_string(), serde_json::Value::Bool(value));
    }
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn insert_opt_i64(
    map: &mut serde_json::Map<String, serde_json::Value>,
    key: &str,
    value: Option<i64>,
) {
    if let Some(value) = value {
        map.insert(key.to_string(), serde_json::Value::Number(value.into()));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn describe_payload_is_json() {
        let payload = describe_payload();
        let json: serde_json::Value = serde_json::from_str(&payload).expect("valid json");
        assert_eq!(json["component"]["name"], "component-betterstack-incident");
    }

    #[test]
    fn handle_message_round_trips() {
        let body = handle_message("handle", "demo");
        assert!(body.contains("demo"));
    }

    #[test]
    fn request_body_has_defaults() {
        let request = IncidentRequest::default();
        let value = build_incident_body(&request);
        assert_eq!(value["name"], "redbutton-demo webhook incident");
        assert_eq!(
            value["summary"],
            "A webhook event triggered redbutton-demo."
        );
    }
}
