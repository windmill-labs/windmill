import type { OperatorSettings } from '$lib/gen'
import type { UserWorkspace } from '$lib/stores'

/** An `operator_settings` key: each admits an operator to one page (`triggers` to all of them). */
export type OperatorPageKey = keyof NonNullable<OperatorSettings>

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
