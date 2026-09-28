import type {
  AttemptStateChangedPayload,
  EffectReconciledPayload,
  TaskStateChangedPayload,
} from './generated.js';

type Assert<T extends true> = T;
type RequiredProperty<T, K extends keyof T> = {} extends Pick<T, K> ? false : true;

// Keep schema nodes with local object properties plus allOf constraints from degrading to `unknown`.
export type GeneratedProtocolTypeAssertions = [
  Assert<RequiredProperty<TaskStateChangedPayload, 'task_id'>>,
  Assert<RequiredProperty<TaskStateChangedPayload, 'from_state'>>,
  Assert<RequiredProperty<AttemptStateChangedPayload, 'attempt_id'>>,
  Assert<RequiredProperty<EffectReconciledPayload, 'effect_intent_id'>>,
  Assert<RequiredProperty<EffectReconciledPayload, 'lease'>>,
];
