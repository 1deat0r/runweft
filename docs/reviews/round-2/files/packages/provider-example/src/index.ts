import type { AdapterDescriptor } from '@runweft/adapter-sdk';
/** No provider SDK, network calls or credential handling are included. */
export const exampleProvider = Object.freeze({
  id: 'example-provider',
  kind: 'provider',
  implementation: 'placeholder',
  supportsExecution: false,
} as const satisfies AdapterDescriptor);
