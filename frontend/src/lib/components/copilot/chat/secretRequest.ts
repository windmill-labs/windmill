import { base } from '$app/paths'
import { DraftService, ResourceService, VariableService } from '$lib/gen'
import { buildResourcesUrl, buildVariablesUrl } from './global/pageNavigation'
import { withWorkspaceParam } from '$lib/components/sessions/sessionMode.svelte'
import type { SecretRequest, SecretRequestOutcome, SecretSubmission } from './shared'
import { persistDraftAt } from './global/userDraftAdapter'
import { canTestUnsaved, testUnsavedResource } from '$lib/components/resourceConnectionTest'
import {
	connectionParamRefusal,
	DEFAULT_SSLMODE,
	parsePostgresConnectionString
} from '$lib/utils/postgresConnectionString'

const ACRONYMS = new Set([
	'api',
	'aws',
	'db',
	'http',
	'https',
	'iam',
	'id',
	'pem',
	'sql',
	'ssl',
	'tls',
	'url',
	'uri'
])

// Keys written as one word, which no casing rule splits.
const WHOLE_KEYS: Record<string, string> = { dbname: 'Database name', sslmode: 'SSL mode' }

/** A resource field's key as a sentence-case label, as the brand guidelines want form labels:
 * `endPoint` reads "End point", `use_ssl` and `useSSL` read "Use SSL". */
export function fieldLabel(key: string): string {
	if (WHOLE_KEYS[key]) return WHOLE_KEYS[key]
	const words = key
		.replace(/([a-z0-9])([A-Z])/g, '$1 $2')
		.replace(/([A-Z]+)([A-Z][a-z])/g, '$1 $2')
		.split(/[\s_-]+/)
		.filter(Boolean)
		.map((w) => (ACRONYMS.has(w.toLowerCase()) ? w.toUpperCase() : w.toLowerCase()))
	const label = words.join(' ')
	return label.charAt(0).toUpperCase() + label.slice(1)
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
	// Required by the postgresql resource type, and a URI may leave it out.
	if (!parts.dbname) return { error: 'It has no database name: add /dbname after the host.' }
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
	return (
		request.resource?.secretFields.map((f) => ({
			...f,
			label: f.label || fieldLabel(f.field)
		})) ?? [{ field: '', label: request.label }]
	)
}

/** Where each secret field is saved, in the order of `fields`. The app-connect form's
 * convention: a lone secret shares its resource's path, several get `<path>_<field>`. */
export function secretVariablePaths(path: string, fields: { field: string }[]): string[] {
	return fields.length === 1 ? [path] : fields.map((f) => `${path}_${f.field}`)
}

/** The save landed but the saved resource does not connect. */
export class SecretCheckFailed extends Error {}

/** The user already has drafts where the card saves, which it replaces only once they confirm. */
export class SecretDraftsInTheWay extends Error {
	constructor(readonly paths: string[]) {
		super(
			`You already have ${paths.length === 1 ? 'a draft' : 'drafts'} at ${paths.join(', ')}. Saving replaces ${paths.length === 1 ? 'it' : 'them'}.`
		)
	}
}

/** Saves one draft. The draft store encrypts a secret variable's value at rest. The caller has
 * checked what draft is there, so it is written over. */
async function saveDraft(
	workspace: string,
	kind: 'variable' | 'resource',
	path: string,
	value: Record<string, unknown>
): Promise<void> {
	const saved = await persistDraftAt(workspace, kind, path, value, { force: true })
	if (saved.status === 'error') throw new Error(saved.message)
}

/** Saves the secrets as drafts of secret variables and, for a resource request, a draft of the
 * resource referencing them, which the session deploys together. Nothing is readable by a run
 * until it is deployed. What is deployed at `path` is replaced only with `replace`, which the
 * card sets once it shows the path as taken, so a save racing that check never overwrites
 * unannounced, and a draft the user has there only with `replaceDrafts`. `own` holds what an earlier try of this request wrote, as `variable:<path>` and
 * `resource:<path>`, which a retry updates rather than calls taken. */
export async function saveSecretRequest(
	request: SecretRequest,
	submission: SecretSubmission,
	own: Set<string>
): Promise<SecretRequestOutcome> {
	const { workspace, resource } = request
	const { path, replace, replaceDrafts } = submission
	const fields = secretFieldsOf(request)
	const variables = secretVariablePaths(path, fields)
	const variableExists = await Promise.all(
		variables.map((p) => VariableService.existsVariable({ workspace, path: p }))
	)
	const replaced = resource
		? await ResourceService.existsResource({ workspace, path })
		: variableExists[0]
	// A resource and a variable can share a path, so what this request wrote is keyed by kind.
	const ownKey = resource ? `resource:${path}` : `variable:${path}`
	if (replaced && !replace && !own.has(ownKey)) throw new Error('This path is already taken.')
	variables.forEach((p, i) => {
		if (variableExists[i] && !replace && !own.has(`variable:${p}`)) {
			throw new Error(`A variable already exists at ${p}. Pick another path.`)
		}
	})
	if (!replaceDrafts) {
		const drafts = new Set(
			(await DraftService.listDrafts({ workspace })).map((d) => `${d.kind}:${d.path}`)
		)
		const inTheWay = [
			...variables.map((p) => `variable:${p}`),
			...(resource ? [ownKey] : [])
		].filter((key) => drafts.has(key) && !own.has(key))
		if (inTheWay.length) {
			throw new SecretDraftsInTheWay(inTheWay.map((key) => key.slice(key.indexOf(':') + 1)))
		}
	}

	for (const [i, p] of variables.entries()) {
		// A replaced variable keeps what describes it and its expiry: deploying the draft
		// overwrites every field.
		const deployed = variableExists[i]
			? await VariableService.getVariable({ workspace, path: p, decryptSecret: false })
			: undefined
		await saveDraft(workspace, 'variable', p, {
			path: p,
			variable: {
				value: submission.secrets[fields[i].field] ?? '',
				is_secret: true,
				description:
					deployed?.description ||
					(resource ? `${fields[i].label} for ${path}` : (request.description ?? request.label))
			},
			...(deployed?.labels ? { labels: deployed.labels } : {}),
			...(deployed?.expires_at ? { expires_at: deployed.expires_at } : {}),
			...(deployed?.value_valid_until ? { value_valid_until: deployed.value_valid_until } : {}),
			wsSpecific: deployed?.ws_specific ?? false
		})
		own.add(`variable:${p}`)
	}

	if (resource) {
		const deployed = replaced ? await ResourceService.getResource({ workspace, path }) : undefined
		await saveDraft(workspace, 'resource', path, {
			path,
			description: request.description || deployed?.description || request.label,
			args: {
				...submission.fields,
				...Object.fromEntries(fields.map((f, i) => [f.field, `$var:${variables[i]}`]))
			},
			...(deployed?.labels ? { labels: deployed.labels } : {}),
			wsSpecific: deployed?.ws_specific ?? false,
			// A path the user picked may hold another type; it becomes the one they confirmed.
			resource_type: resource.type
		})
	}
	own.add(ownKey)
	return { path, replaced, variables }
}

/** Tests the values the user confirmed, where the test writes nothing and needs nothing
 * deployed: a storage test or an AI key. A database test runs as a job, which reads only
 * deployed variables, so it waits for the deploy. */
export async function checkSubmission(
	request: SecretRequest,
	submission: SecretSubmission
): Promise<SecretRequestOutcome['check']> {
	const { resource, workspace } = request
	if (!resource || !canTestUnsaved(resource.type, submission.fields)) return undefined
	return await testUnsavedResource({
		workspace,
		resourceType: resource.type,
		value: { ...submission.fields, ...submission.secrets }
	})
}

// The workspace the secret was saved in, which a fork session does not navigate to.
export function variablePageUrl(path: string, workspace: string): string {
	return withWorkspaceParam(`${base}${buildVariablesUrl({ open: path })}`, workspace)
}

export function resourcePageUrl(path: string, workspace: string): string {
	return withWorkspaceParam(`${base}${buildResourcesUrl({ open: path })}`, workspace)
}

// The database's or provider's own words, which carry no secret, so the model can diagnose them.
export function checkText(outcome: SecretRequestOutcome): string {
	if (!outcome.check) return ''
	if (outcome.check.ok) return ' Its connection test passed.'
	return ` Its connection test failed and the user kept it anyway: ${outcome.check.error.slice(0, 500)}. Tell the user the likely cause and how to fix it at their provider, without opening the resource: to fix it, call request_secret again on the same path with replace: true.`
}

// Every run reads only deployed items, the card's own connection test included.
function notDeployed(resource: boolean): string {
	const them = resource ? 'them' : 'it'
	return `Nothing is deployed: no run, test run or connection test can read ${them} until deployed. Before running anything that uses ${them}, offer to deploy ${resource ? 'the variables with the resource' : 'it'}; when the user only wanted ${them} saved, end by offering it.`
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
			message: `The user typed the secrets on the card and saved it, so nothing is left for them to fill in. ${outcome.replaced ? 'The update of resource' : 'Resource'} ${outcome.path} is saved as a draft, its secrets as drafts of the secret variables ${outcome.variables.join(', ')}.${moved}${checkText(outcome)} ${notDeployed(true)} Reference it as "$res:${outcome.path}". Do not write or open it yourself.`
		})
	}
	return JSON.stringify({
		success: true,
		variable: `$var:${outcome.path}`,
		message: `The user typed the secret on the card and saved it, so nothing is left for them to fill in. ${outcome.replaced ? 'The new value of secret variable' : 'Secret variable'} ${outcome.path} is saved as a draft.${moved} ${notDeployed(false)} Reference it as "$var:${outcome.path}"; you cannot read its value.`
	})
}
