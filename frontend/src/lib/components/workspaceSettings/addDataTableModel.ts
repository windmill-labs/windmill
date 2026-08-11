/**
 * Everything the "add a database" wizard collects, and the one function that acts on it.
 *
 * The wizard writes nothing until the user finishes: steps 1 and 2 gather intent, step 3
 * reviews it, and `runSetup` performs it. That ordering is what lets the review step show
 * the resource path before the resource exists, and what lets the data table row be
 * recorded before a billable Supabase project is created -- a run that dies half way
 * leaves something repairable rather than an orphan on someone's bill.
 *
 * `runSetup` is also the retry: every step probes for its own result first, so calling it
 * again on a half-finished data table resumes instead of duplicating.
 */

import {
	ResourceService,
	SettingService,
	VariableService,
	WorkspaceService,
	type DataTableOrigin,
	type TestDataTableConnectionResponse
} from '$lib/gen'
import type { SetupStep } from '../wizards/SetupChecklist.svelte'
import { parsePostgresConnectionString } from '$lib/utils/postgresConnectionString'
import {
	createSupabaseProject,
	generateDbPassword,
	getSupabasePooler,
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
	own: { mode: 'pick' | 'connstr'; resourcePath: string | undefined; connectionString: string }
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
			mode: 'create',
			project: undefined,
			password: '',
			org: undefined,
			region: DEFAULT_SUPABASE_REGION,
			projectName: defaults.projectName,
			connectionMode: 'session'
		},
		instance: { mode: 'create', dbName: undefined },
		own: { mode: 'pick', resourcePath: undefined, connectionString: '' },
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
	return state.own.mode === 'pick'
		? !!state.own.resourcePath
		: !!parsePostgresConnectionString(state.own.connectionString)
}

/**
 * The connection value a branch can be validated against before anything is saved.
 * Undefined for branches with nothing to validate yet: creating a Supabase project has no
 * database to reach, and an instance database does not exist until setup runs.
 */
export function probeValue(state: WizardState): Record<string, any> | undefined {
	if (state.provider === 'resource' && state.own.mode === 'connstr') {
		const parts = parsePostgresConnectionString(state.own.connectionString)
		if (!parts) return undefined
		return { ...parts, sslmode: parts.sslmode ?? 'prefer' }
	}
	return undefined
}

/** What a finished run will have created, for the review step to state plainly. */
export function originOf(state: WizardState, username: string): DataTableOrigin {
	if (state.provider === 'supabase') {
		const created = state.supabase.mode === 'create'
		return {
			provider: 'supabase',
			project_name: created ? state.supabase.projectName.trim() : state.supabase.project?.name,
			project_ref: created ? undefined : projectRef(state.supabase.project!),
			org: created ? state.supabase.org : projectOrg(state.supabase.project!),
			region: created ? state.supabase.region : state.supabase.project?.region,
			connection_mode: state.supabase.connectionMode,
			connected_by: username,
			connected_at: new Date().toISOString()
		}
	}
	return {
		provider: state.provider === 'instance' ? 'instance' : 'resource',
		connected_by: username,
		connected_at: new Date().toISOString()
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
	} else if (state.own.mode === 'connstr') {
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
	username: string
	/** Required for the Supabase branch; a retry re-authorizes to obtain one. */
	supabaseToken?: string
	/** Asked before creating an instance database, which is destructive enough to confirm. */
	confirmInstanceSetup: (dbName: string) => Promise<boolean>
	/** So the settings page's pool reflects a database this run created. */
	onInstanceDbsChanged?: () => Promise<void>
	onProgress: (steps: SetupStep[]) => void
	/** From `derivePlan`, when resuming a data table whose setup never finished. */
	resumeFrom?: SetupStep[]
	/** Called as soon as the data table row exists, so the caller can offer to leave. */
	onRowCreated?: () => void
	onStatus?: (status: string | undefined) => void
}

export type RunResult = {
	ok: boolean
	report?: TestDataTableConnectionResponse
	error?: string
}

async function exists(kind: 'variable' | 'resource', workspace: string, path: string) {
	return kind === 'variable'
		? VariableService.existsVariable({ workspace, path })
		: ResourceService.existsResource({ workspace, path })
}

/**
 * Adds the data table to the workspace config, marked incomplete, unless it is already
 * there. `edit_datatable_config` replaces the whole map, so the rest is read back and sent
 * with it; the backend refuses to take origin or the flag from this call for a data table
 * that already exists, which is what makes calling it on a retry harmless.
 */
async function ensureRow(
	deps: RunDeps,
	name: string,
	database: { resource_type: 'postgresql' | 'instance'; resource_path: string },
	origin: DataTableOrigin
): Promise<void> {
	const settings = await WorkspaceService.getSettings({ workspace: deps.workspace })
	const datatables: Record<string, any> = { ...(settings.datatable?.datatables ?? {}) }
	if (datatables[name]) return
	datatables[name] = { database, origin, setup_incomplete: true }
	await WorkspaceService.editDataTableConfig({
		workspace: deps.workspace,
		requestBody: { settings: { datatables }, renames: [], deleted_datatables: [] }
	})
	deps.onRowCreated?.()
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
 * Every step is safe to re-run: this is both the first attempt and the retry from an
 * incomplete row, and the two must not diverge or the rarely-exercised one rots.
 */
export async function runSetup(state: WizardState, deps: RunDeps): Promise<RunResult> {
	const planned = plan(state)
	// A retry starts from what `derivePlan` already found in place, so it does not re-do
	// work whose result is still there. Anything it did not vouch for runs again, which is
	// safe: every step below upserts rather than assumes absence.
	const steps: SetupStep[] = planned.map((s, i) => ({
		title: s.title,
		status: deps.resumeFrom?.[i]?.status === 'done' ? 'done' : 'pending'
	}))
	let index = 0
	const advance = (status: 'running' | 'done' | 'failed', description?: string) => {
		steps[index] = { ...steps[index], status, description }
		deps.onProgress([...steps])
	}
	const fail = (message: string): RunResult => {
		advance('failed', message)
		return { ok: false, error: message }
	}

	const path = resourcePathOf(state)
	const name = state.review.name.trim()
	const origin = originOf(state, deps.username)
	const instanceName = state.instance.dbName?.trim() ?? ''

	try {
		await ensureRow(
			deps,
			name,
			state.provider === 'instance'
				? { resource_type: 'instance', resource_path: instanceName }
				: {
						resource_type: 'postgresql',
						resource_path: state.own.mode === 'pick' ? (state.own.resourcePath ?? path) : path
					},
			origin
		)
	} catch (err) {
		return { ok: false, error: `Could not record the data table: ${err}` }
	}

	let project = state.supabase.project
	let resourcePath =
		state.provider === 'resource' && state.own.mode === 'pick' ? state.own.resourcePath! : path

	for (; index < planned.length; index++) {
		if (steps[index].status === 'done') continue
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
					project = await createSupabaseProject(deps.supabaseToken!, {
						name: wanted,
						organizationSlug: state.supabase.org!,
						region: state.supabase.region,
						dbPass: password
					})
				}
				await WorkspaceService.setDataTableSetup({
					workspace: deps.workspace,
					datatableName: name,
					requestBody: { origin: { ...origin, project_ref: projectRef(project) } }
				})
			} else if (planned[index].key === 'wait_healthy') {
				project = await waitUntilSupabaseHealthy(
					deps.supabaseToken!,
					projectRef(project!),
					deps.onStatus
				)
			} else if (planned[index].key === 'save_credentials') {
				if (state.provider === 'supabase') {
					if (state.supabase.mode === 'existing')
						await writeSecret(
							deps.workspace,
							path,
							state.supabase.password,
							`Password for the ${project!.name} Supabase database`
						)
					const pooler =
						state.supabase.connectionMode === 'session'
							? await getSupabasePooler(deps.supabaseToken!, projectRef(project!))
							: undefined
					await writeResource(
						deps.workspace,
						path,
						supabaseResourceValue(project!, path, {
							mode: state.supabase.connectionMode,
							pooler
						}),
						`Supabase project ${project!.name}`
					)
				} else {
					const parts = parsePostgresConnectionString(state.own.connectionString)!
					await writeSecret(
						deps.workspace,
						path,
						parts.password ?? '',
						`Password for the ${parts.host} database`
					)
					await writeResource(
						deps.workspace,
						path,
						{
							host: parts.host,
							user: parts.user,
							port: parts.port ?? 5432,
							dbname: parts.dbname ?? 'postgres',
							sslmode: parts.sslmode ?? 'prefer',
							password: `$var:${path}`,
							region: '',
							root_certificate_pem: '',
							use_iam_auth: false
						},
						`Database for the ${name} data table`
					)
				}
			} else if (planned[index].key === 'setup_instance') {
				const status = await SettingService.setupCustomInstanceDb({
					name: instanceName,
					requestBody: { tag: 'datatable' }
				})
				await deps.onInstanceDbsChanged?.()
				if (!status.success) return fail(status.error ?? 'Setup failed')
			} else {
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
					return { ok: false, report }
				}
				advance('done')
				await WorkspaceService.setDataTableSetup({
					workspace: deps.workspace,
					datatableName: name,
					requestBody: { setup_incomplete: false }
				})
				return { ok: true, report }
			}
			advance('done')
		} catch (err: any) {
			return fail(err?.body ?? err?.message ?? String(err))
		}
	}

	return { ok: true }
}

/**
 * Whether a data table that never finished still needs each step, so a retry picks up
 * where it stopped. Derived rather than stored: a stored position is wrong the moment
 * someone repairs something by hand, and this is cheap to ask.
 */
export async function derivePlan(
	state: WizardState,
	deps: { workspace: string; supabaseToken?: string }
): Promise<SetupStep[]> {
	const path = resourcePathOf(state)
	const steps: SetupStep[] = []
	for (const step of plan(state)) {
		let done = false
		if (step.key === 'create_project' && deps.supabaseToken) {
			const wanted = state.supabase.projectName.trim()
			const list = await listSupabaseProjects(deps.supabaseToken)
			done = list.some(
				(p) => p.name === wanted && (!state.supabase.org || projectOrg(p) === state.supabase.org)
			)
		} else if (step.key === 'save_credentials') {
			done = await exists('resource', deps.workspace, path)
		}
		// `wait_healthy`, `setup_instance` and `check` are cheap to repeat and their result
		// is exactly what a retry wants to re-establish, so they are never assumed done.
		steps.push({ title: step.title, status: done ? 'done' : 'pending' })
	}
	return steps
}
