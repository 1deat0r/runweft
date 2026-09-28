import test from 'node:test';
import assert from 'node:assert/strict';
import { createHash } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import {
  actionDigest,
  attemptReceiptMatchesResult,
  canTransition,
  commandIntentEqual,
  effectReceiptMatchesIntent,
  effectReconciliationMatchesOutcome,
  KNOWN_VARIANTS,
  LIFECYCLE_EVENT_ENTITIES,
  lifecycleEntityForEvent,
  taskSuccessReceiptLink,
  transitionEvents,
  validateWireMessage,
} from '../packages/protocol/dist/index.js';

const fixtureRoot = new URL('./fixtures/protocol/v1/', import.meta.url);
const readFixture = async (name) => JSON.parse(await readFile(new URL(name, fixtureRoot), 'utf8'));

function wireFromRawCase(rawCase) {
  if (rawCase.kind === 'literal') return rawCase.input_bytes ? Uint8Array.from(Buffer.from(rawCase.wire, 'utf8')) : rawCase.wire;
  assert.equal(rawCase.kind, 'constructed');
  const { prefix, repeat_byte: byte, repeat_count: count } = rawCase.wire;
  const wire = prefix + byte.repeat(count);
  const bytes = Buffer.from(wire, 'utf8');
  if (rawCase.expected_byte_length !== undefined) assert.equal(bytes.byteLength, rawCase.expected_byte_length, rawCase.id);
  if (rawCase.expected_sha256 !== undefined) assert.equal(createHash('sha256').update(bytes).digest('hex'), rawCase.expected_sha256, rawCase.id);
  return rawCase.input_bytes ? Uint8Array.from(bytes) : wire;
}

function intentProjection(message) {
  return Object.fromEntries([
    'protocol_version', 'kind', 'variant', 'profile_id', 'project_id', 'profile_incarnation', 'payload', 'extensions',
  ].filter((key) => Object.hasOwn(message, key)).map((key) => [key, message[key]]));
}

async function fakeEffectRoundTrip(effectCase) {
  const { intent, approval, provider_proposal: proposal, fake_tool: tool } = effectCase;
  const current = effectCase.current_authority ?? {
    profile_incarnation: intent.lease.profile_incarnation,
    coordinator_generation: intent.lease.coordinator_generation,
    assigned_peer_id: effectCase.assigned_peer_id,
    lease: intent.lease,
  };
  const base = { invocation_count: 0, calls: [], state: 'recorded' };
  const reject = (code) => ({ ...base, result: { status: 'rejected', code } });

  if (effectCase.forged_event) return reject('unauthorized');
  if (current.profile_incarnation !== intent.lease.profile_incarnation) return reject('stale_profile_incarnation');
  if (current.coordinator_generation !== intent.lease.coordinator_generation) return reject('stale_coordinator_generation');
  if (current.assigned_peer_id !== effectCase.assigned_peer_id || JSON.stringify(current.lease) !== JSON.stringify(intent.lease)) return reject('stale_lease');
  if (!approval) return reject('requires_confirmation');
  if (['action_id', 'target_id', 'artifact_digest', 'grant_revision_id', 'policy_revision_id', 'action_digest'].some((key) => approval[key] !== intent[key])) return reject('requires_confirmation');
  if (proposal.action_id !== intent.action_id || proposal.target_id !== intent.target_id || proposal.artifact_digest !== intent.artifact_digest) return reject('requires_confirmation');
  const digest = await actionDigest(intent.action_id, intent.target_id, intent.artifact_digest, intent.grant_revision_id, intent.policy_revision_id);
  if (digest !== intent.action_digest || digest !== approval.action_digest) return reject('requires_confirmation');

  const call = Object.fromEntries(['effect_intent_id', 'action_id', 'target_id', 'artifact_digest', 'grant_revision_id', 'policy_revision_id', 'action_digest', 'lease'].map((key) => [key, intent[key]]));
  const invoked = { invocation_count: 1, calls: [call], state: 'invoking' };
  if (tool.mode === 'timeout_after_invocation') {
    return { first_state: 'unknown_effect', first_invocation_count: 1, first_result: { status: 'rejected', code: 'unknown_effect' }, repeat_result: { status: 'rejected', code: 'unknown_effect' }, total_invocation_count_after_repeat: 1, calls: [call] };
  }
  if (tool.mode === 'applied') {
    return { ...invoked, state: 'applied', result: { status: 'accepted', receipt: tool.receipt }, order: ['commit_invoking_event', 'call_fake_tool', 'commit_applied_receipt'] };
  }
  return reject('internal');
}

test('all 50 literal protocol fixtures match the TypeScript validator', async () => {
  const corpus = await readFixture('cases.json');
  assert.equal(corpus.cases.length, 50);
  for (const item of corpus.cases) {
    if (item.input.kind === 'lifecycle-transition') {
      assert.equal(canTransition(item.input.entity, item.input.from, item.input.to), true, item.id);
      assert.equal(item.expected.transition.kind, 'lifecycle-transition');
      continue;
    }
    assert.deepEqual(await validateWireMessage(JSON.stringify(item.input), item.authority), item.expected, item.id);
  }
  const behaviorCorpus = await readFixture('behavior-cases.json');
  const allVariants = Object.values(KNOWN_VARIANTS).flat();
  assert.deepEqual(Object.keys(corpus.variant_coverage).sort(), [...allVariants].sort());
  for (const [variant, reference] of Object.entries(corpus.variant_coverage)) {
    const [file, caseId] = reference.split('#');
    const source = file === 'cases.json' ? corpus : behaviorCorpus;
    const fixture = source.cases.find((item) => item.id === caseId);
    assert.ok(fixture, `${variant} points to missing ${reference}`);
    assert.equal(fixture.input?.variant, variant);
    assert.equal(fixture.expected.status, 'accepted');
  }
});

test('behavior fixtures preserve trust boundaries and cross-record evidence links', async () => {
  const corpus = await readFixture('behavior-cases.json');
  for (const item of corpus.cases) {
    assert.ok(item.authority);
    assert.deepEqual(await validateWireMessage(JSON.stringify(item.input), item.authority), item.expected, item.id);
  }
  for (const item of corpus.assertions) {
    let actual;
    switch (item.kind) {
      case 'command_intent_equal': actual = commandIntentEqual(item.left, item.right); break;
      case 'can_transition': actual = canTransition(item.entity, item.from, item.to); break;
      case 'last_event_sequence': actual = item.events.at(-1)?.sequence; break;
      case 'attempt_receipt_matches_result': actual = attemptReceiptMatchesResult(item.accepted_result, item.verification_receipt); break;
      case 'effect_receipt_matches_intent': actual = effectReceiptMatchesIntent(item.intent, item.receipt); break;
      case 'task_success_receipt_link': actual = taskSuccessReceiptLink(item.task_success, item.attempt_success); break;
      case 'effect_reconciliation_matches_outcome': actual = effectReconciliationMatchesOutcome(item.payload); break;
      default: assert.fail(`Unknown assertion kind ${item.kind}`);
    }
    assert.equal(actual, item.expected, item.id);
  }
});

test('raw-wire fixtures enforce byte, duplicate-key, nesting and extension limits', async () => {
  const corpus = await readFixture('raw-cases.json');
  for (const item of corpus.cases) {
    assert.deepEqual(await validateWireMessage(wireFromRawCase(item), item.authority), item.expected, item.id);
  }
  const surrogateCase = corpus.cases.find((item) => item.id === 'escaped-lone-surrogate-is-malformed');
  assert.ok(surrogateCase);
  const directSurrogate = surrogateCase.wire.replace('\\ud800', '\ud800');
  assert.deepEqual(await validateWireMessage(directSurrogate, surrogateCase.authority), surrogateCase.expected);
});

test('generated lifecycle allow-list covers every state pair and its event mapping', async () => {
  const lifecycle = JSON.parse(await readFile(new URL('../schemas/lifecycle-transitions.json', import.meta.url), 'utf8'));
  const expected = new Set();
  const expectedEventEntities = {};
  for (const entity of lifecycle.entities) {
    const states = new Set(entity.states);
    for (const edge of entity.transitions) {
      assert.ok(states.has(edge.from));
      assert.ok(states.has(edge.to));
      expected.add(`${entity.name}\0${edge.from}\0${edge.to}`);
      assert.deepEqual(transitionEvents(entity.name, edge.from, edge.to), edge.events);
      for (const event of edge.events) expectedEventEntities[event] = entity.name;
    }
    for (const from of states) for (const to of states) {
      assert.equal(canTransition(entity.name, from, to), expected.has(`${entity.name}\0${from}\0${to}`), `${entity.name}: ${from} -> ${to}`);
    }
  }
  assert.deepEqual(LIFECYCLE_EVENT_ENTITIES, expectedEventEntities);
  for (const [event, entity] of Object.entries(expectedEventEntities)) assert.equal(lifecycleEntityForEvent(event), entity);
  assert.equal(lifecycleEntityForEvent('not-a-lifecycle-event'), undefined);
});

test('fake provider/tool contract model requires exact approval and intent ordering', async () => {
  const corpus = await readFixture('behavior-cases.json');
  for (const item of corpus.fake_effect_cases) {
    const simulation = await fakeEffectRoundTrip(item);
    const { invocation_count, calls, state, result, order, repeat_result, total_invocation_count_after_repeat } = simulation;
    if (item.expected.invocation_count !== undefined) assert.equal(invocation_count, item.expected.invocation_count, item.id);
    if (item.expected.first_invocation_count !== undefined) assert.equal(simulation.first_invocation_count, item.expected.first_invocation_count, item.id);
    assert.deepEqual(calls, item.expected.calls, item.id);
    assert.equal(state ?? simulation.first_state, item.expected.state ?? item.expected.first_state, item.id);
    if (result) assert.deepEqual(result, item.expected.result ?? item.expected.first_result, item.id);
    if (simulation.first_result) assert.deepEqual(simulation.first_result, item.expected.first_result, item.id);
    if (order) assert.deepEqual(order, item.expected.order, item.id);
    if (repeat_result) assert.deepEqual(repeat_result, item.expected.repeat_result, item.id);
    if (total_invocation_count_after_repeat !== undefined) assert.equal(total_invocation_count_after_repeat, item.expected.total_invocation_count_after_repeat, item.id);
  }
});

test('deduplication, atomic commit, generation fencing and effect recovery contract vectors hold', async () => {
  const corpus = await readFixture('behavior-cases.json');
  for (const item of corpus.command_replay_cases) {
    const { record, retry, expected } = item;
    const sameIdentity = ['profile_id', 'project_id', 'profile_incarnation', 'command_id'].every((key) => record.identity[key] === retry[key]);
    const sameIntent = sameIdentity && commandIntentEqual(record.command, retry);
    if (sameIntent) {
      assert.equal(expected.status, 'accepted', item.id);
      assert.deepEqual(expected.result, record.committed_result, item.id);
      assert.equal(expected.event_count_after_retry, record.committed_events.length, item.id);
      assert.equal(expected.duplicate, true, item.id);
    } else {
      assert.equal(expected.status, 'rejected', item.id);
      assert.equal(expected.code, 'conflict', item.id);
      assert.equal(expected.event_count_after_retry, record.committed_events.length, item.id);
    }
  }
  for (const item of corpus.command_atomicity_cases) {
    const committed = item.attempt.commit === 'succeeded';
    assert.equal(item.expected.dedup_record_count, committed ? 1 : 0, item.id);
    assert.equal(item.expected.event_count, committed ? 1 : 0, item.id);
    if (item.expected.response_sent !== undefined) assert.equal(item.expected.response_sent, committed && item.attempt.response_sent === true, item.id);
    if (committed) assert.equal(item.expected.retry_returns_saved_result, true, item.id);
  }
  const u64Max = 18446744073709551615n;
  for (const item of corpus.generation_cases) {
    const last = BigInt(item.last_generation);
    if (last === u64Max) {
      assert.deepEqual(item.expected, { status: 'rejected', code: 'resource_exhausted' }, item.id);
    } else {
      const requested = BigInt(item.requested_generation);
      if (requested <= last) assert.deepEqual(item.expected, { status: 'rejected', code: 'conflict' }, item.id);
      else assert.deepEqual(item.expected, { status: 'accepted', generation: requested.toString() }, item.id);
    }
  }
  for (const item of corpus.effect_recovery_cases) {
    assert.equal(item.durable_state_before_crash, 'invoking', item.id);
    assert.equal(item.terminal_receipt_committed, false, item.id);
    assert.equal(item.expected.recovered_state, 'unknown_effect', item.id);
    assert.deepEqual(item.expected.repeat_result, { status: 'rejected', code: 'unknown_effect' }, item.id);
    assert.equal(item.expected.calls_after_repeat, item.calls_before_recovery, item.id);
  }
});

test('effect approval digest uses the frozen length-prefixed byte preimage', async () => {
  assert.equal(await actionDigest('workspace.file.write', 'workspace-alpha', 'a'.repeat(64), 'grant-r1', 'policy-r1'), '8e18f06b26df78ccc4f6d360032cf0da628d534baf37e8742cb97a1c7e014511');
});
