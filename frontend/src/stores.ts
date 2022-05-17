import { writable, derived, readable } from 'svelte/store'
import type { Readable } from 'svelte/store'
import type { UserWorkspaceList } from './gen'

export interface UserExt {
	email: string
	username: string
	is_admin: boolean
	created_at: string
	groups: string[]
	pgroups: string[]
}

export const userStore = writable<UserExt | undefined>(undefined)
export const workspaceStore = writable<string | undefined>(undefined)
export const usersWorkspaceStore = writable<UserWorkspaceList | undefined>(undefined)
export const usernameStore: Readable<string | undefined> = derived(
	[usersWorkspaceStore, workspaceStore],
	($values, set) => {
		set($values[0]?.workspaces.find((x) => x.id == $values[1])?.username)
	}
)
export const superadmin = writable<String | false | undefined>(undefined)
