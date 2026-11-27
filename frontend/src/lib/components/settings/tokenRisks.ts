import { MCP_SESSION } from '../mcp/McpAccessPicker.svelte'

export const STALE_TOKEN_DAYS = 90

export type TokenRisk = 'never_expires' | 'unused' | 'superadmin' | 'admin'

export interface TokenRiskFields {
	label?: string
	expiration?: string
	last_used_at: string
	scopes?: string[]
	read_only: boolean
	owner_super_admin: boolean
	owner_workspace_admin: boolean
}

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
		tooltip: 'Unscoped token of a superadmin: it carries full control of the instance',
		summary: (n) =>
			n === 1 ? '1 token carries superadmin rights' : `${n} tokens carry superadmin rights`
	},
	admin: {
		badge: 'Admin',
		color: 'orange',
		tooltip: "Unscoped token of a workspace admin: it carries the admin's full rights",
		summary: (n) => (n === 1 ? '1 token carries admin rights' : `${n} tokens carry admin rights`)
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

/** Login sessions expire on their own and are refreshed by logging in, so they are not reviewed. */
function isSession(label: string | undefined): boolean {
	return label === 'session' || label === 'guest_session'
}

/** A token narrowed by scopes or read-only mode does not carry its owner's full rights. */
function isUnrestricted(t: TokenRiskFields): boolean {
	const scopes = (t.scopes ?? []).filter((s) => s !== MCP_SESSION)
	return !t.read_only && scopes.length === 0
}

export function tokenRisks(t: TokenRiskFields, now: Date = new Date()): TokenRisk[] {
	if (isSession(t.label)) return []
	const risks: TokenRisk[] = []
	if (isUnrestricted(t)) {
		if (t.owner_super_admin) risks.push('superadmin')
		else if (t.owner_workspace_admin) risks.push('admin')
	}
	if (!t.expiration) risks.push('never_expires')
	const unusedMs = now.getTime() - new Date(t.last_used_at).getTime()
	if (unusedMs > STALE_TOKEN_DAYS * 86400000) risks.push('unused')
	return risks
}

export function countTokenRisks(tokens: TokenRiskFields[]): Record<TokenRisk, number> {
	const counts: Record<TokenRisk, number> = {
		superadmin: 0,
		admin: 0,
		never_expires: 0,
		unused: 0
	}
	const now = new Date()
	for (const t of tokens) for (const r of tokenRisks(t, now)) counts[r]++
	return counts
}

export function tokenRiskSummary(counts: Record<TokenRisk, number>): string[] {
	return (Object.keys(TOKEN_RISK_INFO) as TokenRisk[])
		.filter((r) => counts[r] > 0)
		.map((r) => TOKEN_RISK_INFO[r].summary(counts[r]))
}

export function riskyTokenCount(tokens: TokenRiskFields[]): number {
	const now = new Date()
	return tokens.filter((t) => tokenRisks(t, now).length > 0).length
}
