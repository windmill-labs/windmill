import { SettingService, WorkspaceService } from '$lib/gen'

/** Every live workspace of the instance, for a superadmin granting an MCP token their rights. */
export async function listEveryWorkspace(): Promise<{ id: string; name: string }[]> {
	const all: { id: string; name: string; deleted?: boolean }[] = []
	for (let page = 1; ; page++) {
		const batch = await WorkspaceService.listWorkspacesAsSuperAdmin({ page, perPage: 100 })
		all.push(...batch)
		if (batch.length < 100) break
	}
	return all.filter((w) => !w.deleted)
}

/**
 * Whether the instance refuses `?token=` on the MCP endpoints. When it does, an MCP URL is
 * handed over bare and the client reaches it by completing the OAuth flow, so nothing in the
 * UI should offer to mint a token for one.
 *
 * Deliberately uncached: callers read it at the moment an MCP URL is asked for, so a superadmin
 * flipping the setting does not leave open tabs handing out URLs the server now refuses.
 *
 * Throws rather than falling back, so the caller picks the safe default. Guessing `false`
 * here would mint a non-expiring token for a URL the server may refuse.
 */
export async function mcpTokenUrlDisabled(): Promise<boolean> {
	return (
		((await SettingService.getGlobal({
			key: 'mcp_disable_token_query_param'
		})) as boolean | null) ?? false
	)
}
