use std::collections::HashMap;
use std::sync::Arc;

use anyhow::{Context, Result, anyhow, bail};
use greentic_runner_host::config::HostConfig;
use greentic_runner_host::pack::PackRuntime;
use greentic_runner_host::runner::engine::{FlowContext, FlowEngine, FlowExecution, RetryConfig};
use greentic_runner_host::secrets::SecretsBackend;
use serde_json::{Value, json};
use tokio::sync::RwLock;
use uuid::Uuid;

use crate::config::Mode;
use crate::loader::TenantPack;
use crate::types::{Activity, ActivityType};
use greentic_runner_host::storage::{new_session_store, new_state_store};
use greentic_runner_host::wasi::RunnerWasiPolicy;

#[derive(Clone)]
pub struct RunnerBridge {
    mode: Mode,
    allowed_secrets: Vec<String>,
    tenants: Arc<RwLock<HashMap<String, Arc<TenantRuntime>>>>,
}

struct TenantRuntime {
    tenant: String,
    config: Arc<HostConfig>,
    engine: Arc<FlowEngine>,
    messaging_flow_id: String,
}

impl RunnerBridge {
    pub fn new(mode: Mode, allowed_secrets: Vec<String>) -> Self {
        Self {
            mode,
            allowed_secrets,
            tenants: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    pub async fn register_pack(&self, pack: &TenantPack) -> Result<()> {
        tracing::info!(
            tenant = %pack.tenant,
            path = %pack.index_path.display(),
            bindings = %pack.bindings_path.display(),
            "registering pack"
        );
        let config = Arc::new(
            HostConfig::load_from_path(&pack.bindings_path)
                .with_context(|| format!("failed to load bindings for {}", pack.tenant))?,
        );
        ensure_allowed_secrets(&config, &self.allowed_secrets)?;

        let session_store = new_session_store();
        let state_store = new_state_store();
        let wasi_policy = Arc::new(RunnerWasiPolicy::default());
        let secrets_backend = SecretsBackend::from_env(std::env::var("SECRETS_BACKEND").ok())?;
        let secrets_manager = secrets_backend.build_manager()?;
        let oauth_config = config.oauth_broker_config();

        let pack_runtime = Arc::new(
            PackRuntime::load(
                &pack.index_path,
                Arc::clone(&config),
                None,
                None,
                Some(Arc::clone(&session_store)),
                Some(Arc::clone(&state_store)),
                Arc::clone(&wasi_policy),
                Arc::clone(&secrets_manager),
                oauth_config,
                false,
            )
            .await
            .with_context(|| format!("failed to load pack for {}", pack.tenant))?,
        );

        let engine = Arc::new(
            FlowEngine::new(vec![Arc::clone(&pack_runtime)], Arc::clone(&config))
                .await
                .with_context(|| format!("failed to prime flow engine for {}", pack.tenant))?,
        );

        let messaging_flow = engine
            .flow_by_type("messaging")
            .ok_or_else(|| anyhow!("tenant {} has no messaging flow", pack.tenant))?
            .id
            .clone();

        let runtime = Arc::new(TenantRuntime {
            tenant: pack.tenant.clone(),
            config,
            engine,
            messaging_flow_id: messaging_flow,
        });

        self.tenants
            .write()
            .await
            .insert(pack.tenant.clone(), runtime);
        Ok(())
    }

    pub async fn handle_activity(&self, tenant: &str, activity: Activity) -> Result<Vec<Activity>> {
        let runtime = {
            let guard = self.tenants.read().await;
            guard
                .get(tenant)
                .cloned()
                .ok_or_else(|| anyhow!("tenant {tenant} not registered with runner"))?
        };

        let payload = activity_to_flow_input(&activity)?;
        let selection = select_flow(&runtime, &activity);
        let retry_cfg = runtime.config.retry_config();
        let ctx = FlowContext {
            tenant: &runtime.tenant,
            flow_id: selection.flow_id.as_str(),
            node_id: selection.node.as_deref(),
            tool: None,
            action: Some("messaging"),
            session_id: None,
            provider_id: None,
            retry_config: RetryConfig::from(retry_cfg),
            observer: None,
            mocks: None,
        };

        tracing::debug!(
            tenant,
            mode = ?self.mode,
            flow = %selection.flow_id,
            "dispatching activity to flow engine"
        );
        let response: FlowExecution = runtime
            .engine
            .execute(ctx, payload)
            .await
            .with_context(|| format!("flow execution failed for tenant {tenant}"))?;

        flow_value_to_activities(&activity, tenant, response.output)
    }
}

fn ensure_allowed_secrets(config: &HostConfig, allowed: &[String]) -> Result<()> {
    if allowed.is_empty() {
        return Ok(());
    }
    let binding = config
        .messaging_binding()
        .ok_or_else(|| anyhow!("bindings missing flow_type_bindings.messaging"))?;

    for secret in allowed {
        if !binding.secrets.iter().any(|s| s == secret) {
            bail!(
                "tenant {} binding must include secret `{secret}` under flow_type_bindings.messaging.secrets",
                config.tenant
            );
        }
    }
    Ok(())
}

struct FlowSelection {
    flow_id: String,
    node: Option<String>,
}

fn select_flow(runtime: &TenantRuntime, activity: &Activity) -> FlowSelection {
    if let Some(session_tenant) = session_string(activity, &["tenant"])
        && session_tenant != runtime.tenant
    {
        tracing::warn!(
            tenant = %runtime.tenant,
            requested = %session_tenant,
            "session tenant hint ignored"
        );
    }

    let node_hint = resolve_node_hint(activity);
    if let Some(flow_hint) = resolve_flow_hint(activity) {
        if runtime.engine.flow_by_id(&flow_hint).is_some() {
            tracing::debug!(
                tenant = %runtime.tenant,
                flow = %flow_hint,
                "using flow override from activity"
            );
            return FlowSelection {
                flow_id: flow_hint,
                node: node_hint,
            };
        } else {
            tracing::warn!(
                tenant = %runtime.tenant,
                flow = %flow_hint,
                "activity requested flow not found; defaulting"
            );
        }
    }

    FlowSelection {
        flow_id: runtime.messaging_flow_id.clone(),
        node: node_hint,
    }
}

fn resolve_flow_hint(activity: &Activity) -> Option<String> {
    channel_string(activity, &["flowId", "flow_id", "flow"])
        .or_else(|| session_string(activity, &["flow", "flowId", "flow_id"]))
}

fn resolve_node_hint(activity: &Activity) -> Option<String> {
    channel_string(activity, &["nodeId", "node_id", "node"])
        .or_else(|| session_string(activity, &["node", "nodeId", "node_id"]))
}

fn channel_string(activity: &Activity, keys: &[&str]) -> Option<String> {
    let map = activity.channel_data.as_ref()?.as_object()?;
    for key in keys {
        if let Some(value) = map.get(*key).and_then(|v| v.as_str()) {
            return Some(value.to_string());
        }
    }
    None
}

fn session_string(activity: &Activity, keys: &[&str]) -> Option<String> {
    let session = activity
        .channel_data
        .as_ref()?
        .get("session")?
        .as_object()?;
    for key in keys {
        if let Some(value) = session.get(*key).and_then(|v| v.as_str()) {
            return Some(value.to_string());
        }
    }
    None
}

fn activity_to_flow_input(activity: &Activity) -> Result<Value> {
    let full = serde_json::to_value(activity)?;
    Ok(json!({
        "activity": full,
        "type": activity.activity_type.as_str(),
        "text": activity.text,
        "value": activity.value,
        "attachments": activity.attachments,
        "channelData": activity.channel_data,
        "conversation": activity.conversation,
        "from": activity.from,
        "recipient": activity.recipient,
        "entities": activity.entities,
    }))
}

fn flow_value_to_activities(
    reference: &Activity,
    tenant: &str,
    value: Value,
) -> Result<Vec<Activity>> {
    match value {
        Value::Array(items) => {
            let mut results = Vec::with_capacity(items.len());
            for item in items {
                results.push(coerce_activity(reference, tenant, item)?);
            }
            Ok(results)
        }
        other => Ok(vec![coerce_activity(reference, tenant, other)?]),
    }
}

fn coerce_activity(reference: &Activity, tenant: &str, value: Value) -> Result<Activity> {
    match value {
        Value::Null => Ok(default_activity(reference, tenant, None)),
        Value::String(text) => Ok(default_activity(reference, tenant, Some(text))),
        Value::Bool(flag) => Ok(default_activity(reference, tenant, Some(flag.to_string()))),
        Value::Number(num) => Ok(default_activity(reference, tenant, Some(num.to_string()))),
        Value::Object(mut map) => {
            map.entry("type".to_string())
                .or_insert_with(|| Value::String("message".into()));
            let patched = Value::Object(map.clone());
            match serde_json::from_value::<Activity>(patched) {
                Ok(mut activity) => {
                    normalize_outgoing(reference, tenant, &mut activity);
                    Ok(activity)
                }
                Err(err) => {
                    let fallback = Value::Object(map);
                    tracing::warn!(error = %err, payload = %fallback, "invalid activity payload from flow");
                    Ok(default_activity(
                        reference,
                        tenant,
                        Some(fallback.to_string()),
                    ))
                }
            }
        }
        Value::Array(_) => Ok(default_activity(reference, tenant, Some(value.to_string()))),
    }
}

fn default_activity(reference: &Activity, tenant: &str, text: Option<String>) -> Activity {
    let mut activity = Activity {
        activity_type: ActivityType::Message,
        text,
        ..Activity::default()
    };
    normalize_outgoing(reference, tenant, &mut activity);
    activity
}

fn normalize_outgoing(reference: &Activity, tenant: &str, activity: &mut Activity) {
    if matches!(activity.activity_type, ActivityType::Unknown(_)) {
        activity.activity_type = ActivityType::Message;
    }
    if activity.id.is_none() {
        activity.id = Some(Uuid::new_v4().to_string());
    }
    if activity.conversation.is_none() {
        activity.conversation = reference.conversation.clone();
    }
    if activity.from.is_none() {
        activity.from = reference.recipient.clone();
    }
    if activity.recipient.is_none() {
        activity.recipient = reference.from.clone();
    }
    if activity.reply_to_id.is_none() {
        activity.reply_to_id = reference.id.clone();
    }

    let mut channel_data = activity
        .channel_data
        .clone()
        .unwrap_or_else(|| json!({ "tenant": tenant }));

    if let Some(trace_id) = reference.tenant_trace_id() {
        channel_data["traceId"] = Value::String(trace_id);
    } else if let Some(conversation) = &reference.conversation
        && let Some(conv_id) = &conversation.id
    {
        channel_data["traceId"] = Value::String(conv_id.clone());
    }

    activity.channel_data = Some(channel_data);
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn base_activity() -> Activity {
        Activity {
            activity_type: ActivityType::Message,
            id: Some("abc".into()),
            conversation: Some(crate::types::ConversationAccount {
                id: Some("conversation".into()),
                name: None,
            }),
            from: Some(crate::types::ChannelAccount {
                id: Some("user".into()),
                name: None,
            }),
            recipient: Some(crate::types::ChannelAccount {
                id: Some("bot".into()),
                name: None,
            }),
            channel_data: Some(json!({ "traceId": "trace-123", "tenant": "customera" })),
            ..Activity::default()
        }
    }

    #[test]
    fn default_text_activity_mapping() {
        let incoming = base_activity();
        let result = coerce_activity(&incoming, "customera", Value::String("hi".into())).unwrap();
        assert_eq!(result.text.as_deref(), Some("hi"));
        assert_eq!(result.activity_type, ActivityType::Message);
        assert_eq!(
            result.conversation.unwrap().id.as_deref(),
            Some("conversation")
        );
    }

    #[test]
    fn preserves_custom_activity_fields() {
        let incoming = base_activity();
        let outgoing = json!({
            "type": "message",
            "text": "pong",
            "channelData": { "tenant": "customera", "foo": "bar" }
        });
        let mapped = coerce_activity(&incoming, "customera", outgoing).unwrap();
        assert_eq!(mapped.text.as_deref(), Some("pong"));
        assert_eq!(mapped.channel_data.unwrap()["traceId"], "trace-123");
    }

    #[test]
    fn array_payload_maps_to_multiple_activities() {
        let incoming = base_activity();
        let responses =
            flow_value_to_activities(&incoming, "customera", json!(["one", { "text": "two" }]))
                .unwrap();
        assert_eq!(responses.len(), 2);
        assert_eq!(responses[0].text.as_deref(), Some("one"));
        assert_eq!(responses[1].text.as_deref(), Some("two"));
    }

    #[test]
    fn flow_hint_detected_from_channel_data() {
        let mut activity = base_activity();
        activity.channel_data = Some(json!({ "flowId": "weather_bot" }));
        assert_eq!(resolve_flow_hint(&activity).as_deref(), Some("weather_bot"));
    }

    #[test]
    fn node_hint_detected_from_session() {
        let mut activity = base_activity();
        activity.channel_data = Some(json!({ "session": { "node": "qa_node" } }));
        assert_eq!(resolve_node_hint(&activity).as_deref(), Some("qa_node"));
    }
}
