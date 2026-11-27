export const STALE_TOKEN_DAYS = 90

export type TokenRisk = 'superadmin' | 'admin' | 'all_workspaces' | 'never_expires' | 'unused'

export interface TokenRiskFields {
	label?: string
	expiration?: string
	last_used_at: string
	scopes?: string[]
	workspace_id?: string
	read_only: boolean
	super_admin: boolean
	workspace_admin: boolean
}

/** Keyed in the order the summaries and filters list them. */
export const TOKEN_RISK_INFO: Record<
	TokenRisk,
	{
		badge: string
		color: 'red' | 'orange' | 'yellow'
		tooltip: string
		summary: (n: number) => string
	}
> = {
	superadmin: {
		badge: 'Superadmin',
		color: 'red',
		tooltip:
			'Minted with superadmin rights and not narrowed by scopes: it carries full control of the instance',
		summary: (n) =>
			n === 1 ? '1 token carries superadmin rights' : `${n} tokens carry superadmin rights`
	},
	admin: {
		badge: 'Admin',
		color: 'orange',
		tooltip:
			'Minted with admin rights and not narrowed by scopes: it carries full control of the workspace',
		summary: (n) => (n === 1 ? '1 token carries admin rights' : `${n} tokens carry admin rights`)
	},
	all_workspaces: {
		badge: 'All workspaces',
		color: 'orange',
		tooltip: 'Not bound to a workspace: it reaches every workspace its owner belongs to',
		summary: (n) =>
			n === 1 ? '1 token works in every workspace' : `${n} tokens work in every workspace`
	},
	never_expires: {
		badge: 'No expiration',
		color: 'orange',
		tooltip: 'Stays valid until it is deleted',
		summary: (n) => (n === 1 ? '1 token never expires' : `${n} tokens never expire`)
	},
	unused: {
		badge: `Unused ${STALE_TOKEN_DAYS}d+`,
		color: 'yellow',
		tooltip: `Not used in over ${STALE_TOKEN_DAYS} days`,
		summary: (n) =>
			n === 1
				? `1 token has not been used in over ${STALE_TOKEN_DAYS} days`
				: `${n} tokens have not been used in over ${STALE_TOKEN_DAYS} days`
	}
}

export const TOKEN_RISKS = Object.keys(TOKEN_RISK_INFO) as TokenRisk[]

/**
 * Tokens the server mints and expires or renews on its own (sessions, job, app, MCP OAuth and
 * impersonation tokens), which nobody manages by hand. The superadmin listing leaves out the
 * same labels in SQL (`list_tokens_as_super_admin`); change both together.
 */
export function isSelfManagedToken(label: string | undefined): boolean {
	if (!label) return false
	return (
		label === 'session' ||
		label === 'guest_session' ||
		label.toLowerCase().startsWith('ephemeral') ||
		label === 'debugger-token' ||
		label.startsWith('mcp-oauth-') ||
		label.startsWith('embed_app:') ||
		label.startsWith('sdk_app:') ||
		label.startsWith('impersonation:')
	)
}

/**
 * A token narrowed by scopes or read-only mode does not carry its owner's full rights. A job tag
 * filter alone does not narrow it: the backend reads such a token as unscoped
 * (`windmill-api-auth/src/scopes.rs`).
 */
function isUnrestricted(t: TokenRiskFields): boolean {
	return !t.read_only && !(t.scopes ?? []).some((s) => !s.startsWith('if_jobs:filter_tags:'))
}

export function tokenRisks(t: TokenRiskFields, now: Date = new Date()): TokenRisk[] {
	if (isSelfManagedToken(t.label)) return []
	const risks: TokenRisk[] = []
	if (isUnrestricted(t)) {
		if (t.super_admin) risks.push('superadmin')
		else if (t.workspace_admin) risks.push('admin')
	}
	if (!t.workspace_id) risks.push('all_workspaces')
	if (!t.expiration) risks.push('never_expires')
	const unusedMs = now.getTime() - new Date(t.last_used_at).getTime()
	if (unusedMs > STALE_TOKEN_DAYS * 86400000) risks.push('unused')
	return risks
}

export function countTokenRisks(tokens: TokenRiskFields[]): Record<TokenRisk, number> {
	const counts = Object.fromEntries(TOKEN_RISKS.map((r) => [r, 0])) as Record<TokenRisk, number>
	const now = new Date()
	for (const t of tokens) for (const r of tokenRisks(t, now)) counts[r]++
	return counts
}

export function tokenRiskSummary(counts: Record<TokenRisk, number>): string[] {
	return TOKEN_RISKS.filter((r) => counts[r] > 0).map((r) => TOKEN_RISK_INFO[r].summary(counts[r]))
}
