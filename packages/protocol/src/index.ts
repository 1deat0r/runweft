import { Ajv2020 } from 'ajv/dist/2020.js';
import * as formats from 'ajv-formats';
import { ERROR_RETRY_POLICY, KNOWN_VARIANTS, MAX_CONTAINER_DEPTH, MAX_RAW_MESSAGE_BYTES, PEER_POLICY, RUN_PROTOCOL_SCHEMA } from './generated-metadata.js';
import { LIFECYCLE_EVENT_ENTITIES, LIFECYCLE_TRANSITIONS, TRANSITION_COUNT, canTransition, lifecycleEntityForEvent, transitionEvents } from './generated-transitions.js';
import { SCAFFOLD_STATUS, type RuntimeStatus } from './generated-status.js';
export * from './generated.js';
export { ERROR_RETRY_POLICY, KNOWN_VARIANTS, LIFECYCLE_EVENT_ENTITIES, LIFECYCLE_TRANSITIONS, MAX_CONTAINER_DEPTH, MAX_RAW_MESSAGE_BYTES, PEER_POLICY, RUN_PROTOCOL_SCHEMA, SCAFFOLD_STATUS, TRANSITION_COUNT, canTransition, lifecycleEntityForEvent, transitionEvents };
export type { RuntimeStatus };

export type ErrorCode = typeof RUN_PROTOCOL_SCHEMA.$defs.ErrorCode.enum[number];
export type PeerRole = 'user' | 'worker' | 'reconciler' | 'coordinator';
export interface AuthenticatedPeer { role: PeerRole; id: string }
export interface CurrentLease {
  profile_id: string;
  project_id: string;
  run_id: string;
  task_id: string;
  attempt_id: string;
  lease_id: string;
  profile_incarnation: string;
  coordinator_generation: string;
  epoch: string;
  assigned_peer_id: string;
}
export interface AuthorityContext {
  peer: AuthenticatedPeer;
  profile_id: string;
  project_id: string;
  profile_incarnation: string;
  coordinator_generation: string;
  current_lease?: CurrentLease | null;
}
export type ValidationResult =
  | { status: 'accepted'; message: unknown }
  | { status: 'rejected'; code: ErrorCode };

type JsonObject = Record<string, unknown>;
type ScanResult = { duplicateKey: boolean; numberToken: boolean };
const encoder = new TextEncoder();
const malform = (): never => { throw new Error('malformed_payload'); };
const resourceExhausted = (): never => { throw new Error('resource_exhausted'); };
const asObject = (value: unknown): JsonObject | undefined => value !== null && typeof value === 'object' && !Array.isArray(value) ? value as JsonObject : undefined;
const asString = (value: unknown): string | undefined => typeof value === 'string' ? value : undefined;

function hasUnpairedSurrogate(value: string): boolean {
  for (let index = 0; index < value.length; index += 1) {
    const code = value.charCodeAt(index);
    if (code >= 0xd800 && code <= 0xdbff) {
      const next = value.charCodeAt(index + 1);
      if (!(next >= 0xdc00 && next <= 0xdfff)) return true;
      index += 1;
    } else if (code >= 0xdc00 && code <= 0xdfff) return true;
  }
  return false;
}

function inspectWireString(value: string): { byteLength: number; hasUnpairedSurrogate: boolean } {
  let byteLength = 0;
  let unpaired = false;
  for (let index = 0; index < value.length; index += 1) {
    const code = value.charCodeAt(index);
    if (code <= 0x7f) byteLength += 1;
    else if (code <= 0x7ff) byteLength += 2;
    else if (code >= 0xd800 && code <= 0xdbff) {
      const next = value.charCodeAt(index + 1);
      if (next >= 0xdc00 && next <= 0xdfff) {
        byteLength += 4;
        index += 1;
      } else {
        byteLength += 3;
        unpaired = true;
      }
    } else if (code >= 0xdc00 && code <= 0xdfff) {
      byteLength += 3;
      unpaired = true;
    } else byteLength += 3;
    if (byteLength > MAX_RAW_MESSAGE_BYTES) return { byteLength, hasUnpairedSurrogate: unpaired };
  }
  return { byteLength, hasUnpairedSurrogate: unpaired };
}

function jsonUnicodeScalarsValid(value: unknown): boolean {
  if (typeof value === 'string') return !hasUnpairedSurrogate(value);
  if (Array.isArray(value)) return value.every(jsonUnicodeScalarsValid);
  const object = asObject(value);
  return object ? Object.entries(object).every(([key, child]) => !hasUnpairedSurrogate(key) && jsonUnicodeScalarsValid(child)) : true;
}

// This pass detects duplicate decoded keys and JSON numeric tokens before serde/Ajv
// can normalize them. Numeric rejection follows version, variant, and schema precedence.
function scanRawJson(raw: string): ScanResult {
  let offset = 0;
  let duplicateKey = false;
  let numberToken = false;
  const skipWhitespace = () => { while (raw[offset] === ' ' || raw[offset] === '\t' || raw[offset] === '\n' || raw[offset] === '\r') offset += 1; };
  const readString = (): string => {
    if (raw[offset] !== '"') return malform();
    const start = offset++;
    while (offset < raw.length) {
      const code = raw.charCodeAt(offset);
      if (code === 0x22) {
        offset += 1;
        try { return JSON.parse(raw.slice(start, offset)) as string; } catch { return malform(); }
      }
      if (code === 0x5c) {
        offset += 2;
      } else {
        offset += 1;
      }
    }
    return malform();
  };
  const numberPattern = /-?(?:0|[1-9][0-9]*)(?:\.[0-9]+)?(?:[eE][+-]?[0-9]+)?/y;
  const parseValue = (parentDepth: number): void => {
    skipWhitespace();
    const token = raw[offset];
    if (token === '{' || token === '[') {
      const depth = parentDepth + 1;
      if (depth > MAX_CONTAINER_DEPTH) resourceExhausted();
      const object = token === '{';
      offset += 1;
      skipWhitespace();
      if (raw[offset] === (object ? '}' : ']')) { offset += 1; return; }
      const keys = new Set<string>();
      while (offset < raw.length) {
        if (object) {
          const key = readString();
          if (keys.has(key)) duplicateKey = true;
          keys.add(key);
          skipWhitespace();
          if (raw[offset] !== ':') return malform();
          offset += 1;
        }
        parseValue(depth);
        skipWhitespace();
        if (raw[offset] === (object ? '}' : ']')) { offset += 1; return; }
        if (raw[offset] !== ',') return malform();
        offset += 1;
        skipWhitespace();
      }
      return malform();
    }
    if (token === '"') { readString(); return; }
    if (token === 't' && raw.startsWith('true', offset)) { offset += 4; return; }
    if (token === 'f' && raw.startsWith('false', offset)) { offset += 5; return; }
    if (token === 'n' && raw.startsWith('null', offset)) { offset += 4; return; }
    numberPattern.lastIndex = offset;
    const number = numberPattern.exec(raw);
    if (number) { numberToken = true; offset = numberPattern.lastIndex; return; }
    return malform();
  };
  parseValue(0);
  skipWhitespace();
  if (offset !== raw.length) malform();
  return { duplicateKey, numberToken };
}

const ajv = new Ajv2020({ allErrors: true, strict: true, strictTypes: false, strictRequired: false, validateFormats: true });
const installFormats = formats.default as unknown as (instance: Ajv2020, selected: string[]) => void;
installFormats(ajv, ['date-time']);
const validateSchema = ajv.compile(RUN_PROTOCOL_SCHEMA as never);

function extensionValuesValid(extensions: unknown): boolean {
  const root = asObject(extensions);
  if (!root) return extensions === undefined;
  const visit = (value: unknown): boolean => {
    if (typeof value === 'string') return encoder.encode(value).byteLength <= 4096;
    if (Array.isArray(value)) return value.every(visit);
    const object = asObject(value);
    if (object) {
      const entries = Object.entries(object);
      if (entries.length > 32) return false;
      return entries.every(([key, child]) => encoder.encode(key).byteLength <= 128 && visit(child));
    }
    return value === null || typeof value === 'boolean';
  };
  if (Object.keys(root).length > 32) return false;
  return Object.entries(root).every(([key, value]) => encoder.encode(key).byteLength <= 128 && visit(value));
}

function numericRangesValid(message: unknown): boolean {
  const u64Max = BigInt('18446744073709551615');
  const u128Max = BigInt('340282366920938463463374607431768211455');
  const visit = (value: unknown, key?: string, root = false): boolean => {
    if (typeof value === 'string' && key) {
      if (['coordinator_generation', 'sequence', 'epoch', 'lease_epoch'].includes(key)) {
        const number = BigInt(value);
        return number > 0n && number <= u64Max;
      }
      if (key === 'requested_budget_micro_usd') return BigInt(value) >= 0n && BigInt(value) <= u128Max;
      return true;
    }
    if (Array.isArray(value)) return value.every((child) => visit(child));
    const object = asObject(value);
    return object ? Object.entries(object).every(([childKey, child]) => {
      if (root && childKey === 'extensions') return true;
      return visit(child, childKey);
    }) : true;
  };
  try { return visit(message, undefined, true); } catch { return false; }
}

function messageVariants(): Record<string, readonly string[]> {
  return KNOWN_VARIANTS as unknown as Record<string, readonly string[]>;
}

function peerRoleAllowed(kind: string, variant: string, role: string): boolean {
  const messages = PEER_POLICY.messages as unknown as Record<string, Record<string, readonly string[]>>;
  const allowed = messages[kind]?.[variant] ?? messages[kind]?.['*'];
  return allowed?.includes(role) ?? false;
}

function authorityError(message: JsonObject, authority: AuthorityContext): ErrorCode | undefined {
  const kind = asString(message.kind);
  const variant = asString(message.variant);
  const peer = authority?.peer;
  if (!kind || !variant || !peer || !peerRoleAllowed(kind, variant, peer.role)) return 'unauthorized';
  if (kind !== 'error') {
    if (message.profile_id !== authority.profile_id || message.project_id !== authority.project_id) return 'unauthorized';
    if (message.profile_incarnation !== authority.profile_incarnation) return 'stale_profile_incarnation';
    if (message.coordinator_generation !== authority.coordinator_generation) return 'stale_coordinator_generation';
  }
  if (kind === 'command' && variant === 'attempt.result.submit') {
    const current = authority.current_lease;
    const payload = asObject(message.payload);
    const claimedLease = asObject(payload?.lease);
    if (!current) return 'stale_lease';
    if (current.assigned_peer_id !== peer.id) return 'unauthorized';
    if (!claimedLease) return 'stale_lease';
    const fields = ['profile_id', 'project_id', 'run_id', 'task_id', 'attempt_id', 'lease_id', 'profile_incarnation', 'coordinator_generation', 'epoch'] as const;
    if (fields.some((key) => claimedLease[key] !== current[key])) return 'stale_lease';
    if (payload?.run_id !== current.run_id || payload.task_id !== current.task_id || payload.attempt_id !== current.attempt_id) return 'stale_lease';
    if (message.profile_id !== current.profile_id || message.project_id !== current.project_id || message.profile_incarnation !== current.profile_incarnation || message.coordinator_generation !== current.coordinator_generation) return 'stale_lease';
  }
  return undefined;
}

function unknownFieldError(errors: readonly { keyword?: string }[] | null | undefined): boolean {
  return errors?.some((error) => error.keyword === 'additionalProperties') ?? false;
}

type SchemaNode = {
  $ref?: string;
  const?: unknown;
  properties?: Record<string, SchemaNode>;
  allOf?: SchemaNode[];
  oneOf?: SchemaNode[];
  anyOf?: SchemaNode[];
  items?: SchemaNode;
  additionalProperties?: boolean | SchemaNode;
};

const schemaRoot = RUN_PROTOCOL_SCHEMA as unknown as { $defs: Record<string, SchemaNode> };
const messageDefinitionByKind: Record<string, string> = {
  command: 'CommandMessage', event: 'EventMessage', response: 'ResponseMessage', error: 'ErrorMessage',
};

function resolveSchema(node: SchemaNode): SchemaNode {
  if (!node.$ref) return node;
  const name = node.$ref.split('/').at(-1);
  return name ? schemaRoot.$defs[name] ?? {} : {};
}

function variantSchema(kind: string, variant: string): SchemaNode | undefined {
  const messageNode = schemaRoot.$defs[messageDefinitionByKind[kind] ?? ''];
  for (const candidateRef of messageNode?.oneOf ?? []) {
    const candidate = resolveSchema(candidateRef);
    const branches = candidate.allOf ?? [candidate];
    if (branches.some((branch) => resolveSchema(branch).properties?.variant?.const === variant)) return candidate;
  }
  return undefined;
}

function hasUnknownProperty(value: unknown, schemaNode: SchemaNode, visited = new Set<string>()): boolean {
  const node = resolveSchema(schemaNode);
  if (schemaNode.$ref) {
    if (visited.has(schemaNode.$ref)) return false;
    visited.add(schemaNode.$ref);
  }
  const object = asObject(value);
  if (object) {
    const properties = node.properties ?? {};
    if (node.additionalProperties === false && Object.keys(object).some((key) => !Object.hasOwn(properties, key))) return true;
    for (const [key, childSchema] of Object.entries(properties)) {
      if (Object.hasOwn(object, key) && hasUnknownProperty(object[key], childSchema, new Set(visited))) return true;
    }
    if (node.additionalProperties && node.additionalProperties !== true) {
      for (const [key, child] of Object.entries(object)) {
        if (!Object.hasOwn(properties, key) && hasUnknownProperty(child, node.additionalProperties, new Set(visited))) return true;
      }
    }
  }
  for (const part of node.allOf ?? []) if (hasUnknownProperty(value, part, new Set(visited))) return true;
  for (const part of [...(node.oneOf ?? []), ...(node.anyOf ?? [])]) if (hasUnknownProperty(value, part, new Set(visited))) return true;
  if (Array.isArray(value) && node.items) return value.some((child) => hasUnknownProperty(child, node.items!, new Set(visited)));
  return false;
}

function stateTransitionError(message: JsonObject): ErrorCode | undefined {
  if (message.kind !== 'event') return undefined;
  const variant = asString(message.variant);
  const entity = variant ? lifecycleEntityForEvent(variant) : undefined;
  if (!entity) return undefined;
  const payload = asObject(message.payload);
  const from = asString(payload?.from_state);
  const to = asString(payload?.to_state);
  if (!from || !to || !canTransition(entity, from, to) || !transitionEvents(entity, from, to)?.includes(variant!)) return 'invalid_state_transition';
  return undefined;
}

async function actionDigestFieldsMatch(fields: JsonObject): Promise<boolean> {
  const action = asString(fields.action_id);
  const target = asString(fields.target_id);
  const artifact = asString(fields.artifact_digest);
  const grant = asString(fields.grant_revision_id);
  const policy = asString(fields.policy_revision_id);
  const digest = asString(fields.action_digest);
  if (!action || !target || !artifact || !grant || !policy || !digest) return true;
  try { return await actionDigest(action, target, artifact, grant, policy) === digest; } catch { return false; }
}

async function digestBindingError(message: JsonObject): Promise<boolean> {
  const kind = asString(message.kind);
  const variant = asString(message.variant);
  const payload = asObject(message.payload);
  return Boolean(payload && ((kind === 'command' && variant === 'effect.approve') || (kind === 'event' && variant === 'effect.intent.recorded')) && !await actionDigestFieldsMatch(payload));
}

export async function validateWireMessage(rawInput: string | Uint8Array, authority: AuthorityContext): Promise<ValidationResult> {
  let raw: string;
  let byteLength: number;
  try {
    if (typeof rawInput === 'string') {
      raw = rawInput;
      const inspected = inspectWireString(raw);
      byteLength = inspected.byteLength;
      if (byteLength > MAX_RAW_MESSAGE_BYTES) return { status: 'rejected', code: 'resource_exhausted' };
      if (inspected.hasUnpairedSurrogate) return { status: 'rejected', code: 'malformed_payload' };
    } else {
      byteLength = rawInput.byteLength;
      if (byteLength > MAX_RAW_MESSAGE_BYTES) return { status: 'rejected', code: 'resource_exhausted' };
      raw = new TextDecoder('utf-8', { fatal: true, ignoreBOM: true }).decode(rawInput);
    }
  } catch { return { status: 'rejected', code: 'malformed_payload' }; }
  if (byteLength > MAX_RAW_MESSAGE_BYTES) return { status: 'rejected', code: 'resource_exhausted' };
  let scanned: ScanResult;
  try { scanned = scanRawJson(raw); }
  catch (error) {
    return { status: 'rejected', code: error instanceof Error && error.message === 'resource_exhausted' ? 'resource_exhausted' : 'malformed_payload' };
  }
  if (scanned.duplicateKey) return { status: 'rejected', code: 'malformed_payload' };
  let messageValue: unknown;
  try { messageValue = JSON.parse(raw) as unknown; } catch { return { status: 'rejected', code: 'malformed_payload' }; }
  if (!jsonUnicodeScalarsValid(messageValue)) return { status: 'rejected', code: 'malformed_payload' };
  const message = asObject(messageValue);
  if (!message) return { status: 'rejected', code: 'malformed_payload' };
  const version = message.protocol_version;
  if (typeof version !== 'string') return { status: 'rejected', code: 'malformed_payload' };
  if (version !== '1.0') return { status: 'rejected', code: 'unsupported_version' };
  const kind = asString(message.kind);
  const variant = asString(message.variant);
  const variants = messageVariants();
  if (!kind || !Object.hasOwn(variants, kind)) return { status: 'rejected', code: 'malformed_payload' };
  if (variant === undefined) return { status: 'rejected', code: 'malformed_payload' };
  if (!variants[kind]!.includes(variant)) return { status: 'rejected', code: 'unknown_critical_variant' };
  const selectedSchema = variantSchema(kind, variant);
  if (!selectedSchema) return { status: 'rejected', code: 'unknown_critical_variant' };
  const hasUnknown = hasUnknownProperty(message, selectedSchema);
  if (!validateSchema(message)) return { status: 'rejected', code: hasUnknown ? 'unknown_field' : 'malformed_payload' };
  if (scanned.numberToken) return { status: 'rejected', code: 'malformed_payload' };
  if (!numericRangesValid(message) || !extensionValuesValid(message.extensions)) return { status: 'rejected', code: 'malformed_payload' };
  const authError = authorityError(message, authority);
  if (authError) return { status: 'rejected', code: authError };
  if (await digestBindingError(message)) return { status: 'rejected', code: 'malformed_payload' };
  const transitionError = stateTransitionError(message);
  if (transitionError) return { status: 'rejected', code: transitionError };
  return { status: 'accepted', message: messageValue };
}

export async function actionDigest(actionId: string, targetId: string, artifactDigest: string, grantRevisionId: string, policyRevisionId: string): Promise<string> {
  if (!/^[a-f0-9]{64}$/.test(artifactDigest)) throw new TypeError('artifactDigest must be lowercase SHA-256 hex');
  const chunks: Uint8Array[] = [encoder.encode('runweft.effect-approval.v1\0')];
  for (const value of [actionId, targetId, grantRevisionId, policyRevisionId]) {
    const bytes = encoder.encode(value);
    if (bytes.byteLength > 0xffff_ffff) throw new RangeError('digest field exceeds u32 length');
    const length = new Uint8Array(4);
    new DataView(length.buffer).setUint32(0, bytes.byteLength, false);
    chunks.push(length, bytes);
  }
  chunks.push(Uint8Array.from(artifactDigest.match(/.{2}/g)!.map((byte) => Number.parseInt(byte, 16))));
  const size = chunks.reduce((total, chunk) => total + chunk.byteLength, 0);
  const preimage = new Uint8Array(size);
  let offset = 0;
  for (const chunk of chunks) { preimage.set(chunk, offset); offset += chunk.byteLength; }
  const digest = new Uint8Array(await globalThis.crypto.subtle.digest('SHA-256', preimage));
  return [...digest].map((byte) => byte.toString(16).padStart(2, '0')).join('');
}

function canonicalJson(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(canonicalJson).join(',')}]`;
  const object = asObject(value);
  if (object) return `{${Object.keys(object).sort().map((key) => `${JSON.stringify(key)}:${canonicalJson(object[key])}`).join(',')}}`;
  return JSON.stringify(value);
}

export function commandIntentProjection(message: unknown): JsonObject {
  const object = asObject(message) ?? {};
  const fields = ['protocol_version', 'kind', 'variant', 'profile_id', 'project_id', 'profile_incarnation', 'payload', 'extensions'];
  return Object.fromEntries(fields.filter((key) => Object.hasOwn(object, key)).map((key) => [key, object[key]]));
}

export function commandIntentEqual(left: unknown, right: unknown): boolean {
  return canonicalJson(commandIntentProjection(left)) === canonicalJson(commandIntentProjection(right));
}

export function attemptReceiptMatchesResult(result: unknown, receipt: unknown): boolean {
  const accepted = asObject(result);
  const verified = asObject(receipt);
  return Boolean(accepted && verified && ['run_id', 'task_id', 'attempt_id', 'artifact_digest'].every((key) => accepted[key] === verified[key]));
}

export function effectReceiptMatchesIntent(intent: unknown, receipt: unknown): boolean {
  const proposed = asObject(intent);
  const recorded = asObject(receipt);
  return Boolean(proposed && recorded && ['effect_intent_id', 'action_digest', 'target_id', 'artifact_digest'].every((key) => proposed[key] === recorded[key]) && canonicalJson(proposed.lease) === canonicalJson(recorded.lease));
}

export function taskSuccessReceiptLink(taskSuccess: unknown, attemptSuccess: unknown): boolean {
  const task = asObject(taskSuccess);
  const attempt = asObject(attemptSuccess);
  const receipt = asObject(attempt?.verification_receipt);
  return Boolean(task && attempt && receipt && attempt.from_state === 'verifying' && attempt.to_state === 'succeeded' && task.task_id === attempt.task_id && task.attempt_id === attempt.attempt_id && task.verification_receipt_id === receipt.receipt_id);
}

export function effectReconciliationMatchesOutcome(payloadValue: unknown): boolean {
  const payload = asObject(payloadValue);
  return Boolean(payload && ((payload.outcome === 'applied' && payload.to_state === 'reconciled_applied') || (payload.outcome === 'not_applied' && payload.to_state === 'reconciled_not_applied')));
}
