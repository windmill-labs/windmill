import { get } from 'svelte/store'
import { goto } from '$lib/navigation'
import { page } from '$app/state'
import { userWorkspaces, workspaceStore, userStore } from '$lib/stores'
import { findCanonicalDevWorkspace } from '$lib/utils/workspaceHierarchy'
import {
	fetchProtectionRulesForWorkspace,
	isRuleActiveInRulesets,
	canUserBypassRuleKindInRulesets
} from '$lib/workspaceProtectionRules.svelte'

/**
 * When the current ("prod") workspace is locked against direct deployment for this user and has a
 * canonical dev workspace, redirect the current editor into the dev workspace, where edits are
 * funneled. Admins (who bypass the lock) and unlocked workspaces stay in place, preserving the prod
 * escape hatch. No-op when the URL already targets an explicit `?workspace=` (which also prevents a
 * redirect loop, since the dev target carries that param). Returns true if a redirect was issued.
 *
 * Call from an editor page's `onMount`. It re-targets the same path, so it needs no path/kind args.
 */
export async function maybeRedirectEditToDevWorkspace(): Promise<boolean> {
	if (page.url.searchParams.get('workspace')) return false
	const current = get(workspaceStore)
	const dev = findCanonicalDevWorkspace(current, get(userWorkspaces))
	if (!current || !dev) return false

	const rulesets = await fetchProtectionRulesForWorkspace(current)
	if (!isRuleActiveInRulesets(rulesets, 'DisableDirectDeployment')) return false
	if (canUserBypassRuleKindInRulesets(rulesets, 'DisableDirectDeployment', get(userStore))) {
		return false
	}

	const url = new URL(page.url)
	url.searchParams.set('workspace', dev.id)
	await goto(url.pathname + url.search)
	return true
}
