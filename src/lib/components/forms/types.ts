/**
 * FORM-10: Shared types for the public renderer.
 *
 * The renderer works against the wire-format `FieldSchema` discriminated union
 * defined in `$lib/forms/validate` — re-exported here so every field component
 * imports one canonical symbol.
 *
 * Setter / value types are kept permissive (`unknown`) at the boundary so each
 * typed field component can narrow in its own module without fighting the
 * dispatcher's union exhaustion.
 */

import type {
	FieldSchema,
	ValidationError,
	FormData as FormDataMap
} from '$lib/forms/validate';

export type { FieldSchema, ValidationError, FormDataMap };

/** Callback signature the FormRenderer wires to every FormField child. */
export type FieldSetter = (key: string, value: unknown) => void;

/**
 * Props every leaf field component receives. The data bag is the full map so
 * cross-field features (address sub-parts, country/state chain) can read
 * siblings; writes go through `onChange` so the parent owns mutation.
 */
export interface FieldProps {
	readonly field: FieldSchema;
	readonly value: unknown;
	readonly data: FormDataMap;
	readonly error?: string;
	readonly disabled?: boolean;
	readonly onChange: FieldSetter;
}

/**
 * Logic-engine view surfaced to the renderer. The same evaluator drives
 * `show` / `hide` / `required` on client + server; this type is the
 * client-side projection of the server's `LogicRule` model.
 */
export interface LogicCondition {
	readonly op:
		| 'field_equals'
		| 'field_not_equals'
		| 'field_greater_than'
		| 'field_less_than'
		| 'field_contains'
		| 'and'
		| 'or';
	readonly field?: string;
	readonly value?: unknown;
	readonly conditions?: readonly LogicCondition[];
}

export interface LogicAction {
	readonly kind: 'show' | 'hide' | 'require_field' | 'skip_step' | 'set_value';
	readonly field?: string;
	readonly step?: number;
	readonly value?: unknown;
}

export interface LogicRule {
	readonly condition: LogicCondition;
	readonly action: LogicAction;
}

/** Form-level settings surfaced to the renderer. */
export interface FormSettings {
	readonly success_message?: string;
	readonly submit_label?: string;
	readonly multi_step?: boolean;
	readonly save_and_resume?: boolean;
	readonly honeypot_field?: string;
}
