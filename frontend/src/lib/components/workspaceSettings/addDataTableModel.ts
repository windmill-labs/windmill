/**
 * Everything the "add a data table" wizard collects, and the one function that acts on it.
 *
 * The wizard writes nothing until the user finishes: steps 1 and 2 gather intent, step 3
 * reviews it, and `runSetup` performs it. That ordering is what lets the review step show
 * the resource path before the resource exists.
 *
 * `runSetup` is also what Try again calls, so every step has to tolerate the results of a
 * previous attempt still being there.
 */

import {
	ResourceService,
	SettingService,
	VariableService,
	WorkspaceService,
	type TestDataTableConnectionResponse
} from '$lib/gen'
import type { SetupStep } from '../wizards/SetupChecklist.svelte'
import { instanceSetupSteps } from './instanceDbSteps'
import {
	DEFAULT_SSLMODE,
	parsePostgresConnectionString,
	type PostgresConnectionParts
} from '$lib/utils/postgresConnectionString'
import {
	createSupabaseProject,
	generateDbPassword,
	resolveSupabaseConnection,
	listSupabaseProjects,
	projectOrg,
	projectRef,
	supabaseResourceValue,
	waitUntilSupabaseHealthy,
	DEFAULT_SUPABASE_REGION,
	type SupabaseConnectionMode,
	type SupabaseProject
} from './supabaseProvisioning'

export type Provider = 'supabase' | 'instance' | 'resource'

export type WizardState = {
	step: 1 | 2 | 3
	provider: Provider | undefined
	supabase: {
		mode: 'existing' | 'create'
		project: SupabaseProject | undefined
		password: string
		/** Organization slug, for the create mode. */
		org: string | undefined
		region: string
		projectName: string
		connectionMode: SupabaseConnectionMode
	}
	instance: { mode: 'existing' | 'create'; dbName: string | undefined }
	/**
	 * One list: the workspace's Postgres resources, plus the one about to exist. A
	 * connection string is not an alternative to a resource, it is how one is written --
	 * so `creating` and `resourcePath` are the two ways of answering the same question and
	 * are never both set.
	 */
	own: {
		resourcePath: string | undefined
		creating: boolean
		/** Which notation the new resource is being entered in. Same object either way. */
		form: 'string' | 'fields'
		connectionString: string
		fields: PostgresConnectionParts
		/** The resource fields no URI can carry, so they belong to neither notation. */
		advanced: PostgresAdvanced
	}
	review: { name: string; folder: string; resourceName: string }
	/** Result of validating what step 2 collected. Cleared whenever its input changes. */
	probe: {
		checking: boolean
		report: TestDataTableConnectionResponse | undefined
		error: string | undefined
	}
}

export function newWizardState(defaults: {
	name: string
	projectName: string
	folder: string
}): WizardState {
	return {
		step: 1,
		provider: undefined,
		supabase: {
			// Nothing is chosen yet; the step decides between the two once it knows whether the
			// account has any projects. `create` here would be indistinguishable from the user
			// having picked "New project", which is what survives a Back out of the step.
			mode: 'existing',
			project: undefined,
			password: '',
			org: undefined,
			region: DEFAULT_SUPABASE_REGION,
			projectName: defaults.projectName,
			connectionMode: 'session'
		},
		instance: { mode: 'create', dbName: undefined },
		own: {
			resourcePath: undefined,
			creating: false,
			form: 'string',
			connectionString: '',
			fields: emptyFields(),
			advanced: emptyAdvanced()
		},
		review: { name: defaults.name, folder: defaults.folder, resourceName: '' },
		probe: { checking: false, report: undefined, error: undefined }
	}
}

export function clearProbe(state: WizardState) {
	state.probe = { checking: false, report: undefined, error: undefined }
}

/** Path of the resource and secret variable the run will write. They share one. */
export function resourcePathOf(state: WizardState): string {
	return `${state.review.folder}/${state.review.resourceName}`
}

/** True once the branch has everything `runSetup` needs. */
export function intentComplete(state: WizardState): boolean {
	if (state.provider === 'supabase') {
		return state.supabase.mode === 'create'
			? !!state.supabase.projectName.trim() && !!state.supabase.org
			: !!state.supabase.project && !!state.supabase.password
	}
	if (state.provider === 'instance') return !!state.instance.dbName?.trim()
	return state.own.creating ? !!newResourceParts(state) : !!state.own.resourcePath
}

/**
 * The `postgresql` fields outside the connection-string vocabulary: TLS verification and
 * AWS IAM auth. Kept apart from the parts so composing a string cannot appear to drop them.
 */
export type PostgresAdvanced = {
	root_certificate_pem: string
	/**
	 * Undefined is meaningful: the backend then verifies only when a root certificate is
	 * present. Only ever set by an explicit choice.
	 */
	accept_invalid_certs: boolean | undefined
	use_iam_auth: boolean
	region: string
}

function emptyAdvanced(): PostgresAdvanced {
	return {
		root_certificate_pem: '',
		accept_invalid_certs: undefined,
		use_iam_auth: false,
		region: ''
	}
}

/** Whether anything was set, so a notation that cannot show them can say they apply. */
export function hasAdvanced(advanced: PostgresAdvanced): boolean {
	return (
		!!advanced.root_certificate_pem.trim() ||
		advanced.accept_invalid_certs !== undefined ||
		advanced.use_iam_auth ||
		!!advanced.region.trim()
	)
}

function emptyFields(): PostgresConnectionParts {
	return {
		host: '',
		port: 5432,
		dbname: 'postgres',
		user: '',
		password: '',
		sslmode: DEFAULT_SSLMODE
	}
}

const RESERVED_DB_NAMES = ['template0', 'template1', 'postgres']
const VALID_DB_NAME = /^[a-zA-Z][a-zA-Z0-9_-]*$/

/**
 * Why `setup_custom_instance_db` would refuse this name, checked as it is typed rather than
 * at the end of a run that creates a billed project first.
 *
 * Deliberately not exhaustive: the instance may hold databases Windmill did not create, and
 * only the server knows its own database's name. The backend stays the authority; this is
 * here to catch the cases the browser already has the answer to. Empty is not an error --
 * the step is simply incomplete, and shouting at an untouched field is noise.
 */
export function instanceDbNameError(name: string, existing: Iterable<string>): string | undefined {
	const trimmed = name.trim()
	if (!trimmed) return undefined
	if (trimmed.length > 63) return 'A database name cannot exceed 63 characters.'
	if (!VALID_DB_NAME.test(trimmed))
		return 'Start with a letter, then letters, digits, underscores or hyphens only.'
	if (RESERVED_DB_NAMES.includes(trimmed.toLowerCase()))
		return `${trimmed} is a reserved PostgreSQL database name.`
	if (new Set(existing).has(trimmed))
		return `A database called ${trimmed} already exists on this instance.`
	return undefined
}

const VALID_DATATABLE_NAME = /^[a-zA-Z0-9][a-zA-Z0-9_\-.]*$/

/**
 * Why `edit_datatable_config` would refuse this name. Checked as it is typed because the
 * write is the *last* step of the run: by the time the backend rejects it, a Supabase
 * project may have been billed and the resource and secret already written, and the name is
 * no longer editable without closing the wizard.
 *
 * `existing` are the names already in the workspace.
 */
export function datatableNameError(name: string, existing: Iterable<string>): string | undefined {
	const trimmed = name.trim()
	if (!trimmed) return undefined
	if (new Set(existing).has(trimmed))
		return `A data table called ${trimmed} already exists in this workspace.`
	// `validate_datatable_path_segment` runs first on the backend and rejects `..` outright,
	// before the charset check the regex below mirrors.
	if (trimmed.includes('..')) return "A data table name cannot contain '..'."
	if (!VALID_DATATABLE_NAME.test(trimmed))
		return "Start with a letter or digit, then letters, digits, '_', '-' and '.' only — the name has to survive being synced to a git repository."
	return undefined
}

/** What the new resource currently describes, whichever notation it is being written in. */
export function newResourceParts(state: WizardState): PostgresConnectionParts | undefined {
	if (state.own.form === 'string') return parsePostgresConnectionString(state.own.connectionString)
	const fields = state.own.fields
	return fields.host.trim() && fields.user.trim() ? fields : undefined
}

/**
 * Those parts as a `postgresql` resource value -- the one shape everything downstream sees,
 * so nothing after this point knows which notation produced it. The password is the
 * caller's: the literal one when testing before anything is saved, a `$var:` reference once
 * it has somewhere to live.
 */
export function postgresResourceValue(
	parts: PostgresConnectionParts,
	password: string,
	advanced: PostgresAdvanced
): Record<string, any> {
	return {
		host: parts.host,
		user: parts.user,
		port: parts.port ?? 5432,
		dbname: parts.dbname || 'postgres',
		sslmode: parts.sslmode || DEFAULT_SSLMODE,
		password,
		region: advanced.region,
		root_certificate_pem: advanced.root_certificate_pem,
		use_iam_auth: advanced.use_iam_auth,
		// Omitted rather than sent as false: absent is its own state, and the one every
		// resource that predates the flag is in.
		...(advanced.accept_invalid_certs !== undefined
			? { accept_invalid_certs: advanced.accept_invalid_certs }
			: {})
	}
}

/**
 * The connection value a branch can be validated against before anything is saved.
 * Undefined for branches with nothing to validate yet: creating a Supabase project has no
 * database to reach, and an instance database does not exist until setup runs.
 */
export function probeValue(state: WizardState): Record<string, any> | undefined {
	if (state.provider !== 'resource' || !state.own.creating) return undefined
	const parts = newResourceParts(state)
	return parts ? postgresResourceValue(parts, parts.password ?? '', state.own.advanced) : undefined
}

/**
 * Where the Supabase project will live, for the review step to state plainly. Read off
 * the project when it already exists, off what was picked when it is about to be created.
 */
export function supabaseSummary(state: WizardState): { org?: string; region?: string } {
	if (state.supabase.mode === 'create')
		return { org: state.supabase.org, region: state.supabase.region }
	const project = state.supabase.project
	return {
		org: project ? projectOrg(project) : undefined,
		region: project?.region
	}
}

export type RunStepKey =
	| 'create_project'
	| 'wait_healthy'
	| 'save_credentials'
	| 'setup_instance'
	| 'check'

/**
 * The steps this branch will run, in order. The key drives the runner and the title only
 * the display, so rewording a step cannot change what it does.
 */
export function plan(state: WizardState): { key: RunStepKey; title: string }[] {
	const path = resourcePathOf(state)
	const steps: { key: RunStepKey; title: string }[] = []
	if (state.provider === 'supabase') {
		if (state.supabase.mode === 'create') {
			steps.push({
				key: 'create_project',
				title: `Creating ${state.supabase.projectName.trim()} on Supabase`
			})
			steps.push({ key: 'wait_healthy', title: 'Waiting for the database to start' })
		}
		steps.push({ key: 'save_credentials', title: `Saving credentials to ${path}` })
	} else if (state.provider === 'instance') {
		steps.push({
			key: 'setup_instance',
			title: `Setting up ${state.instance.dbName} in the Windmill database`
		})
	} else if (state.own.creating) {
		steps.push({ key: 'save_credentials', title: `Saving the connection to ${path}` })
	}
	steps.push({ key: 'check', title: 'Checking Windmill can store data' })
	return steps
}

/** The same plan as a checklist, all pending. */
export function planSteps(state: WizardState): SetupStep[] {
	return plan(state).map((s) => ({ title: s.title, status: 'pending' }))
}

export type RunDeps = {
	workspace: string
	/** Required for the Supabase branch. */
	supabaseToken?: string
	/** So the settings page's pool reflects a database this run created. */
	onInstanceDbsChanged?: () => Promise<void>
	onProgress: (steps: SetupStep[]) => void
	/** Session pooling was asked for but could not be read; a direct host was written. */
	onPoolerUnavailable?: (reason: string) => void
}

export type RunResult = {
	ok: boolean
	report?: TestDataTableConnectionResponse
	error?: string
	/** The workspace config now holds this data table, whether or not the run went on to pass. */
	rowWritten?: boolean
	/** The secret variable this run wrote, so a retry knows the path is occupied by its own. */
	mintedPath?: string
}

async function exists(kind: 'variable' | 'resource', workspace: string, path: string) {
	return kind === 'variable'
		? VariableService.existsVariable({ workspace, path })
		: ResourceService.existsResource({ workspace, path })
}

/**
 * Adds the data table to the workspace config, once everything it points at exists.
 * `edit_datatable_config` replaces the whole map, so the rest is read back and sent with
 * it. Re-runnable: a second attempt overwrites the entry it wrote.
 */
async function writeRow(
	deps: RunDeps,
	name: string,
	database: { resource_type: 'postgresql' | 'instance'; resource_path: string }
): Promise<void> {
	const settings = await WorkspaceService.getSettings({ workspace: deps.workspace })
	const datatables: Record<string, any> = { ...(settings.datatable?.datatables ?? {}) }
	datatables[name] = { ...(datatables[name] ?? {}), database }
	await WorkspaceService.editDataTableConfig({
		workspace: deps.workspace,
		requestBody: { settings: { datatables }, renames: [], deleted_datatables: [] }
	})
}

async function writeSecret(workspace: string, path: string, value: string, description: string) {
	if (await exists('variable', workspace, path)) {
		await VariableService.updateVariable({
			workspace,
			path,
			requestBody: { value, is_secret: true }
		})
		return
	}
	await VariableService.createVariable({
		workspace,
		requestBody: { path, value, is_secret: true, description, is_oauth: false }
	})
}

async function writeResource(
	workspace: string,
	path: string,
	value: Record<string, any>,
	description: string
) {
	if (await exists('resource', workspace, path)) {
		await ResourceService.updateResource({ workspace, path, requestBody: { value, description } })
		return
	}
	await ResourceService.createResource({
		workspace,
		requestBody: { resource_type: 'postgresql', path, value, description }
	})
}

/**
 * Performs what the wizard collected, reporting each step as it goes.
 *
 * Every step is safe to re-run, because Try again runs the whole plan a second time:
 * each one upserts rather than assuming what it creates is absent.
 */
export async function runSetup(state: WizardState, deps: RunDeps): Promise<RunResult> {
	const planned = plan(state)
	const steps: SetupStep[] = planned.map((s) => ({ title: s.title, status: 'pending' }))
	let index = 0
	const advance = (
		status: 'running' | 'done' | 'failed',
		description?: string,
		substeps?: SetupStep[]
	) => {
		steps[index] = {
			...steps[index],
			status,
			description,
			substeps: substeps ?? steps[index].substeps
		}
		deps.onProgress([...steps])
	}
	let rowWritten = false
	let mintedPath: string | undefined = undefined
	const fail = (message: string): RunResult => {
		advance('failed', message)
		return { ok: false, error: message, rowWritten, mintedPath }
	}

	const path = resourcePathOf(state)
	const name = state.review.name.trim()
	const instanceName = state.instance.dbName?.trim() ?? ''

	let project = state.supabase.project
	let resourcePath =
		state.provider === 'resource' && !state.own.creating ? state.own.resourcePath! : path

	for (; index < planned.length; index++) {
		advance('running')
		try {
			if (planned[index].key === 'create_project') {
				// The password is generated here and can never be read back from Supabase, so it
				// is written to the secret variable before the project that uses it exists. A run
				// that dies right after creation is then still repairable; the reverse order
				// would strand a billed project nobody holds the password to.
				const wanted = state.supabase.projectName.trim()
				const existing = (await listSupabaseProjects(deps.supabaseToken!)).find(
					(p) => p.name === wanted && (!state.supabase.org || projectOrg(p) === state.supabase.org)
				)
				if (existing) {
					if (!(await exists('variable', deps.workspace, path)))
						return fail(
							`A Supabase project called ${wanted} already exists, but Windmill does not hold its password and Supabase cannot return it. Reset the password in Supabase and connect it as an existing project, or delete the project and retry.`
						)
					project = existing
				} else {
					const password = generateDbPassword()
					await writeSecret(
						deps.workspace,
						path,
						password,
						`Password for the ${wanted} Supabase database`
					)
					mintedPath = path
					project = await createSupabaseProject(deps.supabaseToken!, {
						name: wanted,
						organizationSlug: state.supabase.org!,
						region: state.supabase.region,
						dbPass: password
					})
				}
			} else if (planned[index].key === 'wait_healthy') {
				// Minutes of polling with nothing else to show: hang what Supabase reports off the
				// step, so the longest wait in the wizard has something behind its chevron.
				project = await waitUntilSupabaseHealthy(
					deps.supabaseToken!,
					projectRef(project!),
					(status) => advance('running', status)
				)
			} else if (planned[index].key === 'save_credentials') {
				if (state.provider === 'supabase') {
					if (state.supabase.mode === 'existing') {
						await writeSecret(
							deps.workspace,
							path,
							state.supabase.password,
							`Password for the ${project!.name} Supabase database`
						)
						mintedPath = path
					}
					const connection = await resolveSupabaseConnection(
						deps.supabaseToken!,
						project!,
						state.supabase.connectionMode
					)
					if (connection.mode !== state.supabase.connectionMode)
						state.supabase.connectionMode = connection.mode
					if (connection.unavailable) deps.onPoolerUnavailable?.(connection.unavailable)
					await writeResource(
						deps.workspace,
						path,
						supabaseResourceValue(project!, path, connection),
						`Supabase project ${project!.name}`
					)
				} else {
					const parts = newResourceParts(state)!
					await writeSecret(
						deps.workspace,
						path,
						parts.password ?? '',
						`Password for the ${parts.host} database`
					)
					mintedPath = path
					await writeResource(
						deps.workspace,
						path,
						postgresResourceValue(parts, `$var:${path}`, state.own.advanced),
						`Database for the ${name} data table`
					)
				}
			} else if (planned[index].key === 'setup_instance') {
				// The call reports nothing until it returns, so name the checks it is about to run
				// with the first one marked in flight; its answer replaces them when it lands.
				// Otherwise the longest step in the wizard is a single line that sits there.
				advance('running', undefined, instanceSetupSteps(instanceName, undefined, true))
				const status = await SettingService.setupCustomInstanceDb({
					name: instanceName,
					requestBody: { tag: 'datatable' }
				})
				await deps.onInstanceDbsChanged?.()
				const checks = instanceSetupSteps(instanceName, status, false)
				if (!status.success) {
					advance('failed', status.error ?? 'Setup failed', checks)
					return { ok: false, error: status.error ?? 'Setup failed', rowWritten, mintedPath }
				}
				advance('running', undefined, checks)
			} else {
				// The row goes in before the check rather than after it: an instance data table
				// is probed by name, through the very entry being written here.
				await writeRow(
					deps,
					name,
					state.provider === 'instance'
						? { resource_type: 'instance', resource_path: instanceName }
						: { resource_type: 'postgresql', resource_path: resourcePath }
				)
				rowWritten = true
				const report =
					state.provider === 'instance'
						? await WorkspaceService.testDataTableConnection({
								workspace: deps.workspace,
								datatableName: name
							})
						: await WorkspaceService.testDataTableResourceConnection({
								workspace: deps.workspace,
								resourcePath
							})
				if (!report.can_create_table) {
					advance('failed', 'The database is reachable but its user cannot create tables.')
					return { ok: false, report, rowWritten, mintedPath }
				}
				advance('done')
				return { ok: true, report, rowWritten, mintedPath }
			}
			advance('done')
		} catch (err: any) {
			return fail(err?.body ?? err?.message ?? String(err))
		}
	}

	return { ok: true, rowWritten, mintedPath }
}
