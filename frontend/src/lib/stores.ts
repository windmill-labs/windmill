import { browser } from '$app/environment'
import { derived, type Readable, writable, get } from 'svelte/store'
import type { UserWorkspaceList } from '$lib/gen/models/UserWorkspaceList.js'
import { getUserExt } from './user'
import { WorkspaceService, type TokenResponse } from './gen'
import { isCloudHosted } from './utils'

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

let persistedWorkspace = browser && localStorage.getItem('workspace')

export const usageStore = writable<number>(0)
export const runFormStore = writable<any>()
export const oauthStore = writable<TokenResponse | undefined>(undefined)
export const userStore = writable<UserExt | undefined>(undefined)
export const workspaceStore = writable<string | undefined>(
	persistedWorkspace ? String(persistedWorkspace) : undefined
)
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
			...originalWorkspaces.filter((x) => x.id != 'starter' && x.id != 'admins'),
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

export type HubScript = {
	path: string
	summary: string
	approved: boolean
	kind: string
	app: string
	ask_id: number
}

export type HubFlow = {
	id: number
	flow_id: number
	summary: string
	apps: string[]
	approved: boolean
	votes: number
}

export type HubApp = {
	id: number
	app_id: number
	summary: string
	apps: Array<string>
	approved: boolean
	votes: number
}

export type HubItem =
	| { itemType: 'script'; data: HubScript & { marked?: string } }
	| { itemType: 'flow'; data: HubFlow & { marked?: string } }
	| { itemType: 'app'; data: HubApp & { marked?: string } }

export const hubScripts = writable<Array<HubScript> | undefined>(undefined)
export const hubFlows = writable<Array<HubFlow> | undefined>(undefined)
export const hubApps = writable<Array<HubApp> | undefined>(undefined)

if (browser) {
	workspaceStore.subscribe(async (workspace) => {
		if (workspace) {
			try {
				localStorage.setItem('workspace', String(workspace))
			} catch (e) {
				console.error('Could not persist workspace to local storage', e)
			}
			const user = await getUserExt(workspace)
			userStore.set(user)
			if (isCloudHosted() && user?.is_admin) {
				premiumStore.set(await WorkspaceService.getPremiumInfo({ workspace }))
			}
		} else {
			userStore.set(undefined)
		}
	})

	setInterval(async () => {
		try {
			const workspace = get(workspaceStore)
			const user = get(userStore)

			if (workspace && user && !user.is_super_admin && !user.is_admin) {
				userStore.set(await getUserExt(workspace))
				console.log('refreshed user')
			}
		} catch (e) {
			console.error('Could not refresh user', e)
		}
	}, 30000)
}

export function switchWorkspace(workspace: string | undefined) {
	localStorage.removeItem('flow')
	localStorage.removeItem('app')
	workspaceStore.set(workspace)
}

export function clearStores(): void {
	localStorage.removeItem('flow')
	localStorage.removeItem('app')
	localStorage.removeItem('workspace')
	userStore.set(undefined)
	workspaceStore.set(undefined)
	usersWorkspaceStore.set(undefined)
	superadmin.set(undefined)
}
