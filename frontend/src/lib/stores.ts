import { BROWSER } from 'esm-env'
import { derived, type Readable, writable } from 'svelte/store'
import { type WorkspaceDefaultScripts, type TokenResponse, type UserWorkspaceList } from './gen'
import type { IntrospectionQuery } from 'graphql'
import { getLocalSetting } from './utils'

export interface UserExt {
	email: string
	name: string
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

const persistedWorkspace = BROWSER && getWorkspace()

function getWorkspace(): string | undefined {
	try {
		return localStorage.getItem('workspace') ?? undefined
	} catch (e) {
		console.error('error interacting with local storage', e)
	}
	return undefined
}
export const tutorialsToDo = writable<number[]>([])
export const globalEmailInvite = writable<string>('')
export const awarenessStore = writable<Record<string, string>>(undefined)
export const enterpriseLicense = writable<string | undefined>(undefined)
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
export const starStore = writable(1)
export const usersWorkspaceStore = writable<UserWorkspaceList | undefined>(undefined)
export const superadmin = writable<string | false | undefined>(undefined)
export const devopsRole = writable<string | false | undefined>(undefined)
export const lspTokenStore = writable<string | undefined>(undefined)
export const hubBaseUrlStore = writable<string>('https://hub.windmill.dev')
export const userWorkspaces: Readable<
	Array<{
		id: string
		name: string
		username: string
	}>
> = derived([usersWorkspaceStore, superadmin], ([store, superadmin]) => {
	const originalWorkspaces = store?.workspaces ?? []
	if (superadmin) {
		return [
			...originalWorkspaces.filter((x) => x.id != 'admins'),
			{
				id: 'admins',
				name: 'Admins',
				username: 'superadmin'
			}
		]
	} else {
		return originalWorkspaces
	}
})
export const copilotInfo = writable<{
	ai_provider: string
	exists_ai_resource: boolean
	code_completion_enabled: boolean
}>({
	ai_provider: '',
	exists_ai_resource: false,
	code_completion_enabled: false
})
export const codeCompletionLoading = writable<boolean>(false)
export const metadataCompletionEnabled = writable<boolean>(true)
export const stepInputCompletionEnabled = writable<boolean>(true)
export const FORMAT_ON_SAVE_SETTING_NAME = 'formatOnSave'
export const VIM_MODE_SETTING_NAME = 'vimMode'
export const CODE_COMPLETION_SETTING_NAME = 'codeCompletionSessionEnabled'
export const formatOnSave = writable<boolean>(
	getLocalSetting(FORMAT_ON_SAVE_SETTING_NAME) != 'false'
)
export const vimMode = writable<boolean>(getLocalSetting(VIM_MODE_SETTING_NAME) == 'true')
export const codeCompletionSessionEnabled = writable<boolean>(
	getLocalSetting(CODE_COMPLETION_SETTING_NAME) != 'false'
)
export const usedTriggerKinds = writable<string[]>([])

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

export interface SQLSchema {
	lang: 'mysql' | 'bigquery' | 'postgresql' | 'snowflake' | 'mssql'
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

export const instanceSettingsSelectedTab = writable('Core')

export const isCriticalAlertsUIOpen = writable(false)
