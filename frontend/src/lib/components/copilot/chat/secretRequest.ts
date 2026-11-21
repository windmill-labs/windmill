import { base } from '$app/paths'
import { VariableService } from '$lib/gen'
import { buildResourcesUrl, buildVariablesUrl } from './global/pageNavigation'
import { withWorkspaceParam } from '$lib/components/sessions/sessionMode.svelte'
import type { SecretRequest, SecretRequestOutcome } from './shared'

/** Saves the value as a secret variable at `path`, deployed rather than drafted: the AI
 * references it from a resource or a run right after, and a draft is not readable by either.
 * An existing variable keeps its other fields and gets the new value, but only with `replace`:
 * the card sets it once it shows the path as taken, so a save racing that check never
 * overwrites a secret unannounced. */
export async function saveSecretVariable(
	request: SecretRequest,
	value: string,
	path: string,
	replace: boolean
): Promise<SecretRequestOutcome> {
	const { workspace } = request
	const exists = await VariableService.existsVariable({ workspace, path })
	if (exists && !replace) throw new Error('This path is already taken.')
	if (exists) {
		await VariableService.updateVariable({
			workspace,
			path,
			requestBody: { value, is_secret: true }
		})
	} else {
		await VariableService.createVariable({
			workspace,
			requestBody: {
				path,
				value,
				is_secret: true,
				description: request.description ?? request.label
			}
		})
	}
	return { path, replaced: exists }
}

export function secretVariableExists(workspace: string, path: string): Promise<boolean> {
	return VariableService.existsVariable({ workspace, path })
}

// The workspace the secret was saved in, which a fork session does not navigate to.
export function variablePageUrl(path: string, workspace: string): string {
	return withWorkspaceParam(`${base}${buildVariablesUrl({ open: path })}`, workspace)
}

export function resourcePageUrl(path: string, workspace: string): string {
	return withWorkspaceParam(`${base}${buildResourcesUrl({ open: path })}`, workspace)
}

/** What the model reads back. Never the value: only where it now lives. */
export function secretRequestToolResult(
	outcome: SecretRequestOutcome | undefined,
	requestedPath: string
): string {
	if (!outcome) {
		return JSON.stringify({
			success: false,
			error:
				'The user dismissed the secret request. Do not ask them to paste the value in chat; ask whether to try again later.'
		})
	}
	const moved =
		outcome.path === requestedPath
			? ''
			: ` The user chose this path instead of ${requestedPath}: use ${outcome.path} from now on.`
	return JSON.stringify({
		success: true,
		variable: `$var:${outcome.path}`,
		message: `${outcome.replaced ? 'Replaced the value of' : 'Created'} secret variable ${outcome.path}.${moved} It is deployed. Reference it as "$var:${outcome.path}"; you cannot read its value. If it goes into a resource, write the resource now without opening it, then call deploy_workspace_item for it if you have that tool: its card asks the user to confirm, so do not ask in text.`
	})
}
