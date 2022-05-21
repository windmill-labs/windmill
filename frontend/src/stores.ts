import { browser } from '$app/env'
import type { Readable } from 'svelte/store'
import { derived, writable } from 'svelte/store'
import type { UserWorkspaceList } from './gen'

export interface UserExt {
	email: string
	username: string
	is_admin: boolean
	created_at: string
	groups: string[]
	pgroups: string[]
}
let persistedWorkspace = browser && localStorage.getItem('workspace')

export const userStore = writable<UserExt | undefined>(undefined)
export let workspaceStore = writable<string | undefined>(
	persistedWorkspace ? JSON.parse(persistedWorkspace) : undefined
)
export const usersWorkspaceStore = writable<UserWorkspaceList | undefined>(undefined)
export const usernameStore: Readable<string | undefined> = derived(
	[usersWorkspaceStore, workspaceStore],
	($values, set) => {
		set($values[0]?.workspaces.find((x) => x.id == $values[1])?.username)
	}
)
export const superadmin = writable<String | false | undefined>(undefined)

if (browser) {
	workspaceStore.subscribe((workspace) => {
		console.log(workspace)
		if (workspace) {
			localStorage.setItem('workspace', JSON.stringify(workspace))
		}
	})
}
