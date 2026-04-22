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
    source_provider: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct PresentOutput {
    scenario: String,
    playbook_id: String,
    summary: String,
    text: String,
    provider_hint: String,
    #[serde(rename = "renderedCard")]
    rendered_card: Value,
    adaptive_card: Value,
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

    if query.is_empty() || query == "oauth_login_success" {
        let welcome = welcome_card();
        return PresentOutput {
            scenario: "welcome".to_string(),
            playbook_id: "tx.playbook.welcome".to_string(),
            summary: "Welcome to the Telco-X demo.".to_string(),
            text: "Welcome to the Telco-X demo.".to_string(),
            provider_hint,
            rendered_card: welcome.clone(),
            adaptive_card: welcome,
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

    if query == "menu:network-traffic-routing" {
        let card = network_menu_card();
        return PresentOutput {
            scenario: "menu-network-traffic-routing".to_string(),
            playbook_id: "tx.menu.network_traffic_routing".to_string(),
            summary: "Network Traffic & Routing".to_string(),
            text: "Network Traffic & Routing".to_string(),
            provider_hint,
            rendered_card: card.clone(),
            adaptive_card: card,
            presentation: json!({
                "kind": "menu",
                "category": "network-traffic-routing"
            }),
        };
    }

    if query == "menu:capacity-port-management" {
        let card = capacity_menu_card();
        return PresentOutput {
            scenario: "menu-capacity-port-management".to_string(),
            playbook_id: "tx.menu.capacity_port_management".to_string(),
            summary: "Capacity & Port Management".to_string(),
            text: "Capacity & Port Management".to_string(),
            provider_hint,
            rendered_card: card.clone(),
            adaptive_card: card,
            presentation: json!({
                "kind": "menu",
                "category": "capacity-port-management"
            }),
        };
    }

    if query == "menu:service-assurance" {
        let card = service_assurance_menu_card();
        return PresentOutput {
            scenario: "menu-service-assurance".to_string(),
            playbook_id: "tx.menu.service_assurance".to_string(),
            summary: "Service Assurance".to_string(),
            text: "Service Assurance".to_string(),
            provider_hint,
            rendered_card: card.clone(),
            adaptive_card: card,
            presentation: json!({
                "kind": "menu",
                "category": "service-assurance"
            }),
        };
    }

    if query == "menu:performance-root-cause" {
        let card = performance_menu_card();
        return PresentOutput {
            scenario: "menu-performance-root-cause".to_string(),
            playbook_id: "tx.menu.performance_root_cause".to_string(),
            summary: "Performance & Root Cause".to_string(),
            text: "Performance & Root Cause".to_string(),
            provider_hint,
            rendered_card: card.clone(),
            adaptive_card: card,
            presentation: json!({
                "kind": "menu",
                "category": "performance-root-cause"
            }),
        };
    }

    let resolvers = ResolverCatalog::from_fixture().expect("resolver fixture");
    let fixtures = AdapterFixtures::from_fixture().expect("adapter fixture");

    if is_service_degradation_query(&query) {
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
            text: summary,
            provider_hint,
            rendered_card: adaptive_card.clone(),
            adaptive_card,
            presentation: presentation_json,
        };
    }

    let (scenario, run) = select_run(&query, &resolvers, &fixtures);
    let presentation = present_run(&run);
    let presentation_json = serde_json::to_value(&presentation).expect("presentation json");
    let adaptive_model = parse_presentation(&presentation_json).expect("adaptive presentation");
    let adaptive_card = adaptive_card_from_presentation(&adaptive_model);

    PresentOutput {
        scenario: scenario.to_string(),
        playbook_id: run.playbook_id,
        summary: run.summary.clone(),
        text: run.summary.clone(),
        provider_hint,
        rendered_card: adaptive_card.clone(),
        adaptive_card,
        presentation: presentation_json,
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
                            "step": "show prefix traffic"
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
                            "step": "show overutilised aci ports"
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
                            "step": "run vm rca"
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
                            "step": ""
                        }
                    }
                ]
            }
        ]
    })
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
            source_provider: Some("teams".to_string()),
        };
        let output = execute_present(&input);
        assert_eq!(output.scenario, "port-utilisation");
        assert_eq!(output.playbook_id, "tx.playbook.port_utilisation");
        assert!(output.adaptive_card.is_object());
    }

    #[test]
    fn change_query_selects_change_correlation() {
        let input = PresentInput {
            query: Some("show recent change correlation".to_string()),
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
            source_provider: Some("webchat".to_string()),
        };
        let output = execute_present(&input);
        assert_eq!(output.scenario, "welcome");
        assert_eq!(output.playbook_id, "tx.playbook.welcome");
        assert_eq!(output.text, "Welcome to the Telco-X demo.");
        assert_eq!(
            output.rendered_card["body"][0]["items"][0]["columns"][1]["items"][0]["text"],
            "Telco-X Demo"
        );
        assert_eq!(
            output.rendered_card["body"][2]["actions"][0]["data"]["step"],
            "menu:network-traffic-routing"
        );
        assert_eq!(
            output.rendered_card["body"][3]["actions"][0]["data"]["step"],
            "menu:capacity-port-management"
        );
        assert_eq!(
            output.rendered_card["body"][4]["actions"][0]["data"]["step"],
            "menu:service-assurance"
        );
        assert_eq!(
            output.rendered_card["body"][5]["actions"][0]["data"]["step"],
            "menu:performance-root-cause"
        );
    }

    #[test]
    fn degradation_query_runs_multi_playbook_triage() {
        let input = PresentInput {
            query: Some("investigate service degradation".to_string()),
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
            query: Some("menu:capacity-port-management".to_string()),
            source_provider: Some("webchat".to_string()),
        };
        let output = execute_present(&input);
        assert_eq!(output.scenario, "menu-capacity-port-management");
        assert_eq!(
            output.rendered_card["body"][0]["text"],
            "Capacity & Port Management"
        );
        assert_eq!(
            output.rendered_card["body"][3]["actions"][0]["data"]["step"],
            "show free aci ports"
        );
    }

    #[test]
    fn network_query_selects_prefix_traffic() {
        let input = PresentInput {
            query: Some("show prefix traffic".to_string()),
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
            source_provider: None,
        };
        let output = execute_present(&input);
        assert_eq!(output.scenario, "free-ports");
        assert_eq!(output.playbook_id, "tx.playbook.free_ports");
    }
}
