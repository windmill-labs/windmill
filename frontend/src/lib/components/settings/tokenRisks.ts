import type { TokenRisk, TokenRiskCounts } from '$lib/gen'

/** Mirrors the 90 days the `token_risk` view (backend migrations) flags as unused. */
export const STALE_TOKEN_DAYS = 90

/** How each risk the server flags reads, keyed in the order summaries and filters list them. */
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

export function tokenRiskSummary(counts: TokenRiskCounts | undefined): string[] {
	if (!counts) return []
	return TOKEN_RISKS.filter((r) => counts[r] > 0).map((r) => TOKEN_RISK_INFO[r].summary(counts[r]))
}
