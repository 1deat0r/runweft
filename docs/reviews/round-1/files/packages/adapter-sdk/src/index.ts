/** Metadata only. This SDK cannot load plugins, access credentials or invoke tools. */
export type { RuntimeStatus } from '@runweft/protocol';
export interface AdapterDescriptor {
  readonly id: string;
  readonly kind: 'provider';
  readonly implementation: 'placeholder';
  readonly supportsExecution: false;
}
