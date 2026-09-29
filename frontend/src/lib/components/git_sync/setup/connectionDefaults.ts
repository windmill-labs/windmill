import type { GitSyncRepository } from '../GitSyncContext.svelte'

/**
 * Defaults for a connection that has not been saved yet, once its resource is known.
 * Only fills what the user has not set, so it is safe to call again.
 */
export function applyNewConnectionDefaults(
	repo: GitSyncRepository,
	o: {
		mode: 'sync' | 'promotion'
		/** Windmill holds a credential for the repository: a GitHub App or a stored token. */
		managedCredential: boolean
		isGithubApp: boolean
		isFork: boolean
		ee: boolean
	}
): void {
	if (!repo.isUnsavedConnection) return
	// Pulling from Git defaults on only where Windmill can receive webhooks. Polling is
	// opt-in for token repositories, and forks never get the parent-only defaults (the
	// backend rejects them).
	if (
		o.mode === 'sync' &&
		o.managedCredential &&
		!o.isFork &&
		o.ee &&
		repo.auto_pull === undefined
	) {
		repo.auto_pull = { enabled: true, mode: 'auto', sync_forks: true }
	}
	// A promotion deploy pushes a wm_deploy/** branch that exists to be merged; without a
	// pull request it is an orphaned branch. Fork PRs stay opt-in everywhere.
	if (
		o.mode === 'promotion' &&
		o.managedCredential &&
		o.ee &&
		repo.promotion_open_prs === undefined
	) {
		repo.promotion_open_prs = true
	}
	// Webhook with a polling fallback is the only delivery for app repositories.
	if (o.isGithubApp && repo.auto_pull?.mode === 'polling') {
		repo.auto_pull = { ...repo.auto_pull, mode: 'auto' }
	}
}
