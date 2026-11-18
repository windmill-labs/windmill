/**
 * Known remote MCP servers, so connecting one is a choice from a list rather
 * than a pasted URL. The entries are the trust boundary as much as the
 * convenience: an MCP server receives the token the resource points at, and its
 * tool descriptions are fed to the model, so reaching a named server should be
 * the normal path and an arbitrary URL the deliberate exception.
 *
 * `auth` records how a server hands out client credentials, which decides which
 * connect flow can be offered:
 *  - `dcr`: the server's authorization server advertises a registration
 *    endpoint, so Windmill can register itself (the MCP OAuth connect).
 *  - `oauth_app`: no dynamic registration; every MCP host has to bring a
 *    pre-registered app, so the connect goes through the instance's configured
 *    OAuth client named by `connectClient`.
 */
import type { Component } from 'svelte'
import GithubIcon from '$lib/components/icons/GithubIcon.svelte'
import LinearIcon from '$lib/components/icons/LinearIcon.svelte'
import NotionIcon from '$lib/components/icons/NotionIcon.svelte'
import SentryIcon from '$lib/components/icons/SentryIcon.svelte'
import StripeIcon from '$lib/components/icons/StripeIcon.svelte'

export type McpAuthKind = 'dcr' | 'oauth_app'

export type McpRegistryEntry = {
	id: string
	name: string
	url: string
	auth: McpAuthKind
	/** Imported per entry rather than through `appIconComponent`, which would pull
	 * the whole icon barrel into the chat bundle for a handful of logos. These take
	 * width/height as css lengths and ignore lucide's `size`, so callers pass both. */
	icon: Component<any>
	/** For `oauth_app`: the Windmill OAuth connect (and resource type) to use. */
	connectClient?: string
	/** What this server takes as a static token. Servers that document OAuth only
	 * are called out rather than left silent: pasting a token there fails at first
	 * use, not at save time. */
	tokenHint?: string
	docsUrl?: string
}

export const MCP_REGISTRY: McpRegistryEntry[] = [
	{
		id: 'github',
		name: 'GitHub',
		icon: GithubIcon,
		url: 'https://api.githubcopilot.com/mcp/',
		auth: 'oauth_app',
		connectClient: 'github',
		tokenHint:
			'Create a personal access token in GitHub settings. repo and read:org cover most tools.',
		docsUrl: 'https://github.com/github/github-mcp-server'
	},
	{
		id: 'notion',
		name: 'Notion',
		icon: NotionIcon,
		url: 'https://mcp.notion.com/mcp',
		auth: 'dcr',
		tokenHint:
			'Notion documents OAuth only for its hosted server, so a static token may be rejected.',
		docsUrl: 'https://developers.notion.com/docs/mcp'
	},
	{
		id: 'linear',
		name: 'Linear',
		icon: LinearIcon,
		url: 'https://mcp.linear.app/mcp',
		auth: 'dcr',
		tokenHint:
			'Use a Linear API key. The Read permission is enough for the read tools.',
		docsUrl: 'https://linear.app/docs/mcp'
	},
	{
		id: 'sentry',
		name: 'Sentry',
		icon: SentryIcon,
		url: 'https://mcp.sentry.dev/mcp',
		auth: 'dcr',
		tokenHint:
			'Sentry documents OAuth only for its hosted server, so a static token may be rejected.',
		docsUrl: 'https://docs.sentry.io/product/sentry-mcp/'
	},
	{
		id: 'stripe',
		name: 'Stripe',
		icon: StripeIcon,
		url: 'https://mcp.stripe.com',
		auth: 'dcr',
		tokenHint:
			'Use a restricted key (rk_...) granting only the permissions you want to give the chat.',
		docsUrl: 'https://docs.stripe.com/mcp'
	}
]

export function findMcpEntry(id: string): McpRegistryEntry | undefined {
	return MCP_REGISTRY.find((e) => e.id === id)
}

function hostnameOf(url: string): string | undefined {
	try {
		return new URL(url).hostname
	} catch {
		return undefined
	}
}

/**
 * The entry a url belongs to, matched on host so a pasted url reaches the same
 * flow as its suggestion. GitHub is the case that matters: its server does not
 * support dynamic registration, so without this a typed url would be offered a
 * discovery that can only fail.
 */
export function findMcpEntryByUrl(url: unknown): McpRegistryEntry | undefined {
	if (typeof url !== 'string') return undefined
	const host = hostnameOf(url)
	return host ? MCP_REGISTRY.find((e) => hostnameOf(e.url) === host) : undefined
}
