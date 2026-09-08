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
import { claim, stillOurs, type Claims } from './setupClaims'
import { probeDatatableConnection } from './datatableProbe'
import {
	DEFAULT_SSLMODE,
	parsePostgresConnectionString,
	unsupportedConnectionParam,
	type PostgresConnectionParts
} from '$lib/utils/postgresConnectionString'
import {
	createSupabaseProject,
	generateDbPassword,
	resolveSupabaseConnection,
	listSupabaseProjects,
	projectOrg,
	projectRef,
	orgSlug,
	supabaseResourceValue,
	waitUntilSupabaseHealthy,
	DEFAULT_SUPABASE_REGION,
	type SupabaseConnectionMode,
	type SupabaseOrg,
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
		/**
		 * The whole organization, not its slug: the API is called with the slug, but a slug is a
		 * random string and the review step has a person reading it.
		 */
		org: SupabaseOrg | undefined
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
	if (!state.own.creating) return !!state.own.resourcePath
	// Text that will not parse leaves the fields on their last good values, which is what makes
	// it correctable -- but the connection on screen is then not the one they describe, and
	// testing or saving the old one behind an unparseable string points the data table
	// somewhere nobody asked for.
	if (
		state.own.form === 'string' &&
		(!parsePostgresConnectionString(state.own.connectionString) ||
			unsupportedConnectionParam(state.own.connectionString))
	)
		return false
	return !!newResourceParts(state)
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
 * Why `setup_custom_instance_db` would refuse this name, checked as it is typed. Deliberately
 * not exhaustive -- the backend stays the authority, this only catches what the browser
 * already knows. Empty is incomplete rather than wrong.
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
 * Why `edit_datatable_config` would refuse this name, checked as it is typed because the write
 * is the *last* step of the run: by the time the backend rejects it a Supabase project may
 * have been billed. `existing` are the names already in the workspace.
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

/**
 * What the new resource describes. The fields are the connection; a connection string is a way
 * of writing one down, parsed into the fields as it is typed. Reading it back out here instead
 * would put every gap in the URI grammar between the user and what gets saved.
 */
export function newResourceParts(state: WizardState): PostgresConnectionParts | undefined {
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
		return { org: state.supabase.org?.name, region: state.supabase.region }
	const project = state.supabase.project
	return {
		// The name when the organization is known, its identifier only as a last resort.
		org: state.supabase.org?.name ?? (project ? projectOrg(project) : undefined),
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

/** A Supabase project this session created, and the path holding its only password. */
export type CreatedProject = { name: string; path: string }

export type RunDeps = {
	workspace: string
	/** Required for the Supabase branch. */
	supabaseToken?: string
	/** So the settings page's pool reflects a database this run created. */
	onInstanceDbsChanged?: () => Promise<void>
	onProgress: (steps: SetupStep[]) => void
	/** Session pooling was asked for but could not be read; a direct host was written. */
	onPoolerUnavailable?: (reason: string) => void
	/**
	 * The Supabase project an earlier attempt in this session created. Minting a second password
	 * over the first one's variable would lose the only copy of credentials Supabase will not
	 * repeat, so a run that would do that refuses -- but only once it has seen that the project
	 * is really there, since the name is also recorded when a create could not be confirmed.
	 */
	createdProjects: CreatedProject[]
	/**
	 * What earlier attempts in this session wrote, and this one may therefore write over again.
	 * The pre-flight checks the names are free, but the Supabase branch then spends minutes
	 * provisioning, and every wizard suggests the same `main` -- so a second admin can take the
	 * name or the path in between.
	 */
	claims: Claims
	/** Stands in as the mark where the object was written but its timestamp could not be read back. */
	username: string
}

export type RunResult = {
	ok: boolean
	report?: TestDataTableConnectionResponse
	error?: string
	/**
	 * The workspace config still holds this data table. False when the run never got that far,
	 * and when a refused instance database was taken back out again -- so the name is free and
	 * the caller must not claim it.
	 */
	rowWritten?: boolean
	/** A row this run had written is gone again, so a claim on the name has to go with it. */
	rowRolledBack?: boolean
	/**
	 * Every project created this session, each guarding the path holding its only password.
	 * Supabase never shows that password again, so the variable there is the only copy and no
	 * later attempt may write over it.
	 */
	createdProjects: CreatedProject[]
	/** What this run holds now, for the next attempt to be given back. */
	claims: Claims
}

/**
 * Why a run will not write at a path that already holds a created project's password. Names
 * the path the password is actually at, which is not always the one the wizard is pointing at
 * now -- the review step can be edited after a failure.
 */
function createdSecretRefusal(projectName: string, passwordPath: string): string {
	return `The password of the Supabase project ${projectName}, which this setup created, is stored at ${passwordPath}. Writing here would replace it and Supabase cannot show that password again. Name the project ${projectName} again to carry on with it, or use a different path.`
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
	claims: Claims,
	name: string,
	database: { resource_type: 'postgresql' | 'instance'; resource_path: string }
): Promise<Claims> {
	const settings = await WorkspaceService.getSettings({ workspace: deps.workspace })
	const datatables: Record<string, any> = { ...(settings.datatable?.datatables ?? {}) }
	// Free when the pre-flight looked, taken by the time we write: repointing it here would
	// silently hand another admin's data table a database they never chose.
	if (
		datatables[name] &&
		!stillOurs(claims, 'row', name, datatables[name]?.database?.resource_path)
	) {
		throw new Error(
			`A data table called ${name} was created while this setup was running. Choose another name and try again.`
		)
	}
	datatables[name] = { ...(datatables[name] ?? {}), database }
	await WorkspaceService.editDataTableConfig({
		workspace: deps.workspace,
		requestBody: { settings: { datatables }, renames: [], deleted_datatables: [] }
	})
	return claim(claims, 'row', name, database.resource_path)
}

/**
 * `removed` — the row this run wrote is gone. `kept` — the undo could not reach the server, so
 * it is still there and the caller has to keep saying so. `foreign` — the name now points
 * somewhere this run never wrote, so there is nothing of ours to take back.
 */
type Rollback = 'removed' | 'kept' | 'foreign'

async function removeRow(deps: RunDeps, claims: Claims, name: string): Promise<Rollback> {
	try {
		const settings = await WorkspaceService.getSettings({ workspace: deps.workspace })
		const datatables: Record<string, any> = { ...(settings.datatable?.datatables ?? {}) }
		// Only take back the row this run put there. Between writing it and probing it, another
		// admin can have pointed the same name somewhere else, and deleting that is worse than
		// leaving ours behind.
		if (!stillOurs(claims, 'row', name, datatables[name]?.database?.resource_path)) return 'foreign'
		delete datatables[name]
		// Not `deleted_datatables`: that exists to cascade migration bookkeeping and deployment
		// records for a data table that was really in use, and this one never got that far.
		await WorkspaceService.editDataTableConfig({
			workspace: deps.workspace,
			requestBody: { settings: { datatables }, renames: [], deleted_datatables: [] }
		})
		return 'removed'
	} catch {
		return 'kept'
	}
}

/**
 * The read answers both questions at once: whether anything is there, and who last wrote it.
 * Replacing this run's own work is required for Try again; replacing anyone else's loses a
 * generated Supabase password, which Supabase never shows twice.
 */
async function writeSecret(
	deps: RunDeps,
	claims: Claims,
	path: string,
	value: string,
	description: string
): Promise<Claims> {
	const held = await secretMark(deps, path)
	if (held) {
		if (!stillOurs(claims, 'secret', path, held)) throw new Error(pathTakenLate('variable', path))
		await VariableService.updateVariable({
			workspace: deps.workspace,
			path,
			requestBody: { value, is_secret: true }
		})
	} else {
		await VariableService.createVariable({
			workspace: deps.workspace,
			requestBody: { path, value, is_secret: true, description, is_oauth: false }
		})
	}
	return claim(claims, 'secret', path, (await secretMark(deps, path)) ?? deps.username)
}

/**
 * A revision, not an author: the same person editing the variable in another tab leaves
 * `edited_by` unchanged, and that write is no more ours to discard than a stranger's.
 * `undefined` when nothing is there.
 */
async function secretMark(deps: RunDeps, path: string): Promise<string | undefined> {
	// `decryptSecret` defaults to true, and the handler audit-logs a decryption when it does.
	// Only the timestamp is wanted, and it is on the response either way -- asking for the
	// plaintext records decrypting a secret nothing reads, including someone else's on the
	// retry that is about to refuse it.
	const held = await VariableService.getVariable({
		workspace: deps.workspace,
		path,
		decryptSecret: false
	}).catch(() => undefined)
	return held ? (held.edited_at ?? held.edited_by ?? '') : undefined
}

function pathTakenLate(kind: 'variable' | 'resource', path: string): string {
	return `A ${kind} was created at ${path} while this setup was running. Choose another path and try again.`
}

async function writeResource(
	deps: RunDeps,
	claims: Claims,
	path: string,
	value: Record<string, any>,
	description: string
): Promise<Claims> {
	const held = await resourceMark(deps, path)
	if (held) {
		if (!stillOurs(claims, 'resource', path, held)) throw new Error(pathTakenLate('resource', path))
		await ResourceService.updateResource({
			workspace: deps.workspace,
			path,
			requestBody: { value, description }
		})
	} else {
		await ResourceService.createResource({
			workspace: deps.workspace,
			requestBody: { resource_type: 'postgresql', path, value, description }
		})
	}
	// Read back rather than claim the username: `created_by` survives an update, so it cannot
	// tell an edit by somebody else from no edit at all. `edited_at` moves on every write, which
	// is what makes the next attempt able to see one that happened in between.
	return claim(claims, 'resource', path, (await resourceMark(deps, path)) ?? deps.username)
}

/** `undefined` when nothing is there. */
async function resourceMark(deps: RunDeps, path: string): Promise<string | undefined> {
	const held = await ResourceService.getResource({ workspace: deps.workspace, path }).catch(
		() => undefined
	)
	return held ? (held.edited_at ?? held.created_by ?? '') : undefined
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
	let rowRolledBack = false
	let claims = deps.claims
	let createdProjects: CreatedProject[] = [...deps.createdProjects]
	/** Records a created project once, so a second attempt cannot displace the first one's guard. */
	const rememberProject = (name: string, at: string) => {
		if (!createdProjects.some((p) => p.path === at))
			createdProjects = [...createdProjects, { name, path: at }]
	}
	const fail = (message: string): RunResult => {
		advance('failed', message)
		return {
			ok: false,
			error: message,
			rowWritten,
			rowRolledBack,
			claims,
			createdProjects
		}
	}

	const path = resourcePathOf(state)
	const name = state.review.name.trim()
	/**
	 * An earlier attempt stored a created project's password here. Supabase hands that out once
	 * and every write upserts, so every route back to this path refuses. Each created project
	 * guards its own path -- checking only the latest unlocked the earlier one's password.
	 */
	const guardedHere = deps.createdProjects.find((p) => p.path === path)
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
				const inOrg = (name: string) => (p: SupabaseProject) =>
					p.name === name && (!state.supabase.org || projectOrg(p) === orgSlug(state.supabase.org))
				const projects = await listSupabaseProjects(deps.supabaseToken!)
				const existing = projects.find(inOrg(wanted))
				if (existing) {
					if (!(await exists('variable', deps.workspace, path))) {
						// A project this same session created is the one case where the password is
						// held after all, just not here: the path has been edited since. Saying so
						// beats telling someone to reset or delete a project that is working.
						const elsewhere = deps.createdProjects.find((p) => p.name === wanted)
						if (elsewhere)
							return fail(
								`The password for ${wanted}, which this setup created, is stored at ${elsewhere.path}, not at ${path}. Set the path back to ${elsewhere.path} to carry on with that project.`
							)
						return fail(
							`A Supabase project called ${wanted} already exists, but Windmill does not hold its password and Supabase cannot return it. Reset the password in Supabase and connect it as an existing project, or delete the project and retry.`
						)
					}
					project = existing
				} else {
					// The project has to still exist for its password to be worth protecting: a name
					// recorded from a create that could not be confirmed is a false alarm, and
					// refusing on it leaves the session with nothing it can do. Matched by name
					// across every organization -- a namesake costs a rename, a miss costs the
					// password.
					const earlier = guardedHere?.name
					if (earlier && projects.some((p) => p.name === earlier)) {
						return fail(createdSecretRefusal(earlier, guardedHere!.path))
					}
					const password = generateDbPassword()
					claims = await writeSecret(
						deps,
						claims,
						path,
						password,
						`Password for the ${wanted} Supabase database`
					)
					try {
						project = await createSupabaseProject(deps.supabaseToken!, {
							name: wanted,
							organizationSlug: orgSlug(state.supabase.org!),
							region: state.supabase.region,
							dbPass: password
						})
						// From here the password in `path` is the only copy of a billed project's
						// credentials, and every later write to that path upserts.
						rememberProject(wanted, path)
					} catch (err) {
						// A refusal and a lost response look the same from here, and only one of them
						// bills. Ask Supabase which it was: a project that turned up is ours, holds the
						// password just written, and is what the rest of the run is for. If even that
						// cannot be answered -- an expired token answers nothing -- record the name
						// anyway, and let the next attempt's own lookup decide whether it was real.
						const appeared = await listSupabaseProjects(deps.supabaseToken!).then(
							(after) => after.find(inOrg(wanted)),
							() => {
								rememberProject(wanted, path)
								return undefined
							}
						)
						if (!appeared) throw err
						rememberProject(wanted, path)
						project = appeared
					}
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
						if (guardedHere) return fail(createdSecretRefusal(guardedHere.name, path))
						claims = await writeSecret(
							deps,
							claims,
							path,
							state.supabase.password,
							`Password for the ${project!.name} Supabase database`
						)
					}
					const connection = await resolveSupabaseConnection(
						deps.supabaseToken!,
						project!,
						state.supabase.connectionMode
					)
					if (connection.mode !== state.supabase.connectionMode)
						state.supabase.connectionMode = connection.mode
					if (connection.unavailable) deps.onPoolerUnavailable?.(connection.unavailable)
					claims = await writeResource(
						deps,
						claims,
						path,
						supabaseResourceValue(project!, path, connection),
						`Supabase project ${project!.name}`
					)
				} else {
					if (guardedHere) return fail(createdSecretRefusal(guardedHere.name, path))
					const parts = newResourceParts(state)!
					claims = await writeSecret(
						deps,
						claims,
						path,
						parts.password ?? '',
						`Password for the ${parts.host} database`
					)
					claims = await writeResource(
						deps,
						claims,
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
					return {
						ok: false,
						error: status.error ?? 'Setup failed',
						rowWritten,
						rowRolledBack,
						claims,
						createdProjects
					}
				}
				advance('running', undefined, checks)
			} else if (state.provider === 'instance') {
				// An instance data table is probed by name, through the very entry being written
				// here, so this is the one branch that cannot check first. A database Windmill
				// cannot store data in must not stay in the config, so a refusal takes the row
				// back out -- leaving it would also block retrying under the same name.
				const database = { resource_type: 'instance' as const, resource_path: instanceName }
				claims = await writeRow(deps, claims, name, database)
				rowWritten = true
				const report = await WorkspaceService.testDataTableConnection({
					workspace: deps.workspace,
					datatableName: name
				}).catch(async (err) => {
					// A probe that never answered leaves the same unusable row behind as one that
					// answered no -- an unreachable database or a timeout lands here -- so it takes
					// the same way out rather than the bare outer catch.
					const rollback = await removeRow(deps, claims, name)
					rowRolledBack = rollback === 'removed'
					// `foreign` means the name is somebody else's now: our row is not there to
					// hand back to the collision checks, and a retry must not write over theirs.
					rowWritten = rollback === 'kept'
					throw err
				})
				if (!report.can_create_table) {
					const rollback = await removeRow(deps, claims, name)
					rowRolledBack = rollback === 'removed'
					rowWritten = rollback === 'kept'
					advance('failed', 'The database is reachable but its user cannot create tables.')
					return {
						ok: false,
						report,
						rowWritten,
						rowRolledBack,
						claims,
						createdProjects
					}
				}
				advance('done')
				return {
					ok: true,
					report,
					rowWritten,
					rowRolledBack,
					claims,
					createdProjects
				}
			} else {
				// Checked through the resource, so nothing is written until the database has proved
				// it can hold a data table.
				const report = await probeDatatableConnection(deps.workspace, `$res:${resourcePath}`)
				if (!report.can_create_table) {
					advance('failed', 'The database is reachable but its user cannot create tables.')
					return {
						ok: false,
						report,
						rowWritten,
						rowRolledBack,
						claims,
						createdProjects
					}
				}
				claims = await writeRow(deps, claims, name, {
					resource_type: 'postgresql',
					resource_path: resourcePath
				})
				rowWritten = true
				advance('done')
				return {
					ok: true,
					report,
					rowWritten,
					rowRolledBack,
					claims,
					createdProjects
				}
			}
			advance('done')
		} catch (err: any) {
			return fail(err?.body ?? err?.message ?? String(err))
		}
	}

	return {
		ok: true,
		rowWritten,
		rowRolledBack,
		claims,
		createdProjects
	}
}
