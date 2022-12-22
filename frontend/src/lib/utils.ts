/* eslint-disable @typescript-eslint/explicit-module-boundary-types */
import { goto } from '$app/navigation'
import {
	FlowService,
	FolderService,
	Script,
	ScriptService,
	UserService,
	type Flow,
	type User
} from '$lib/gen'
import { toast } from '@zerodevx/svelte-toast'
import type { Schema, SupportedLanguage } from './common'
import { hubScripts, workspaceStore, type UserExt } from './stores'
import { page } from '$app/stores'
import { get } from 'svelte/store'

export function validateUsername(username: string): string {
	if (username != '' && !/^\w+$/.test(username)) {
		return 'username can only contain letters and numbers'
	} else {
		return ''
	}
}

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
		return `today at ${date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`
	} else {
		let dAgo = daysAgo(date)
		if (dAgo == 0) {
			return `yesterday at ${date.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' })}`
		} else if (dAgo > 7) {
			return `${dAgo + 1} days ago at ${date.toLocaleTimeString([], {
				hour: '2-digit',
				minute: '2-digit'
			})}`
		} else {
			return displayDate(dateString)
		}
	}
}

export function displayDate(dateString: string | undefined, displaySecond = false): string {
	const date = new Date(dateString ?? '')
	if (date.toString() === 'Invalid Date') {
		return ''
	} else {
		return `${date.getFullYear()}/${date.getMonth() + 1
			}/${date.getDate()} at ${date.toLocaleTimeString([], {
				hour: '2-digit',
				minute: '2-digit',
				second: displaySecond ? '2-digit' : undefined
			})}`
	}
}

export function msToSec(ms: number | undefined): string {
	if (ms === undefined) return '?'
	return (ms / 1000).toLocaleString(undefined, { maximumFractionDigits: 3 })
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

export function sleep(ms: number): Promise<void> {
	return new Promise((resolve) => setTimeout(resolve, ms))
}

export function validatePassword(password: string): boolean {
	const re = /^(?=.*[\d])(?=.*[!@#$%^&*])[\w!@#$%^&*]{8,30}$/
	return re.test(password)
}

export function clickOutside(node: Node): { destroy(): void } {
	const handleClick = (event: MouseEvent) => {
		if (node && !node.contains(<HTMLElement>event.target) && !event.defaultPrevented) {
			node.dispatchEvent(new CustomEvent<MouseEvent>('click_outside', { detail: event }))
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
	action?: ((event?: MouseEvent) => Promise<void>) | ((event?: MouseEvent) => void)
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

export async function isOwner(path: string, user: UserExt, workspace: string): Promise<boolean> {
	if (user.is_admin && (workspace != 'starter' || user.is_super_admin)) {
		return true
	} else if (workspace == 'starter') {
		return false
	} else {
		return await UserService.isOwnerOfPath({ path: path, workspace: workspace })
	}
}

export function isObviousOwner(path: string, user?: UserExt): boolean {
	if (!user) {
		return false
	}
	if (user.is_admin || user.is_super_admin) {
		return true
	}
	let userOwner = `u/${user.username}`
	if (path.startsWith(userOwner)) {
		return true
	}
	if (user.pgroups.findIndex((x) => path.startsWith(x)) != -1) {
		return true
	}
	if (user.folders.findIndex((x) => path.startsWith('f/' + x)) != -1) {
		return true
	}
	return false
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
	if (isObviousOwner(path, user)) {
		return true
	}
	let userOwner = `u/${user.username}`
	if (keys.includes(userOwner) && extra_perms[userOwner]) {
		return true
	}
	if (user.pgroups.findIndex((x) => keys.includes(x) && extra_perms[x]) != -1) {
		return true
	}
	if (user.folders.findIndex((x) => path.startsWith('f/' + x)) != -1) {
		return true
	}
	return false
}

export function emptyString(str: string | undefined | null): boolean {
	return str === undefined || str === null || str === ''
}

export function defaultIfEmptyString(str: string | undefined, dflt: string): string {
	return emptyString(str) ? dflt : str!
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

export function pathIsEmpty(path: string): boolean {
	return path == undefined || path.split('/')[2] == ''
}

export function encodeState(state: any): string {
	return btoa(encodeURIComponent(JSON.stringify(state)))
}

export function decodeState(query: string): any {
	return JSON.parse(decodeURIComponent(atob(query)))
}

export function decodeArgs(queryArgs: string | undefined): any {
	if (queryArgs) {
		const parsed = decodeState(queryArgs)
		Object.entries(parsed).forEach(([k, v]) => {
			if (v == '<function call>') {
				parsed[k] = undefined
			}
		})
		return parsed
	}
	return {}
}

export async function setQuery(url: URL, key: string, value: string): Promise<void> {
	url.searchParams.set(key, value)
	await goto(`?${url.searchParams.toString()}`)
}

export function setQueryWithoutLoad(url: URL, key: string, value: string): void {
	const nurl = new URL(url.toString())
	nurl.searchParams.set(key, value)
	try {
		history.replaceState(history.state, '', nurl.toString())
	} catch (e) {
		console.error(e)
	}
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

export function removeMarkdown(text: string): string {
	return text.replace(/[[\*|\-|#\_]/g, '')
}
export function truncate(s: string, n: number, suffix: string = '...'): string {
	if (!s) {
		return ''
	}
	if (s.length <= n) {
		return s
	} else {
		return s.substring(0, n) + suffix
	}
}

export function truncateRev(s: string, n: number, prefix: string = '...'): string {
	if (!s) {
		return prefix
	}
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

export function schemaToTsType(schema: Schema): string {
	if (!schema || !schema.properties) {
		return 'any'
	}
	const propKeys = Object.keys(schema.properties)

	const types = propKeys
		.map((key: string) => {
			const prop = schema.properties[key]
			const isOptional = !schema.required.includes(key)
			const prefix = `${key}${isOptional ? '?' : ''}`
			let type: string = 'any'
			if (prop.type === 'string') {
				type = 'string'
			} else if (prop.type === 'number' || prop.type === 'integer') {
				type = 'number'
			} else if (prop.type === 'boolean') {
				type = 'boolean'
			} else if (prop.type === 'array') {
				let type = prop.items?.type ?? 'any'
				if (type === 'integer') {
					type = 'number'
				}
				type = `${type}[]`
			}

			return `${prefix}: ${type}`
		})
		.join(';')

	return `{ ${types} }`
}

export function schemaToObject(schema: Schema, args: Record<string, any>): Object {
	const object = {}
	if (!schema || !schema.properties) {
		return object
	}
	const propKeys = Object.keys(schema.properties)

	propKeys.forEach((key: string) => {
		object[key] = args[key] ?? null
	})
	return object
}

export type InputCat =
	| 'string'
	| 'number'
	| 'boolean'
	| 'list'
	| 'resource-object'
	| 'enum'
	| 'date'
	| 'base64'
	| 'resource-string'
	| 'object'
	| 'sql'

export function setInputCat(
	type: string | undefined,
	format: string | undefined,
	itemsType: string | undefined,
	enum_: any,
	contentEncoding: string | undefined
): InputCat {
	if (type === 'number' || type === 'integer') {
		return 'number'
	} else if (type === 'boolean') {
		return 'boolean'
	} else if (type == 'array' && itemsType != undefined) {
		return 'list'
	} else if (type == 'object' && format?.startsWith('resource')) {
		return 'resource-object'
	} else if (!type || type == 'object' || type == 'array') {
		return 'object'
	} else if (type == 'string' && enum_) {
		return 'enum'
	} else if (type == 'string' && format == 'date-time') {
		return 'date'
	} else if (type == 'string' && format == 'sql') {
		return 'sql'
	} else if (type == 'string' && contentEncoding == 'base64') {
		return 'base64'
	} else if (type == 'string' && format?.startsWith('resource')) {
		return 'resource-string'
	} else {
		return 'string'
	}
}

export function scriptPathToHref(path: string): string {
	if (path.startsWith('hub/')) {
		return 'https://hub.windmill.dev/from_version/' + path.substring(4)
	} else {
		return `/scripts/get/${path}`
	}
}

export async function getScriptByPath(path: string): Promise<{
	content: string
	language: SupportedLanguage
	schema: any
}> {
	if (path.startsWith('hub/')) {
		const { content, language, schema } = await ScriptService.getHubScriptByPath({ path })

		return {
			content,
			language: language as SupportedLanguage,
			schema
		}
	} else {
		const script = await ScriptService.getScriptByPath({
			workspace: get(workspaceStore)!,
			path: path ?? ''
		})
		return {
			content: script.content,
			language: script.language,
			schema: script.schema
		}
	}
}

export async function loadHubScripts() {
	try {
		const scripts = (await ScriptService.listHubScripts()).asks ?? []
		const processed = scripts
			.map((x) => ({
				path: `hub/${x.id}/${x.app}/${x.summary.toLowerCase().replaceAll(/\s+/g, '_')}`,
				summary: `${x.summary} (${x.app})`,
				approved: x.approved,
				kind: x.kind,
				app: x.app,
				views: x.views,
				votes: x.votes,
				ask_id: x.ask_id
			}))
			.sort((a, b) => b.views - a.views)
		hubScripts.set(processed)
	} catch {
		console.error('Hub is not available')
	}
}

export async function loadHubFlows() {
	try {
		const flows = (await FlowService.listHubFlows()).flows ?? []
		const processed = flows.sort((a, b) => b.votes - a.votes)
		return processed
	} catch {
		console.error('Hub is not available')
	}
}

export function formatCron(inp: string): string {
	// Allow for cron expressions inputted by the user to omit month and year
	let splitted = inp.split(' ')
	splitted = splitted.filter(String) //remove empty string elements
	if (6 - splitted.length > 0) {
		return splitted.concat(Array(6 - splitted.length).fill('*')).join(' ')
	} else {
		return inp
	}
}

export function flowToHubUrl(flow: Flow): URL {
	const url = new URL('https://hub.windmill.dev/flows/add')
	const openFlow = {
		value: flow.value,
		summary: flow.summary,
		description: flow.description,
		schema: flow.schema
	}
	url.searchParams.append('flow', encodeState(openFlow))
	return url
}

export function scriptToHubUrl(
	content: string,
	summary: string,
	description: string,
	kind: Script.kind,
	language: Script.language,
	schema: Schema | undefined,
	lock: string | undefined
): URL {
	const url = new URL('https://hub.windmill.dev/scripts/add')

	url.searchParams.append('content', content)
	url.searchParams.append('summary', summary)
	url.searchParams.append('description', description)
	url.searchParams.append('kind', kind)
	url.searchParams.append('language', language)
	url.searchParams.append('schema', JSON.stringify(schema, null, 2))
	lock && url.searchParams.append('lockfile', lock)

	return url
}

export function classNames(...classes: Array<string | undefined>): string {
	return classes.filter(Boolean).join(' ')
}

export function scriptLangToEditorLang(
	lang: Script.language
): 'typescript' | 'python' | 'go' | 'shell' {
	if (lang == 'deno') {
		return 'typescript'
	} else if (lang == 'python3') {
		return 'python'
	} else if (lang == 'bash') {
		return 'shell'
	} else {
		return lang
	}
}

export async function copyToClipboard(value: string, sendToast = true): Promise<boolean> {
	let success = false
	if (navigator?.clipboard) {
		success = await navigator.clipboard
			.writeText(value)
			.then(() => true)
			.catch(() => false)
	}
	sendToast &&
		sendUserToast(success ? 'Copied to clipboard!' : "Couldn't copy to clipboard", !success)
	return success
}

export function pluralize(quantity: number, word: string, customPlural?: string) {
	if (quantity <= 1) {
		return `${quantity} ${word}`
	} else if (customPlural) {
		return `${quantity} ${customPlural}}`
	} else {
		return `${quantity} ${word}s`
	}
}

export function capitalize(word: string): string {
	return word ? word.charAt(0).toUpperCase() + word.slice(1) : ''
}

export function isCloudHosted(): boolean {
	return get(page).url.hostname == 'app.windmill.dev'
}

export function isObject(obj: any) {
	return typeof obj === 'object'
}
