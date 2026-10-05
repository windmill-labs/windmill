import type { OperatorSettings } from '$lib/gen'
import type { UserWorkspace } from '$lib/stores'

/**
 * An `operator_settings` key that admits an operator to one page (`triggers` to all of them).
 *
 * Only the visibility flags: the same object also carries the write rights `builder_flows`,
 * `manage_schedules` and `manage_triggers`, which the `=== true` rule below answers wrongly. A
 * write right is granted unless withdrawn, so its absence means "never configured" rather than
 * "no" — see `docs/operator-write-rights.md`.
 */
export type OperatorPageKey = Extract<
	keyof NonNullable<OperatorSettings>,
	| 'runs'
	| 'schedules'
	| 'resources'
	| 'variables'
	| 'assets'
	| 'audit_logs'
	| 'triggers'
	| 'groups'
	| 'folders'
	| 'workers'
>

/**
 * Whether the sidebar shows the page behind `key`. Everyone but an operator sees every page
 * (other gates, such as admin-only entries, still apply); an operator sees only the pages the
 * workspace's `operator_settings` turn on.
 */
export function sidebarPageAllowed(
	isOperator: boolean | undefined,
	workspace: UserWorkspace | undefined,
	key: OperatorPageKey
): boolean {
	return !isOperator || workspace?.operator_settings?.[key] === true
}

/**
 * Whether the user is an operator in `workspace`. The server nulls
 * `operator_settings` for a non-operator, so a non-null value narrows an operator
 * correctly; the converse does not hold — the column is itself nullable, so a
 * genuine operator whose workspace never had settings written also reads as NULL.
 * Only rely on `true`, never on `false` meaning "not an operator".
 */
export function isOperatorInWorkspace(workspace: UserWorkspace | undefined): boolean {
	return workspace?.operator_settings != null
}
