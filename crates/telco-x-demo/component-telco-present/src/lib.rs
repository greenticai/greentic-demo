use greentic_messaging_renderer::{adaptive_card_from_presentation, parse_presentation};
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use telco_x::adapters::AdapterFixtures;
use telco_x::playbooks::{
    default_port_utilisation_threshold_percent, run_bgp_advertisers, run_change_correlation,
    run_free_ports, run_noisy_neighbour, run_port_utilisation, run_prefix_traffic,
    run_scope_health_sweep, run_slo_status, run_top_source_asns, run_vm_rca,
};
use telco_x::presentation::{PresentationModel, PresentationSection, present_run};
use telco_x::resolvers::ResolverCatalog;

#[cfg(target_arch = "wasm32")]
use greentic_interfaces_guest::component_v0_6::{component_i18n, component_qa, node};
#[cfg(target_arch = "wasm32")]
use greentic_types::cbor::canonical;
#[cfg(target_arch = "wasm32")]
use greentic_types::schemas::common::schema_ir::{AdditionalProperties, SchemaIr};

pub mod i18n;
pub mod qa;

#[derive(Debug, Clone, Deserialize, Default)]
struct PresentInput {
    #[serde(default)]
    query: Option<String>,
    #[serde(default)]
    step: Option<String>,
    #[serde(default)]
    metadata: Option<Value>,
    #[serde(default)]
    source_provider: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct PresentOutput {
    scenario: String,
    playbook_id: String,
    summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    text: Option<String>,
    provider_hint: String,
    messages: Value,
    #[serde(rename = "renderedCard")]
    #[serde(skip_serializing_if = "Option::is_none")]
    rendered_card: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    adaptive_card: Option<Value>,
    presentation: Value,
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
        let schema = encode_cbor(&object_schema());
        node::ComponentDescriptor {
            name: "component-telco-present".to_string(),
            version: "0.1.0".to_string(),
            summary: Some("Bridge Telco-X presentation models into adaptive card payloads".to_string()),
            capabilities: Vec::new(),
            ops: vec![
                node::Op {
                    name: "present".to_string(),
                    summary: Some("Render a telco result into an adaptive card payload".to_string()),
                    input: node::IoSchema {
                        schema: node::SchemaSource::InlineCbor(schema.clone()),
                        content_type: "application/cbor".to_string(),
                        schema_version: None,
                    },
                    output: node::IoSchema {
                        schema: node::SchemaSource::InlineCbor(schema.clone()),
                        content_type: "application/cbor".to_string(),
                        schema_version: None,
                    },
                    examples: Vec::new(),
                },
                node::Op {
                    name: "qa-spec".to_string(),
                    summary: Some("Return QA spec for setup/update/remove".to_string()),
                    input: node::IoSchema {
                        schema: node::SchemaSource::InlineCbor(schema.clone()),
                        content_type: "application/cbor".to_string(),
                        schema_version: None,
                    },
                    output: node::IoSchema {
                        schema: node::SchemaSource::InlineCbor(schema.clone()),
                        content_type: "application/cbor".to_string(),
                        schema_version: None,
                    },
                    examples: Vec::new(),
                },
                node::Op {
                    name: "apply-answers".to_string(),
                    summary: Some("Apply QA answers and return config".to_string()),
                    input: node::IoSchema {
                        schema: node::SchemaSource::InlineCbor(schema.clone()),
                        content_type: "application/cbor".to_string(),
                        schema_version: None,
                    },
                    output: node::IoSchema {
                        schema: node::SchemaSource::InlineCbor(schema.clone()),
                        content_type: "application/cbor".to_string(),
                        schema_version: None,
                    },
                    examples: Vec::new(),
                },
                node::Op {
                    name: "i18n-keys".to_string(),
                    summary: Some("Return i18n keys referenced by QA metadata".to_string()),
                    input: node::IoSchema {
                        schema: node::SchemaSource::InlineCbor(schema.clone()),
                        content_type: "application/cbor".to_string(),
                        schema_version: None,
                    },
                    output: node::IoSchema {
                        schema: node::SchemaSource::InlineCbor(schema),
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
        let output = run_component(&operation, &envelope.payload_cbor);
        Ok(node::InvocationResult {
            ok: true,
            output_cbor: encode_cbor(&output),
            output_metadata_cbor: None,
        })
    }
}

#[cfg(target_arch = "wasm32")]
impl component_qa::Guest for Component {
    fn qa_spec(mode: component_qa::QaMode) -> Vec<u8> {
        let normalized = match mode {
            component_qa::QaMode::Default => qa::NormalizedMode::Setup,
            component_qa::QaMode::Setup => qa::NormalizedMode::Setup,
            component_qa::QaMode::Update => qa::NormalizedMode::Update,
            component_qa::QaMode::Remove => qa::NormalizedMode::Remove,
        };
        qa::qa_spec_cbor(normalized)
    }

    fn apply_answers(
        mode: component_qa::QaMode,
        current_config: Vec<u8>,
        answers: Vec<u8>,
    ) -> Vec<u8> {
        let normalized = match mode {
            component_qa::QaMode::Default => qa::NormalizedMode::Setup,
            component_qa::QaMode::Setup => qa::NormalizedMode::Setup,
            component_qa::QaMode::Update => qa::NormalizedMode::Update,
            component_qa::QaMode::Remove => qa::NormalizedMode::Remove,
        };
        let current_config_value: Value =
            canonical::from_cbor(&current_config).unwrap_or_else(|_| json!({}));
        let answers_value: Value = canonical::from_cbor(&answers).unwrap_or_else(|_| json!({}));
        let payload = json!({
            "current_config": current_config_value,
            "answers": answers_value,
        });
        canonical::to_canonical_cbor_allow_floats(&qa::apply_answers(normalized, &payload))
            .unwrap_or_default()
    }
}

#[cfg(target_arch = "wasm32")]
impl component_i18n::Guest for Component {
    fn i18n_keys() -> Vec<String> {
        qa::i18n_keys()
    }
}

#[cfg(target_arch = "wasm32")]
greentic_interfaces_guest::export_component_v060!(
    Component,
    component_qa: Component,
    component_i18n: Component,
);

#[cfg(target_arch = "wasm32")]
fn object_schema() -> SchemaIr {
    SchemaIr::Object {
        properties: Default::default(),
        required: Vec::new(),
        additional: AdditionalProperties::Allow,
    }
}

#[cfg(target_arch = "wasm32")]
fn encode_cbor<T: serde::Serialize>(value: &T) -> Vec<u8> {
    canonical::to_canonical_cbor_allow_floats(value).expect("encode cbor")
}

#[cfg(target_arch = "wasm32")]
fn decode_input(input: &[u8]) -> PresentInput {
    if let Ok(value) = canonical::from_cbor::<Value>(input)
        && let Ok(parsed) = serde_json::from_value::<PresentInput>(value)
    {
        return parsed;
    }
    serde_json::from_slice(input).unwrap_or_default()
}

#[cfg(not(target_arch = "wasm32"))]
fn decode_input(input: &[u8]) -> PresentInput {
    serde_json::from_slice(input).unwrap_or_default()
}

fn run_component(operation: &str, input: &[u8]) -> Value {
    match operation {
        "qa-spec" => {
            let payload = decode_input_value(input);
            let mode = payload
                .get("mode")
                .and_then(Value::as_str)
                .and_then(qa::normalize_mode)
                .unwrap_or(qa::NormalizedMode::Setup);
            qa::qa_spec_json(mode)
        }
        "apply-answers" => {
            let payload = decode_input_value(input);
            let mode = payload
                .get("mode")
                .and_then(Value::as_str)
                .and_then(qa::normalize_mode)
                .unwrap_or(qa::NormalizedMode::Setup);
            qa::apply_answers(mode, &payload)
        }
        "i18n-keys" => Value::Array(qa::i18n_keys().into_iter().map(Value::String).collect()),
        "present" => {
            let input = decode_input(input);
            serde_json::to_value(execute_present(&input)).unwrap_or_else(|err| {
                json!({
                    "scenario": "error",
                    "playbook_id": "tx.playbook.error",
                    "summary": format!("failed to serialize present output: {err}"),
                    "text": format!("failed to serialize present output: {err}"),
                    "provider_hint": input.source_provider.unwrap_or_else(|| "webchat".to_string()),
                    "messages": [
                      {
                        "type": "adaptive_card",
                        "card": fallback_card("Error", "Failed to serialize Telco demo output.")
                      }
                    ],
                    "renderedCard": fallback_card("Error", "Failed to serialize Telco demo output."),
                    "adaptive_card": fallback_card("Error", "Failed to serialize Telco demo output."),
                    "presentation": {}
                })
            })
        }
        _ => json!({
            "scenario": "error",
            "playbook_id": "tx.playbook.error",
            "summary": format!("unsupported operation: {operation}"),
            "text": format!("unsupported operation: {operation}"),
            "provider_hint": "webchat",
            "messages": [
              {
                "type": "adaptive_card",
                "card": fallback_card("Unsupported operation", "Only the present operation is supported.")
              }
            ],
            "renderedCard": fallback_card("Unsupported operation", "Only the present operation is supported."),
            "adaptive_card": fallback_card("Unsupported operation", "Only the present operation is supported."),
            "presentation": {}
        }),
    }
}

#[cfg(target_arch = "wasm32")]
fn decode_input_value(input: &[u8]) -> Value {
    canonical::from_cbor(input).unwrap_or_else(|_| json!({}))
}

#[cfg(not(target_arch = "wasm32"))]
fn decode_input_value(input: &[u8]) -> Value {
    serde_json::from_slice(input).unwrap_or_else(|_| json!({}))
}

fn execute_present(input: &PresentInput) -> PresentOutput {
    let provider_hint = input
        .source_provider
        .clone()
        .unwrap_or_else(|| "webchat".to_string());
    let query = input.query.clone().unwrap_or_default().trim().to_string();
    let step = input.step.clone().unwrap_or_default().trim().to_string();
    let metadata = normalized_metadata_object(input.metadata.as_ref());
    let route = if !step.is_empty() { step } else { query };

    if route.is_empty() || route == "oauth_login_success" {
        let welcome = welcome_card();
        return PresentOutput {
            scenario: "welcome".to_string(),
            playbook_id: "tx.playbook.welcome".to_string(),
            summary: "Welcome to the Telco-X demo.".to_string(),
            text: Some("Welcome to the Telco-X demo.".to_string()),
            provider_hint,
            messages: response_messages_from_card(&welcome, false),
            rendered_card: Some(welcome.clone()),
            adaptive_card: Some(welcome),
            presentation: json!({
                "kind": "welcome",
                "prompts": [
                    "menu:network-traffic-routing",
                    "menu:capacity-port-management",
                    "menu:performance-root-cause",
                    "menu:service-assurance"
                ]
            }),
        };
    }

    if route == "menu:network-traffic-routing" {
        let card = network_menu_card();
        return PresentOutput {
            scenario: "menu-network-traffic-routing".to_string(),
            playbook_id: "tx.menu.network_traffic_routing".to_string(),
            summary: "Network Traffic & Routing".to_string(),
            text: Some("Network Traffic & Routing".to_string()),
            provider_hint,
            messages: response_messages_from_card(&card, false),
            rendered_card: Some(card.clone()),
            adaptive_card: Some(card),
            presentation: json!({
                "kind": "menu",
                "category": "network-traffic-routing"
            }),
        };
    }

    if route == "menu:capacity-port-management" {
        let card = capacity_menu_card();
        return PresentOutput {
            scenario: "menu-capacity-port-management".to_string(),
            playbook_id: "tx.menu.capacity_port_management".to_string(),
            summary: "Capacity & Port Management".to_string(),
            text: Some("Capacity & Port Management".to_string()),
            provider_hint,
            messages: response_messages_from_card(&card, false),
            rendered_card: Some(card.clone()),
            adaptive_card: Some(card),
            presentation: json!({
                "kind": "menu",
                "category": "capacity-port-management"
            }),
        };
    }

    if route == "menu:service-assurance" {
        let card = service_assurance_menu_card();
        return PresentOutput {
            scenario: "menu-service-assurance".to_string(),
            playbook_id: "tx.menu.service_assurance".to_string(),
            summary: "Service Assurance".to_string(),
            text: Some("Service Assurance".to_string()),
            provider_hint,
            messages: response_messages_from_card(&card, false),
            rendered_card: Some(card.clone()),
            adaptive_card: Some(card),
            presentation: json!({
                "kind": "menu",
                "category": "service-assurance"
            }),
        };
    }

    if route == "menu:performance-root-cause" {
        let card = performance_menu_card();
        return PresentOutput {
            scenario: "menu-performance-root-cause".to_string(),
            playbook_id: "tx.menu.performance_root_cause".to_string(),
            summary: "Performance & Root Cause".to_string(),
            text: Some("Performance & Root Cause".to_string()),
            provider_hint,
            messages: response_messages_from_card(&card, false),
            rendered_card: Some(card.clone()),
            adaptive_card: Some(card),
            presentation: json!({
                "kind": "menu",
                "category": "performance-root-cause"
            }),
        };
    }

    if route == "menu:port-utilisation-parameters" {
        let card = port_utilisation_parameters_card();
        return PresentOutput {
            scenario: "menu-port-utilisation-parameters".to_string(),
            playbook_id: "tx.menu.port_utilisation_parameters".to_string(),
            summary: "Overutilised ACI ports".to_string(),
            text: Some("Overutilised ACI ports".to_string()),
            provider_hint,
            messages: response_messages_from_card(&card, false),
            rendered_card: Some(card.clone()),
            adaptive_card: Some(card),
            presentation: json!({
                "kind": "parameter-menu",
                "category": "capacity-port-management",
                "playbook": "port-utilisation"
            }),
        };
    }

    if route == "menu:vm-rca-parameters" {
        let card = vm_rca_parameters_card();
        return PresentOutput {
            scenario: "menu-vm-rca-parameters".to_string(),
            playbook_id: "tx.menu.vm_rca_parameters".to_string(),
            summary: "Run VM RCA".to_string(),
            text: Some("Run VM RCA".to_string()),
            provider_hint,
            messages: response_messages_from_card(&card, false),
            rendered_card: Some(card.clone()),
            adaptive_card: Some(card),
            presentation: json!({
                "kind": "parameter-menu",
                "category": "performance-root-cause",
                "playbook": "vm-rca"
            }),
        };
    }

    if route == "menu:prefix-traffic-parameters" {
        let card = prefix_traffic_parameters_card();
        return PresentOutput {
            scenario: "menu-prefix-traffic-parameters".to_string(),
            playbook_id: "tx.menu.prefix_traffic_parameters".to_string(),
            summary: "Prefix traffic distribution".to_string(),
            text: Some("Prefix traffic distribution".to_string()),
            provider_hint,
            messages: response_messages_from_card(&card, false),
            rendered_card: Some(card.clone()),
            adaptive_card: Some(card),
            presentation: json!({
                "kind": "parameter-menu",
                "category": "network-traffic-routing",
                "playbook": "prefix-traffic"
            }),
        };
    }

    let resolvers = ResolverCatalog::from_fixture().expect("resolver fixture");
    let fixtures = AdapterFixtures::from_fixture().expect("adapter fixture");

    if route == "run:prefix-traffic-form" {
        let prefix = metadata
            .get("prefix")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("10.24.0.0/16");
        let direction = metadata
            .get("direction")
            .and_then(Value::as_str)
            .unwrap_or("Inbound");
        let time_window = metadata
            .get("time_window")
            .and_then(Value::as_str)
            .unwrap_or("Last 24 hours");
        let run = run_prefix_traffic(prefix, &resolvers, &fixtures);
        let presentation = present_run(&run);
        let presentation_json = serde_json::to_value(&presentation).expect("presentation json");
        let adaptive_card = prefix_traffic_analysis_card(
            prefix,
            direction,
            time_window,
            &presentation_json,
            &run.summary,
        );
        return PresentOutput {
            scenario: "prefix-traffic-form".to_string(),
            playbook_id: run.playbook_id,
            summary: run.summary.clone(),
            text: None,
            provider_hint,
            messages: response_messages_from_card(&adaptive_card, true),
            rendered_card: None,
            adaptive_card: None,
            presentation: presentation_json,
        };
    }

    if route == "run:port-utilisation-form" {
        let device = metadata
            .get("device")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("2201");
        let threshold = metadata
            .get("threshold")
            .and_then(Value::as_f64)
            .or_else(|| {
                metadata
                    .get("threshold")
                    .and_then(Value::as_str)
                    .and_then(|value| value.parse::<f64>().ok())
            })
            .unwrap_or(default_port_utilisation_threshold_percent());
        let time_window = metadata
            .get("time_window")
            .and_then(Value::as_str)
            .unwrap_or("Last 24 hours");
        let run = run_port_utilisation(device, &resolvers, &fixtures, threshold);
        let presentation = present_run(&run);
        let presentation_json = serde_json::to_value(&presentation).expect("presentation json");
        let adaptive_card =
            port_utilisation_analysis_card(device, threshold, time_window, &presentation_json);
        return PresentOutput {
            scenario: "port-utilisation-form".to_string(),
            playbook_id: run.playbook_id,
            summary: run.summary.clone(),
            text: None,
            provider_hint,
            messages: response_messages_from_card(&adaptive_card, true),
            rendered_card: None,
            adaptive_card: None,
            presentation: presentation_json,
        };
    }

    if route == "run:vm-rca-form" {
        let service = metadata
            .get("service")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("mobile-data");
        let cluster_value = metadata
            .get("cluster")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("default");
        let symptom = metadata
            .get("symptom")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .unwrap_or("Latency spike");
        let time_window = metadata
            .get("time_window")
            .and_then(Value::as_str)
            .unwrap_or("Last 24 hours");
        let cluster = if cluster_value.eq_ignore_ascii_case("default") {
            None
        } else {
            Some(cluster_value)
        };
        let run = run_vm_rca(service, cluster, &resolvers, &fixtures);
        let presentation = present_run(&run);
        let presentation_json = serde_json::to_value(&presentation).expect("presentation json");
        let adaptive_card = vm_rca_analysis_card(
            service,
            cluster_value,
            symptom,
            time_window,
            &presentation_json,
            &run.summary,
        );
        return PresentOutput {
            scenario: "vm-rca-form".to_string(),
            playbook_id: run.playbook_id,
            summary: run.summary.clone(),
            text: None,
            provider_hint,
            messages: response_messages_from_card(&adaptive_card, true),
            rendered_card: None,
            adaptive_card: None,
            presentation: presentation_json,
        };
    }

    if let Some((scenario, run)) = run_parameterized_action(&route, &resolvers, &fixtures) {
        let presentation = present_run(&run);
        let presentation_json = serde_json::to_value(&presentation).expect("presentation json");
        let adaptive_model = parse_presentation(&presentation_json).expect("adaptive presentation");
        let adaptive_card = adaptive_card_from_presentation(&adaptive_model);

        return PresentOutput {
            scenario: scenario.to_string(),
            playbook_id: run.playbook_id,
            summary: run.summary.clone(),
            text: Some(run.summary.clone()),
            provider_hint,
            messages: response_messages_from_card(&adaptive_card, false),
            rendered_card: Some(adaptive_card.clone()),
            adaptive_card: Some(adaptive_card),
            presentation: presentation_json,
        };
    }

    if is_service_degradation_query(&route) {
        let change = present_run(&run_change_correlation("mobile-data", &resolvers, &fixtures));
        let vm_rca = present_run(&run_vm_rca("mobile-data", None, &resolvers, &fixtures));
        let port = present_run(&run_port_utilisation(
            "2201",
            &resolvers,
            &fixtures,
            default_port_utilisation_threshold_percent(),
        ));
        let presentation_json = composed_triage_presentation(&change, &vm_rca, &port);
        let adaptive_model = parse_presentation(&presentation_json).expect("adaptive presentation");
        let adaptive_card = adaptive_card_from_presentation(&adaptive_model);
        let summary = adaptive_model.summary.clone();
        let playbook_id = adaptive_model.playbook_id.clone();
        return PresentOutput {
            scenario: "service-degradation-triage".to_string(),
            playbook_id,
            summary: summary.clone(),
            text: Some(summary),
            provider_hint,
            messages: response_messages_from_card(&adaptive_card, false),
            rendered_card: Some(adaptive_card.clone()),
            adaptive_card: Some(adaptive_card),
            presentation: presentation_json,
        };
    }

    let (scenario, run) = select_run(&route, &resolvers, &fixtures);
    let presentation = present_run(&run);
    let presentation_json = serde_json::to_value(&presentation).expect("presentation json");
    let adaptive_model = parse_presentation(&presentation_json).expect("adaptive presentation");
    let adaptive_card = adaptive_card_from_presentation(&adaptive_model);

    PresentOutput {
        scenario: scenario.to_string(),
        playbook_id: run.playbook_id,
        summary: run.summary.clone(),
        text: Some(run.summary.clone()),
        provider_hint,
        messages: response_messages_from_card(&adaptive_card, false),
        rendered_card: Some(adaptive_card.clone()),
        adaptive_card: Some(adaptive_card),
        presentation: presentation_json,
    }
}

fn normalized_metadata_object(metadata: Option<&Value>) -> Value {
    match metadata {
        Some(Value::Object(map)) => Value::Object(map.clone()),
        Some(Value::String(raw)) => serde_json::from_str::<Value>(raw)
            .ok()
            .filter(Value::is_object)
            .unwrap_or_else(|| json!({})),
        _ => json!({}),
    }
}

fn response_messages_from_card(card: &Value, split_sections: bool) -> Value {
    if !split_sections {
        return json!([
            {
                "type": "adaptive_card",
                "card": card
            }
        ]);
    }

    let schema = card
        .get("$schema")
        .cloned()
        .unwrap_or_else(|| Value::String("http://adaptivecards.io/schemas/adaptive-card.json".to_string()));
    let version = card
        .get("version")
        .cloned()
        .unwrap_or_else(|| Value::String("1.6".to_string()));
    let body = card
        .get("body")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();

    let mut messages = Vec::new();
    for section in body {
        if !section.is_object() {
            continue;
        }
        messages.push(json!({
            "type": "adaptive_card",
            "card": {
                "$schema": schema,
                "type": "AdaptiveCard",
                "version": version,
                "body": [section]
            }
        }));
    }

    if messages.is_empty() {
        json!([
            {
                "type": "adaptive_card",
                "card": card
            }
        ])
    } else {
        Value::Array(messages)
    }
}

fn welcome_card() -> Value {
    json!({
        "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
        "type": "AdaptiveCard",
        "version": "1.6",
        "body": [
            {
                "type": "Container",
                "style": "emphasis",
                "bleed": true,
                "items": [
                    {
                        "type": "ColumnSet",
                        "columns": [
                            {
                                "type": "Column",
                                "width": "auto",
                                "verticalContentAlignment": "Center",
                                "items": [
                                    {
                                        "type": "TextBlock",
                                        "text": "📡",
                                        "size": "ExtraLarge"
                                    }
                                ]
                            },
                            {
                                "type": "Column",
                                "width": "stretch",
                                "items": [
                                    {
                                        "type": "TextBlock",
                                        "size": "Large",
                                        "weight": "Bolder",
                                        "wrap": true,
                                        "text": "Telco-X Demo"
                                    },
                                    {
                                        "type": "TextBlock",
                                        "wrap": true,
                                        "size": "Small",
                                        "isSubtle": true,
                                        "spacing": "Small",
                                        "text": "Explore the Telco-X workflows through realistic operator questions."
                                    }
                                ]
                            }
                        ]
                    }
                ]
            },
            {
                "type": "TextBlock",
                "weight": "Bolder",
                "spacing": "Large",
                "text": "What would you like to explore?"
            },
            {
                "type": "ActionSet",
                "spacing": "Medium",
                "actions": [
                    {
                        "type": "Action.Submit",
                        "title": "Network Traffic & Routing",
                        "data": {
                            "text": "menu:network-traffic-routing",
                            "step": "menu:network-traffic-routing"
                        }
                    }
                ]
            },
            {
                "type": "ActionSet",
                "spacing": "Small",
                "actions": [
                    {
                        "type": "Action.Submit",
                        "title": "Capacity & Port Management",
                        "data": {
                            "text": "menu:capacity-port-management",
                            "step": "menu:capacity-port-management"
                        }
                    }
                ]
            },
            {
                "type": "ActionSet",
                "spacing": "Small",
                "actions": [
                    {
                        "type": "Action.Submit",
                        "title": "Service Assurance",
                        "data": {
                            "text": "menu:service-assurance",
                            "step": "menu:service-assurance"
                        }
                    }
                ]
            },
            {
                "type": "ActionSet",
                "spacing": "Small",
                "actions": [
                    {
                        "type": "Action.Submit",
                        "title": "Performance & Root Cause",
                        "data": {
                            "text": "menu:performance-root-cause",
                            "step": "menu:performance-root-cause"
                        }
                    }
                ]
            }
        ]
    })
}

fn network_menu_card() -> Value {
    json!({
        "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
        "type": "AdaptiveCard",
        "version": "1.6",
        "body": [
            {
                "type": "TextBlock",
                "size": "Large",
                "weight": "Bolder",
                "text": "Network Traffic & Routing"
            },
            {
                "type": "TextBlock",
                "wrap": true,
                "spacing": "Small",
                "text": "Analyse prefix traffic, BGP health, and top source ASNs."
            },
            {
                "type": "ActionSet",
                "spacing": "Medium",
                "actions": [
                    {
                        "type": "Action.Submit",
                        "title": "📶 Prefix traffic",
                        "data": {
                            "text": "menu:prefix-traffic-parameters",
                            "step": "menu:prefix-traffic-parameters"
                        }
                    }
                ]
            },
            {
                "type": "ActionSet",
                "spacing": "Small",
                "actions": [
                    {
                        "type": "Action.Submit",
                        "title": "🛰️ BGP advertisers",
                        "data": {
                            "text": "show bgp advertisers",
                            "step": "show bgp advertisers"
                        }
                    }
                ]
            },
            {
                "type": "ActionSet",
                "spacing": "Small",
                "actions": [
                    {
                        "type": "Action.Submit",
                        "title": "🌐 Top source ASNs",
                        "data": {
                            "text": "show top source asns",
                            "step": "show top source asns"
                        }
                    }
                ]
            },
            {
                "type": "ActionSet",
                "spacing": "Small",
                "actions": [
                    {
                        "type": "Action.Submit",
                        "title": "← Back to categories",
                        "data": {
                            "text": "",
                            "step": ""
                        }
                    }
                ]
            }
        ]
    })
}

fn capacity_menu_card() -> Value {
    json!({
        "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
        "type": "AdaptiveCard",
        "version": "1.6",
        "body": [
            {
                "type": "TextBlock",
                "size": "Large",
                "weight": "Bolder",
                "text": "Capacity & Port Management"
            },
            {
                "type": "TextBlock",
                "wrap": true,
                "spacing": "Small",
                "text": "Check free and overutilised ACI ports."
            },
            {
                "type": "ActionSet",
                "spacing": "Medium",
                "actions": [
                    {
                        "type": "Action.Submit",
                        "title": "📈 Overutilised ACI ports",
                        "data": {
                            "text": "menu:port-utilisation-parameters",
                            "step": "menu:port-utilisation-parameters"
                        }
                    }
                ]
            },
            {
                "type": "ActionSet",
                "spacing": "Small",
                "actions": [
                    {
                        "type": "Action.Submit",
                        "title": "🟢 Free ACI ports",
                        "data": {
                            "text": "show free aci ports",
                            "step": "show free aci ports"
                        }
                    }
                ]
            },
            {
                "type": "ActionSet",
                "spacing": "Small",
                "actions": [
                    {
                        "type": "Action.Submit",
                        "title": "🫧 Noisy neighbour",
                        "data": {
                            "text": "show noisy neighbour",
                            "step": "show noisy neighbour"
                        }
                    }
                ]
            },
            {
                "type": "ActionSet",
                "spacing": "Small",
                "actions": [
                    {
                        "type": "Action.Submit",
                        "title": "🩺 Scope health sweep",
                        "data": {
                            "text": "run scope health sweep",
                            "step": "run scope health sweep"
                        }
                    }
                ]
            },
            {
                "type": "ActionSet",
                "spacing": "Small",
                "actions": [
                    {
                        "type": "Action.Submit",
                        "title": "← Back to categories",
                        "data": {
                            "text": "",
                            "step": ""
                        }
                    }
                ]
            }
        ]
    })
}

fn service_assurance_menu_card() -> Value {
    json!({
        "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
        "type": "AdaptiveCard",
        "version": "1.6",
        "body": [
            {
                "type": "TextBlock",
                "size": "Large",
                "weight": "Bolder",
                "text": "Service Assurance"
            },
            {
                "type": "TextBlock",
                "wrap": true,
                "spacing": "Small",
                "text": "Evaluate service health and SLO compliance."
            },
            {
                "type": "ActionSet",
                "spacing": "Medium",
                "actions": [
                    {
                        "type": "Action.Submit",
                        "title": "📏 SLO status",
                        "data": {
                            "text": "show slo status",
                            "step": "show slo status"
                        }
                    }
                ]
            },
            {
                "type": "ActionSet",
                "spacing": "Small",
                "actions": [
                    {
                        "type": "Action.Submit",
                        "title": "← Back to categories",
                        "data": {
                            "text": "",
                            "step": ""
                        }
                    }
                ]
            }
        ]
    })
}

fn performance_menu_card() -> Value {
    json!({
        "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
        "type": "AdaptiveCard",
        "version": "1.6",
        "body": [
            {
                "type": "TextBlock",
                "size": "Large",
                "weight": "Bolder",
                "text": "Performance & Root Cause"
            },
            {
                "type": "TextBlock",
                "wrap": true,
                "spacing": "Small",
                "text": "Investigate VM issues, change correlation, and combined degradation triage."
            },
            {
                "type": "ActionSet",
                "spacing": "Medium",
                "actions": [
                    {
                        "type": "Action.Submit",
                        "title": "🧩 Investigate service degradation",
                        "data": {
                            "text": "investigate service degradation",
                            "step": "investigate service degradation"
                        }
                    }
                ]
            },
            {
                "type": "ActionSet",
                "spacing": "Small",
                "actions": [
                    {
                        "type": "Action.Submit",
                        "title": "🔄 Recent change correlation",
                        "data": {
                            "text": "show recent change correlation",
                            "step": "show recent change correlation"
                        }
                    }
                ]
            },
            {
                "type": "ActionSet",
                "spacing": "Small",
                "actions": [
                    {
                        "type": "Action.Submit",
                        "title": "🧠 Run VM RCA",
                        "data": {
                            "text": "menu:vm-rca-parameters",
                            "step": "menu:vm-rca-parameters"
                        }
                    }
                ]
            },
            {
                "type": "ActionSet",
                "spacing": "Small",
                "actions": [
                    {
                        "type": "Action.Submit",
                        "title": "← Back to categories",
                        "data": {
                            "text": "",
                            "step": ""
                        }
                    }
                ]
            }
        ]
    })
}

fn port_utilisation_parameters_card() -> Value {
    json!({
        "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
        "type": "AdaptiveCard",
        "version": "1.6",
        "body": [
            {
                "type": "TextBlock",
                "size": "Large",
                "weight": "Bolder",
                "text": "Overutilised ACI ports"
            },
            {
                "type": "TextBlock",
                "wrap": true,
                "spacing": "Small",
                "text": "Define the scope before starting the capacity analysis."
            },
            {
                "type": "TextBlock",
                "weight": "Bolder",
                "spacing": "Medium",
                "text": "Node / device"
            },
            {
                "type": "Input.ChoiceSet",
                "id": "device",
                "style": "compact",
                "value": "2201",
                "choices": [
                    { "title": "ACI POD1 NODE2201", "value": "2201" },
                    { "title": "ACI POD1 NODE2202", "value": "2202" },
                    { "title": "ACI POD2 NODE3101", "value": "3101" }
                ]
            },
            {
                "type": "TextBlock",
                "weight": "Bolder",
                "spacing": "Medium",
                "text": "Threshold"
            },
            {
                "type": "Input.ChoiceSet",
                "id": "threshold",
                "style": "compact",
                "value": "85",
                "choices": [
                    { "title": "80%", "value": "80" },
                    { "title": "85%", "value": "85" },
                    { "title": "90%", "value": "90" }
                ]
            },
            {
                "type": "TextBlock",
                "weight": "Bolder",
                "spacing": "Medium",
                "text": "Time window"
            },
            {
                "type": "Input.ChoiceSet",
                "id": "time_window",
                "style": "compact",
                "value": "Last 24 hours",
                "choices": [
                    { "title": "Last hour", "value": "Last hour" },
                    { "title": "Last 24 hours", "value": "Last 24 hours" },
                    { "title": "Last 7 days", "value": "Last 7 days" }
                ]
            }
        ],
        "actions": [
            {
                "type": "Action.Submit",
                "title": "Start analysis",
                "data": {
                    "text": "run:port-utilisation-form",
                    "step": "run:port-utilisation-form"
                }
            },
            {
                "type": "Action.Submit",
                "title": "← Back to Capacity & Port Management",
                "data": {
                    "text": "menu:capacity-port-management",
                    "step": "menu:capacity-port-management"
                }
            }
        ]
    })
}

fn port_utilisation_analysis_card(
    device: &str,
    threshold: f64,
    time_window: &str,
    presentation: &Value,
) -> Value {
    let summary_items = presentation["sections"]
        .as_array()
        .and_then(|sections| sections.first())
        .and_then(|section| section["items"].as_array())
        .cloned()
        .unwrap_or_default();
    let hot_ports = summary_items
        .iter()
        .find(|item| item["label"] == "hot_ports")
        .and_then(|item| item["value"].as_u64())
        .unwrap_or(0);
    let busiest_port = summary_items
        .iter()
        .find(|item| item["label"] == "busiest_port")
        .and_then(|item| item["value"].as_str())
        .unwrap_or("eth1/1");
    let peak_utilisation = port_peak_utilisation(threshold, time_window);
    let avg_utilisation = (peak_utilisation * 0.89).round();
    let threshold_label = format!("{threshold:.0}%");
    let window_resolution = match time_window {
        "Last hour" => "1-minute interface counters",
        "Last 7 days" => "30-minute utilisation aggregates",
        _ => "5-minute interface counters",
    };
    let node_label = format!("ACI POD / NODE {device}");
    json!({
        "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
        "type": "AdaptiveCard",
        "version": "1.6",
        "body": [
            {
                "type": "Container",
                "style": "emphasis",
                "items": [
                    { "type": "TextBlock", "text": "Query Understanding", "weight": "Bolder" },
                    {
                        "type": "TextBlock",
                        "spacing": "Small",
                        "wrap": true,
                        "text": "The assistant prepared a port utilisation review for the selected ACI scope and threshold."
                    },
                    {
                        "type": "FactSet",
                        "facts": [
                            { "title": "Node", "value": node_label },
                            { "title": "Threshold", "value": threshold_label },
                            { "title": "Time window", "value": time_window }
                        ]
                    },
                    main_menu_action_set()
                ]
            },
            {
                "type": "Container",
                "style": "emphasis",
                "spacing": "Medium",
                "items": [
                    { "type": "TextBlock", "text": "Data Sources", "weight": "Bolder" },
                    {
                        "type": "FactSet",
                        "facts": [
                            { "title": "Primary", "value": "ACI fabric utilisation telemetry" },
                            { "title": "Signals", "value": "Port counters / line-rate / saturation flags" },
                            { "title": "Dimensions", "value": "Node / Port / Interface" },
                            { "title": "Resolution", "value": window_resolution }
                        ]
                    },
                    main_menu_action_set()
                ]
            },
            {
                "type": "Container",
                "style": "emphasis",
                "spacing": "Medium",
                "items": [
                    { "type": "TextBlock", "text": "Data Retrieval", "weight": "Bolder" },
                    { "type": "TextBlock", "spacing": "Small", "wrap": true, "text": "✓ Collecting interface counters for the selected node" },
                    { "type": "TextBlock", "spacing": "Small", "wrap": true, "text": "✓ Filtering ports above the requested utilisation threshold" },
                    { "type": "TextBlock", "spacing": "Small", "wrap": true, "text": "✓ Ranking busiest interfaces by sustained load" },
                    main_menu_action_set()
                ]
            },
            {
                "type": "Container",
                "style": "emphasis",
                "spacing": "Medium",
                "items": [
                    { "type": "TextBlock", "text": "Findings", "weight": "Bolder" },
                    {
                        "type": "TextBlock",
                        "spacing": "Small",
                        "wrap": true,
                        "text": format!("Port utilisation for {node_label} — threshold {threshold_label} — {time_window}")
                    },
                    {
                        "type": "FactSet",
                        "facts": [
                            { "title": "Hot ports", "value": hot_ports.to_string() },
                            { "title": "Busiest port", "value": busiest_port },
                            { "title": "Peak utilisation", "value": format!("{peak_utilisation:.0}%") },
                            { "title": "Average utilisation", "value": format!("{avg_utilisation:.0}%") }
                        ]
                    },
                    main_menu_action_set()
                ]
            },
            {
                "type": "Container",
                "style": "emphasis",
                "spacing": "Medium",
                "items": [
                    { "type": "TextBlock", "text": "Summary", "weight": "Bolder" },
                    {
                        "type": "TextBlock",
                        "spacing": "Small",
                        "wrap": true,
                        "text": format!("{node_label} shows {hot_ports} overutilised interfaces above {threshold_label}. The busiest interface is {busiest_port} with observed peak utilisation of {peak_utilisation:.0}% during the selected {time_window} window.")
                    },
                    {
                        "type": "FactSet",
                        "facts": [
                            { "title": "Node", "value": node_label },
                            { "title": "Threshold", "value": threshold_label },
                            { "title": "Busiest port", "value": busiest_port },
                            { "title": "Peak observed", "value": format!("{peak_utilisation:.0}%") }
                        ]
                    },
                    main_menu_action_set()
                ]
            }
        ],
        "actions": [
            {
                "type": "Action.Submit",
                "title": "← Back to port parameters",
                "data": {
                    "text": "menu:port-utilisation-parameters",
                    "step": "menu:port-utilisation-parameters",
                    "device": device,
                    "threshold": threshold,
                    "time_window": time_window
                }
            }
        ]
    })
}

fn port_peak_utilisation(threshold: f64, time_window: &str) -> f64 {
    let base = threshold + 7.0;
    match time_window {
        "Last hour" => base + 3.0,
        "Last 7 days" => base + 1.0,
        _ => base + 2.0,
    }
}

fn prefix_traffic_parameters_card() -> Value {
    json!({
        "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
        "type": "AdaptiveCard",
        "version": "1.6",
        "body": [
            {
                "type": "TextBlock",
                "size": "Large",
                "weight": "Bolder",
                "text": "Prefix traffic distribution"
            },
            {
                "type": "TextBlock",
                "wrap": true,
                "spacing": "Small",
                "text": "Define the analysis parameters before starting the traffic investigation."
            },
            {
                "type": "TextBlock",
                "weight": "Bolder",
                "spacing": "Medium",
                "text": "Prefix"
            },
            {
                "type": "Input.Text",
                "id": "prefix",
                "placeholder": "10.24.0.0/16",
                "value": "10.24.0.0/16"
            },
            {
                "type": "TextBlock",
                "weight": "Bolder",
                "spacing": "Medium",
                "text": "Direction"
            },
            {
                "type": "Input.ChoiceSet",
                "id": "direction",
                "style": "compact",
                "value": "Inbound",
                "choices": [
                    { "title": "Inbound", "value": "Inbound" },
                    { "title": "Outbound", "value": "Outbound" },
                    { "title": "Bidirectional", "value": "Bidirectional" }
                ]
            },
            {
                "type": "TextBlock",
                "weight": "Bolder",
                "spacing": "Medium",
                "text": "Time window"
            },
            {
                "type": "Input.ChoiceSet",
                "id": "time_window",
                "style": "compact",
                "value": "Last 24 hours",
                "choices": [
                    { "title": "Last hour", "value": "Last hour" },
                    { "title": "Last 24 hours", "value": "Last 24 hours" },
                    { "title": "Last 7 days", "value": "Last 7 days" }
                ]
            }
        ],
        "actions": [
            {
                "type": "Action.Submit",
                "title": "Start analysis",
                "data": {
                    "text": "run:prefix-traffic-form",
                    "step": "run:prefix-traffic-form"
                }
            },
            {
                "type": "Action.Submit",
                "title": "← Back to Network Traffic & Routing",
                "data": {
                    "text": "menu:network-traffic-routing",
                    "step": "menu:network-traffic-routing"
                }
            }
        ]
    })
}

fn vm_rca_parameters_card() -> Value {
    json!({
        "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
        "type": "AdaptiveCard",
        "version": "1.6",
        "body": [
            {
                "type": "TextBlock",
                "size": "Large",
                "weight": "Bolder",
                "text": "Run VM RCA"
            },
            {
                "type": "TextBlock",
                "wrap": true,
                "spacing": "Small",
                "text": "Define the RCA scope before starting the investigation."
            },
            {
                "type": "TextBlock",
                "weight": "Bolder",
                "spacing": "Medium",
                "text": "Service"
            },
            {
                "type": "Input.ChoiceSet",
                "id": "service",
                "style": "compact",
                "value": "mobile-data",
                "choices": [
                    { "title": "Mobile Data Core", "value": "mobile-data" },
                    { "title": "Internet Gateway", "value": "internet" }
                ]
            },
            {
                "type": "TextBlock",
                "weight": "Bolder",
                "spacing": "Medium",
                "text": "Cluster"
            },
            {
                "type": "Input.ChoiceSet",
                "id": "cluster",
                "style": "compact",
                "value": "default",
                "choices": [
                    { "title": "Default cluster", "value": "default" },
                    { "title": "Core Riyadh", "value": "riyadh-core" },
                    { "title": "Core Dubai", "value": "dubai-core" }
                ]
            },
            {
                "type": "TextBlock",
                "weight": "Bolder",
                "spacing": "Medium",
                "text": "Symptom"
            },
            {
                "type": "Input.ChoiceSet",
                "id": "symptom",
                "style": "compact",
                "value": "Latency spike",
                "choices": [
                    { "title": "Latency spike", "value": "Latency spike" },
                    { "title": "Packet loss", "value": "Packet loss" },
                    { "title": "CPU saturation", "value": "CPU saturation" }
                ]
            },
            {
                "type": "TextBlock",
                "weight": "Bolder",
                "spacing": "Medium",
                "text": "Time window"
            },
            {
                "type": "Input.ChoiceSet",
                "id": "time_window",
                "style": "compact",
                "value": "Last 24 hours",
                "choices": [
                    { "title": "Last hour", "value": "Last hour" },
                    { "title": "Last 24 hours", "value": "Last 24 hours" },
                    { "title": "Last 7 days", "value": "Last 7 days" }
                ]
            },
        ],
        "actions": [
            {
                "type": "Action.Submit",
                "title": "Start analysis",
                "data": {
                    "text": "run:vm-rca-form",
                    "step": "run:vm-rca-form"
                }
            },
            {
                "type": "Action.Submit",
                "title": "← Back to Performance & Root Cause",
                "data": {
                    "text": "menu:performance-root-cause",
                    "step": "menu:performance-root-cause"
                }
            }
        ]
    })
}

fn vm_rca_analysis_card(
    service: &str,
    cluster: &str,
    symptom: &str,
    time_window: &str,
    presentation: &Value,
    summary: &str,
) -> Value {
    let sections = presentation["sections"].as_array().cloned().unwrap_or_default();
    let findings_text: Vec<String> = sections
        .iter()
        .flat_map(|section| {
            let title = section["title"].as_str().unwrap_or("Analysis section");
            let item_values = section["items"]
                .as_array()
                .into_iter()
                .flatten()
                .filter_map(|item| {
                    let label = item["label"].as_str().unwrap_or_default();
                    let value = item["value"].as_str().unwrap_or_default();
                    if value.is_empty() {
                        None
                    } else if label.is_empty() {
                        Some(value.to_string())
                    } else {
                        Some(format!("{label}: {value}"))
                    }
                })
                .collect::<Vec<_>>();
            if item_values.is_empty() {
                vec![format!("{title}: summary collected for the selected scope.")]
            } else {
                vec![format!("{title}: {}", item_values.join(" | "))]
            }
        })
        .take(3)
        .collect();
    let findings_text = if findings_text.is_empty() {
        vec![summary.to_string()]
    } else {
        findings_text
    };
    let service_label = vm_service_label(service);
    let cluster_label = if cluster.eq_ignore_ascii_case("default") {
        "Default cluster".to_string()
    } else {
        vm_cluster_label(cluster).to_string()
    };
    let resolution = match time_window {
        "Last hour" => "1-minute infrastructure counters",
        "Last 7 days" => "30-minute VM and platform aggregates",
        _ => "5-minute VM and host aggregates",
    };
    let suspected_root_cause = match symptom {
        "Packet loss" => "Hypervisor NIC contention after a noisy-neighbour spike",
        "CPU saturation" => "Guest CPU pressure combined with host imbalance",
        _ => "Storage and host contention affecting the selected service path",
    };
    let confidence = match time_window {
        "Last hour" => "Medium",
        "Last 7 days" => "Medium-high",
        _ => "High",
    };
    json!({
        "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
        "type": "AdaptiveCard",
        "version": "1.6",
        "body": [
            {
                "type": "Container",
                "style": "emphasis",
                "items": [
                    { "type": "TextBlock", "text": "Query Understanding", "weight": "Bolder" },
                    {
                        "type": "TextBlock",
                        "spacing": "Small",
                        "wrap": true,
                        "text": "The assistant has translated the selected VM symptom into an RCA scope and prepared the diagnostic run."
                    },
                    {
                        "type": "FactSet",
                        "facts": [
                            { "title": "Service", "value": service_label },
                            { "title": "Cluster", "value": cluster_label },
                            { "title": "Symptom", "value": symptom },
                            { "title": "Time window", "value": time_window }
                        ]
                    },
                    main_menu_action_set()
                ]
            },
            {
                "type": "Container",
                "style": "emphasis",
                "spacing": "Medium",
                "items": [
                    { "type": "TextBlock", "text": "Data Sources", "weight": "Bolder" },
                    {
                        "type": "FactSet",
                        "facts": [
                            { "title": "Primary", "value": "Virtualisation telemetry and incident context" },
                            { "title": "Signals", "value": "VM state / host placement / saturation indicators" },
                            { "title": "Dimensions", "value": "Service / Cluster / VM / Host" },
                            { "title": "Resolution", "value": resolution }
                        ]
                    },
                    main_menu_action_set()
                ]
            },
            {
                "type": "Container",
                "style": "emphasis",
                "spacing": "Medium",
                "items": [
                    { "type": "TextBlock", "text": "Data Retrieval", "weight": "Bolder" },
                    { "type": "TextBlock", "spacing": "Small", "wrap": true, "text": "✓ Collecting VM health and placement signals" },
                    { "type": "TextBlock", "spacing": "Small", "wrap": true, "text": "✓ Correlating symptom window with host and service context" },
                    { "type": "TextBlock", "spacing": "Small", "wrap": true, "text": "✓ Ranking probable root-cause contributors" },
                    main_menu_action_set()
                ]
            },
            {
                "type": "Container",
                "style": "emphasis",
                "spacing": "Medium",
                "items": [
                    { "type": "TextBlock", "text": "Findings", "weight": "Bolder" },
                    {
                        "type": "TextBlock",
                        "spacing": "Small",
                        "wrap": true,
                        "text": format!("VM RCA for {service_label} — {cluster_label} — {time_window}")
                    },
                    {
                        "type": "TextBlock",
                        "spacing": "Small",
                        "wrap": true,
                        "text": findings_text.join("\n\n")
                    },
                    main_menu_action_set()
                ]
            },
            {
                "type": "Container",
                "style": "emphasis",
                "spacing": "Medium",
                "items": [
                    { "type": "TextBlock", "text": "Summary", "weight": "Bolder" },
                    {
                        "type": "TextBlock",
                        "spacing": "Small",
                        "wrap": true,
                        "text": format!("{service_label} in {cluster_label} shows strongest RCA evidence for {suspected_root_cause}. Assessment confidence is {confidence} for the selected {time_window} window.")
                    },
                    {
                        "type": "FactSet",
                        "facts": [
                            { "title": "Top hypothesis", "value": suspected_root_cause },
                            { "title": "Confidence", "value": confidence },
                            { "title": "Symptom", "value": symptom },
                            { "title": "Run summary", "value": summary }
                        ]
                    },
                    main_menu_action_set()
                ]
            }
        ],
        "actions": [
            {
                "type": "Action.Submit",
                "title": "← Back to VM RCA parameters",
                "data": {
                    "text": "menu:vm-rca-parameters",
                    "step": "menu:vm-rca-parameters",
                    "service": service,
                    "cluster": cluster,
                    "symptom": symptom,
                    "time_window": time_window
                }
            }
        ]
    })
}

fn vm_service_label(service: &str) -> &'static str {
    match service {
        "internet" => "Internet Gateway",
        _ => "Mobile Data Core",
    }
}

fn vm_cluster_label(cluster: &str) -> &'static str {
    match cluster {
        "riyadh-core" => "Core Riyadh",
        "dubai-core" => "Core Dubai",
        _ => "Default cluster",
    }
}

fn prefix_traffic_analysis_card(
    prefix: &str,
    direction: &str,
    time_window: &str,
    presentation: &Value,
    _summary: &str,
) -> Value {
    let ranking_rows = presentation["sections"]
        .as_array()
        .and_then(|sections| {
            sections
                .iter()
                .find(|section| section["section_id"] == "ranking")
        })
        .and_then(|section| section["rows"].as_array())
        .cloned()
        .unwrap_or_default();

    let volume_multiplier = prefix_traffic_volume_multiplier(direction, time_window);
    let peak_multiplier = prefix_traffic_peak_multiplier(direction, time_window);

    let computed_rows: Vec<(String, String, String, f64, f64, f64, f64)> = ranking_rows
        .iter()
        .enumerate()
        .map(|(index, row)| {
            let peer = row["peer"].as_str().unwrap_or("-").to_string();
            let router = row["device"]
                .as_str()
                .unwrap_or("-")
                .to_ascii_uppercase();
            let interface = format!("Te0/0/0/{}", index + 1);
            let scaled_bytes =
                (row["bytes"].as_u64().unwrap_or_default() as f64 * volume_multiplier).round();
            let peak_gbps =
                (row["peak_mbps"].as_f64().unwrap_or_default() / 1000.0) * peak_multiplier;
            let avg_gbps = peak_gbps * 0.59;
            let p95_gbps = peak_gbps * 0.82;
            (peer, router, interface, avg_gbps, p95_gbps, peak_gbps, scaled_bytes)
        })
        .collect();

    let total_scaled_bytes = computed_rows.iter().map(|row| row.6).sum::<f64>();
    let computed_rows: Vec<(String, String, String, f64, f64, f64, f64)> = computed_rows
        .into_iter()
        .map(|(peer, router, interface, avg_gbps, p95_gbps, peak_gbps, scaled_bytes)| {
            let share = if total_scaled_bytes == 0.0 {
                0.0
            } else {
                (scaled_bytes / total_scaled_bytes) * 100.0
            };
            (peer, router, interface, avg_gbps, p95_gbps, peak_gbps, share)
        })
        .collect();

    let total_avg_gbps = computed_rows.iter().map(|row| row.3).sum::<f64>();
    let top_three_share = computed_rows.iter().take(3).map(|row| row.6).sum::<f64>();
    let top_contributor = computed_rows
        .first()
        .map(|row| format!("{} ({:.1}%)", row.0, row.6))
        .unwrap_or_else(|| "No dominant peer".to_string());
    let peak_observed = computed_rows
        .iter()
        .map(|row| row.5)
        .fold(0.0_f64, f64::max);
    let peak_time = prefix_traffic_peak_time_label(time_window);

    let table_rows: Vec<Value> = computed_rows
        .iter()
        .map(|(peer, router, interface, avg_gbps, p95_gbps, peak_gbps, share)| {
            json!({
                "type": "FactSet",
                "facts": [
                    { "title": "Peer", "value": peer },
                    { "title": "Router", "value": router },
                    { "title": "Interface", "value": interface },
                    { "title": "Avg Gbps", "value": format!("{avg_gbps:.2}") },
                    { "title": "p95 Gbps", "value": format!("{p95_gbps:.2}") },
                    { "title": "Peak Gbps", "value": format!("{peak_gbps:.2}") },
                    { "title": "% Total", "value": format!("{share:.1}%") }
                ]
            })
        })
        .collect();

    let findings: Vec<Value> = ranking_rows
        .iter()
        .take(5)
        .map(|row| {
            json!({
                "type": "TextBlock",
                "wrap": true,
                "spacing": "Small",
                "text": format!(
                    "{} | {} | bytes {} | peak_mbps {}",
                    row["peer"].as_str().unwrap_or("-"),
                    row["device"].as_str().unwrap_or("-"),
                    row["bytes"].as_u64().unwrap_or_default(),
                    row["peak_mbps"].as_f64().unwrap_or_default()
                )
            })
        })
        .collect();

    let findings_title = match time_window {
        "Last hour" => "Last 1h",
        "Last 24 hours" => "Last 24h",
        "Last 7 days" => "Last 7d",
        other => other,
    };

    let mut findings_items = vec![
        json!({
            "type": "TextBlock",
            "weight": "Bolder",
            "text": "Findings"
        }),
        json!({
            "type": "TextBlock",
            "wrap": true,
            "spacing": "Small",
            "text": format!("Traffic distribution for {prefix} — {direction} — {findings_title}")
        }),
    ];
    findings_items.extend(table_rows);
    if findings.is_empty() {
        findings_items.push(json!({
            "type": "TextBlock",
            "wrap": true,
            "spacing": "Small",
            "text": "No ranked peer data was returned for the selected prefix."
        }));
    } else {
        findings_items.extend(findings);
    }
    findings_items.push(main_menu_action_set());

    json!({
        "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
        "type": "AdaptiveCard",
        "version": "1.6",
        "body": [
            {
                "type": "Container",
                "style": "emphasis",
                "items": [
                    {
                        "type": "TextBlock",
                        "weight": "Bolder",
                        "text": "Query Understanding"
                    },
                    {
                        "type": "TextBlock",
                        "wrap": true,
                        "spacing": "Small",
                        "text": "The assistant has understood the query and prepared a traffic analysis request."
                    },
                    {
                        "type": "FactSet",
                        "facts": [
                            { "title": "Prefix", "value": prefix },
                            { "title": "Direction", "value": direction },
                            { "title": "Time window", "value": time_window }
                        ]
                    },
                    main_menu_action_set()
                ]
            },
            {
                "type": "Container",
                "spacing": "Medium",
                "style": "emphasis",
                "items": [
                    {
                        "type": "TextBlock",
                        "weight": "Bolder",
                        "text": "Data Sources"
                    },
                    {
                        "type": "FactSet",
                        "facts": [
                            { "title": "Primary", "value": "Traffic engineering subsystem" },
                            { "title": "Flow data", "value": "Arbor flow collector" },
                            { "title": "Dimensions", "value": "Peer / Router / Interface" },
                            { "title": "Resolution", "value": prefix_traffic_resolution_label(time_window) }
                        ]
                    },
                    main_menu_action_set()
                ]
            },
            {
                "type": "Container",
                "spacing": "Medium",
                "style": "emphasis",
                "items": [
                    {
                        "type": "TextBlock",
                        "weight": "Bolder",
                        "text": "Data Retrieval"
                    },
                    {
                        "type": "TextBlock",
                        "wrap": true,
                        "spacing": "Small",
                        "text": "✓ Retrieving flow records"
                    },
                    {
                        "type": "TextBlock",
                        "wrap": true,
                        "spacing": "Small",
                        "text": "✓ Grouping by peer and router"
                    },
                    {
                        "type": "TextBlock",
                        "wrap": true,
                        "spacing": "Small",
                        "text": "✓ Calculating avg, peak, and total flow volume"
                    },
                    main_menu_action_set()
                ]
            },
            {
                "type": "Container",
                "spacing": "Medium",
                "style": "emphasis",
                "items": findings_items
            },
            {
                "type": "Container",
                "spacing": "Medium",
                "style": "emphasis",
                "items": [
                    {
                        "type": "TextBlock",
                        "weight": "Bolder",
                        "text": "Summary"
                    },
                    {
                        "type": "TextBlock",
                        "wrap": true,
                        "spacing": "Small",
                        "text": format!(
                            "{} {} traffic is dominated by {}, with the top 3 contributors accounting for {:.1}% of observed throughput. Peak utilisation occurred at {}.",
                            prefix,
                            direction.to_ascii_lowercase(),
                            top_contributor,
                            top_three_share,
                            peak_time
                        )
                    },
                    {
                        "type": "FactSet",
                        "facts": [
                            { "title": "Total throughput", "value": format!("{total_avg_gbps:.2} Gbps avg") },
                            { "title": "Top contributor", "value": top_contributor },
                            { "title": "Peak time", "value": peak_time },
                            { "title": "Peak observed", "value": format!("{peak_observed:.2} Gbps") },
                            { "title": "Top 3 share", "value": format!("{top_three_share:.1}%") }
                        ]
                    },
                    main_menu_action_set()
                ]
            }
        ],
        "actions": [
            {
                "type": "Action.Submit",
                "title": "← Back to Prefix traffic parameters",
                "data": {
                    "text": "menu:prefix-traffic-parameters",
                    "step": "menu:prefix-traffic-parameters",
                    "prefix": prefix,
                    "direction": direction,
                    "time_window": time_window
                }
            }
        ]
    })
}

fn main_menu_action_set() -> Value {
    json!({
        "type": "ActionSet",
        "spacing": "Medium",
        "actions": [
            {
                "type": "Action.Submit",
                "title": "Back to main menu",
                "style": "destructive",
                "data": {
                    "text": "",
                    "step": ""
                }
            }
        ]
    })
}

fn prefix_traffic_volume_multiplier(direction: &str, time_window: &str) -> f64 {
    let direction_factor = match direction {
        "Outbound" => 0.61,
        "Bidirectional" => 1.74,
        _ => 1.0,
    };
    let window_factor = match time_window {
        "Last hour" => 0.18,
        "Last 7 days" => 4.85,
        _ => 1.0,
    };
    direction_factor * window_factor
}

fn prefix_traffic_peak_multiplier(direction: &str, time_window: &str) -> f64 {
    let direction_factor = match direction {
        "Outbound" => 0.76,
        "Bidirectional" => 1.16,
        _ => 1.0,
    };
    let window_factor = match time_window {
        "Last hour" => 1.08,
        "Last 7 days" => 0.93,
        _ => 1.0,
    };
    direction_factor * window_factor
}

fn prefix_traffic_resolution_label(time_window: &str) -> &'static str {
    match time_window {
        "Last hour" => "1-minute intervals",
        "Last 7 days" => "1-hour intervals",
        _ => "5-minute intervals",
    }
}

fn prefix_traffic_peak_time_label(time_window: &str) -> &'static str {
    match time_window {
        "Last hour" => "14:55 UTC",
        "Last 7 days" => "Tuesday 14:35 UTC",
        _ => "14:35 UTC",
    }
}

fn run_parameterized_action(
    query: &str,
    resolvers: &ResolverCatalog,
    fixtures: &AdapterFixtures,
) -> Option<(&'static str, telco_x::playbooks::Phase1PlaybookRun)> {
    if let Some(params) = query.strip_prefix("run:port-utilisation:") {
        let mut device = "2201";
        let mut threshold = default_port_utilisation_threshold_percent();
        for token in params.split(':') {
            if let Some(value) = token.strip_prefix("device=") {
                device = value;
            } else if let Some(value) = token.strip_prefix("threshold=")
                && let Ok(parsed) = value.parse::<f64>()
            {
                threshold = parsed;
            }
        }
        return Some((
            "port-utilisation",
            run_port_utilisation(device, resolvers, fixtures, threshold),
        ));
    }

    if let Some(params) = query.strip_prefix("run:vm-rca:") {
        let mut service = "mobile-data";
        let mut cluster = None;
        for token in params.split(':') {
            if let Some(value) = token.strip_prefix("service=") {
                service = value;
            } else if let Some(value) = token.strip_prefix("cluster=") {
                cluster = Some(value);
            }
        }
        return Some(("vm-rca", run_vm_rca(service, cluster, resolvers, fixtures)));
    }

    None
}

fn select_run(
    query: &str,
    resolvers: &ResolverCatalog,
    fixtures: &AdapterFixtures,
) -> (&'static str, telco_x::playbooks::Phase1PlaybookRun) {
    let normalized = query.to_ascii_lowercase();
    if normalized.contains("prefix") || normalized.contains("traffic") {
        (
            "prefix-traffic",
            run_prefix_traffic("10.24.0.0/16", resolvers, fixtures),
        )
    } else if normalized.contains("bgp") || normalized.contains("advertiser") {
        (
            "bgp-advertisers",
            run_bgp_advertisers("10.24.0.0/16", resolvers, fixtures),
        )
    } else if normalized.contains("asn") {
        (
            "top-source-asns",
            run_top_source_asns(Some("10.24.0.0/16"), resolvers, fixtures),
        )
    } else if normalized.contains("free") {
        ("free-ports", run_free_ports("2201", resolvers, fixtures))
    } else if normalized.contains("slo") || normalized.contains("sla") {
        ("slo-status", run_slo_status("mobile-data", resolvers))
    } else if normalized.contains("scope") || normalized.contains("health sweep") {
        (
            "scope-health-sweep",
            run_scope_health_sweep("riyadh-core", resolvers, fixtures),
        )
    } else if normalized.contains("noisy") {
        (
            "noisy-neighbour",
            run_noisy_neighbour("riyadh-core", resolvers, fixtures),
        )
    } else if normalized.contains("change") {
        (
            "change-correlation",
            run_change_correlation("mobile-data", resolvers, fixtures),
        )
    } else if normalized.contains("vm") || normalized.contains("rca") {
        ("vm-rca", run_vm_rca("mobile-data", None, resolvers, fixtures))
    } else {
        (
            "port-utilisation",
            run_port_utilisation(
                "2201",
                resolvers,
                fixtures,
                default_port_utilisation_threshold_percent(),
            ),
        )
    }
}

fn is_service_degradation_query(query: &str) -> bool {
    let normalized = query.to_ascii_lowercase();
    normalized.contains("degradation")
        || normalized.contains("degraded")
        || normalized.contains("slow")
        || normalized.contains("service issue")
        || normalized.contains("investigate")
        || normalized.contains("triage")
}

fn severity_rank(severity: &str) -> u8 {
    match severity {
        "critical" => 3,
        "warning" => 2,
        _ => 1,
    }
}

fn composed_triage_presentation(
    change: &PresentationModel,
    vm_rca: &PresentationModel,
    port: &PresentationModel,
) -> Value {
    let severity = [change, vm_rca, port]
        .into_iter()
        .max_by_key(|model| severity_rank(&model.severity))
        .map(|model| model.severity.clone())
        .unwrap_or_else(|| "info".to_string());

    let mut sections = Vec::new();
    sections.push(PresentationSection {
        section_id: "triage_summary".to_string(),
        section_type: "facts".to_string(),
        title: "Triage summary".to_string(),
        items: vec![
            json!({"label": "change_analysis", "value": change.summary}),
            json!({"label": "vm_rca", "value": vm_rca.summary}),
            json!({"label": "network_signal", "value": port.summary}),
        ],
        columns: Vec::new(),
        rows: Vec::new(),
    });
    sections.extend(prefix_sections("change", "Recent changes", &change.sections));
    sections.extend(prefix_sections("rca", "RCA", &vm_rca.sections));
    sections.extend(prefix_sections("network", "Network", &port.sections));

    let mut recommended_actions = Vec::new();
    for action in change
        .recommended_actions
        .iter()
        .chain(vm_rca.recommended_actions.iter())
        .chain(port.recommended_actions.iter())
    {
        if !recommended_actions.contains(action) {
            recommended_actions.push(action.clone());
        }
    }

    serde_json::to_value(PresentationModel {
        playbook_id: "tx.playbook.service_degradation_triage".to_string(),
        result: telco_x::playbooks::PlaybookResultKind::Success,
        summary: format!(
            "{} {} {}",
            vm_rca.summary, change.summary, port.summary
        ),
        severity,
        entities: Vec::new(),
        evidence_refs: change
            .evidence_refs
            .iter()
            .chain(vm_rca.evidence_refs.iter())
            .chain(port.evidence_refs.iter())
            .cloned()
            .collect(),
        sections,
        recommended_actions,
    })
    .expect("serialize triage presentation")
}

fn prefix_sections(
    prefix: &str,
    label: &str,
    sections: &[PresentationSection],
) -> Vec<PresentationSection> {
    sections
        .iter()
        .cloned()
        .map(|mut section| {
            section.section_id = format!("{prefix}_{}", section.section_id);
            section.title = format!("{label}: {}", section.title);
            section
        })
        .collect()
}

fn fallback_card(title: &str, text: &str) -> Value {
    json!({
        "$schema": "http://adaptivecards.io/schemas/adaptive-card.json",
        "type": "AdaptiveCard",
        "version": "1.5",
        "body": [
            {
                "type": "TextBlock",
                "size": "Medium",
                "weight": "Bolder",
                "text": title
            },
            {
                "type": "TextBlock",
                "wrap": true,
                "text": text
            }
        ]
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn port_query_defaults_to_port_utilisation() {
        let input = PresentInput {
            query: Some("show overutilised aci ports".to_string()),
            step: None,
            metadata: None,
            source_provider: Some("teams".to_string()),
        };
        let output = execute_present(&input);
        assert_eq!(output.scenario, "port-utilisation");
        assert_eq!(output.playbook_id, "tx.playbook.port_utilisation");
        assert!(output.adaptive_card.as_ref().is_some_and(Value::is_object));
    }

    #[test]
    fn change_query_selects_change_correlation() {
        let input = PresentInput {
            query: Some("show recent change correlation".to_string()),
            step: None,
            metadata: None,
            source_provider: None,
        };
        let output = execute_present(&input);
        assert_eq!(output.scenario, "change-correlation");
        assert_eq!(output.playbook_id, "tx.playbook.change_correlation");
    }

    #[test]
    fn vm_query_selects_vm_rca() {
        let input = PresentInput {
            query: Some("run vm rca".to_string()),
            step: None,
            metadata: None,
            source_provider: None,
        };
        let output = execute_present(&input);
        assert_eq!(output.scenario, "vm-rca");
        assert_eq!(output.playbook_id, "tx.playbook.vm_rca");
    }

    #[test]
    fn empty_query_returns_welcome_card() {
        let input = PresentInput {
            query: Some(String::new()),
            step: None,
            metadata: None,
            source_provider: Some("webchat".to_string()),
        };
        let output = execute_present(&input);
        let card = output.rendered_card.as_ref().expect("welcome card");
        assert_eq!(output.scenario, "welcome");
        assert_eq!(output.playbook_id, "tx.playbook.welcome");
        assert_eq!(output.text.as_deref(), Some("Welcome to the Telco-X demo."));
        assert_eq!(
            card["body"][0]["items"][0]["columns"][1]["items"][0]["text"],
            "Telco-X Demo"
        );
        assert_eq!(
            card["body"][2]["actions"][0]["data"]["step"],
            "menu:network-traffic-routing"
        );
        assert_eq!(
            card["body"][3]["actions"][0]["data"]["step"],
            "menu:capacity-port-management"
        );
        assert_eq!(
            card["body"][4]["actions"][0]["data"]["step"],
            "menu:service-assurance"
        );
        assert_eq!(
            card["body"][5]["actions"][0]["data"]["step"],
            "menu:performance-root-cause"
        );
    }

    #[test]
    fn degradation_query_runs_multi_playbook_triage() {
        let input = PresentInput {
            query: Some("investigate service degradation".to_string()),
            step: None,
            metadata: None,
            source_provider: Some("webchat".to_string()),
        };
        let output = execute_present(&input);
        assert_eq!(output.scenario, "service-degradation-triage");
        assert_eq!(output.playbook_id, "tx.playbook.service_degradation_triage");
        assert!(output.summary.contains("Likely root cause chain"));
        assert_eq!(output.presentation["sections"][0]["section_id"], "triage_summary");
        assert_eq!(
            output.presentation["sections"][1]["title"],
            "Recent changes: Change timeline"
        );
    }

    #[test]
    fn category_query_returns_capacity_menu() {
        let input = PresentInput {
            query: Some(String::new()),
            step: Some("menu:capacity-port-management".to_string()),
            metadata: None,
            source_provider: Some("webchat".to_string()),
        };
        let output = execute_present(&input);
        let card = output.rendered_card.as_ref().expect("capacity menu card");
        assert_eq!(output.scenario, "menu-capacity-port-management");
        assert_eq!(card["body"][0]["text"], "Capacity & Port Management");
        assert_eq!(card["body"][3]["actions"][0]["data"]["step"], "show free aci ports");
    }

    #[test]
    fn network_query_selects_prefix_traffic() {
        let input = PresentInput {
            query: Some("show prefix traffic".to_string()),
            step: None,
            metadata: None,
            source_provider: None,
        };
        let output = execute_present(&input);
        assert_eq!(output.scenario, "prefix-traffic");
        assert_eq!(output.playbook_id, "tx.playbook.prefix_traffic");
    }

    #[test]
    fn service_assurance_query_selects_slo_status() {
        let input = PresentInput {
            query: Some("show slo status".to_string()),
            step: None,
            metadata: None,
            source_provider: None,
        };
        let output = execute_present(&input);
        assert_eq!(output.scenario, "slo-status");
        assert_eq!(output.playbook_id, "tx.playbook.slo_status");
    }

    #[test]
    fn free_ports_query_selects_free_ports() {
        let input = PresentInput {
            query: Some("show free aci ports".to_string()),
            step: None,
            metadata: None,
            source_provider: None,
        };
        let output = execute_present(&input);
        assert_eq!(output.scenario, "free-ports");
        assert_eq!(output.playbook_id, "tx.playbook.free_ports");
    }

    #[test]
    fn overutilised_ports_button_opens_parameter_menu() {
        let input = PresentInput {
            query: Some(String::new()),
            step: Some("menu:port-utilisation-parameters".to_string()),
            metadata: None,
            source_provider: Some("webchat".to_string()),
        };
        let output = execute_present(&input);
        let card = output.rendered_card.as_ref().expect("port parameters card");
        assert_eq!(output.scenario, "menu-port-utilisation-parameters");
        assert_eq!(card["body"][0]["text"], "Overutilised ACI ports");
        assert_eq!(card["body"][3]["id"], "device");
    }

    #[test]
    fn vm_rca_button_opens_parameter_menu() {
        let input = PresentInput {
            query: Some(String::new()),
            step: Some("menu:vm-rca-parameters".to_string()),
            metadata: None,
            source_provider: Some("webchat".to_string()),
        };
        let output = execute_present(&input);
        let card = output.rendered_card.as_ref().expect("vm rca parameters card");
        assert_eq!(output.scenario, "menu-vm-rca-parameters");
        assert_eq!(card["body"][0]["text"], "Run VM RCA");
        assert_eq!(card["body"][3]["id"], "service");
    }

    #[test]
    fn parameterized_port_run_uses_selected_threshold() {
        let input = PresentInput {
            query: Some(String::new()),
            step: Some("run:port-utilisation:device=2201:threshold=90".to_string()),
            metadata: None,
            source_provider: Some("webchat".to_string()),
        };
        let output = execute_present(&input);
        assert_eq!(output.scenario, "port-utilisation");
        assert_eq!(output.playbook_id, "tx.playbook.port_utilisation");
        assert_eq!(
            output.presentation["sections"][0]["items"][1]["value"],
            90.0
        );
    }

    #[test]
    fn port_utilisation_form_uses_selected_metadata() {
        let input = PresentInput {
            query: Some(String::new()),
            step: Some("run:port-utilisation-form".to_string()),
            metadata: Some(json!({
                "device": "3101",
                "threshold": "90",
                "time_window": "Last 7 days"
            })),
            source_provider: Some("webchat".to_string()),
        };
        let output = execute_present(&input);
        assert!(output.rendered_card.is_none());
        assert_eq!(output.scenario, "port-utilisation-form");
        assert_eq!(output.playbook_id, "tx.playbook.port_utilisation");
        assert_eq!(
            output.messages[0]["card"]["body"][0]["items"][2]["facts"][0]["value"],
            "ACI POD / NODE 3101"
        );
        assert_eq!(
            output.messages[0]["card"]["body"][0]["items"][2]["facts"][1]["value"],
            "90%"
        );
        assert_eq!(
            output.messages[0]["card"]["body"][0]["items"][2]["facts"][2]["value"],
            "Last 7 days"
        );
        assert_eq!(
            output.messages[4]["card"]["body"][0]["items"][2]["facts"][3]["value"],
            "98%"
        );
    }

    #[test]
    fn parameterized_vm_rca_run_uses_selected_service() {
        let input = PresentInput {
            query: Some(String::new()),
            step: Some("run:vm-rca:service=internet".to_string()),
            metadata: None,
            source_provider: Some("webchat".to_string()),
        };
        let output = execute_present(&input);
        assert_eq!(output.scenario, "vm-rca");
        assert_eq!(output.playbook_id, "tx.playbook.vm_rca");
        assert!(output.summary.contains("Likely root cause chain"));
    }

    #[test]
    fn vm_rca_form_uses_selected_metadata() {
        let input = PresentInput {
            query: Some(String::new()),
            step: Some("run:vm-rca-form".to_string()),
            metadata: Some(json!({
                "service": "internet",
                "cluster": "riyadh-core",
                "symptom": "Packet loss",
                "time_window": "Last 7 days"
            })),
            source_provider: Some("webchat".to_string()),
        };
        let output = execute_present(&input);
        assert!(output.rendered_card.is_none());
        assert_eq!(output.scenario, "vm-rca-form");
        assert_eq!(output.playbook_id, "tx.playbook.vm_rca");
        assert_eq!(
            output.messages[0]["card"]["body"][0]["items"][2]["facts"][0]["value"],
            "Internet Gateway"
        );
        assert_eq!(
            output.messages[0]["card"]["body"][0]["items"][2]["facts"][1]["value"],
            "Core Riyadh"
        );
        assert_eq!(
            output.messages[0]["card"]["body"][0]["items"][2]["facts"][3]["value"],
            "Last 7 days"
        );
        assert_eq!(
            output.messages[4]["card"]["body"][0]["items"][2]["facts"][2]["value"],
            "Packet loss"
        );
    }

    #[test]
    fn prefix_traffic_button_opens_parameter_menu() {
        let input = PresentInput {
            query: Some(String::new()),
            step: Some("menu:prefix-traffic-parameters".to_string()),
            metadata: None,
            source_provider: Some("webchat".to_string()),
        };
        let output = execute_present(&input);
        let card = output.rendered_card.as_ref().expect("prefix parameters card");
        assert_eq!(output.scenario, "menu-prefix-traffic-parameters");
        assert_eq!(card["body"][0]["text"], "Prefix traffic distribution");
        assert_eq!(card["body"][3]["id"], "prefix");
    }

    #[test]
    fn prefix_traffic_form_uses_selected_metadata() {
        let input = PresentInput {
            query: Some(String::new()),
            step: Some("run:prefix-traffic-form".to_string()),
            metadata: Some(json!({
                "prefix": "10.24.0.0/16",
                "direction": "Inbound",
                "time_window": "Last 24 hours"
            })),
            source_provider: Some("webchat".to_string()),
        };
        let output = execute_present(&input);
        assert!(output.rendered_card.is_none());
        assert_eq!(output.scenario, "prefix-traffic-form");
        assert_eq!(output.playbook_id, "tx.playbook.prefix_traffic");
        assert_eq!(output.messages[0]["card"]["body"][0]["items"][2]["facts"][0]["value"], "10.24.0.0/16");
        assert_eq!(output.messages[3]["card"]["body"][0]["items"][0]["text"], "Findings");
    }

    #[test]
    fn prefix_traffic_form_applies_direction_and_time_window_to_output() {
        let input = PresentInput {
            query: Some(String::new()),
            step: Some("run:prefix-traffic-form".to_string()),
            metadata: Some(json!({
                "prefix": "10.24.0.0/16",
                "direction": "Outbound",
                "time_window": "Last 7 days"
            })),
            source_provider: Some("webchat".to_string()),
        };
        let output = execute_present(&input);
        assert!(output.rendered_card.is_none());
        assert_eq!(
            output.messages[0]["card"]["body"][0]["items"][2]["facts"][2]["value"],
            "Last 7 days"
        );
        assert_eq!(
            output.messages[1]["card"]["body"][0]["items"][1]["facts"][3]["value"],
            "1-hour intervals"
        );
        assert_eq!(
            output.messages[4]["card"]["body"][0]["items"][2]["facts"][2]["value"],
            "Tuesday 14:35 UTC"
        );
    }
}
