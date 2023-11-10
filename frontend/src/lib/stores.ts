import { BROWSER } from 'esm-env'
import { derived, type Readable, writable } from 'svelte/store'
import type { UserWorkspaceList } from '$lib/gen/models/UserWorkspaceList.js'
import type { TokenResponse } from './gen'
import type { IntrospectionQuery } from 'graphql'

export interface UserExt {
	email: string
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

const persistedWorkspace = BROWSER && localStorage.getItem('workspace')

export const tutorialsToDo = writable<number[]>([])
export const globalEmailInvite = writable<string>('')
export const awarenessStore = writable<Record<string, string>>(undefined)
export const enterpriseLicense = writable<string | undefined>(undefined)
export const workerTags = writable<string[] | undefined>(undefined)
export const usageStore = writable<number>(0)
export const runFormStore = writable<any>()
export const oauthStore = writable<TokenResponse | undefined>(undefined)
export const userStore = writable<UserExt | undefined>(undefined)
export const workspaceStore = writable<string | undefined>(
	persistedWorkspace ? String(persistedWorkspace) : undefined
)
export const dbClockDrift = writable<number | undefined>(undefined)
export const premiumStore = writable<{ premium: boolean; usage?: number }>({ premium: false })
export const starStore = writable(1)
export const usersWorkspaceStore = writable<UserWorkspaceList | undefined>(undefined)
export const superadmin = writable<String | false | undefined>(undefined)
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
	exists_openai_resource_path: boolean
	code_completion_enabled: boolean
}>({
	exists_openai_resource_path: false,
	code_completion_enabled: false
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

export interface SQLSchema {
	lang: 'mysql' | 'bigquery' | 'postgresql' | 'snowflake' | 'mssql'
	schema: SQLBaseSchema
	publicOnly: boolean | undefined
}

export interface GraphqlSchema {
	lang: 'graphql'
	schema: IntrospectionQuery
}

export type DBSchema = SQLSchema | GraphqlSchema

type DBSchemas = Partial<Record<string, DBSchema>>

export const dbSchemas = writable<DBSchemas>({})
