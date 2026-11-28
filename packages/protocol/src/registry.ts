// Typed access to the wire format field registry (ADR-VJGJ).
import { readFileSync } from 'node:fs';

export interface RegistryField {
  key: string;
  name: string;
  /** Property name in the decoded form, or null when the decoder does not expose it. */
  decoded: string | null;
  scope: 'header' | 'event';
  /** Event types (`y`) the field is used with; null for header fields. */
  types: 'all' | number[] | null;
  type: 'string' | 'int' | 'decimal' | 'list';
  unit: 'ms' | 'percent' | 'min' | null;
  required: boolean;
  since: string;
  example: string;
  /** Present on the property family `p.<name>`. */
  family?: boolean;
  exampleName?: string;
  description: string;
}

export interface RegistryEventType {
  y: number;
  name: string;
  status: 'active' | 'reserved' | 'retired';
  since: string;
  description: string;
}

export interface RegistryQueryParameter {
  key: string;
  name: string;
  type: 'int';
  unit: string | null;
  min: number;
  max: number;
  since: string;
  example: string;
  description: string;
}

export interface Registry {
  protocol: number;
  revision: string;
  eventTypes: RegistryEventType[];
  fields: RegistryField[];
  /** Parameters of the collect request URL; not keys of the body (revision 1.4). */
  queryParameters: RegistryQueryParameter[];
}

export function loadRegistry(): Registry {
  const url = new URL('../../../contracts/protocol/fields.json', import.meta.url);
  return JSON.parse(readFileSync(url, 'utf8')) as Registry;
}
