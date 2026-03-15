#[cfg(target_arch = "wasm32")]
use std::collections::BTreeMap;

use rand::rngs::SmallRng;
use rand::{Rng, SeedableRng};

#[cfg(target_arch = "wasm32")]
use greentic_interfaces_guest::component_v0_6::node;
#[cfg(target_arch = "wasm32")]
use greentic_types::cbor::canonical;
#[cfg(target_arch = "wasm32")]
use greentic_types::schemas::common::schema_ir::{AdditionalProperties, SchemaIr};
#[cfg(target_arch = "wasm32")]
use greentic_types::schemas::component::v0_6_0::{ComponentInfo, I18nText};

pub mod i18n;
pub mod i18n_bundle;
pub mod qa;

const COMPONENT_NAME: &str = "component-random";
const COMPONENT_ORG: &str = "ai.greentic";
const COMPONENT_VERSION: &str = "0.1.0";

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct RouteOption {
    pub route: String,
    #[serde(default)]
    pub audio_url: Option<String>,
    #[serde(default)]
    pub incident_name: Option<String>,
    #[serde(default)]
    pub incident_summary: Option<String>,
    #[serde(default)]
    pub http_endpoint: Option<String>,
    #[serde(default = "default_weight")]
    pub weight: u32,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, Default)]
pub struct RandomRequest {
    #[serde(default)]
    pub routes: Vec<RouteOption>,
    #[serde(default)]
    pub seed: Option<u64>,
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize, PartialEq, Eq)]
pub struct RandomSelection {
    pub route: String,
    #[serde(default)]
    pub audio_url: Option<String>,
    #[serde(default)]
    pub incident_name: Option<String>,
    #[serde(default)]
    pub incident_summary: Option<String>,
    #[serde(default)]
    pub http_endpoint: Option<String>,
    pub index: usize,
}

fn default_weight() -> u32 {
    1
}

pub fn select_route(request: &RandomRequest) -> Result<RandomSelection, String> {
    if request.routes.is_empty() {
        return Err("routes must not be empty".to_string());
    }

    let total_weight: u64 = request
        .routes
        .iter()
        .map(|route| u64::from(route.weight.max(1)))
        .sum();
    if total_weight == 0 {
        return Err("routes must contain at least one positive weight".to_string());
    }

    let mut rng = request
        .seed
        .map(SmallRng::seed_from_u64)
        .unwrap_or_else(SmallRng::from_entropy);
    let ticket = rng.gen_range(0..total_weight);

    let mut cursor = 0_u64;
    for (index, route) in request.routes.iter().enumerate() {
        cursor += u64::from(route.weight.max(1));
        if ticket < cursor {
            return Ok(RandomSelection {
                route: route.route.clone(),
                audio_url: route.audio_url.clone(),
                incident_name: route.incident_name.clone(),
                incident_summary: route.incident_summary.clone(),
                http_endpoint: route.http_endpoint.clone(),
                index,
            });
        }
    }

    Err("failed to choose route".to_string())
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
            summary: Some("Select a random route for the red button demo".to_string()),
            capabilities: Vec::new(),
            ops: vec![
                node::Op {
                    name: "select-route".to_string(),
                    summary: Some("Choose one configured route".to_string()),
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
        "select-route" => execute_select_route(&value),
        _ => execute_select_route(&value),
    };
    encode_cbor(&output)
}

#[cfg_attr(not(target_arch = "wasm32"), allow(dead_code))]
fn execute_select_route(value: &serde_json::Value) -> serde_json::Value {
    let request: RandomRequest = match serde_json::from_value(value.clone()) {
        Ok(request) => request,
        Err(error) => {
            return serde_json::json!({ "ok": false, "error": format!("invalid input: {error}") });
        }
    };
    match select_route(&request) {
        Ok(selection) => serde_json::json!({
            "ok": true,
            "route": selection.route,
            "audio_url": selection.audio_url,
            "incident_name": selection.incident_name,
            "incident_summary": selection.incident_summary,
            "http_endpoint": selection.http_endpoint,
            "index": selection.index
        }),
        Err(error) => serde_json::json!({ "ok": false, "error": error }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn route(name: &str) -> RouteOption {
        RouteOption {
            route: name.to_string(),
            audio_url: Some(format!("https://example.com/{name}.mp3")),
            incident_name: None,
            incident_summary: None,
            http_endpoint: None,
            weight: 1,
        }
    }

    #[test]
    fn no_routes_errors() {
        let request = RandomRequest::default();
        assert_eq!(
            select_route(&request),
            Err("routes must not be empty".to_string())
        );
    }

    #[test]
    fn one_route_is_always_selected() {
        let request = RandomRequest {
            routes: vec![route("only")],
            seed: None,
        };
        assert_eq!(select_route(&request).expect("selected").route, "only");
    }

    #[test]
    fn multiple_routes_can_be_selected_deterministically() {
        let request = RandomRequest {
            routes: vec![route("a"), route("b"), route("c")],
            seed: Some(7),
        };
        let first = select_route(&request).expect("selected");
        let second = select_route(&request).expect("selected");
        assert_eq!(first, second);
    }

    #[test]
    fn weighted_routes_bias_selection() {
        let request = RandomRequest {
            routes: vec![
                RouteOption {
                    weight: 1,
                    ..route("low")
                },
                RouteOption {
                    weight: 100,
                    ..route("high")
                },
            ],
            seed: Some(2),
        };
        assert_eq!(select_route(&request).expect("selected").route, "high");
    }
}
