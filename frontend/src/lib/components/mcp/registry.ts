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
export type McpAuthKind = 'dcr' | 'oauth_app'

export type McpRegistryEntry = {
	id: string
	name: string
	url: string
	auth: McpAuthKind
	/** For `oauth_app`: the Windmill OAuth connect (and resource type) to use. */
	connectClient?: string
	/** Shown when falling back to entering a token by hand. */
	tokenHint?: string
	docsUrl?: string
}

export const MCP_REGISTRY: McpRegistryEntry[] = [
	{
		id: 'github',
		name: 'GitHub',
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
		url: 'https://mcp.notion.com/mcp',
		auth: 'dcr',
		docsUrl: 'https://developers.notion.com/docs/mcp'
	},
	{
		id: 'linear',
		name: 'Linear',
		url: 'https://mcp.linear.app/mcp',
		auth: 'dcr',
		docsUrl: 'https://linear.app/docs/mcp'
	},
	{
		id: 'sentry',
		name: 'Sentry',
		url: 'https://mcp.sentry.dev/mcp',
		auth: 'dcr',
		docsUrl: 'https://docs.sentry.io/product/sentry-mcp/'
	},
	{
		id: 'stripe',
		name: 'Stripe',
		url: 'https://mcp.stripe.com',
		auth: 'dcr',
		docsUrl: 'https://docs.stripe.com/mcp'
	},
	{
		id: 'paypal',
		name: 'PayPal',
		url: 'https://mcp.paypal.com/mcp',
		auth: 'dcr',
		docsUrl: 'https://developer.paypal.com/tools/mcp-server/'
	}
]

export function findMcpEntry(id: string): McpRegistryEntry | undefined {
	return MCP_REGISTRY.find((e) => e.id === id)
}
