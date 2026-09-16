import { SettingService } from '$lib/gen'

let cached: Promise<boolean> | undefined

/**
 * Whether the instance refuses `?token=` on the MCP endpoints. When it does, an MCP URL is
 * handed over bare and the client reaches it by completing the OAuth flow, so nothing in the
 * UI should offer to mint a token for one.
 */
export function mcpTokenUrlDisabled(): Promise<boolean> {
	cached ??= SettingService.getGlobal({ key: 'mcp_disable_token_query_param' })
		.then((v) => (v as boolean | null) ?? false)
		.catch((err) => {
			console.error('Failed to load the MCP token setting:', err)
			// Retry on the next caller rather than pinning the fallback for the session.
			cached = undefined
			return false
		})
	return cached
}
