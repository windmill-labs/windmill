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

/** Tells the model what the user changed on the card, so its reply matches what was deployed:
 *  it chose none of it. Undefined when nothing changed. */
export function describeDeployEdits(
	edits: DeployEdits | undefined,
	deploy: {
		/** The message the model passed. */
		requestedMessage: string | undefined
		/** The path the model asked to deploy, and where the item landed. */
		path: string
		deployedPath: string
	}
): string | undefined {
	if (!edits) return undefined
	const changed: string[] = []
	const message = edits.deploymentMessage || undefined
	if (edits.deploymentMessage !== undefined && message !== deploy.requestedMessage) {
		changed.push(message ? `the deployment message to "${message}"` : 'the deployment message to none')
	}
	if (edits.deployPath && deploy.deployedPath === edits.deployPath && edits.deployPath !== deploy.path) {
		changed.push(`the path to ${edits.deployPath}`)
	}
	if (edits.appAccess) {
		const who =
			edits.appAccess === 'publisher'
				? 'members only'
				: edits.appAccess === 'guest'
					? 'guests'
					: 'anyone with the link'
		changed.push(`who can open the app to ${who}`)
	}
	if (edits.onBehalfOf) {
		const user = edits.onBehalfOf.permissionedAs ?? edits.onBehalfOf.email
		changed.push(user ? `the user it runs on behalf of to ${user}` : 'it to run as whoever starts each run')
	}
	return changed.length ? `Before confirming, the user changed ${changed.join(', ')}.` : undefined
}
