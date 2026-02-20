import { BROWSER } from 'esm-env'
import { derived, type Readable, writable } from 'svelte/store'

import type { IntrospectionQuery } from 'graphql'
import {
	CancelablePromise,
	type OperatorSettings,
	type TokenResponse,
	type UserWorkspaceList,
	type Workspace,
	type WorkspaceDefaultScripts,
	WorkspaceService
} from './gen'
import { getLocalSetting, type StateStore } from './utils'
import { createState } from './svelte5Utils.svelte'
import { DEFAULT_HUB_BASE_URL } from './hub'

export interface UserExt {
	email: string
	name?: string
	username: string
	is_admin: boolean
	is_super_admin: boolean
	operator: boolean
	created_at: string
	groups: string[]
	pgroups: string[]
	folders: string[]
	folders_owners: string[]
}

export interface UserWorkspace {
	id: string
	name: string
	username: string
	color?: string
	operator_settings?: OperatorSettings
	parent_workspace_id?: string | null
	disabled: boolean
}

const persistedWorkspace = BROWSER && getWorkspaceFromStorage()

export function getWorkspaceFromStorage(): string | undefined {
	try {
		return sessionStorage.getItem('workspace') ?? localStorage.getItem('workspace') ?? undefined
	} catch (e) {
		console.error('error interacting with local storage', e)
	}
	return undefined
}
export function clearWorkspaceFromStorage() {
	localStorage.removeItem('workspace')
	sessionStorage.removeItem('workspace')
}

export const tutorialsToDo = writable<number[]>([])
export const skippedAll = writable<boolean>(false)
export const globalEmailInvite = writable<string>('')
export const awarenessStore = writable<Record<string, string>>(undefined)
export const enterpriseLicense = writable<string | undefined>(undefined)
export const whitelabelNameStore = derived([enterpriseLicense], ([enterpriseLicense]) => {
	if (enterpriseLicense?.endsWith('__whitelabel')) {
		return enterpriseLicense.split('__whitelabel')[0]
	}
	return undefined
})
export const workerTags = writable<string[] | undefined>(undefined)
export const usageStore = writable<number>(0)
export const workspaceUsageStore = writable<number>(0)
export const initialArgsStore = writable<any>(undefined)
export const oauthStore = writable<TokenResponse | undefined>(undefined)
export const userStore = writable<UserExt | undefined>(undefined)
export const workspaceStore = writable<string | undefined>(
	persistedWorkspace ? String(persistedWorkspace) : undefined
)
export const defaultScripts = writable<WorkspaceDefaultScripts | undefined>(undefined)
export const dbClockDrift = writable<number | undefined>(undefined)
export const isPremiumStore = writable<boolean>(false)
export const usersWorkspaceStore = writable<UserWorkspaceList | undefined>(undefined)
export const superadmin = writable<string | false | undefined>(undefined)
export const devopsRole = writable<string | false | undefined>(undefined)
export const lspTokenStore = writable<string | undefined>(undefined)
export const hubBaseUrlStore = writable<string>(DEFAULT_HUB_BASE_URL)
export const userWorkspaces: Readable<Array<UserWorkspace>> = derived(
	[usersWorkspaceStore, superadmin],
	([store, superadmin]) => {
		const originalWorkspaces = store?.workspaces ?? []
		if (superadmin) {
			return [
				...originalWorkspaces.filter((x) => x.id != 'admins'),
				{
					id: 'admins',
					name: 'Admins',
					username: 'superadmin',
					color: undefined,
					operator_settings: undefined,
					disabled: false
				}
			]
		} else {
			return originalWorkspaces
		}
	}
)

export const codeCompletionLoading = writable<boolean>(false)
export const metadataCompletionEnabled = writable<boolean>(true)
export const stepInputCompletionEnabled = writable<boolean>(true)
export const FORMAT_ON_SAVE_SETTING_NAME = 'formatOnSave'
export const VIM_MODE_SETTING_NAME = 'vimMode'
export const RELATIVE_LINE_NUMBERS_SETTING_NAME = 'relativeLineNumbers'
export const CODE_COMPLETION_SETTING_NAME = 'codeCompletionSessionEnabled'
export const COPILOT_SESSION_MODEL_SETTING_NAME = 'copilotSessionModel'
export const COPILOT_SESSION_PROVIDER_SETTING_NAME = 'copilotSessionProvider'
export const formatOnSave = writable<boolean>(
	getLocalSetting(FORMAT_ON_SAVE_SETTING_NAME) != 'false'
)
export const vimMode = writable<boolean>(getLocalSetting(VIM_MODE_SETTING_NAME) == 'true')
export const relativeLineNumbers = writable<boolean>(
	getLocalSetting(RELATIVE_LINE_NUMBERS_SETTING_NAME) == 'true'
)
export const codeCompletionSessionEnabled = writable<boolean>(
	getLocalSetting(CODE_COMPLETION_SETTING_NAME) != 'false'
)

export const usedTriggerKinds = writable<string[]>([])

export let globalDbManagerDrawer: StateStore<any | undefined> = createState({
	val: undefined
})

type SQLBaseSchema = {
	[schemaKey: string]: {
		[tableKey: string]: {
			[columnKey: string]: {
				type: string
				default: string
				required: boolean
			}
		}
	}
}

export const SQLSchemaLanguages = [
	'mysql',
	'bigquery',
	'postgresql',
	'snowflake',
	'mssql',
	'oracledb'
] as const

export interface SQLSchema {
	lang: (typeof SQLSchemaLanguages)[number] | 'ducklake'
	schema: SQLBaseSchema
	publicOnly: boolean | undefined
	stringified: string
}

export interface GraphqlSchema {
	lang: 'graphql'
	schema: IntrospectionQuery
	stringified: string
}

export type DBSchema = SQLSchema | GraphqlSchema

export type DBSchemas = Partial<Record<string, DBSchema>>

export const dbSchemas = writable<DBSchemas>({})

export const instanceSettingsSelectedTab = writable('users')

export const isCriticalAlertsUIOpen = writable(false)

let getWorkspacePromise: CancelablePromise<Workspace> | null = null
export const workspaceColor: Readable<string | null | undefined> = derived(
	[workspaceStore, usersWorkspaceStore, superadmin],
	([workspaceStore, usersWorkspaceStore, superadmin], set: (value: string | undefined) => void) => {
		if (!workspaceStore || !usersWorkspaceStore) {
			return
		}

		// First try to get the color from usersWorkspaceStore
		const workspace = usersWorkspaceStore.workspaces.find((w) => w.id === workspaceStore)

		if (workspace) {
			set(workspace.color)
			return
		}

		// If workspace not found and user is superadmin, get it as superadmin
		if (!superadmin) {
			set(undefined)
			return
		}

		getWorkspacePromise?.cancel()

		getWorkspacePromise = WorkspaceService.getWorkspaceAsSuperAdmin({
			workspace: workspaceStore
		})

		getWorkspacePromise
			.then((workspace) => set(workspace.color))
			.catch((error) => {
				console.error('error getting workspace as superadmin', error)
				set(undefined)
			})
	}
)

export const isCurrentlyInTutorial: StateStore<boolean> = createState({ val: false })

export function getFlatTableNamesFromSchema(dbSchema: DBSchema | undefined): string[] {
	const schema = dbSchema?.schema ?? {}
	const tableNames: string[] = []

	for (const schemaKey in schema) {
		for (const tableKey in schema[schemaKey]) {
			tableNames.push(`${schemaKey}.${tableKey}`)
		}
	}

	return tableNames
}
