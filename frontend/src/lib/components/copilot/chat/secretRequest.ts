import { base } from '$app/paths'
import { ResourceService, VariableService } from '$lib/gen'
import { buildResourcesUrl, buildVariablesUrl } from './global/pageNavigation'
import { withWorkspaceParam } from '$lib/components/sessions/sessionMode.svelte'
import type { SecretRequest, SecretRequestOutcome, SecretSubmission } from './shared'
import {
	connectionParamRefusal,
	DEFAULT_SSLMODE,
	parsePostgresConnectionString
} from '$lib/utils/postgresConnectionString'

/** The resource types whose card can take a connection string, by the secret field it fills. */
export const CONNECTION_STRING_TYPES: Record<string, { field: string; label: string }> = {
	postgresql: { field: 'password', label: 'Password' }
}

/** Splits a pasted connection string into the secret and the fields the card shows, with the
 * same parser as the resource editor, so both store the same resource for the same string. */
export function connectionStringParts(
	connectionString: string
): { secret: string; fields: Record<string, unknown> } | { error: string } {
	const trimmed = connectionString.trim()
	const parts = parsePostgresConnectionString(trimmed)
	if (!parts) return { error: 'This is not a postgres:// connection string.' }
	const refusal = connectionParamRefusal(trimmed)
	if (refusal) return { error: refusal }
	// Saved as a secret variable, which cannot be empty.
	if (!parts.password) return { error: 'It has no password: connect with the details instead.' }
	const { password, ...rest } = parts
	return {
		secret: password,
		fields: Object.fromEntries(
			Object.entries({ ...rest, sslmode: rest.sslmode ?? DEFAULT_SSLMODE }).filter(
				([, v]) => v !== undefined
			)
		)
	}
}

/** The values the card asks for. A bare variable is one field, keyed `""`. */
export function secretFieldsOf(request: SecretRequest): { field: string; label: string }[] {
	return request.resource?.secretFields ?? [{ field: '', label: request.label }]
}

/** Where each secret field is saved, in the order of `fields`. The app-connect form's
 * convention: a lone secret shares its resource's path, several get `<path>_<field>`. */
export function secretVariablePaths(path: string, fields: { field: string }[]): string[] {
	return fields.length === 1 ? [path] : fields.map((f) => `${path}_${f.field}`)
}

/** Saves the secrets as secret variables and, for a resource request, creates the resource
 * referencing them, all deployed rather than drafted: the AI references them right after, and
 * a draft is readable by neither a run nor another resource. What is at `path` is replaced only
 * with `replace`, which the card sets once it shows the path as taken, so a save racing that
 * check never overwrites unannounced. `own` holds the variables an earlier try of this request
 * wrote, which a retry after a refused resource updates rather than calls taken. */
export async function saveSecretRequest(
	request: SecretRequest,
	submission: SecretSubmission,
	own: Set<string>
): Promise<SecretRequestOutcome> {
	const { workspace, resource } = request
	const { path, replace } = submission
	const fields = secretFieldsOf(request)
	const variables = secretVariablePaths(path, fields)
	const variableExists = await Promise.all(
		variables.map((p) => VariableService.existsVariable({ workspace, path: p }))
	)
	const replaced = resource
		? await ResourceService.existsResource({ workspace, path })
		: variableExists[0]
	if (replaced && !replace) throw new Error('This path is already taken.')
	variables.forEach((p, i) => {
		if (variableExists[i] && !replace && !own.has(p)) {
			throw new Error(`A variable already exists at ${p}. Pick another path.`)
		}
	})

	for (const [i, p] of variables.entries()) {
		const value = submission.secrets[fields[i].field] ?? ''
		if (variableExists[i]) {
			await VariableService.updateVariable({
				workspace,
				path: p,
				requestBody: { value, is_secret: true }
			})
		} else {
			await VariableService.createVariable({
				workspace,
				requestBody: {
					path: p,
					value,
					is_secret: true,
					description: resource
						? `${fields[i].label} for ${path}`
						: (request.description ?? request.label)
				}
			})
		}
		own.add(p)
	}

	if (resource) {
		const value = {
			...submission.fields,
			...Object.fromEntries(fields.map((f, i) => [f.field, `$var:${variables[i]}`]))
		}
		if (replaced) {
			await ResourceService.updateResource({
				workspace,
				path,
				// A path the user picked may hold another type; it becomes the one they confirmed.
				requestBody: { value, resource_type: resource.type, description: request.description }
			})
		} else {
			await ResourceService.createResource({
				workspace,
				requestBody: {
					path,
					resource_type: resource.type,
					value,
					description: request.description ?? request.label
				}
			})
		}
	}
	return { path, replaced, variables }
}

/** Whether anything `request_secret` would write is already there, for refusing a taken path
 * before the card shows. */
export async function takenSecretPath(request: SecretRequest): Promise<string | undefined> {
	const { workspace, path, resource } = request
	if (resource && (await ResourceService.existsResource({ workspace, path }))) {
		return `A resource already exists at ${path}`
	}
	for (const p of secretVariablePaths(path, secretFieldsOf(request))) {
		if (await VariableService.existsVariable({ workspace, path: p })) {
			return `A variable already exists at ${p}`
		}
	}
	return undefined
}

// The workspace the secret was saved in, which a fork session does not navigate to.
export function variablePageUrl(path: string, workspace: string): string {
	return withWorkspaceParam(`${base}${buildVariablesUrl({ open: path })}`, workspace)
}

export function resourcePageUrl(path: string, workspace: string): string {
	return withWorkspaceParam(`${base}${buildResourcesUrl({ open: path })}`, workspace)
}

/** What the model reads back. Never a value: only where it now lives. */
export function secretRequestToolResult(
	request: SecretRequest,
	outcome: SecretRequestOutcome | undefined
): string {
	if (!outcome) {
		return JSON.stringify({
			success: false,
			error:
				'The user dismissed the secret request. Do not ask them to paste the value in chat; ask whether to try again later.'
		})
	}
	const moved =
		outcome.path === request.path
			? ''
			: ` The user chose this path instead of ${request.path}: use ${outcome.path} from now on.`
	if (request.resource) {
		return JSON.stringify({
			success: true,
			resource: `$res:${outcome.path}`,
			message: `${outcome.replaced ? 'Updated' : 'Created'} resource ${outcome.path}, deployed, with its secrets in the secret variables ${outcome.variables.join(', ')}.${moved} Reference it as "$res:${outcome.path}". Do not write, open or deploy it: it is done.`
		})
	}
	return JSON.stringify({
		success: true,
		variable: `$var:${outcome.path}`,
		message: `${outcome.replaced ? 'Replaced the value of' : 'Created'} secret variable ${outcome.path}.${moved} It is deployed. Reference it as "$var:${outcome.path}"; you cannot read its value.`
	})
}
