import { WorkspaceService, type ProtectionRuleset, type ProtectionRuleKind } from './gen'
import type { UserExt } from './stores'

/**
 * Internal reactive state using Svelte 5 $state rune
 */
let state = $state<{
	rulesets: ProtectionRuleset[] | undefined
	loading: boolean
	error: string | undefined
	workspace: string | undefined
}>({
	rulesets: undefined,
	loading: false,
	error: undefined,
	workspace: undefined
})

/**
 * Exported reactive state object with readonly getters
 */
export const protectionRulesState = {
	get rulesets() {
		return state.rulesets
	},
	get loading() {
		return state.loading
	},
	get error() {
		return state.error
	},
	get workspace() {
		return state.workspace
	}
}

/**
 * Internal function to reset state (used by storeUtils)
 */
export function resetProtectionRules() {
	state.rulesets = undefined
	state.loading = false
	state.error = undefined
	state.workspace = undefined
}

/**
 * Loads protection rules for a workspace from the API and updates the state
 * Early returns if already loading the same workspace to prevent duplicate requests
 */
export async function loadProtectionRules(workspace: string): Promise<void> {
	// Early return if already loading for this workspace
	if (state.loading && state.workspace === workspace) {
		return
	}

	state.loading = true
	state.workspace = workspace

	try {
		const rulesets = await WorkspaceService.listProtectionRules({ workspace })
		state.rulesets = rulesets
		state.loading = false
		state.error = undefined
	} catch (error) {
		console.error('Failed to load protection rulesets:', error)
		// Fail open: set empty array to allow operations
		state.rulesets = []
		state.loading = false
		state.error = error instanceof Error ? error.message : 'Unknown error'
	}
}

/**
 * Fetches protection rules for a specific workspace without updating the state
 * @param workspace The workspace ID to fetch rules for
 * @returns Array of protection rulesets, or empty array on error
 */
export async function fetchProtectionRulesForWorkspace(
	workspace: string
): Promise<ProtectionRuleset[]> {
	try {
		const rulesets = await WorkspaceService.listProtectionRules({ workspace })
		return rulesets
	} catch (error) {
		console.error(`Failed to fetch protection rules for workspace ${workspace}:`, error)
		return []
	}
}

/**
 * Checks if a user can bypass a specific ruleset
 * @param ruleset The protection ruleset to check
 * @param userInfo The user information
 * @returns true if user can bypass (is admin, in bypass_users, or has group in bypass_groups)
 */
export function canUserBypassRule(ruleset: ProtectionRuleset, userInfo: UserExt): boolean {
	// Admin always bypasses
	if (userInfo.is_admin) {
		return true
	}

	if (ruleset.bypass_users.includes(userInfo.username)) {
		return true
	}

	if (ruleset.bypass_groups.some((bg) => userInfo.groups.includes(bg))) {
		return true
	}

	return false
}

/**
 * Checks if a specific rule type is active in ANY ruleset
 * FIXED: No longer uses await without async context
 * @param ruleKind The rule type to check
 * @returns true if the rule is active in at least one ruleset, false if not loaded or not active
 */
export function isRuleActive(ruleKind: ProtectionRuleKind): boolean {
	// Safe default: return false if rules not loaded yet
	if (!state.rulesets) {
		return false
	}

	return state.rulesets.some((ruleset) => ruleset.rules.includes(ruleKind))
}

/**
 * Checks if a user can bypass a specific rule type
 * @param ruleKind The rule type to check
 * @param userInfo The user information
 * @returns true if the rule is not active OR user can bypass ALL rulesets containing it
 */
export function canUserBypassRuleKind(
	ruleKind: ProtectionRuleKind,
	userInfo: UserExt | undefined
): boolean {
	// If no user info, default to permissive
	if (!userInfo) {
		return false
	}

	if (!state.rulesets) {
		return true // No rules loaded, allow
	}

	// Find all rulesets containing this rule
	const rulesetsWithThisRule = state.rulesets.filter((rs) => rs.rules.includes(ruleKind))

	if (rulesetsWithThisRule.length === 0) {
		return true // Rule not active
	}

	// User must be able to bypass ALL rulesets containing this rule
	return rulesetsWithThisRule.every((rs) => canUserBypassRule(rs, userInfo))
}

/**
 * Returns all rulesets that contain a specific rule kind
 */
export function getActiveRulesetsForKind(ruleKind: ProtectionRuleKind): ProtectionRuleset[] {
	if (!state.rulesets) return []
	return state.rulesets.filter((rs) => rs.rules.includes(ruleKind))
}

/**
 * Checks if a specific rule kind is active in given rulesets (workspace-agnostic version)
 * @param rulesets Array of protection rulesets to check
 * @param ruleKind The rule type to check
 * @returns true if the rule is active in at least one ruleset
 */
export function isRuleActiveInRulesets(
	rulesets: ProtectionRuleset[],
	ruleKind: ProtectionRuleKind
): boolean {
	return rulesets.some((ruleset) => ruleset.rules.includes(ruleKind))
}

/**
 * Checks if user can bypass a rule kind in given rulesets (workspace-agnostic version)
 * @param rulesets Array of protection rulesets to check
 * @param ruleKind The rule type to check
 * @param userInfo The user information
 * @returns true if the rule is not active OR user can bypass ALL rulesets containing it
 */
export function canUserBypassRuleKindInRulesets(
	rulesets: ProtectionRuleset[],
	ruleKind: ProtectionRuleKind,
	userInfo: UserExt | undefined
): boolean {
	// If no user info, default to not allowing bypass
	if (!userInfo) {
		return false
	}

	// Find all rulesets containing this rule
	const rulesetsWithThisRule = rulesets.filter((rs) => rs.rules.includes(ruleKind))

	if (rulesetsWithThisRule.length === 0) {
		return true // Rule not active
	}

	// User must be able to bypass ALL rulesets containing this rule
	return rulesetsWithThisRule.every((rs) => canUserBypassRule(rs, userInfo))
}

/**
 * Returns rulesets that contain a specific rule kind from given rulesets (workspace-agnostic version)
 * @param rulesets Array of protection rulesets to filter
 * @param ruleKind The rule type to filter by
 * @returns Array of rulesets containing the specified rule
 */
export function getActiveRulesetsForKindInRulesets(
	rulesets: ProtectionRuleset[],
	ruleKind: ProtectionRuleKind
): ProtectionRuleset[] {
	return rulesets.filter((rs) => rs.rules.includes(ruleKind))
}
