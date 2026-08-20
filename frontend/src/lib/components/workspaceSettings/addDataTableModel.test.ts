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

import {
	intentComplete,
	newResourceParts,
	newWizardState,
	runSetup,
	type WizardState
} from './addDataTableModel'
import { noClaims } from './setupClaims'

/** Nothing at the path: the reads that answer "is this ours?" find no object. */
function nothingThere() {
	getVariableMock.mockRejectedValue(new Error('not found'))
	getResourceMock.mockRejectedValue(new Error('not found'))
}

/** A resource that exists, with the timestamp the claim is marked by. */
function resourceEditedAt(at: string) {
	getResourceMock.mockResolvedValue({ path: 'p', created_by: 'alice', edited_at: at })
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
	createdProjects: createdProjectName
		? [{ name: createdProjectName, path: createdProjectPath }]
		: []
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
			username: 'alice',
			createdProjects: []
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
			username: 'alice',
			createdProjects: []
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
			username: 'alice',
			createdProjects: []
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

// `created_by` survives an update, so it cannot tell an edit by somebody else from no edit at
// all. The claim is marked by `edited_at`, which moves on every write.
describe('runSetup writing over a resource', () => {
	function ownResource(): WizardState {
		const state = newWizardState({ name: 'main', projectName: 'x', folder: 'f/team' })
		state.provider = 'resource'
		state.own.creating = true
		state.review.resourceName = 'db'
		state.own.fields = {
			host: 'h',
			port: 5432,
			dbname: 'd',
			user: 'u',
			password: 'p',
			sslmode: 'require'
		}
		return state
	}

	beforeEach(() => {
		vi.clearAllMocks()
		existsVariableMock.mockResolvedValue(false)
		getVariableMock.mockRejectedValue(new Error('not found'))
		getSettingsMock.mockResolvedValue({ datatable: { datatables: {} } })
		editDataTableConfigMock.mockResolvedValue(undefined)
		testDataTableConnectionMock.mockResolvedValue({ can_create_table: true })
	})

	it('refuses a resource edited since this run claimed it', async () => {
		resourceEditedAt('2026-01-02T00:00:00Z')
		const result = await runSetup(ownResource(), {
			workspace: 'w',
			onProgress: () => {},
			// Claimed when it looked like this; someone has written to it since.
			claims: [{ kind: 'resource' as const, path: 'f/team/db', mark: '2026-01-01T00:00:00Z' }],
			username: 'alice',
			createdProjects: []
		} as any)
		expect(result.ok).toBe(false)
		expect(result.error).toContain('f/team/db')
	})
})

describe('runSetup writing over its own secret', () => {
	const ownDb = (): WizardState => {
		const state = newWizardState({ name: 'main', projectName: 'x', folder: 'f/team' })
		state.provider = 'resource'
		state.own.creating = true
		state.review.resourceName = 'db'
		state.own.fields = {
			host: 'h',
			port: 5432,
			dbname: 'd',
			user: 'u',
			password: 'p',
			sslmode: 'require'
		}
		return state
	}

	beforeEach(() => {
		vi.clearAllMocks()
		getResourceMock.mockRejectedValue(new Error('not found'))
		getSettingsMock.mockResolvedValue({ datatable: { datatables: {} } })
		editDataTableConfigMock.mockResolvedValue(undefined)
		testDataTableConnectionMock.mockResolvedValue({ can_create_table: true })
	})

	// The same person editing the variable in another tab leaves `edited_by` unchanged, so an
	// author is not enough to tell that write from none.
	it('refuses a secret edited since this run claimed it, even by the same user', async () => {
		getVariableMock.mockResolvedValue({ edited_by: 'alice', edited_at: '2026-01-02T00:00:00Z' })
		const result = await runSetup(ownDb(), {
			workspace: 'w',
			onProgress: () => {},
			claims: [{ kind: 'secret' as const, path: 'f/team/db', mark: '2026-01-01T00:00:00Z' }],
			username: 'alice',
			createdProjects: []
		} as any)
		expect(result.ok).toBe(false)
		expect(result.error).toContain('f/team/db')
	})

	// A create whose confirmation also failed records the project name pessimistically. The
	// variable it wrote is still its own, and a retry has to be able to reuse the path.
	it('reuses the variable a previous attempt wrote when its project was never confirmed', async () => {
		getVariableMock.mockResolvedValue({ edited_by: 'alice', edited_at: '2026-01-01T00:00:00Z' })
		listSupabaseProjectsMock.mockResolvedValue([])
		createSupabaseProjectMock.mockResolvedValue({ id: '2', name: 'later' })
		const state = creating()
		const result = await runSetup(state, {
			...deps('later'),
			claims: [{ kind: 'secret' as const, path: MINTED_PATH, mark: '2026-01-01T00:00:00Z' }]
		} as any)
		expect(result.error ?? '').not.toContain('was created at')
	})
})

// Editing a valid string into an invalid one keeps the fields, so they stay correctable. What
// must not happen is testing or saving those fields while the string on screen says otherwise.
describe('intentComplete with a connection string on screen', () => {
	function typed(connectionString: string): WizardState {
		const state = newWizardState({ name: 'main', projectName: 'x', folder: 'f/team' })
		state.provider = 'resource'
		state.own.creating = true
		state.own.form = 'string'
		state.own.connectionString = connectionString
		state.own.fields = {
			host: 'db.example.com',
			port: 5432,
			dbname: 'mydb',
			user: 'u',
			password: 'p',
			sslmode: 'require'
		}
		return state
	}

	it('refuses a string that will not parse, whatever the fields still hold', () => {
		expect(intentComplete(typed('postgres://u:p@db.example.com:5432/mydb'))).toBe(true)
		expect(intentComplete(typed('postgres://u:p@db.exa'))).toBe(false)
		expect(intentComplete(typed(''))).toBe(false)
	})

	it('is unaffected once the fields are the notation on screen', () => {
		const state = typed('nonsense')
		state.own.form = 'fields'
		expect(intentComplete(state)).toBe(true)
	})
})

// Each created project guards its own path. Keeping only the latest let a second attempt at
// another path unlock the first project's password, which Supabase will never show again.
describe('runSetup guarding more than one created project', () => {
	beforeEach(() => {
		vi.clearAllMocks()
		existsVariableMock.mockResolvedValue(false)
		nothingThere()
	})

	it('still refuses the first project’s path after a second was created elsewhere', async () => {
		listSupabaseProjectsMock.mockResolvedValue([
			{ id: '1', name: 'first', organization_id: 'acme' }
		])
		const state = creating()
		const result = await runSetup(state, {
			...deps(),
			createdProjects: [
				{ name: 'first', path: MINTED_PATH },
				{ name: 'second', path: 'f/team/other' }
			]
		} as any)
		expect(result.ok).toBe(false)
		expect(result.error).toContain('first')
		expect(createVariableMock).not.toHaveBeenCalled()
	})
})
