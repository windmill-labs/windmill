import { beforeEach, describe, expect, it, vi } from 'vitest'

const listSupabaseProjectsMock = vi.fn()
const createSupabaseProjectMock = vi.fn()
vi.mock('./supabaseProvisioning', async (importOriginal) => ({
	...(await importOriginal<typeof import('./supabaseProvisioning')>()),
	listSupabaseProjects: (...a: any[]) => listSupabaseProjectsMock(...a),
	createSupabaseProject: (...a: any[]) => createSupabaseProjectMock(...a),
	generateDbPassword: () => 'generated-password',
	// Whatever the run does after creating a project is not what these tests are about, and the
	// real ones poll Supabase until it answers.
	waitUntilSupabaseHealthy: async (_t: string, _r: string) => ({ id: '2', name: 'later' }),
	resolveSupabaseConnection: async () => {
		throw new Error('stop the run here')
	}
}))

const existsVariableMock = vi.fn()
const getVariableMock = vi.fn()
const getResourceMock = vi.fn()
const createVariableMock = vi.fn()
const getSettingsMock = vi.fn()
const editDataTableConfigMock = vi.fn()
const testDataTableConnectionMock = vi.fn()
const setupCustomInstanceDbMock = vi.fn()
vi.mock('$lib/gen', () => ({
	VariableService: {
		existsVariable: (...a: any[]) => existsVariableMock(...a),
		getVariable: (...a: any[]) => getVariableMock(...a),
		createVariable: (...a: any[]) => createVariableMock(...a),
		updateVariable: vi.fn()
	},
	ResourceService: {
		existsResource: vi.fn(),
		getResource: (...a: any[]) => getResourceMock(...a),
		createResource: vi.fn(),
		updateResource: vi.fn()
	},
	SettingService: { setupCustomInstanceDb: (...a: any[]) => setupCustomInstanceDbMock(...a) },
	WorkspaceService: {
		getSettings: (...a: any[]) => getSettingsMock(...a),
		editDataTableConfig: (...a: any[]) => editDataTableConfigMock(...a),
		testDataTableConnection: (...a: any[]) => testDataTableConnectionMock(...a)
	}
}))

import { newResourceParts, newWizardState, runSetup, type WizardState } from './addDataTableModel'
import { noClaims } from './setupClaims'

/** Nothing at the path: the reads that answer "is this ours?" find no object. */
function nothingThere() {
	getVariableMock.mockRejectedValue(new Error('not found'))
	getResourceMock.mockRejectedValue(new Error('not found'))
}

/** A wizard about to create the Supabase project `later`, in the organization `acme`. */
function creating(): WizardState {
	const state = newWizardState({ name: 'main', projectName: 'later', folder: 'f/team' })
	state.provider = 'supabase'
	state.supabase.mode = 'create'
	state.supabase.org = 'acme'
	state.review.resourceName = 'db'
	return state
}

/** The path `creating()` writes to, and where an earlier attempt's password would sit. */
const MINTED_PATH = 'f/team/db'

const deps = (createdProjectName?: string, createdProjectPath = MINTED_PATH) => ({
	workspace: 'w',
	supabaseToken: 'token',
	onProgress: () => {},
	claims: noClaims,
	username: 'alice',
	createdProjectName,
	createdProjectPath: createdProjectName ? createdProjectPath : undefined
})

// `writeSecret` overwrites in place, and Supabase never shows a project's password twice, so
// minting a second one at the path where an earlier project's is stored destroys the only copy.
describe('runSetup refusing to mint over a project it already created', () => {
	beforeEach(() => {
		vi.clearAllMocks()
		existsVariableMock.mockResolvedValue(false)
		nothingThere()
	})

	it('refuses while the earlier project is still there', async () => {
		listSupabaseProjectsMock.mockResolvedValue([
			{ id: '1', name: 'earlier', organization_id: 'acme' }
		])
		const result = await runSetup(creating(), deps('earlier'))
		expect(result.ok).toBe(false)
		expect(result.error).toContain('earlier')
		expect(createVariableMock).not.toHaveBeenCalled()
		expect(createSupabaseProjectMock).not.toHaveBeenCalled()
	})

	// The name is also recorded when a create could not be confirmed -- an expired token answers
	// neither the create nor the lookup. Refusing on that forever would strand the session.
	it('proceeds when no project by that name exists after all', async () => {
		listSupabaseProjectsMock.mockResolvedValue([])
		createSupabaseProjectMock.mockResolvedValue({ id: '2', name: 'later' })
		await runSetup(creating(), deps('earlier'))
		expect(createSupabaseProjectMock).toHaveBeenCalled()
	})

	// Connecting the created project as an existing one reaches the same secret by another
	// route: the project list on step 2 is where it now appears, so this is the likely move.
	it('refuses to write over the secret from the existing-project branch', async () => {
		const state = creating()
		state.supabase.mode = 'existing'
		state.supabase.project = { id: '1', name: 'earlier' } as any
		state.supabase.password = 'typed-by-hand'
		const result = await runSetup(state, deps('earlier'))
		expect(result.ok).toBe(false)
		expect(result.error).toContain(MINTED_PATH)
		expect(createVariableMock).not.toHaveBeenCalled()
	})

	// Aimed somewhere else, there is nothing to protect -- and over-refusing here would block
	// the ordinary way out of every refusal above, which is to choose another path.
	it('writes when the run is aimed at a different path', async () => {
		const state = creating()
		state.supabase.mode = 'existing'
		state.supabase.project = { id: '1', name: 'earlier' } as any
		state.supabase.password = 'typed-by-hand'
		await runSetup(state, deps('earlier', 'f/team/somewhere-else'))
		expect(createVariableMock).toHaveBeenCalled()
	})

	// The organization selected now is not the one the earlier project was created under, and
	// switching it is one of the ways to arrive here.
	it('refuses a project listed under a different organization', async () => {
		listSupabaseProjectsMock.mockResolvedValue([
			{ id: '1', name: 'earlier', organization_id: 'other-org' }
		])
		const result = await runSetup(creating(), deps('earlier'))
		expect(result.ok).toBe(false)
		expect(createSupabaseProjectMock).not.toHaveBeenCalled()
	})
})

// The instance branch is the one that has to write its row before it can probe it, since the
// probe is by data table name. A database Windmill cannot store data in must not stay in the
// config -- and a probe that throws leaves exactly the same unusable row as one that says no.
describe('runSetup rolling the instance row back', () => {
	function usingInstanceDb(): WizardState {
		const state = newWizardState({ name: 'main', projectName: 'x', folder: 'f/team' })
		state.provider = 'instance'
		state.instance = { mode: 'existing', dbName: 'shared' }
		return state
	}

	// The rollback reads the config back before deleting, so the config has to behave like one:
	// a mock that always answers empty would let a rollback that never finds its own row pass.
	let datatables: Record<string, any>

	beforeEach(() => {
		vi.clearAllMocks()
		datatables = {}
		getSettingsMock.mockImplementation(async () => ({ datatable: { datatables } }))
		editDataTableConfigMock.mockImplementation(async ({ requestBody }: any) => {
			datatables = { ...requestBody.settings.datatables }
		})
		setupCustomInstanceDbMock.mockResolvedValue({ success: true, logs: {} })
		nothingThere()
	})

	// The pre-flight runs once, before a Supabase create that can take minutes, and every
	// wizard suggests the same `main` -- so the name can be taken by the time the row is
	// written. Repointing it would hand another admin's data table a database nobody chose.
	it('refuses a name that was taken while it was running', async () => {
		datatables = { main: { database: { resource_path: 'someone-else' } } }
		const result = await runSetup(usingInstanceDb(), {
			workspace: 'w',
			onProgress: () => {},
			claims: noClaims,
			username: 'alice'
		} as any)
		expect(result.ok).toBe(false)
		expect(result.error).toContain('main')
		expect(editDataTableConfigMock).not.toHaveBeenCalled()
	})

	// Rolling back is just as dangerous once someone else owns the name: the row under it is
	// no longer the one this run wrote.
	it('leaves a row it no longer recognises alone', async () => {
		// Repointed by someone else while this run was probing it.
		testDataTableConnectionMock.mockImplementation(async () => {
			datatables = { main: { database: { resource_path: 'someone-else' } } }
			throw new Error('connection refused')
		})
		const result = await runSetup(usingInstanceDb(), {
			workspace: 'w',
			onProgress: () => {},
			claims: noClaims,
			username: 'alice'
		} as any)
		expect(result.ok).toBe(false)
		expect(result.rowRolledBack).toBe(false)
		// One call: the write. The rollback found a row it did not write and left it.
		expect(editDataTableConfigMock).toHaveBeenCalledTimes(1)
		// And the name is not handed back as ours: claiming it would let Try again write over
		// the row the other admin now owns.
		expect(result.rowWritten).toBe(false)
	})

	it('takes the row back out when the probe never answers', async () => {
		testDataTableConnectionMock.mockRejectedValue(new Error('connection refused'))
		const result = await runSetup(usingInstanceDb(), {
			workspace: 'w',
			supabaseToken: undefined,
			onProgress: () => {},
			claims: noClaims,
			username: 'alice'
		} as any)
		expect(result.ok).toBe(false)
		expect(result.error).toContain('connection refused')
		expect(result.rowRolledBack).toBe(true)
		expect(result.rowWritten).toBe(false)
		const lastWrite = editDataTableConfigMock.mock.calls.at(-1)?.[0]
		expect(lastWrite.requestBody.settings.datatables).not.toHaveProperty('main')
	})
})

// The fields are the connection; a connection string is a way of writing one down. Reading the
// resource back out of the string is what let a URI grammar gap change what got saved.
describe('newResourceParts', () => {
	function typedByHand(): WizardState {
		const state = newWizardState({ name: 'main', projectName: 'x', folder: 'f/team' })
		state.provider = 'resource'
		state.own.creating = true
		state.own.fields = {
			host: 'db.example.com',
			port: 5432,
			dbname: 'mydb',
			user: 'u',
			password: 'p',
			sslmode: 'prefer'
		}
		return state
	}

	it('reads the fields whichever notation is on screen', () => {
		const state = typedByHand()
		state.own.form = 'string'
		state.own.connectionString = 'postgres://u:p@db.example.com:5432/mydb'
		// The string names no sslmode. The choice on the fields is what gets saved.
		expect(newResourceParts(state)?.sslmode).toBe('prefer')
		state.own.form = 'fields'
		expect(newResourceParts(state)?.sslmode).toBe('prefer')
	})

	it('is unaffected by a string that cannot be parsed', () => {
		const state = typedByHand()
		state.own.form = 'string'
		state.own.connectionString = 'not a uri'
		expect(newResourceParts(state)?.host).toBe('db.example.com')
	})
})
