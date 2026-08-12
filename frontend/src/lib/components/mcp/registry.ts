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
	/** What this server accepts as a static token, when its documentation says.
	 * Servers that document OAuth only are called out rather than left silent:
	 * pasting a token there fails at first use, not at save time. */
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
			'Create a personal access token at github.com/settings/tokens. Grant the permissions you are willing to give the chat — repo and read:org cover most tools; a fine-grained token reaches fewer endpoints than a classic one.',
		docsUrl: 'https://github.com/github/github-mcp-server'
	},
	{
		id: 'notion',
		name: 'Notion',
		icon: NotionIcon,
		url: 'https://mcp.notion.com/mcp',
		auth: 'dcr',
		tokenHint:
			'Notion documents OAuth for its hosted server, so a static integration token may be rejected.',
		docsUrl: 'https://developers.notion.com/docs/mcp'
	},
	{
		id: 'linear',
		name: 'Linear',
		icon: LinearIcon,
		url: 'https://mcp.linear.app/mcp',
		auth: 'dcr',
		tokenHint:
			'Use a Linear API key. A key with only the Read permission is enough for the read tools.',
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
			'Use a restricted API key (rk_...) granting only the permissions you are willing to give the chat.',
		docsUrl: 'https://docs.stripe.com/mcp'
	}
]

export function findMcpEntry(id: string): McpRegistryEntry | undefined {
	return MCP_REGISTRY.find((e) => e.id === id)
}
