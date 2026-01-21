import { writable, get } from 'svelte/store'
import { WorkspaceService, type ProtectionRuleset, type ProtectionRuleKind } from './gen'
import type { UserExt } from './stores'

/**
 * Store for workspace protection rules state
 */
export const protectionRulesStore = writable<{
	rulesets: ProtectionRuleset[] | undefined
	loading: boolean
	error: string | undefined
}>({
	rulesets: undefined,
	loading: false,
	error: undefined
})

/**
 * Loads protection rules for a workspace from the API and updates the store
 */
export async function loadProtectionRules(workspace: string): Promise<void> {
	protectionRulesStore.update((state) => ({ ...state, loading: true }))

	try {
		const rulesets = await WorkspaceService.listProtectionRules({ workspace })
		protectionRulesStore.set({
			rulesets,
			loading: false,
			error: undefined
		})
	} catch (error) {
		console.error('Failed to load protection rulesets:', error)
		protectionRulesStore.set({
			rulesets: [],
			loading: false,
			error: error instanceof Error ? error.message : 'Unknown error'
		})
	}
}

/**
 * Fetches protection rules for a specific workspace without updating the store
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

	// Check if username in bypass_users (with 'u/' prefix)
	const userWithPrefix = `u/${userInfo.username}`
	if (ruleset.bypass_users.includes(userWithPrefix)) {
		return true
	}

	// Check if any user group in bypass_groups (with 'g/' prefix)
	const userGroupsWithPrefix = userInfo.groups.map((g) => `g/${g}`)
	if (ruleset.bypass_groups.some((bg) => userGroupsWithPrefix.includes(bg))) {
		return true
	}

	return false
}

/**
 * Checks if a specific rule type is active in ANY ruleset
 * @param ruleKind The rule type to check
 * @returns true if the rule is active in at least one ruleset
 */
export function isRuleActive(ruleKind: ProtectionRuleKind): boolean {
	const state = get(protectionRulesStore)
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
		return true
	}

	const state = get(protectionRulesStore)
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
