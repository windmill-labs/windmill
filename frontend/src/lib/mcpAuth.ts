import { SettingService } from '$lib/gen'

/**
 * Whether the instance refuses `?token=` on the MCP endpoints. When it does, an MCP URL is
 * handed over bare and the client reaches it by completing the OAuth flow, so nothing in the
 * UI should offer to mint a token for one.
 *
 * Deliberately uncached: callers read it at the moment an MCP URL is asked for, so a superadmin
 * flipping the setting does not leave open tabs handing out URLs the server now refuses.
 */
export async function mcpTokenUrlDisabled(): Promise<boolean> {
	try {
		return (
			((await SettingService.getGlobal({
				key: 'mcp_disable_token_query_param'
			})) as boolean | null) ?? false
		)
	} catch (err) {
		console.error('Failed to load the MCP token setting:', err)
		return false
	}
}
