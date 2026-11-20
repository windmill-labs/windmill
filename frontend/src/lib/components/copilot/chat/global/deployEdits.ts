/** The user an item runs on behalf of; both unset runs it as whoever starts each run. */
export type OnBehalfOf = { email: string | undefined; permissionedAs: string | undefined }

/** What the user changed on a pending deploy's card before confirming it. */
export type DeployEdits = {
	deploymentMessage?: string
	/** Set only when the user picked a different user than the draft holds. */
	onBehalfOf?: OnBehalfOf
	/** A new app's access, set only when the user picked another than the draft's. */
	appAccess?: 'publisher' | 'guest' | 'anonymous'
	/** Scripts, flows and apps never deployed: the path to create the item at, when the user
	 *  picked another than the draft's. */
	deployPath?: string
}

// Keyed by tool call id. The card's edits stay out of the tool's arguments: those are the model's
// schema, which the MCP server serves too, and the model never chooses who an item runs as.
const pending = new Map<string, DeployEdits>()

/** Called by the deploy card right before it confirms the call. */
export function setDeployEdits(toolCallId: string, edits: DeployEdits): void {
	pending.set(toolCallId, edits)
}

/** Read once by the deploy, so a declined or failed call leaves nothing behind for a retry. */
export function takeDeployEdits(toolCallId: string): DeployEdits | undefined {
	const edits = pending.get(toolCallId)
	pending.delete(toolCallId)
	return edits
}
