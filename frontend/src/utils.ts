/* eslint-disable @typescript-eslint/explicit-module-boundary-types */
import { goto } from '$app/navigation'
import { toast } from '@zerodevx/svelte-toast'
import { get } from 'svelte/store'
import { CancelablePromise, UserService, type User } from './gen'
import { clearStores, superadmin, userStore, workspaceStore, type UserExt } from './stores'

export function isToday(someDate: Date): boolean {
	const today = new Date()
	return (
		someDate.getDate() == today.getDate() &&
		someDate.getMonth() == today.getMonth() &&
		someDate.getFullYear() == today.getFullYear()
	)
}

export function daysAgo(someDate: Date): number {
	const today = new Date()
	return Math.floor((today.getTime() - someDate.getTime()) / 86400000)
}

export function secondsAgo(date: Date) {
	return Math.floor((new Date().getTime() - date.getTime()) / 1000)
}

export function displayDaysAgo(dateString: string): string {
	const date = new Date(dateString)
	const nbSecondsAgo = secondsAgo(date)
	if (nbSecondsAgo < 600) {
		return `${nbSecondsAgo}s ago`
	} else if (isToday(date)) {
		return `today at ${date.toLocaleTimeString()}`
	} else if (daysAgo(date) === 0) {
		return `${daysAgo(date) + 1} day ago`
	} else {
		return `${daysAgo(date) + 1} day ago`
	}
}

export function displayDate(dateString: string | undefined): string {
	const date = new Date(dateString ?? '')
	if (date.toString() === 'Invalid Date') {
		return ''
	} else {
		return `${date.getFullYear()}/${
			date.getMonth() + 1
		}/${date.getDate()} at ${date.toLocaleTimeString()}`
	}
}

export function getToday() {
	var today = new Date()
	return today
}

export function sendUserToast(message: string, error: boolean = false): void {
	if (error) {
		toast.push(message, {
			theme: {
				'--toastBackground': '#FEE2E2',
				'--toastBarBackground': '#FEE2E2'
			}
		})
	} else toast.push(message)
}

export function truncateHash(hash: string): string {
	if (hash.length >= 6) {
		return hash.substr(hash.length - 6)
	} else {
		return hash
	}
}

async function loadStore(workspace: string): Promise<UserExt | undefined> {
	try {
		const user = await UserService.whoami({ workspace })
		const nuser = mapUserToUserExt(user)
		userStore.set(nuser)
		return nuser
	} catch (error) {
		userStore.set(undefined)
		return undefined
	}
}

export async function getUser(workspace: string): Promise<UserExt | undefined> {
	const user = get(userStore)
	if (user === undefined) {
		return loadStore(workspace)
	} else {
		return user
	}
}

export function logoutWithRedirect(rd?: string): void {
	const error = encodeURIComponent('You have been logged out because your session has expired.')
	goto(`/user/login?error=${error}${rd ? '&rd=' + encodeURIComponent(rd) : ''}`)
}

export async function handle401<T>(
	promise: CancelablePromise<T> | Promise<T>,
	rd?: string
): Promise<T> {
	// Redirects to login if the `promise` returns a 401 due to lack of authentication
	// Optionnally provide `rd`, to which the user will be redirected after logging back in

	return promise.catch(async (error) => {
		if (error.status === 401) {
			if (getUser(get(workspaceStore)!) === undefined) {
				logoutWithRedirect(rd)
				// eslint-disable-next-line @typescript-eslint/no-explicit-any
				return null as any
			} else {
				throw Error('You do not have enough privilege to access this')
			}
		} else {
			throw error
		}
	})
}

export function sleep(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms))
}

export function validatePassword(password: string): boolean {
	const re = /^(?=.*[\d])(?=.*[!@#$%^&*])[\w!@#$%^&*]{8,30}$/
	return re.test(password)
}

export async function refreshSuperadmin(): Promise<void> {
	if (get(superadmin) == undefined) {
		UserService.globalWhoami().then((x) => {
			if (x.super_admin) {
				superadmin.set(x.email)
			} else {
				superadmin.set(false)
			}
		})
	}
}

export async function logout(logoutMessage?: string): Promise<void> {
	try {
		clearStores()
		goto(`/user/login${logoutMessage ? '?error=' + encodeURIComponent(logoutMessage) : ''}`)
		await UserService.logout()
		sendUserToast('you have been logged out')
	} catch (error) {
		goto(
			`/user/login?error=${encodeURIComponent(
				'There was a problem logging you out, check the logs'
			)}`
		)
		console.error(error)
	}
}

// eslint-disable-next-line @typescript-eslint/no-explicit-any
export function clickOutside(node: any): any {
	const handleClick = (event: Event) => {
		if (node && !node.contains(event.target) && !event.defaultPrevented) {
			node.dispatchEvent(new CustomEvent('click_outside', node))
		}
	}

	document.addEventListener('click', handleClick, true)

	return {
		destroy() {
			document.removeEventListener('click', handleClick, true)
		}
	}
}

export type DropdownType = 'action' | 'delete'

export interface DropdownItem {
	// If a DropdownItem has an action, it will be declared as a button
	// If a DropdownItem has no action and an href, it will be declared as a link
	// If a DropdownItem has no action and no href, it will be created as a text line
	displayName: string
	eventName?: string //the event to send when clicking this item
	action?: (() => Promise<void>) | (() => void)
	href?: string
	separatorTop?: boolean
	separatorBottom?: boolean
	type?: DropdownType
	disabled?: boolean
	icon?: any | undefined
}

export function emptySchema() {
	return {
		$schema: 'https://json-schema.org/draft/2020-12/schema',
		properties: {},
		required: [],
		type: 'object'
	}
}
export function simpleSchema() {
	return {
		$schema: 'https://json-schema.org/draft/2020-12/schema',
		type: 'object',
		properties: {
			name: {
				description: 'The name to hello world to',
				type: 'string'
			}
		},
		required: []
	}
}

export function removeItemAll<T>(arr: T[], value: T) {
	var i = 0
	while (i < arr.length) {
		if (arr[i] === value) {
			arr.splice(i, 1)
		} else {
			++i
		}
	}
	return arr
}

export function canWrite(
	path: string,
	extra_perms: Record<string, boolean>,
	user?: UserExt
): boolean {
	let keys = Object.keys(extra_perms)
	if (!user) {
		return false
	}
	if (user.is_admin) {
		return true
	}
	let userOwner = `u/${user.username}`
	if (path.startsWith(userOwner)) {
		return true
	}
	if (keys.includes(userOwner) && extra_perms[userOwner]) {
		return true
	}
	if (
		user.pgroups.findIndex((x) => path.startsWith(x) || (keys.includes(x) && extra_perms[x])) != -1
	) {
		return true
	}
	return false
}

export function removeKeysWithEmptyValues(obj: any): any {
	Object.keys(obj).forEach((key) => (obj[key] === undefined ? delete obj[key] : {}))
}

export function allTrue(dict: { [id: string]: boolean }): boolean {
	return Object.values(dict).every(Boolean)
}

export function forLater(scheduledString: string): boolean {
	return new Date() < new Date(scheduledString)
}

export function elapsedSinceSecs(date: string): number {
	return Math.round((new Date().getTime() - new Date(date).getTime()) / 1000)
}

export function groupBy<T>(
	scripts: T[],
	toGroup: (t: T) => string,
	dflts: string[] = []
): [string, T[]][] {
	let r: Record<string, T[]> = {}
	for (const dflt of dflts) {
		r[dflt] = []
	}

	scripts.forEach((sc) => {
		let section = toGroup(sc)
		if (section in r) {
			r[section].push(sc)
		} else {
			r[section] = [sc]
		}
	})

	return Object.entries(r).sort((s1, s2) => {
		let n1 = s1[0]
		let n2 = s2[0]

		if (n1 > n2) {
			return 1
		} else if (n1 < n2) {
			return -1
		} else {
			return 0
		}
	})
}

export function truncate(s: string, n: number, suffix: string = '...'): string {
	if (s.length <= n) {
		return s
	} else {
		return s.substring(0, n) + suffix
	}
}

export function truncateRev(s: string, n: number, prefix: string = '...'): string {
	if (s.length <= n) {
		return s
	} else {
		return prefix + s.substring(s.length - n, s.length)
	}
}

export function isString(value: any) {
	return typeof value === 'string' || value instanceof String
}

export function mapUserToUserExt(user: User): UserExt {
	return {
		...user,
		groups: user.groups!,
		pgroups: user.groups!.map((x) => `g/${x}`)
	}
}
