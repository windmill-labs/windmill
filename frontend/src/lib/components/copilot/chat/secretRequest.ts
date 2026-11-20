import { base } from '$app/paths'
import { VariableService } from '$lib/gen'
import { buildResourcesUrl, buildVariablesUrl } from './global/pageNavigation'
import type { SecretRequest, SecretRequestOutcome } from './shared'

/** Saves the value as a secret variable at `path` (the requested one unless the user chose
 * another), deployed rather than drafted: the AI references it from a resource or a run right
 * after, and a draft is not readable by either. An existing variable at the path keeps its
 * other fields and gets the new value. */
export async function saveSecretVariable(
	request: SecretRequest,
	value: string,
	path: string = request.path
): Promise<SecretRequestOutcome> {
	const { workspace } = request
	const exists = await VariableService.existsVariable({ workspace, path })
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

export function variablePageUrl(path: string): string {
	return `${base}${buildVariablesUrl({ open: path })}`
}

export function resourcePageUrl(path: string): string {
	return `${base}${buildResourcesUrl({ open: path })}`
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
			: ` The user saved it there instead of ${requestedPath}, which already held a variable: use ${outcome.path} from now on, and check a path is free before asking for a secret again.`
	return JSON.stringify({
		success: true,
		variable: `$var:${outcome.path}`,
		message: `${outcome.replaced ? 'Replaced the value of' : 'Created'} secret variable ${outcome.path}.${moved} It is deployed. Reference it as "$var:${outcome.path}"; you cannot read its value. If it goes into a resource, write the resource now without opening it, then call deploy_workspace_item for it if you have that tool: its card asks the user to confirm, so do not ask in text.`
	})
}
