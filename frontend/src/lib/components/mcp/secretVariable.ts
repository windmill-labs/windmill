import { ResourceService, VariableService } from '$lib/gen'

/**
 * What this connection writes in its token variable's description, and the only
 * proof of ownership available: nothing else records that a variable belongs to
 * an MCP connection, and the path alone proves nothing.
 */
function mcpTokenDescription(resourcePath: string): string {
	return `MCP connection token for ${resourcePath}`
}

/**
 * Store a connection's token at `path`.
 *
 * Deleting an MCP connection deletes the resource but deliberately keeps its
 * token variable, because `delete_resource` cascade-deletes every variable the
 * value references and that credential may still belong to another resource. So
 * reconnecting the same server lands on an existing path, which is the only case
 * this may write over: a variable it did not write is refused, since overwriting
 * one silently retargets every `$var:` reference to it at the same time.
 *
 * An OAuth token cannot be patched in place: `EditVariable` carries no `account`
 * or `is_oauth`, so the variable would go on refreshing through the previous
 * authorization and the old grant would overwrite the token just minted. It has
 * to be recreated — and `delete_variable` takes the resource at the same path
 * with it (`variables.rs`, symmetric with `delete_resource`), which the occupied
 * path check above also covers, since that path is the resource's own.
 */
export async function upsertSecretVariable(args: {
	workspace: string
	path: string
	value: string
	/** The MCP resource this token belongs to; stamped into the description. */
	resourcePath: string
	isOauth?: boolean
	account?: number
}): Promise<void> {
	const { workspace, path, value, resourcePath, isOauth, account } = args
	const description = mcpTokenDescription(resourcePath)

	// The path picker rejects an occupied path, but it validates on a debounce and
	// this runs on the click: without the check, a fast save would rotate the token
	// of the connection already living there and only then fail to create its
	// resource, leaving that server holding a credential meant for another one.
	if (await ResourceService.existsResource({ workspace, path: resourcePath })) {
		throw new Error(`A connection already exists at ${resourcePath}. Pick another path.`)
	}

	if (await VariableService.existsVariable({ workspace, path })) {
		const current = await VariableService.getVariable({ workspace, path, decryptSecret: false })
		if (current.description !== description) {
			throw new Error(`Variable at path ${path} already exists. Delete it or pick another path.`)
		}
		if (!isOauth) {
			// `is_secret` is inherited from the row when omitted, so it is restated
			// rather than assumed.
			await VariableService.updateVariable({
				workspace,
				path,
				requestBody: { value, is_secret: true }
			})
			return
		}
		await VariableService.deleteVariable({ workspace, path })
	}

	await VariableService.createVariable({
		workspace,
		requestBody: { path, value, is_secret: true, is_oauth: isOauth, account, description }
	})
}
