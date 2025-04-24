// /* eslint-disable @typescript-eslint/explicit-module-boundary-types */
// import { goto } from '$lib/navigation'
// import { AppService, type Flow, FlowService, Script, ScriptService, type User } from '$lib/gen'
// import { toast } from '@zerodevx/svelte-toast'
// import type { Schema, SupportedLanguage } from './common'
// import { hubScripts, type UserExt, workspaceStore } from './stores'
// import { page } from '$app/stores'
// import { get } from 'svelte/store'

import { deepEqual } from 'fast-equals'
import YAML from 'yaml'
import { type UserExt } from './stores'
import { sendUserToast } from './toast'
import type { Job, Script } from './gen'
import type { EnumType, SchemaProperty } from './common'
import type { Schema } from './common'
export { sendUserToast }
import type { AnyMeltElement } from '@melt-ui/svelte'
import type { RunsSelectionMode } from './components/runs/RunsBatchActionsDropdown.svelte'
import type { TriggerKind } from './components/triggers'

export function isJobCancelable(j: Job): boolean {
	return j.type === 'QueuedJob' && !j.schedule_path && !j.canceled
}
export function isJobReRunnable(j: Job): boolean {
	return (j.job_kind === 'script' || j.job_kind === 'flow') && j.parent_job === undefined
}

export function isJobSelectable(selectionType: RunsSelectionMode) {
	const f: (j: Job) => boolean = {
		cancel: isJobCancelable,
		're-run': isJobReRunnable
	}[selectionType]
	return f
}

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

export function displayDateOnly(dateString: string | Date | undefined): string {
	const date = new Date(dateString ?? '')
	if (date.toString() === 'Invalid Date') {
		return ''
	} else {
		return date.toLocaleDateString([], {
			year: 'numeric',
			month: '2-digit',
			day: '2-digit'
		})
	}
}

export function subtractDaysFromDateString(
	dateString: string | undefined,
	days: number
): string | undefined {
	if (dateString == undefined) {
		return undefined
	}
	let date = new Date(dateString)
	date.setDate(date.getDate() - days)
	return date.toISOString()
}

export function displayDate(
	dateString: string | Date | undefined,
	displaySecond = false,
	displayDate = true
): string {
	const date = new Date(dateString ?? '')
	if (Number.isNaN(date.valueOf())) {
		return ''
	} else {
		const timeChoices: Intl.DateTimeFormatOptions = {
			hour: '2-digit',
			minute: '2-digit',
			second: displaySecond ? '2-digit' : undefined
		}
		const dateChoices: Intl.DateTimeFormatOptions = displayDate
			? {
					day: 'numeric',
					month: 'numeric'
				}
			: {}
		return date.toLocaleString(undefined, {
			...timeChoices,
			...dateChoices
		})
	}
}

export function displayTime(dateString: string | Date | undefined): string {
	const date = new Date(dateString ?? '')
	if (date.toString() === 'Invalid Date') {
		return ''
	} else {
		return `${date.toLocaleTimeString([], {
			hour: '2-digit',
			minute: '2-digit',
			second: '2-digit'
		})}.${date.getMilliseconds()}`
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

export function removeTriggerKindIfUnused(
	length: number,
	triggerKind: TriggerKind,
	usedTriggerKinds: string[]
) {
	if (length === 0 && usedTriggerKinds.includes(triggerKind)) {
		return usedTriggerKinds.filter((kind) => kind != triggerKind)
	}
	return usedTriggerKinds
}

export function msToReadableTime(ms: number | undefined): string {
	if (ms === undefined) return '?'

	const seconds = Math.floor(ms / 1000)
	const minutes = Math.floor(seconds / 60)
	const hours = Math.floor(minutes / 60)
	const days = Math.floor(hours / 24)

	if (days > 0) {
		return `${days}d ${hours % 24}h ${minutes % 60}m ${seconds % 60}s`
	} else if (hours > 0) {
		return `${hours}h ${minutes % 60}m ${seconds % 60}s`
	} else if (minutes > 0) {
		return `${minutes}m ${seconds % 60}s`
	} else {
		return `${msToSec(ms)}s`
	}
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

const portalDivs = ['app-editor-select']

interface ClickOutsideOptions {
	capture?: boolean
	exclude?: (() => Promise<HTMLElement[]>) | HTMLElement[] | undefined
	stopPropagation?: boolean
	customEventName?: string
}

export function clickOutside(
	node: Node,
	options?: ClickOutsideOptions | boolean
): { destroy(): void; update(newOptions: ClickOutsideOptions | boolean): void } {
	const handleClick = async (event: MouseEvent) => {
		const target = event.target as HTMLElement
		const opts = typeof options === 'boolean' ? { capture: options } : options

		let excludedElements: HTMLElement[] = []
		if (opts?.exclude) {
			if (Array.isArray(opts.exclude)) {
				excludedElements = opts.exclude
			} else {
				excludedElements = await opts.exclude()
			}
		}

		const isExcluded = excludedElements.some((excludedEl) => {
			const contains = excludedEl?.contains?.(target)
			const isTarget = target === excludedEl
			return contains || isTarget
		})

		if (node && !node.contains(target) && !event.defaultPrevented && !isExcluded) {
			const portalDivsSelector = portalDivs.map((id) => `#${id}`).join(', ')
			const parent = target.closest(portalDivsSelector)

			if (!parent) {
				if (opts?.stopPropagation) {
					event.stopPropagation()
				}
				node.dispatchEvent(new CustomEvent<MouseEvent>('click_outside', { detail: event }))
			}
		}

		if (opts?.customEventName) {
			node.dispatchEvent(new CustomEvent<MouseEvent>(opts.customEventName, { detail: event }))
		}
	}

	const capture = typeof options === 'boolean' ? options : (options?.capture ?? true)
	document.addEventListener('click', handleClick, capture ?? true)

	return {
		update(newOptions: ClickOutsideOptions | boolean) {
			options = newOptions
		},
		destroy() {
			document.removeEventListener('click', handleClick, capture ?? true)
		}
	}
}

export function pointerDownOutside(
	node: Node,
	options?: ClickOutsideOptions
): { destroy(): void; update(newOptions: ClickOutsideOptions): void } {
	const handlePointerDown = async (event: PointerEvent) => {
		if (!event.isTrusted) return

		if (options?.customEventName) {
			node.dispatchEvent(
				new CustomEvent<PointerEvent>(options.customEventName, {
					detail: event,
					bubbles: true
				})
			)
		}
		const target = event.target as HTMLElement

		let excludedElements: HTMLElement[] = []
		if (options?.exclude) {
			if (Array.isArray(options.exclude)) {
				excludedElements = options.exclude
			} else {
				excludedElements = await options.exclude()
			}
		}

		const isExcluded = excludedElements.some((excludedEl) => {
			const contains = excludedEl?.contains?.(target)
			const isTarget = target === excludedEl
			return contains || isTarget
		})

		if (node && !node.contains(target) && !event.defaultPrevented && !isExcluded) {
			const portalDivsSelector = portalDivs.map((id) => `#${id}`).join(', ')
			const parent = target.closest(portalDivsSelector)

			if (!parent) {
				if (options?.stopPropagation) {
					event.stopPropagation()
				}
				node.dispatchEvent(new CustomEvent<PointerEvent>('pointerdown_outside', { detail: event }))
				return false
			}
		}
	}

	const capture = options?.capture ?? true
	document.addEventListener('pointerdown', handlePointerDown, capture ?? true)

	return {
		update(newOptions: ClickOutsideOptions) {
			options = newOptions
		},
		destroy() {
			document.removeEventListener('pointerdown', handlePointerDown, capture ?? true)
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

export function emptySchema(): Schema {
	return {
		$schema: 'https://json-schema.org/draft/2020-12/schema' as string | undefined,
		properties: {},
		required: [],
		type: 'object'
	}
}

export function simpleSchema(): Schema {
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

export function emptyStringTrimmed(str: string | undefined | null): boolean {
	return str === undefined || str === null || str === '' || str.trim().length === 0
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
	| 'oneOf'
	| 'dynselect'

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
	} else if (type == 'object' && format?.startsWith('dynselect-')) {
		return 'dynselect'
	} else if (!type || type == 'object' || type == 'array') {
		return 'object'
	} else if (type == 'string' && enum_) {
		return 'enum'
	} else if (type == 'string' && format == 'date-time') {
		return 'date'
	} else if (type == 'string' && format == 'date') {
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
	} else if (type == 'oneOf') {
		return 'oneOf'
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

export function cronV1toV2(inp: string): string {
	let splitted = inp.split(' ')
	splitted = splitted.filter(String)
	// subtract 1 from day of week (last element of array if element is a number >= 1 )
	let dowStr = splitted[splitted.length - 1]
	if (dowStr && !isNaN(parseInt(dowStr))) {
		let dow = parseInt(dowStr)
		if (dow > 0) {
			splitted[splitted.length - 1] = (dow - 1).toString()
		}
	}
	return splitted.concat(Array(6 - splitted.length).fill('*')).join(' ')
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
	if (quantity == 1) {
		return `${quantity} ${word}`
	} else if (customPlural) {
		return `${quantity} ${customPlural}}`
	} else {
		return `${quantity} ${word}s`
	}
}

export function addDeterminant(word: string): string {
	return (/^[aeiou]/i.test(word) ? 'an ' : 'a ') + word
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

export function isObject(obj: any): obj is Record<string, any> {
	return obj != null && typeof obj === 'object' && !Array.isArray(obj)
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
	return isMac() ? '⌘' : 'Ctrl+'
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
		} else {
			if (merged) {
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
	let keys = Object.keys(extra_perms ?? {})
	if (!user) {
		return false
	}
	if (isObviousOwner(path, user)) {
		return true
	}
	let userOwner = `u/${user.username}`
	if (keys.includes(userOwner) && extra_perms?.[userOwner]) {
		return true
	}
	if (user.pgroups.findIndex((x) => keys.includes(x) && extra_perms?.[x]) != -1) {
		return true
	}
	if (user.folders.findIndex((x) => path.startsWith('f/' + x + '/') && user.folders[x]) != -1) {
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

export function computeSharableHash(args: any) {
	let nargs = {}
	for (let k in args) {
		let v = args[k]
		if (v !== undefined) {
			// if
			let size = roughSizeOfObject(v) > 1000000
			if (size) {
				console.error(`Value at key ${k} too big (${size}) to be shared`)
				return ''
			}
			nargs[k] = JSON.stringify(v)
		}
	}
	try {
		let r = new URLSearchParams(nargs).toString()
		return r.length > 1000000 ? '' : r
	} catch (e) {
		console.error('Error computing sharable hash', e)
		return ''
	}
}

export function toCamel(s: string) {
	return s.replace(/([-_][a-z])/gi, ($1) => {
		return $1.toUpperCase().replace('-', '').replace('_', '')
	})
}

export function cleanExpr(expr: string | undefined): string {
	if (!expr) {
		return ''
	}
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
				bytes += 2 * i.length
				stack.push(value[i])
			}
		}
	}
	return bytes
}

export type Value = {
	language?: Script['language']
	content?: string
	path?: string
	draft_only?: boolean
	value?: any
	draft?: Value
	[key: string]: any
}

export function replaceFalseWithUndefined(obj: any) {
	return replaceFalseWithUndefinedRec(structuredClone(obj))
}

function replaceFalseWithUndefinedRec(obj: any) {
	// Check if the input is an object and not null
	if (obj !== null && typeof obj === 'object') {
		for (const key in obj) {
			if (obj.hasOwnProperty(key)) {
				// If the value is false, replace it with undefined
				if (obj[key] === false) {
					// delete obj[key];
					obj[key] = undefined
				} else {
					// If the value is an object, call the function recursively
					replaceFalseWithUndefinedRec(obj[key])
				}
			}
		}
	}
	return obj
}

export function cleanValueProperties(obj: Value) {
	if (typeof obj !== 'object') {
		return obj
	} else {
		let newObj: any = {}
		for (const key of Object.keys(obj)) {
			if (key !== 'parent_hash' && key !== 'draft' && key !== 'draft_only') {
				newObj[key] = structuredClone(obj[key])
			}
		}
		return newObj
	}
}

export function orderedJsonStringify(obj: any, space?: string | number) {
	const allKeys = new Set()
	JSON.stringify(
		obj,
		(key, value) => (value != undefined && value != null && allKeys.add(key), value)
	)
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

function evalJs(expr: string) {
	let template = `
return function (fields) {
"use strict";
return ${expr.startsWith('return ') ? expr.substring(7) : expr}
}
`
	let functor = Function(template)
	return functor()
}
export function computeShow(argName: string, expr: string | undefined, args: any) {
	if (expr) {
		try {
			let r = evalJs(expr)(args ?? {})
			return r
		} catch (e) {
			console.error(`Impossible to eval ${expr}:`, e)
			return true
		}
	}
	return true
}

function urlizeTokenInternal(token: string, formatter: 'html' | 'md'): string {
	if (token.startsWith('http://') || token.startsWith('https://')) {
		if (formatter == 'html') {
			return `<a href="${token}" target="_blank" rel="noopener noreferrer">${token}</a>`
		} else {
			return `[${token}](${token})`
		}
	} else {
		return token
	}
}

export function urlize(input: string, formatter: 'html' | 'md'): string {
	if (!input) return ''

	return input
		.split('\n')
		.map((line) => {
			return line
				.split(' ')
				.map((word) => urlizeTokenInternal(word, formatter))
				.join(' ')
		})
		.join('\n')
}

export function storeLocalSetting(name: string, value: string | undefined) {
	if (value != undefined) {
		localStorage.setItem(name, value)
	} else {
		localStorage.removeItem(name)
	}
}

export function getLocalSetting(name: string) {
	try {
		return localStorage.getItem(name)
	} catch (e) {
		return undefined
	}
}

export function computeKind(
	enum_: EnumType,
	contentEncoding: 'base64' | 'binary' | undefined,
	pattern: string | undefined,
	format: string | undefined
): 'base64' | 'none' | 'pattern' | 'enum' | 'resource' | 'format' | 'date-time' {
	if (enum_ != undefined) {
		return 'enum'
	}
	if (contentEncoding == 'base64') {
		return 'base64'
	}
	if (pattern != undefined) {
		return 'pattern'
	}
	if (format == 'date-time') {
		return 'date-time'
	}
	if (format != undefined && format != '') {
		if (format?.startsWith('resource')) {
			return 'resource'
		}
		return 'format'
	}
	return 'none'
}

// Used to check whether a placeholder should be displayed in the input field, based on the schema
export function shouldDisplayPlaceholder(
	type: string | undefined,
	format: string | undefined,
	enum_: EnumType,
	contentEncoding: 'base64' | 'binary' | undefined,
	pattern: string | undefined,
	extra: Record<string, any> | undefined
): boolean {
	if (type === 'string') {
		const kind = computeKind(enum_, contentEncoding, pattern, format)

		if (kind === 'format' && format) {
			const whiteList = ['email', 'hostname', 'ipv4', 'uri', 'uuid']
			return whiteList.includes(format)
		}

		return kind === 'none' || kind === 'pattern'
	}

	if (type === 'number' || type === 'integer') {
		return extra?.['min'] === undefined || extra?.['max'] === undefined
	}

	return type === undefined
}

export function getSchemaFromProperties(properties: { [name: string]: SchemaProperty }): Schema {
	return {
		properties: Object.fromEntries(Object.entries(properties).filter(([k, v]) => k !== 'label')),
		required: Object.keys(properties).filter((k) => properties[k].required),
		$schema: '',
		type: 'object',
		order: Object.keys(properties).filter((k) => k !== 'label')
	}
}

export function validateFileExtension(ext: string) {
	const validExtensionRegex = /^[a-zA-Z0-9]+([._][a-zA-Z0-9]+)*$/
	return validExtensionRegex.test(ext)
}

export function isFlowPreview(job_kind: Job['job_kind'] | undefined) {
	return !!job_kind && (job_kind === 'flowpreview' || job_kind === 'flownode')
}

export function isNotFlow(job_kind: Job['job_kind'] | undefined) {
	return job_kind !== 'flow' && job_kind !== 'singlescriptflow' && !isFlowPreview(job_kind)
}

export function isScriptPreview(job_kind: Job['job_kind'] | undefined) {
	return (
		!!job_kind && (job_kind === 'preview' || job_kind === 'flowscript' || job_kind === 'appscript')
	)
}

export function conditionalMelt(node: HTMLElement, meltItem: AnyMeltElement | undefined) {
	if (meltItem) {
		return meltItem(node)
	}
	return { destroy: () => {} }
}

export type Item = {
	displayName: string
	action?: (e: MouseEvent) => void
	icon?: any
	iconColor?: string
	href?: string
	disabled?: boolean
	type?: 'action' | 'delete'
	hide?: boolean | undefined
}

export function isObjectTooBig(obj: any): boolean {
	const MAX_DEPTH = 10
	const MAX_ITEMS = 50

	function analyze(obj: any, currentDepth: number = 0): { totalItems: number; maxDepth: number } {
		if (currentDepth > MAX_DEPTH) {
			return { totalItems: 1, maxDepth: currentDepth }
		}

		if (typeof obj !== 'object' || obj === null) {
			return { totalItems: 1, maxDepth: currentDepth }
		}

		let totalItems = 1
		let maxDepth = currentDepth

		for (const key in obj) {
			const result = analyze(obj[key], currentDepth + 1)

			if (result.maxDepth > MAX_DEPTH) {
				return result
			}
			totalItems += result.totalItems
			if (result.maxDepth > maxDepth) {
				maxDepth = result.maxDepth
			}
		}

		return { totalItems, maxDepth }
	}

	const { totalItems, maxDepth } = analyze(obj)
	return maxDepth > MAX_DEPTH || totalItems > MAX_ITEMS
}

export function localeConcatAnd(items: string[]) {
	if (!items.length) return ''
	if (items.length === 1) return items[0]
	return items.slice(0, -1).join(', ') + ' and ' + items[items.length - 1]
}

export function formatDate(dateString: string | undefined): string {
	if (!dateString) return ''
	const date = new Date(dateString)
	return new Intl.DateTimeFormat('en-US', {
		year: 'numeric',
		month: 'short',
		day: 'numeric',
		hour: '2-digit',
		minute: '2-digit'
	}).format(date)
}

export function formatDateShort(dateString: string | undefined): string {
	if (!dateString) return ''
	const date = new Date(dateString)
	const now = new Date()

	// If date is today, only show time
	if (date.toDateString() === now.toDateString()) {
		return new Intl.DateTimeFormat('en-US', {
			hour: '2-digit',
			minute: '2-digit'
		}).format(date)
	}

	// If date is this year, show only month and day
	if (date.getFullYear() === now.getFullYear()) {
		return new Intl.DateTimeFormat('en-US', {
			month: 'short',
			day: 'numeric'
		}).format(date)
	}

	// If date is from another year, only show the date with year
	return new Intl.DateTimeFormat('en-US', {
		year: 'numeric',
		month: 'short',
		day: 'numeric'
	}).format(date)
}

export function getOS() {
	const userAgent = window.navigator.userAgent
	const platform = window.navigator.platform
	const macosPlatforms = ['Macintosh', 'MacIntel', 'MacPPC', 'Mac68K']
	const windowsPlatforms = ['Win32', 'Win64', 'Windows', 'WinCE']
	const iosPlatforms = ['iPhone', 'iPad', 'iPod']
	if (macosPlatforms.includes(platform)) {
		return 'macOS' as const
	} else if (iosPlatforms.includes(platform)) {
		return 'iOS' as const
	} else if (windowsPlatforms.includes(platform)) {
		return 'Windows' as const
	} else if (/Android/.test(userAgent)) {
		return 'Android' as const
	} else if (/Linux/.test(platform)) {
		return 'Linux' as const
	}

	return 'Unknown OS' as const
}
