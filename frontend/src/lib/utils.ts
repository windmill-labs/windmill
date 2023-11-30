// /* eslint-disable @typescript-eslint/explicit-module-boundary-types */
// import { goto } from '$app/navigation'
// import { AppService, type Flow, FlowService, Script, ScriptService, type User } from '$lib/gen'
// import { toast } from '@zerodevx/svelte-toast'
// import type { Schema, SupportedLanguage } from './common'
// import { hubScripts, type UserExt, workspaceStore } from './stores'
// import { page } from '$app/stores'
// import { get } from 'svelte/store'

import { deepEqual } from 'fast-equals'
import YAML from 'yaml'
import type { UserExt } from './stores'
import { sendUserToast } from './toast'
import type { Script } from './gen'
import { cloneDeep } from 'lodash'
export { sendUserToast }

export function validateUsername(username: string): string {
	if (username != '' && !/^[a-zA-Z]\w+$/.test(username)) {
		return 'username can only contain letters and numbers and must start with a letter'
	} else {
		return ''
	}
}

export function parseQueryParams(url: string | undefined) {
	if (!url) return {}
	const index = url.indexOf('?')
	if (index == -1) return {}
	const paramArr = url?.slice(index + 1)?.split('&')
	const params: Record<string, string> = {}
	paramArr?.map((param) => {
		const [key, val] = param.split('=')
		params[key] = decodeURIComponent(val)
	})
	return params
}

export function displayDate(dateString: string | Date | undefined, displaySecond = false): string {
	const date = new Date(dateString ?? '')
	if (date.toString() === 'Invalid Date') {
		return ''
	} else {
		return `${date.toLocaleTimeString([], {
			hour: '2-digit',
			minute: '2-digit',
			second: displaySecond ? '2-digit' : undefined
		})} ${date.getDate()}/${date.getMonth() + 1}`
	}
}

export function displaySize(sizeInBytes: number | undefined): string | undefined {
	if (sizeInBytes === undefined) {
		return undefined
	}
	const units = ['B', 'KiB', 'MiB', 'GiB', 'TiB', 'PiB', 'EiB', 'ZiB', 'YiB']
	let size = sizeInBytes
	let unit_idx = 0
	while (unit_idx < units.length - 1 && size > 1024) {
		size /= 1024
		unit_idx += 1
	}
	return `${size.toFixed(1)}${units[unit_idx]}`
}

export function msToSec(ms: number | undefined, maximumFractionDigits?: number): string {
	if (ms === undefined) return '?'
	return (ms / 1000).toLocaleString(undefined, {
		maximumFractionDigits: maximumFractionDigits ?? 3,
		minimumFractionDigits: maximumFractionDigits
	})
}

export function getToday() {
	var today = new Date()
	return today
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

export function addIfNotExists<T>(e: T, arr: Array<T> | undefined): Array<T> {
	if (!arr) {
		return [e]
	} else if (arr.includes(e)) {
		return arr
	} else {
		return arr.concat([e])
	}
}

export function validatePassword(password: string): boolean {
	const re = /^(?=.*[\d])(?=.*[!@#$%^&*])[\w!@#$%^&*]{8,30}$/
	return re.test(password)
}

export function clickOutside(node: Node, capture?: boolean): { destroy(): void } {
	const handleClick = (event: MouseEvent) => {
		if (node && !node.contains(<HTMLElement>event.target) && !event.defaultPrevented) {
			node.dispatchEvent(new CustomEvent<MouseEvent>('click_outside', { detail: event }))
		}
	}

	document.addEventListener('click', handleClick, capture ?? true)

	return {
		destroy() {
			document.removeEventListener('click', handleClick, capture ?? true)
		}
	}
}

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
	type?: 'action' | 'delete'
	disabled?: boolean
	icon?: any | undefined
}

export const DELETE = 'delete' as 'delete'

export function emptySchema() {
	return {
		$schema: 'https://json-schema.org/draft/2020-12/schema' as string | undefined,
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
	try {
		return JSON.parse(decodeURIComponent(atob(query)))
	} catch (e) {
		sendUserToast('Impossible to parse state', true)
		return {}
	}
}

export function itemsExists<T>(arr: T[] | undefined, item: T): boolean {
	if (!arr) {
		return false
	}
	for (const i of arr) {
		if (deepEqual(i, item)) {
			return true
		}
	}
	return false
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

let debounced: NodeJS.Timeout | undefined = undefined
export function setQueryWithoutLoad(
	url: URL,
	args: { key: string; value: string | null | undefined }[],
	bounceTime?: number
): void {
	debounced && clearTimeout(debounced)
	debounced = setTimeout(() => {
		const nurl = new URL(url.toString())
		for (const { key, value } of args) {
			if (value) {
				nurl.searchParams.set(key, value)
			} else {
				nurl.searchParams.delete(key)
			}
		}

		try {
			history.replaceState(history.state, '', nurl.toString())
		} catch (e) {
			console.error(e)
		}
	}, bounceTime ?? 200)
}

export function groupBy<K, V>(
	items: V[],
	toGroup: (t: V) => K,
	toSort: (t: V) => string,
	dflts: K[] = []
): [K, V[]][] {
	let r: Map<K, V[]> = new Map()
	for (const dflt of dflts) {
		r.set(dflt as K, [])
	}

	items.forEach((sc) => {
		let section = toGroup(sc)
		if (r.has(section)) {
			let arr = r.get(section)!
			arr.push(sc)
			arr.sort((a, b) => toSort(a).localeCompare(toSort(b)))
		} else {
			r.set(section, [sc])
		}
	})

	return [...r.entries()].sort((s1, s2) => {
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

export type InputCat =
	| 'string'
	| 'email'
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
	| 'yaml'
	| 'currency'

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
	} else if (type == 'string' && format == 'yaml') {
		return 'yaml'
	} else if (type == 'string' && contentEncoding == 'base64') {
		return 'base64'
	} else if (type == 'string' && format == 'email') {
		return 'email'
	} else if (type == 'string' && format == 'currency') {
		return 'currency'
	} else {
		return 'string'
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

export function classNames(...classes: Array<string | undefined>): string {
	return classes.filter(Boolean).join(' ')
}

export async function copyToClipboard(value?: string, sendToast = true): Promise<boolean> {
	if (!value) {
		return false
	}

	let success = false
	if (navigator?.clipboard && window.isSecureContext) {
		success = await navigator.clipboard
			.writeText(value)
			.then(() => true)
			.catch(() => false)
	} else {
		const textArea = document.createElement('textarea')
		textArea.value = value
		textArea.style.position = 'fixed'
		textArea.style.left = '-999999px'

		document.body.appendChild(textArea)
		textArea.select()

		try {
			document.execCommand('copy')
			success = true
		} catch (error) {
			// ignore (success = false)
		} finally {
			textArea.remove()
		}
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

export function addWhitespaceBeforeCapitals(word?: string): string {
	if (!word) {
		return ''
	}
	return word.replace(/([A-Z])/g, ' $1').trim()
}

export function isObject(obj: any) {
	return typeof obj === 'object'
}

export function debounce(func: (...args: any[]) => any, wait: number) {
	let timeout: any
	return function (...args: any[]) {
		// @ts-ignore
		const context = this
		clearTimeout(timeout)
		timeout = setTimeout(() => func.apply(context, args), wait)
	}
}

export function throttle<T>(func: (...args: any[]) => T, wait: number) {
	let timeout: any
	return function (...args: any[]) {
		if (!timeout) {
			timeout = setTimeout(() => {
				timeout = null
				// @ts-ignore
				func.apply(this, args)
			}, wait)
		}
	}
}

export function isMac(): boolean {
	return navigator.userAgent.indexOf('Mac OS X') !== -1
}

export function getModifierKey(): string {
	return isMac() ? '⌘' : 'Ctrl'
}

export function isValidHexColor(color: string): boolean {
	return /^#(([A-F0-9]{2}){3,4}|[A-F0-9]{3})$/i.test(color)
}

export function sortObject<T>(o: T & object): T {
	return Object.keys(o)
		.sort()
		.reduce((obj, key) => {
			obj[key] = o[key]
			return obj
		}, {}) as T
}

export function generateRandomString(len: number = 24): string {
	let chars = 'ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789'
	let result = ''

	for (let i = 0; i < len; i++) {
		result += chars.charAt(Math.floor(Math.random() * chars.length))
	}

	return result
}

export function deepMergeWithPriority<T>(target: T, source: T): T {
	if (typeof target !== 'object' || typeof source !== 'object') {
		return source
	}

	const merged = { ...target }

	for (const key in source) {
		if (source.hasOwnProperty(key) && merged?.hasOwnProperty(key)) {
			if (target?.hasOwnProperty(key)) {
				merged[key] = deepMergeWithPriority(target[key], source[key])
			} else {
				merged[key] = source[key]
			}
		}
	}

	return merged
}

export function canWrite(
	path: string,
	extra_perms: Record<string, boolean>,
	user?: UserExt
): boolean {
	if (user?.is_admin || user?.is_super_admin) {
		return true
	}
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

export function isOwner(
	path: string,
	user: UserExt | undefined,
	workspace: string | undefined
): boolean {
	if (!user || !workspace) {
		return false
	}
	if (user.is_super_admin) {
		return true
	}
	if (workspace == 'admin') {
		return false
	} else if (user.is_admin) {
		return true
	} else if (path.startsWith('u/' + user.username + '/')) {
		return true
	} else if (path.startsWith('f/')) {
		return user.folders_owners.some((x) => path.startsWith('f/' + x + '/'))
	} else {
		return false
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

export function extractCustomProperties(styleStr: string): string {
	let properties = styleStr.split(';')
	let customProperties = properties.filter((property) => property.trim().startsWith('--'))
	let customStyleStr = customProperties.join(';')

	return customStyleStr
}

export function toCamel(s: string) {
	return s.replace(/([-_][a-z])/gi, ($1) => {
		return $1.toUpperCase().replace('-', '').replace('_', '')
	})
}

export function cleanExpr(expr: string): string {
	return expr
		.split('\n')
		.filter((x) => x != '' && !x.startsWith(`import `))
		.join('\n')
}

const dynamicTemplateRegex = new RegExp(/\$\{(.*)\}/)

export function isCodeInjection(expr: string | undefined): boolean {
	if (!expr) {
		return false
	}

	return dynamicTemplateRegex.test(expr)
}

export async function tryEvery({
	tryCode,
	timeoutCode,
	interval,
	timeout
}: {
	tryCode: () => Promise<any>
	timeoutCode: () => void
	interval: number
	timeout: number
}) {
	const times = Math.floor(timeout / interval)

	let i = 0
	while (i < times) {
		await sleep(interval)
		try {
			await tryCode()
			break
		} catch (err) {}
		i++
	}
	if (i >= times) {
		timeoutCode()
	}
}

export function roughSizeOfObject(object: object | string) {
	if (typeof object == 'string') {
		return object.length * 2
	}
	var objectList: any[] = []
	var stack = [object]
	var bytes = 0

	while (stack.length) {
		let value: any = stack.pop()

		if (typeof value === 'boolean') {
			bytes += 4
		} else if (typeof value === 'string') {
			bytes += value.length * 2
		} else if (typeof value === 'number') {
			bytes += 8
		} else if (typeof value === 'object' && objectList.indexOf(value) === -1) {
			objectList.push(value)

			for (var i in value) {
				stack.push(value[i])
			}
		}
	}
	return bytes
}

export type Value = {
	language?: Script.language
	content?: string
	path?: string
	draft_only?: boolean
	value?: any
	draft?: Value
	[key: string]: any
}

export function cleanValueProperties(obj: Value) {
	if (typeof obj !== 'object') {
		return obj
	} else {
		let newObj: any = {}
		for (const key of Object.keys(obj)) {
			if (key !== 'parent_hash' && key !== 'draft' && key !== 'draft_only') {
				newObj[key] = cloneDeep(obj[key])
			}
		}
		return newObj
	}
}

export function orderedJsonStringify(obj: any, space?: string | number) {
	const allKeys = new Set()
	JSON.stringify(obj, (key, value) => (allKeys.add(key), value))
	return JSON.stringify(obj, (Array.from(allKeys) as string[]).sort(), space)
}

function sortObjectKeys(obj: any): any {
	if (obj && typeof obj === 'object' && !Array.isArray(obj)) {
		const sortedObj: any = {}
		Object.keys(obj)
			.sort()
			.forEach((key) => {
				sortedObj[key] = sortObjectKeys(obj[key])
			})
		return sortedObj
	} else if (Array.isArray(obj)) {
		return obj.map((item) => sortObjectKeys(item))
	} else {
		return obj
	}
}

export function orderedYamlStringify(obj: any) {
	const sortedObj = sortObjectKeys(obj)
	return YAML.stringify(sortedObj)
}
