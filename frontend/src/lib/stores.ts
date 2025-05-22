import { BROWSER } from 'esm-env'
import { derived, type Readable, writable } from 'svelte/store'

import type { IntrospectionQuery } from 'graphql'
import {
	type AIConfig,
	type AIProvider,
	type AIProviderModel,
	type OperatorSettings,
	type TokenResponse,
	type UserWorkspaceList,
	type WorkspaceDefaultScripts,
	WorkspaceService
} from './gen'
import { getLocalSetting } from './utils'

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
	color: string | null
	operator_settings?: OperatorSettings
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
export const starStore = writable(1)
export const usersWorkspaceStore = writable<UserWorkspaceList | undefined>(undefined)
export const superadmin = writable<string | false | undefined>(undefined)
export const devopsRole = writable<string | false | undefined>(undefined)
export const lspTokenStore = writable<string | undefined>(undefined)
export const hubBaseUrlStore = writable<string>('https://hub.windmill.dev')
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
					color: null,
					operator_settings: null
				}
			]
		} else {
			return originalWorkspaces
		}
	}
)
export const copilotInfo = writable<{
	enabled: boolean
	codeCompletionModel?: AIProviderModel
	defaultModel?: AIProviderModel
	aiModels: AIProviderModel[]
}>({
	enabled: false,
	codeCompletionModel: undefined,
	defaultModel: undefined,
	aiModels: []
})

export function setCopilotInfo(aiConfig: AIConfig) {
	if (Object.keys(aiConfig.providers ?? {}).length > 0) {
		const aiModels = Object.entries(aiConfig.providers ?? {}).flatMap(
			([provider, providerConfig]) =>
				providerConfig.models.map((m) => ({ model: m, provider: provider as AIProvider }))
		)

		copilotSessionModel.update((model) => {
			if (
				model &&
				!aiModels.some((m) => m.model === model.model && m.provider === model.provider)
			) {
				return undefined
			}
			return model
		})

		copilotInfo.set({
			enabled: true,
			codeCompletionModel: aiConfig.code_completion_model,
			defaultModel: aiConfig.default_model,
			aiModels: aiModels
		})
	} else {
		copilotSessionModel.set(undefined)

		copilotInfo.set({
			enabled: false,
			codeCompletionModel: undefined,
			defaultModel: undefined,
			aiModels: []
		})
	}
}

export const codeCompletionLoading = writable<boolean>(false)
export const metadataCompletionEnabled = writable<boolean>(true)
export const stepInputCompletionEnabled = writable<boolean>(true)
export const FORMAT_ON_SAVE_SETTING_NAME = 'formatOnSave'
export const VIM_MODE_SETTING_NAME = 'vimMode'
export const CODE_COMPLETION_SETTING_NAME = 'codeCompletionSessionEnabled'
export const COPILOT_SESSION_MODEL_SETTING_NAME = 'copilotSessionModel'
export const COPILOT_SESSION_PROVIDER_SETTING_NAME = 'copilotSessionProvider'
export const formatOnSave = writable<boolean>(
	getLocalSetting(FORMAT_ON_SAVE_SETTING_NAME) != 'false'
)
export const vimMode = writable<boolean>(getLocalSetting(VIM_MODE_SETTING_NAME) == 'true')
export const codeCompletionSessionEnabled = writable<boolean>(
	getLocalSetting(CODE_COMPLETION_SETTING_NAME) != 'false'
)

const sessionModel = getLocalSetting(COPILOT_SESSION_MODEL_SETTING_NAME)
const sessionProvider = getLocalSetting(COPILOT_SESSION_PROVIDER_SETTING_NAME)
export const copilotSessionModel = writable<AIProviderModel | undefined>(
	sessionModel && sessionProvider
		? {
				model: sessionModel,
				provider: sessionProvider as AIProvider
			}
		: undefined
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

export const SQLSchemaLanguages = [
	'mysql',
	'bigquery',
	'postgresql',
	'snowflake',
	'mssql',
	'oracledb'
] as const

export interface SQLSchema {
	lang: (typeof SQLSchemaLanguages)[number]
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

export const workspaceColor: Readable<string | null | undefined> = derived(
	[workspaceStore, usersWorkspaceStore, superadmin],
	([workspaceStore, usersWorkspaceStore, superadmin], set: (value: string | undefined) => void) => {
		if (!workspaceStore) {
			set(undefined)
			return
		}

		// First try to get the color from usersWorkspaceStore
		const color = usersWorkspaceStore?.workspaces.find((w) => w.id === workspaceStore)?.color

		if (color) {
			set(color)
			return
		}

		// If not found and user is superadmin, try to get it from superadmin list
		if (!superadmin) {
			set(undefined)
			return
		}

		WorkspaceService.listWorkspacesAsSuperAdmin().then((workspaces) => {
			const superadminColor = workspaces.find((w) => w.id === workspaceStore)?.color
			set(superadminColor)
		})
	}
)

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
