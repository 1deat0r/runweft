//! Schema-first Runweft protocol validation. This crate does not execute work or effects.

use std::collections::HashSet;
use std::sync::OnceLock;

use jsonschema::{Draft, Validator};
use serde::Serialize;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};

mod generated;
mod generated_metadata;
mod generated_status;
mod generated_transitions;

pub use generated::*;
pub use generated_metadata::{
    MAX_CONTAINER_DEPTH, MAX_RAW_MESSAGE_BYTES, U64_MAX, U128_MAX, is_error_code, is_known_variant,
    peer_role_allowed, retry_advice,
};
pub use generated_status::{RuntimeStatus, scaffold_status};
pub use generated_transitions::{
    TRANSITION_COUNT, can_transition, lifecycle_entity_for_event, transition_events,
};

pub const MALFORMED_PAYLOAD: &str = "malformed_payload";
pub const UNKNOWN_FIELD: &str = "unknown_field";
pub const UNKNOWN_CRITICAL_VARIANT: &str = "unknown_critical_variant";
pub const UNSUPPORTED_VERSION: &str = "unsupported_version";
pub const RESOURCE_EXHAUSTED: &str = "resource_exhausted";
pub const UNAUTHORIZED: &str = "unauthorized";
pub const STALE_PROFILE_INCARNATION: &str = "stale_profile_incarnation";
pub const STALE_COORDINATOR_GENERATION: &str = "stale_coordinator_generation";
pub const STALE_LEASE: &str = "stale_lease";
pub const INVALID_STATE_TRANSITION: &str = "invalid_state_transition";

const PROTOCOL_SCHEMA: &str = include_str!("../../../schemas/run-protocol.schema.json");
const KNOWN_KINDS: [&str; 4] = ["command", "event", "response", "error"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthenticatedPeer {
    pub role: String,
    pub id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CurrentLease {
    pub profile_id: String,
    pub project_id: String,
    pub run_id: String,
    pub task_id: String,
    pub attempt_id: String,
    pub lease_id: String,
    pub profile_incarnation: String,
    pub coordinator_generation: String,
    pub epoch: String,
    pub assigned_peer_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuthorityContext {
    pub peer: AuthenticatedPeer,
    pub profile_id: String,
    pub project_id: String,
    pub profile_incarnation: String,
    pub coordinator_generation: String,
    pub current_lease: Option<CurrentLease>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "lowercase")]
pub enum ValidationResult {
    Accepted { message: Value },
    Rejected { code: &'static str },
}

fn schema_validator() -> &'static Validator {
    static VALIDATOR: OnceLock<Validator> = OnceLock::new();
    VALIDATOR.get_or_init(|| {
        let schema: Value = serde_json::from_str(PROTOCOL_SCHEMA)
            .expect("embedded protocol schema must be valid JSON");
        jsonschema::options()
            .with_draft(Draft::Draft202012)
            .build(&schema)
            .expect("embedded protocol schema must compile")
    })
}

#[derive(Default)]
struct RawScan {
    bytes: Vec<u8>,
    cursor: usize,
    duplicate_key: bool,
    number_token: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum ScanError {
    Malformed,
    TooDeep,
}

impl RawScan {
    fn new(bytes: &[u8]) -> Self {
        Self {
            bytes: bytes.to_vec(),
            ..Self::default()
        }
    }

    fn scan(mut self) -> Result<Self, ScanError> {
        self.value(0)?;
        self.whitespace();
        if self.cursor != self.bytes.len() {
            return Err(ScanError::Malformed);
        }
        Ok(self)
    }

    fn whitespace(&mut self) {
        while matches!(
            self.bytes.get(self.cursor),
            Some(b' ' | b'\t' | b'\n' | b'\r')
        ) {
            self.cursor += 1;
        }
    }

    fn value(&mut self, parent_depth: usize) -> Result<(), ScanError> {
        self.whitespace();
        match self.bytes.get(self.cursor).copied() {
            Some(b'{') => self.object(parent_depth + 1),
            Some(b'[') => self.array(parent_depth + 1),
            Some(b'"') => self.string().map(|_| ()),
            Some(b't') => self.literal(b"true"),
            Some(b'f') => self.literal(b"false"),
            Some(b'n') => self.literal(b"null"),
            Some(b'-' | b'0'..=b'9') => self.number(),
            _ => Err(ScanError::Malformed),
        }
    }

    fn object(&mut self, depth: usize) -> Result<(), ScanError> {
        if depth > MAX_CONTAINER_DEPTH {
            return Err(ScanError::TooDeep);
        }
        self.cursor += 1;
        self.whitespace();
        if self.bytes.get(self.cursor) == Some(&b'}') {
            self.cursor += 1;
            return Ok(());
        }
        let mut keys = HashSet::new();
        loop {
            let key_bytes = self.string()?;
            let key: String =
                serde_json::from_slice(key_bytes).map_err(|_| ScanError::Malformed)?;
            if !keys.insert(key) {
                self.duplicate_key = true;
            }
            self.whitespace();
            if self.bytes.get(self.cursor) != Some(&b':') {
                return Err(ScanError::Malformed);
            }
            self.cursor += 1;
            self.value(depth)?;
            self.whitespace();
            match self.bytes.get(self.cursor) {
                Some(b'}') => {
                    self.cursor += 1;
                    return Ok(());
                }
                Some(b',') => {
                    self.cursor += 1;
                    self.whitespace();
                }
                _ => return Err(ScanError::Malformed),
            }
        }
    }

    fn array(&mut self, depth: usize) -> Result<(), ScanError> {
        if depth > MAX_CONTAINER_DEPTH {
            return Err(ScanError::TooDeep);
        }
        self.cursor += 1;
        self.whitespace();
        if self.bytes.get(self.cursor) == Some(&b']') {
            self.cursor += 1;
            return Ok(());
        }
        loop {
            self.value(depth)?;
            self.whitespace();
            match self.bytes.get(self.cursor) {
                Some(b']') => {
                    self.cursor += 1;
                    return Ok(());
                }
                Some(b',') => {
                    self.cursor += 1;
                    self.whitespace();
                }
                _ => return Err(ScanError::Malformed),
            }
        }
    }

    // Returns the byte slice including quotes. serde_json decodes keys so escaped
    // spellings of the same key are treated as duplicates.
    fn string(&mut self) -> Result<&[u8], ScanError> {
        if self.bytes.get(self.cursor) != Some(&b'"') {
            return Err(ScanError::Malformed);
        }
        let start = self.cursor;
        self.cursor += 1;
        while let Some(byte) = self.bytes.get(self.cursor).copied() {
            match byte {
                b'"' => {
                    self.cursor += 1;
                    return Ok(&self.bytes[start..self.cursor]);
                }
                b'\\' => {
                    self.cursor += 1;
                    let escaped = self
                        .bytes
                        .get(self.cursor)
                        .copied()
                        .ok_or(ScanError::Malformed)?;
                    self.cursor += 1;
                    if escaped == b'u' {
                        for _ in 0..4 {
                            if self.bytes.get(self.cursor).is_none() {
                                return Err(ScanError::Malformed);
                            }
                            self.cursor += 1;
                        }
                    }
                }
                _ => self.cursor += 1,
            }
        }
        Err(ScanError::Malformed)
    }

    fn literal(&mut self, literal: &[u8]) -> Result<(), ScanError> {
        if self.bytes.get(self.cursor..self.cursor + literal.len()) == Some(literal) {
            self.cursor += literal.len();
            Ok(())
        } else {
            Err(ScanError::Malformed)
        }
    }

    fn number(&mut self) -> Result<(), ScanError> {
        let start = self.cursor;
        if self.bytes.get(self.cursor) == Some(&b'-') {
            self.cursor += 1;
        }
        match self.bytes.get(self.cursor) {
            Some(b'0') => self.cursor += 1,
            Some(b'1'..=b'9') => {
                self.cursor += 1;
                while matches!(self.bytes.get(self.cursor), Some(b'0'..=b'9')) {
                    self.cursor += 1;
                }
            }
            _ => return Err(ScanError::Malformed),
        }
        if self.bytes.get(self.cursor) == Some(&b'.') {
            self.cursor += 1;
            let digits_start = self.cursor;
            while matches!(self.bytes.get(self.cursor), Some(b'0'..=b'9')) {
                self.cursor += 1;
            }
            if digits_start == self.cursor {
                return Err(ScanError::Malformed);
            }
        }
        if matches!(self.bytes.get(self.cursor), Some(b'e' | b'E')) {
            self.cursor += 1;
            if matches!(self.bytes.get(self.cursor), Some(b'+' | b'-')) {
                self.cursor += 1;
            }
            let digits_start = self.cursor;
            while matches!(self.bytes.get(self.cursor), Some(b'0'..=b'9')) {
                self.cursor += 1;
            }
            if digits_start == self.cursor {
                return Err(ScanError::Malformed);
            }
        }
        if self.cursor == start {
            return Err(ScanError::Malformed);
        }
        self.number_token = true;
        Ok(())
    }
}

fn schema_defs() -> &'static Value {
    static SCHEMA: OnceLock<Value> = OnceLock::new();
    SCHEMA.get_or_init(|| {
        serde_json::from_str(PROTOCOL_SCHEMA).expect("embedded protocol schema must be valid JSON")
    })
}

fn resolve_schema<'a>(schema: &'a Value, root: &'a Value) -> &'a Value {
    let Some(reference) = schema.get("$ref").and_then(Value::as_str) else {
        return schema;
    };
    let name = reference.rsplit('/').next().unwrap_or_default();
    root.get("$defs")
        .and_then(|defs| defs.get(name))
        .unwrap_or(&Value::Null)
}

fn variant_schema(kind: &str, variant: &str) -> Option<&'static Value> {
    let definition = match kind {
        "command" => "CommandMessage",
        "event" => "EventMessage",
        "response" => "ResponseMessage",
        "error" => "ErrorMessage",
        _ => return None,
    };
    let root = schema_defs();
    for reference in root
        .get("$defs")?
        .get(definition)?
        .get("oneOf")?
        .as_array()?
    {
        let candidate = resolve_schema(reference, root);
        let branches = candidate
            .get("allOf")
            .and_then(Value::as_array)
            .map_or_else(|| vec![candidate], |all| all.iter().collect());
        if branches.iter().any(|branch| {
            resolve_schema(branch, root)
                .pointer("/properties/variant/const")
                .and_then(Value::as_str)
                == Some(variant)
        }) {
            return Some(candidate);
        }
    }
    None
}

fn has_unknown_property(
    value: &Value,
    schema: &Value,
    root: &Value,
    active_refs: &mut HashSet<String>,
) -> bool {
    let reference = schema.get("$ref").and_then(Value::as_str);
    if let Some(reference) = reference
        && !active_refs.insert(reference.to_string())
    {
        return false;
    }
    let node = resolve_schema(schema, root);
    let mut unknown = false;
    if let Some(object) = value.as_object() {
        let properties = node.get("properties").and_then(Value::as_object);
        if node.get("additionalProperties") == Some(&Value::Bool(false)) {
            unknown = object
                .keys()
                .any(|key| !properties.is_some_and(|p| p.contains_key(key)));
        }
        if !unknown && let Some(properties) = properties {
            unknown = properties.iter().any(|(key, child_schema)| {
                object.get(key).is_some_and(|child| {
                    has_unknown_property(child, child_schema, root, &mut active_refs.clone())
                })
            });
        }
        if !unknown
            && let Some(additional) = node
                .get("additionalProperties")
                .filter(|node| node.is_object())
        {
            unknown = object.iter().any(|(key, child)| {
                !properties.is_some_and(|p| p.contains_key(key))
                    && has_unknown_property(child, additional, root, &mut active_refs.clone())
            });
        }
    } else if let Some(array) = value.as_array()
        && let Some(items) = node.get("items")
    {
        unknown = array
            .iter()
            .any(|child| has_unknown_property(child, items, root, &mut active_refs.clone()));
    }
    if !unknown {
        for keyword in ["allOf", "oneOf", "anyOf"] {
            if let Some(parts) = node.get(keyword).and_then(Value::as_array)
                && parts
                    .iter()
                    .any(|part| has_unknown_property(value, part, root, &mut active_refs.clone()))
            {
                unknown = true;
                break;
            }
        }
    }
    if let Some(reference) = reference {
        active_refs.remove(reference);
    }
    unknown
}

fn extension_value_valid(value: &Value) -> bool {
    match value {
        Value::Null | Value::Bool(_) => true,
        Value::String(text) => text.len() <= 4096,
        Value::Array(values) => values.iter().all(extension_value_valid),
        Value::Object(object) => {
            object.len() <= 32
                && object
                    .iter()
                    .all(|(key, child)| key.len() <= 128 && extension_value_valid(child))
        }
        Value::Number(_) => false,
    }
}

fn extensions_valid(message: &Value) -> bool {
    match message.get("extensions") {
        None => true,
        Some(Value::Object(extensions)) => {
            extensions.len() <= 32
                && extensions
                    .iter()
                    .all(|(key, value)| key.len() <= 128 && extension_value_valid(value))
        }
        Some(_) => false,
    }
}

fn numeric_ranges_valid(value: &Value, key: Option<&str>, root: bool) -> bool {
    if let (Some(key), Some(text)) = (key, value.as_str()) {
        match key {
            "coordinator_generation" | "sequence" | "epoch" | "lease_epoch" => {
                return text.parse::<u64>().is_ok_and(|value| value > 0);
            }
            "requested_budget_micro_usd" => return text.parse::<u128>().is_ok(),
            _ => {}
        }
    }
    match value {
        Value::Array(values) => values
            .iter()
            .all(|child| numeric_ranges_valid(child, None, false)),
        Value::Object(object) => object.iter().all(|(child_key, child)| {
            if root && child_key == "extensions" {
                true
            } else {
                numeric_ranges_valid(child, Some(child_key), false)
            }
        }),
        _ => true,
    }
}

fn authority_error(message: &Value, authority: &AuthorityContext) -> Option<&'static str> {
    let kind = message.get("kind")?.as_str()?;
    let variant = message.get("variant")?.as_str()?;
    if !peer_role_allowed(kind, variant, &authority.peer.role) {
        return Some(UNAUTHORIZED);
    }
    if kind == "error" {
        return None;
    }
    if message.get("profile_id").and_then(Value::as_str) != Some(&authority.profile_id)
        || message.get("project_id").and_then(Value::as_str) != Some(&authority.project_id)
    {
        return Some(UNAUTHORIZED);
    }
    if message.get("profile_incarnation").and_then(Value::as_str)
        != Some(&authority.profile_incarnation)
    {
        return Some(STALE_PROFILE_INCARNATION);
    }
    if message
        .get("coordinator_generation")
        .and_then(Value::as_str)
        != Some(&authority.coordinator_generation)
    {
        return Some(STALE_COORDINATOR_GENERATION);
    }
    if kind == "command" && variant == "attempt.result.submit" {
        let Some(lease) = authority.current_lease.as_ref() else {
            return Some(STALE_LEASE);
        };
        if authority.peer.id != lease.assigned_peer_id {
            return Some(UNAUTHORIZED);
        }
        let Some(claimed) = message.pointer("/payload/lease") else {
            return Some(STALE_LEASE);
        };
        let matches = [
            ("profile_id", lease.profile_id.as_str()),
            ("project_id", lease.project_id.as_str()),
            ("run_id", lease.run_id.as_str()),
            ("task_id", lease.task_id.as_str()),
            ("attempt_id", lease.attempt_id.as_str()),
            ("lease_id", lease.lease_id.as_str()),
            ("profile_incarnation", lease.profile_incarnation.as_str()),
            (
                "coordinator_generation",
                lease.coordinator_generation.as_str(),
            ),
            ("epoch", lease.epoch.as_str()),
        ]
        .iter()
        .all(|(key, expected)| claimed.get(*key).and_then(Value::as_str) == Some(*expected));
        let payload_matches = [
            ("run_id", lease.run_id.as_str()),
            ("task_id", lease.task_id.as_str()),
            ("attempt_id", lease.attempt_id.as_str()),
        ]
        .iter()
        .all(|(key, expected)| {
            message
                .pointer(&format!("/payload/{key}"))
                .and_then(Value::as_str)
                == Some(*expected)
        });
        if !matches
            || !payload_matches
            || message.get("profile_id").and_then(Value::as_str) != Some(&lease.profile_id)
            || message.get("project_id").and_then(Value::as_str) != Some(&lease.project_id)
            || message.get("profile_incarnation").and_then(Value::as_str)
                != Some(&lease.profile_incarnation)
            || message
                .get("coordinator_generation")
                .and_then(Value::as_str)
                != Some(&lease.coordinator_generation)
        {
            return Some(STALE_LEASE);
        }
    }
    None
}

fn calculate_action_digest(
    action_id: &str,
    target_id: &str,
    artifact_digest: &str,
    grant_revision_id: &str,
    policy_revision_id: &str,
) -> Option<String> {
    if artifact_digest.len() != 64
        || !artifact_digest
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
    {
        return None;
    }
    let mut preimage = b"runweft.effect-approval.v1\0".to_vec();
    for value in [action_id, target_id, grant_revision_id, policy_revision_id] {
        let bytes = value.as_bytes();
        let length = u32::try_from(bytes.len()).ok()?;
        preimage.extend_from_slice(&length.to_be_bytes());
        preimage.extend_from_slice(bytes);
    }
    for pair in artifact_digest.as_bytes().as_chunks::<2>().0 {
        let hex = std::str::from_utf8(pair).ok()?;
        preimage.push(u8::from_str_radix(hex, 16).ok()?);
    }
    Some(format!("{:x}", Sha256::digest(preimage)))
}

pub fn effect_action_digest(
    action_id: &str,
    target_id: &str,
    artifact_digest: &str,
    grant_revision_id: &str,
    policy_revision_id: &str,
) -> Option<String> {
    calculate_action_digest(
        action_id,
        target_id,
        artifact_digest,
        grant_revision_id,
        policy_revision_id,
    )
}

fn digest_binding_valid(message: &Value) -> bool {
    let kind = message.get("kind").and_then(Value::as_str);
    let variant = message.get("variant").and_then(Value::as_str);
    if !matches!(
        (kind, variant),
        (Some("command"), Some("effect.approve")) | (Some("event"), Some("effect.intent.recorded"))
    ) {
        return true;
    }
    let Some(payload) = message.get("payload").and_then(Value::as_object) else {
        return true;
    };
    let field = |name: &str| payload.get(name).and_then(Value::as_str);
    let (Some(action), Some(target), Some(artifact), Some(grant), Some(policy), Some(digest)) = (
        field("action_id"),
        field("target_id"),
        field("artifact_digest"),
        field("grant_revision_id"),
        field("policy_revision_id"),
        field("action_digest"),
    ) else {
        return false;
    };
    calculate_action_digest(action, target, artifact, grant, policy).as_deref() == Some(digest)
}

fn state_transition_valid(message: &Value) -> Result<(), &'static str> {
    if message.get("kind").and_then(Value::as_str) != Some("event") {
        return Ok(());
    }
    let Some(variant) = message.get("variant").and_then(Value::as_str) else {
        return Ok(());
    };
    let Some(entity) = lifecycle_entity_for_event(variant) else {
        return Ok(());
    };
    let payload = message.get("payload").ok_or(INVALID_STATE_TRANSITION)?;
    let from = payload
        .get("from_state")
        .and_then(Value::as_str)
        .ok_or(INVALID_STATE_TRANSITION)?;
    let to = payload
        .get("to_state")
        .and_then(Value::as_str)
        .ok_or(INVALID_STATE_TRANSITION)?;
    if !can_transition(entity, from, to)
        || !transition_events(entity, from, to).is_some_and(|events| events.contains(&variant))
    {
        return Err(INVALID_STATE_TRANSITION);
    }
    Ok(())
}

/// Validate raw v1 JSON against the frozen schema and authenticated caller context.
/// The returned JSON value preserves the parsed message; no execution or persistence occurs.
fn validate_wire_message_inner(
    raw: &[u8],
    authority: &AuthorityContext,
) -> Result<Value, &'static str> {
    if raw.len() > MAX_RAW_MESSAGE_BYTES {
        return Err(RESOURCE_EXHAUSTED);
    }
    let scan = RawScan::new(raw).scan().map_err(|error| match error {
        ScanError::Malformed => MALFORMED_PAYLOAD,
        ScanError::TooDeep => RESOURCE_EXHAUSTED,
    })?;
    if scan.duplicate_key {
        return Err(MALFORMED_PAYLOAD);
    }
    let text = std::str::from_utf8(raw).map_err(|_| MALFORMED_PAYLOAD)?;
    let message: Value = serde_json::from_str(text).map_err(|_| MALFORMED_PAYLOAD)?;
    if !message.is_object() {
        return Err(MALFORMED_PAYLOAD);
    }
    let version = message.get("protocol_version").and_then(Value::as_str);
    let Some(version) = version else {
        return Err(MALFORMED_PAYLOAD);
    };
    if version != "1.0" {
        return Err(UNSUPPORTED_VERSION);
    }
    let kind = message
        .get("kind")
        .and_then(Value::as_str)
        .ok_or(MALFORMED_PAYLOAD)?;
    let variant = message
        .get("variant")
        .and_then(Value::as_str)
        .ok_or(MALFORMED_PAYLOAD)?;
    if !KNOWN_KINDS.contains(&kind) {
        return Err(MALFORMED_PAYLOAD);
    }
    if !is_known_variant(kind, variant) {
        return Err(UNKNOWN_CRITICAL_VARIANT);
    }
    let selected_schema = variant_schema(kind, variant).ok_or(UNKNOWN_CRITICAL_VARIANT)?;
    let unknown = has_unknown_property(
        &message,
        selected_schema,
        schema_defs(),
        &mut HashSet::new(),
    );
    if !schema_validator().is_valid(&message) {
        return Err(if unknown {
            UNKNOWN_FIELD
        } else {
            MALFORMED_PAYLOAD
        });
    }
    if scan.number_token
        || !numeric_ranges_valid(&message, None, true)
        || !extensions_valid(&message)
    {
        return Err(MALFORMED_PAYLOAD);
    }
    if let Some(error) = authority_error(&message, authority) {
        return Err(error);
    }
    if !digest_binding_valid(&message) {
        return Err(MALFORMED_PAYLOAD);
    }
    state_transition_valid(&message)?;
    Ok(message)
}

/// Return the same normalized accepted/rejected result shape as the TypeScript validator.
pub fn validate_wire_message(raw: &[u8], authority: &AuthorityContext) -> ValidationResult {
    match validate_wire_message_inner(raw, authority) {
        Ok(message) => ValidationResult::Accepted { message },
        Err(code) => ValidationResult::Rejected { code },
    }
}

/// Projection used for command idempotency: generation and command ID are deliberately excluded.
pub fn command_intent_projection(message: &Value) -> Value {
    const FIELDS: [&str; 8] = [
        "protocol_version",
        "kind",
        "variant",
        "profile_id",
        "project_id",
        "profile_incarnation",
        "payload",
        "extensions",
    ];
    let mut projection = Map::new();
    for field in FIELDS {
        if let Some(value) = message.get(field) {
            projection.insert(field.to_string(), value.clone());
        }
    }
    Value::Object(projection)
}

pub fn command_intent_equal(left: &Value, right: &Value) -> bool {
    command_intent_projection(left) == command_intent_projection(right)
}

pub fn attempt_receipt_matches_result(result: &Value, receipt: &Value) -> bool {
    ["run_id", "task_id", "attempt_id", "artifact_digest"]
        .iter()
        .all(|field| result.get(field) == receipt.get(field))
}

pub fn effect_receipt_matches_intent(intent: &Value, receipt: &Value) -> bool {
    [
        "effect_intent_id",
        "action_digest",
        "target_id",
        "artifact_digest",
    ]
    .iter()
    .all(|field| intent.get(field) == receipt.get(field))
        && intent.get("lease") == receipt.get("lease")
}

pub fn task_success_receipt_link(task_success: &Value, attempt_success: &Value) -> bool {
    let Some(receipt) = attempt_success.get("verification_receipt") else {
        return false;
    };
    attempt_success.get("from_state").and_then(Value::as_str) == Some("verifying")
        && attempt_success.get("to_state").and_then(Value::as_str) == Some("succeeded")
        && task_success.get("task_id") == attempt_success.get("task_id")
        && task_success.get("attempt_id") == attempt_success.get("attempt_id")
        && task_success.get("verification_receipt_id") == receipt.get("receipt_id")
}

pub fn effect_reconciliation_matches_outcome(payload: &Value) -> bool {
    matches!(
        (
            payload.get("outcome").and_then(Value::as_str),
            payload.get("to_state").and_then(Value::as_str)
        ),
        (Some("applied"), Some("reconciled_applied"))
            | (Some("not_applied"), Some("reconciled_not_applied"))
    )
}

pub fn last_event_sequence(events: &[Value]) -> Option<&str> {
    events.last()?.get("sequence")?.as_str()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn authority_from(value: &Value) -> AuthorityContext {
        let peer = value.get("peer").expect("peer");
        let current = value
            .get("current_lease")
            .and_then(Value::as_object)
            .map(|lease| CurrentLease {
                profile_id: lease["profile_id"].as_str().unwrap().to_string(),
                project_id: lease["project_id"].as_str().unwrap().to_string(),
                run_id: lease["run_id"].as_str().unwrap().to_string(),
                task_id: lease["task_id"].as_str().unwrap().to_string(),
                attempt_id: lease["attempt_id"].as_str().unwrap().to_string(),
                lease_id: lease["lease_id"].as_str().unwrap().to_string(),
                profile_incarnation: lease["profile_incarnation"].as_str().unwrap().to_string(),
                coordinator_generation: lease["coordinator_generation"]
                    .as_str()
                    .unwrap()
                    .to_string(),
                epoch: lease["epoch"].as_str().unwrap().to_string(),
                assigned_peer_id: lease["assigned_peer_id"].as_str().unwrap().to_string(),
            });
        AuthorityContext {
            peer: AuthenticatedPeer {
                role: peer["role"].as_str().unwrap().to_string(),
                id: peer["id"].as_str().unwrap().to_string(),
            },
            profile_id: value["profile_id"].as_str().unwrap().to_string(),
            project_id: value["project_id"].as_str().unwrap().to_string(),
            profile_incarnation: value["profile_incarnation"].as_str().unwrap().to_string(),
            coordinator_generation: value["coordinator_generation"]
                .as_str()
                .unwrap()
                .to_string(),
            current_lease: current,
        }
    }

    fn assert_expected(result: ValidationResult, expected: &Value, case_id: &str) {
        assert_eq!(
            serde_json::to_value(result).unwrap(),
            *expected,
            "{case_id}"
        );
    }

    #[test]
    fn exact_main_and_behavior_fixtures_match() {
        let main: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/protocol/v1/cases.json"
        ))
        .unwrap();
        let behavior: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/protocol/v1/behavior-cases.json"
        ))
        .unwrap();
        assert_eq!(main["cases"].as_array().unwrap().len(), 50);
        for fixture in main["cases"].as_array().unwrap() {
            if fixture["input"]["kind"] == "lifecycle-transition" {
                let transition = &fixture["input"];
                assert!(can_transition(
                    transition["entity"].as_str().unwrap(),
                    transition["from"].as_str().unwrap(),
                    transition["to"].as_str().unwrap()
                ));
                continue;
            }
            let authority = authority_from(&fixture["authority"]);
            let raw = serde_json::to_vec(&fixture["input"]).unwrap();
            assert_expected(
                validate_wire_message(&raw, &authority),
                &fixture["expected"],
                fixture["id"].as_str().unwrap(),
            );
        }
        for fixture in behavior["cases"].as_array().unwrap() {
            let authority = authority_from(&fixture["authority"]);
            let raw = serde_json::to_vec(&fixture["input"]).unwrap();
            assert_expected(
                validate_wire_message(&raw, &authority),
                &fixture["expected"],
                fixture["id"].as_str().unwrap(),
            );
        }
    }

    #[test]
    fn raw_wire_fixtures_match_exact_bytes_and_limits() {
        let corpus: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/protocol/v1/raw-cases.json"
        ))
        .unwrap();
        for fixture in corpus["cases"].as_array().unwrap() {
            let wire = if fixture["kind"] == "literal" {
                fixture["wire"].as_str().unwrap().as_bytes().to_vec()
            } else {
                let descriptor = &fixture["wire"];
                let prefix = descriptor["prefix"].as_str().unwrap();
                let byte = descriptor["repeat_byte"].as_str().unwrap().as_bytes()[0];
                let count = descriptor["repeat_count"].as_u64().unwrap() as usize;
                let mut wire = prefix.as_bytes().to_vec();
                wire.extend(std::iter::repeat_n(byte, count));
                assert_eq!(
                    wire.len(),
                    fixture["expected_byte_length"].as_u64().unwrap() as usize
                );
                assert_eq!(
                    format!("{:x}", Sha256::digest(&wire)),
                    fixture["expected_sha256"].as_str().unwrap()
                );
                wire
            };
            let authority = authority_from(&fixture["authority"]);
            assert_expected(
                validate_wire_message(&wire, &authority),
                &fixture["expected"],
                fixture["id"].as_str().unwrap(),
            );
        }
    }

    #[test]
    fn behavior_assertions_match_literal_oracles() {
        let corpus: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/protocol/v1/behavior-cases.json"
        ))
        .unwrap();
        for assertion in corpus["assertions"].as_array().unwrap() {
            if assertion["kind"] == "last_event_sequence" {
                assert_eq!(
                    last_event_sequence(assertion["events"].as_array().unwrap()),
                    assertion["expected"].as_str(),
                    "{}",
                    assertion["id"]
                );
                continue;
            }
            let actual = match assertion["kind"].as_str().unwrap() {
                "command_intent_equal" => {
                    command_intent_equal(&assertion["left"], &assertion["right"])
                }
                "can_transition" => can_transition(
                    assertion["entity"].as_str().unwrap(),
                    assertion["from"].as_str().unwrap(),
                    assertion["to"].as_str().unwrap(),
                ),
                "last_event_sequence" => unreachable!("handled above"),
                "attempt_receipt_matches_result" => attempt_receipt_matches_result(
                    &assertion["accepted_result"],
                    &assertion["verification_receipt"],
                ),
                "effect_receipt_matches_intent" => {
                    effect_receipt_matches_intent(&assertion["intent"], &assertion["receipt"])
                }
                "task_success_receipt_link" => task_success_receipt_link(
                    &assertion["task_success"],
                    &assertion["attempt_success"],
                ),
                "effect_reconciliation_matches_outcome" => {
                    effect_reconciliation_matches_outcome(&assertion["payload"])
                }
                other => panic!("unknown behavior assertion {other}"),
            };
            assert_eq!(
                actual,
                assertion["expected"].as_bool().unwrap(),
                "{}",
                assertion["id"]
            );
        }
    }

    #[test]
    fn lifecycle_allow_list_is_exhaustive_and_event_mapped() {
        let lifecycle: Value =
            serde_json::from_str(include_str!("../../../schemas/lifecycle-transitions.json"))
                .unwrap();
        let mut expected = HashSet::new();
        for entity in lifecycle["entities"].as_array().unwrap() {
            let name = entity["name"].as_str().unwrap();
            let states = entity["states"].as_array().unwrap();
            for transition in entity["transitions"].as_array().unwrap() {
                let from = transition["from"].as_str().unwrap();
                let to = transition["to"].as_str().unwrap();
                assert!(states.iter().any(|state| state.as_str() == Some(from)));
                assert!(states.iter().any(|state| state.as_str() == Some(to)));
                expected.insert((name.to_string(), from.to_string(), to.to_string()));
                let actual = transition_events(name, from, to).unwrap();
                let events = transition["events"].as_array().unwrap();
                assert_eq!(actual.len(), events.len());
                for (actual, expected) in actual.iter().zip(events) {
                    assert_eq!(*actual, expected.as_str().unwrap());
                }
            }
            for from in states {
                for to in states {
                    let key = (
                        name.to_string(),
                        from.as_str().unwrap().to_string(),
                        to.as_str().unwrap().to_string(),
                    );
                    assert_eq!(
                        can_transition(&key.0, &key.1, &key.2),
                        expected.contains(&key)
                    );
                }
            }
        }
        assert_eq!(expected.len(), TRANSITION_COUNT);
    }

    #[test]
    fn replay_atomicity_generation_and_recovery_fixtures_hold() {
        let corpus: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/protocol/v1/behavior-cases.json"
        ))
        .unwrap();
        for fixture in corpus["command_replay_cases"].as_array().unwrap() {
            let record = &fixture["record"];
            let retry = &fixture["retry"];
            let same_identity = [
                "profile_id",
                "project_id",
                "profile_incarnation",
                "command_id",
            ]
            .iter()
            .all(|field| record["identity"][*field] == retry[*field]);
            let same_intent = same_identity && command_intent_equal(&record["command"], retry);
            if same_intent {
                assert_eq!(fixture["expected"]["status"], "accepted");
                assert_eq!(fixture["expected"]["result"], record["committed_result"]);
                assert_eq!(
                    fixture["expected"]["event_count_after_retry"],
                    record["committed_events"].as_array().unwrap().len()
                );
                assert_eq!(fixture["expected"]["duplicate"], true);
            } else {
                assert_eq!(fixture["expected"]["status"], "rejected");
                assert_eq!(fixture["expected"]["code"], "conflict");
                assert_eq!(
                    fixture["expected"]["event_count_after_retry"],
                    record["committed_events"].as_array().unwrap().len()
                );
            }
        }
        for fixture in corpus["command_atomicity_cases"].as_array().unwrap() {
            let committed = fixture["attempt"]["commit"] == "succeeded";
            assert_eq!(
                fixture["expected"]["dedup_record_count"],
                u64::from(committed)
            );
            assert_eq!(fixture["expected"]["event_count"], u64::from(committed));
            if let Some(expected_response_sent) = fixture["expected"].get("response_sent") {
                assert_eq!(
                    expected_response_sent,
                    &(committed && fixture["attempt"]["response_sent"] == true)
                );
            }
            if committed {
                assert_eq!(fixture["expected"]["retry_returns_saved_result"], true);
            }
        }
        for fixture in corpus["generation_cases"].as_array().unwrap() {
            let last = fixture["last_generation"]
                .as_str()
                .unwrap()
                .parse::<u64>()
                .unwrap();
            if last == u64::MAX {
                assert_eq!(fixture["expected"]["status"], "rejected");
                assert_eq!(fixture["expected"]["code"], "resource_exhausted");
            } else {
                let requested = fixture["requested_generation"]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap();
                if requested <= last {
                    assert_eq!(fixture["expected"]["status"], "rejected");
                    assert_eq!(fixture["expected"]["code"], "conflict");
                } else {
                    assert_eq!(fixture["expected"]["status"], "accepted");
                    assert_eq!(fixture["expected"]["generation"], requested.to_string());
                }
            }
        }
        for fixture in corpus["effect_recovery_cases"].as_array().unwrap() {
            assert_eq!(fixture["durable_state_before_crash"], "invoking");
            assert_eq!(fixture["terminal_receipt_committed"], false);
            assert_eq!(fixture["expected"]["recovered_state"], "unknown_effect");
            assert_eq!(
                fixture["expected"]["repeat_result"]["code"],
                "unknown_effect"
            );
            assert_eq!(
                fixture["expected"]["calls_after_repeat"],
                fixture["calls_before_recovery"]
            );
        }
    }

    #[test]
    fn fake_effect_stub_invokes_only_after_approval_and_never_replays_ambiguity() {
        let corpus: Value = serde_json::from_str(include_str!(
            "../../../tests/fixtures/protocol/v1/behavior-cases.json"
        ))
        .unwrap();
        for fixture in corpus["fake_effect_cases"].as_array().unwrap() {
            let intent = &fixture["intent"];
            let expected = &fixture["expected"];
            let current = fixture.get("current_authority");
            let mut rejection = None;
            if fixture.get("forged_event").is_some() {
                rejection = Some(UNAUTHORIZED);
            } else if current
                .and_then(|authority| authority["coordinator_generation"].as_str())
                .unwrap_or_else(|| intent["lease"]["coordinator_generation"].as_str().unwrap())
                != intent["lease"]["coordinator_generation"]
            {
                rejection = Some(STALE_COORDINATOR_GENERATION);
            } else if current
                .and_then(|authority| authority["assigned_peer_id"].as_str())
                .unwrap_or_else(|| fixture["assigned_peer_id"].as_str().unwrap())
                != fixture["assigned_peer_id"]
                || current
                    .and_then(|authority| authority.get("lease"))
                    .unwrap_or(&intent["lease"])
                    != &intent["lease"]
            {
                rejection = Some(STALE_LEASE);
            }
            let approval = &fixture["approval"];
            if rejection.is_none() {
                if approval.is_null() {
                    rejection = Some("requires_confirmation");
                } else {
                    let fields = [
                        "action_id",
                        "target_id",
                        "artifact_digest",
                        "grant_revision_id",
                        "policy_revision_id",
                        "action_digest",
                    ];
                    let exact_approval = fields
                        .iter()
                        .all(|field| approval[*field] == intent[*field]);
                    let proposal = &fixture["provider_proposal"];
                    let exact_proposal = ["action_id", "target_id", "artifact_digest"]
                        .iter()
                        .all(|field| proposal[*field] == intent[*field]);
                    let digest = effect_action_digest(
                        intent["action_id"].as_str().unwrap(),
                        intent["target_id"].as_str().unwrap(),
                        intent["artifact_digest"].as_str().unwrap(),
                        intent["grant_revision_id"].as_str().unwrap(),
                        intent["policy_revision_id"].as_str().unwrap(),
                    );
                    if !exact_approval
                        || !exact_proposal
                        || digest.as_deref() != intent["action_digest"].as_str()
                    {
                        rejection = Some("requires_confirmation");
                    }
                }
            }
            if let Some(code) = rejection {
                assert_eq!(expected["invocation_count"], 0, "{}", fixture["id"]);
                assert_eq!(
                    expected["calls"],
                    serde_json::json!([]),
                    "{}",
                    fixture["id"]
                );
                assert_eq!(
                    expected["result"]["status"], "rejected",
                    "{}",
                    fixture["id"]
                );
                assert_eq!(expected["result"]["code"], code, "{}", fixture["id"]);
                continue;
            }
            let call = serde_json::json!({
                "effect_intent_id": intent["effect_intent_id"],
                "action_id": intent["action_id"],
                "target_id": intent["target_id"],
                "artifact_digest": intent["artifact_digest"],
                "grant_revision_id": intent["grant_revision_id"],
                "policy_revision_id": intent["policy_revision_id"],
                "action_digest": intent["action_digest"],
                "lease": intent["lease"],
            });
            assert_eq!(
                expected["calls"],
                serde_json::json!([call]),
                "{}",
                fixture["id"]
            );
            match fixture["fake_tool"]["mode"].as_str().unwrap() {
                "applied" => {
                    let receipt = &fixture["fake_tool"]["receipt"];
                    assert!(effect_receipt_matches_intent(intent, receipt));
                    assert_eq!(expected["invocation_count"], 1);
                    assert_eq!(expected["state"], "applied");
                    assert_eq!(expected["result"]["status"], "accepted");
                    assert_eq!(expected["result"]["receipt"], *receipt);
                    assert_eq!(
                        expected["order"],
                        serde_json::json!([
                            "commit_invoking_event",
                            "call_fake_tool",
                            "commit_applied_receipt"
                        ])
                    );
                }
                "timeout_after_invocation" => {
                    assert_eq!(expected["first_invocation_count"], 1);
                    assert_eq!(expected["first_state"], "unknown_effect");
                    assert_eq!(expected["first_result"]["code"], "unknown_effect");
                    assert_eq!(expected["repeat_result"]["code"], "unknown_effect");
                    assert_eq!(expected["total_invocation_count_after_repeat"], 1);
                }
                mode => panic!("unexpected fake tool mode {mode}"),
            }
        }
    }

    #[test]
    fn effect_approval_digest_matches_frozen_vector() {
        assert_eq!(
            effect_action_digest(
                "workspace.file.write",
                "workspace-alpha",
                &"a".repeat(64),
                "grant-r1",
                "policy-r1"
            )
            .as_deref(),
            Some("8e18f06b26df78ccc4f6d360032cf0da628d534baf37e8742cb97a1c7e014511")
        );
    }
}
