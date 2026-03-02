// /* eslint-disable @typescript-eslint/explicit-module-boundary-types */
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
import type { Job, RunnableKind, Script, ScriptLang, Retry } from './gen'
import type { EnumType, SchemaProperty } from './common'
import type { Schema } from './common'
export { sendUserToast }
import type { AnyMeltElement } from '@melt-ui/svelte'
import type { TriggerKind } from './components/triggers'
import { stateSnapshot } from './svelte5Utils.svelte'
export namespace OpenApi {
	export enum OpenApiVersion {
		V2,
		V3,
		V3_1
	}

	export function isV2(doc: OpenAPI.Document): doc is OpenAPIV2.Document {
		return 'swagger' in doc && doc.swagger === '2.0'
	}

	export function isV3(doc: OpenAPI.Document): doc is OpenAPIV3.Document {
		return 'openapi' in doc && typeof doc.openapi === 'string' && doc.openapi.startsWith('3.0')
	}

	export function isV3_1(doc: OpenAPI.Document): doc is OpenAPIV3_1.Document {
		return 'openapi' in doc && typeof doc.openapi === 'string' && doc.openapi.startsWith('3.1')
	}

	export function getOpenApiVersion(version: string): OpenApiVersion {
		if (version.startsWith('2.0')) {
			return OpenApiVersion.V2
		} else if (version.startsWith('3.0')) {
			return OpenApiVersion.V3
		} else {
			return OpenApiVersion.V3_1
		}
	}

	/**
	 * Parses and validates an OpenAPI specification provided as a string in either JSON or YAML format.
	 *
	 * @param api - A string containing a valid OpenAPI specification in JSON or YAML format.
	 * @returns A promise that resolves to a tuple:
	 *   - The first element is the validated OpenAPI document.
	 *   - The second element is the detected OpenAPI version (2, 3.0, or 3.1).
	 *
	 * @throws Will throw an error if the specification is invalid or cannot be parsed.
	 */
	export async function parse(api: string): Promise<[OpenAPI.Document, OpenApiVersion]> {
		const { validate, dereference } = await import('@scalar/openapi-parser')
		const { valid, errors } = await validate(api)

		if (!valid) {
			const errorMessage = errors
				? errors.map((error) => error.message).join('\n')
				: 'Invalid OpenAPI document'
			throw new Error(errorMessage)
		}

		const document = await dereference(api)

		const version = getOpenApiVersion(document.version!)

		return [document.schema, version]
	}
}

export function isJobCancelable(j: Job): boolean {
	return j.type === 'QueuedJob' && !j.schedule_path && !j.canceled
}

export function isJobReRunnable(j: Job): boolean {
	return (j.job_kind === 'script' || j.job_kind === 'flow') && j.parent_job === undefined
}

export const WORKER_NAME_PREFIX = 'wk'

export function escapeHtml(unsafe: string) {
	return unsafe
		.replace(/&/g, '&amp;')
		.replace(/</g, '&lt;')
		.replace(/>/g, '&gt;')
		.replace(/"/g, '&quot;')
		.replace(/'/g, '&#039;')
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

export function retrieveCommonWorkerPrefix(workerName: string): string {
	const lastDashIndex = workerName.lastIndexOf('-')

	return workerName.substring(0, lastDashIndex)
}

export function subtractDaysFromDateString(
	dateString: string | null,
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

export function msToReadableTime(ms: number | undefined, maximumFractionDigits?: number): string {
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
		return `${msToSec(ms, maximumFractionDigits)}s`
	}
}

export function msToReadableTimeShort(
	ms: number | undefined,
	maximumFractionDigits?: number
): string {
	if (ms === undefined) return '?'

	const seconds = Math.floor(ms / 1000)
	const minutes = Math.floor(seconds / 60)
	const hours = Math.floor(minutes / 60)
	const days = Math.floor(hours / 24)

	if (days > 0) {
		return `${days}d`
	} else if (hours > 0) {
		return `${hours}h`
	} else if (minutes > 0) {
		return `${minutes}m`
	} else {
		return `${msToSec(ms, maximumFractionDigits)}s`
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

const portalDivs = ['#app-editor-select', '.select-dropdown-portal']

interface ClickOutsideOptions {
	capture?: boolean
	exclude?: (() => Promise<HTMLElement[]>) | HTMLElement[] | undefined
	stopPropagation?: boolean
	customEventName?: string
	eventToListenName?: 'click' | 'pointerdown'
	// on:click_outside cannot be used with svelte 5
	onClickOutside?: (event: MouseEvent) => void
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
			const portalDivsSelector = portalDivs.join(', ')
			const parent = target.closest(portalDivsSelector)

			if (!parent) {
				if (opts?.stopPropagation) {
					event.stopPropagation()
				}
				node.dispatchEvent(new CustomEvent<MouseEvent>('click_outside', { detail: event }))
				if (typeof options === 'object') options.onClickOutside?.(event)
			}
		}

		if (opts?.customEventName) {
			node.dispatchEvent(new CustomEvent<MouseEvent>(opts.customEventName, { detail: event }))
		}
	}

	const capture = typeof options === 'boolean' ? options : (options?.capture ?? true)
	const eventToListenName = (typeof options === 'object' && options.eventToListenName) || 'click'
	document.addEventListener(eventToListenName, handleClick, capture ?? true)

	return {
		update(newOptions: ClickOutsideOptions | boolean) {
			options = newOptions
		},
		destroy() {
			document.removeEventListener(eventToListenName, handleClick, capture ?? true)
		}
	}
}

export function undefinedIfEmpty(obj: any): any {
	if (Object.keys(obj).length === 0) {
		return undefined
	}
	return obj
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
			const portalDivsSelector = portalDivs.join(', ')
			const parent = target.closest(portalDivsSelector)

			if (!parent) {
				if (options?.stopPropagation) {
					event.stopPropagation()
				}
				node.dispatchEvent(new CustomEvent<PointerEvent>('pointerdown_outside', { detail: event }))
				if (typeof options === 'object') options.onClickOutside?.(event)
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
	| 'dynamic'
	| 'json-schema'
	| 'ai-provider'

export namespace DynamicInput {
	const DYN_FORMAT_PREFIX = ['dynmultiselect-', 'dynselect-']

	export type HelperScript =
		| { source: 'deployed'; path: string; runnable_kind: RunnableKind }
		| { source: 'inline'; code: string; lang: ScriptLang }

	export const generatePythonFnTemplate = (functionName: string): string => {
		return `
def ${functionName}():
    return [
        { "label": "Foo", "value": "foo" },
        { "label": "Bar", "value": "bar" }
    ]

`
	}

	export const generateJsFnTemplate = (functionName: string): string => {
		return `
// you can use filterText to filter the results from the backend
// you can refer to other args directly as parameters (e.g. foobar: string)
export function ${functionName}(filterText: string) {
	return [
		{ label: 'Foo', value: 'foo' },
		{ label: 'Bar', value: 'bar' }
	];
}

`
	}

	export const generateDefaultTemplateFn = (functionName: string, lang: ScriptLang): string => {
		return lang === 'bun'
			? generateJsFnTemplate(functionName)
			: generatePythonFnTemplate(functionName)
	}

	export const getGenerateTemplateFn = (lang: ScriptLang) => {
		return lang === 'bun' ? generateJsFnTemplate : generatePythonFnTemplate
	}

	export const isDynInputFormat = (format?: string) => {
		if (!format) return false
		return DYN_FORMAT_PREFIX.some((format_prefix) => format.startsWith(format_prefix))
	}
}

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
	} else if (type == 'object' && format == 'ai-provider') {
		return 'ai-provider'
	} else if (type == 'object' && DynamicInput.isDynInputFormat(format)) {
		return 'dynamic'
	} else if (!type || type == 'object' || type == 'array') {
		return 'object'
	} else if (type == 'string' && enum_) {
		return 'enum'
	} else if (type == 'string' && ['date-time', 'naive-date-time', 'date'].includes(format!)) {
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

export function scriptLangArrayToCommaList(languages: ScriptLang[]): string {
	return languages.join(',')
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

export function download(filename: string, fileContent: string, mimeType?: string) {
	const blob = new Blob([fileContent], {
		type: mimeType
	})
	const url = window.URL.createObjectURL(blob)
	const a = document.createElement('a')
	a.href = url
	a.download = filename
	a.click()
	setTimeout(() => URL.revokeObjectURL(url), 100)
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
		return `${quantity} ${customPlural}`
	} else {
		return `${quantity} ${word}s`
	}
}

export function addDeterminant(word: string): string {
	return (/^[aeiou]/i.test(word) ? 'an ' : 'a ') + word
}

export { capitalize } from './sharedUtils'

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
	return {
		debounced: function (...args: any[]) {
			// @ts-ignore
			const context = this
			clearTimeout(timeout)
			timeout = setTimeout(() => func.apply(context, args), wait)
		},
		clearDebounce: () => clearTimeout(timeout)
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

/**
 * Calculates the relative luminance of a color according to WCAG 2.1
 * @param r Red component (0-255)
 * @param g Green component (0-255)
 * @param b Blue component (0-255)
 * @returns Relative luminance value (0-1)
 */
function getRelativeLuminance(r: number, g: number, b: number): number {
	const [rs, gs, bs] = [r, g, b].map((val) => {
		val = val / 255
		return val <= 0.03928 ? val / 12.92 : Math.pow((val + 0.055) / 1.055, 2.4)
	})
	return 0.2126 * rs + 0.7152 * gs + 0.0722 * bs
}

/**
 * Converts hex color to HSL
 * @param hex Hex color string (e.g., "#FF0000")
 * @returns Array of [hue (0-360), saturation (0-100), lightness (0-100)]
 */
function hexToHsl(hex: string): [number, number, number] {
	// Normalize hex color
	let normalizedHex = hex.replace('#', '')
	if (normalizedHex.length === 3) {
		normalizedHex = normalizedHex
			.split('')
			.map((char) => char + char)
			.join('')
	}

	const r = parseInt(normalizedHex.substring(0, 2), 16) / 255
	const g = parseInt(normalizedHex.substring(2, 4), 16) / 255
	const b = parseInt(normalizedHex.substring(4, 6), 16) / 255

	const max = Math.max(r, g, b)
	const min = Math.min(r, g, b)
	let h = 0
	let s = 0
	const l = (max + min) / 2

	if (max !== min) {
		const d = max - min
		s = l > 0.5 ? d / (2 - max - min) : d / (max + min)

		switch (max) {
			case r:
				h = ((g - b) / d + (g < b ? 6 : 0)) / 6
				break
			case g:
				h = ((b - r) / d + 2) / 6
				break
			case b:
				h = ((r - g) / d + 4) / 6
				break
		}
	}

	return [h * 360, s * 100, l * 100]
}

/**
 * Converts HSL to hex color
 * @param h Hue (0-360)
 * @param s Saturation (0-100)
 * @param l Lightness (0-100)
 * @returns Hex color string
 */
function hslToHex(h: number, s: number, l: number): string {
	h = h / 360
	s = s / 100
	l = l / 100

	let r: number, g: number, b: number

	if (s === 0) {
		r = g = b = l // Achromatic
	} else {
		const hue2rgb = (p: number, q: number, t: number) => {
			if (t < 0) t += 1
			if (t > 1) t -= 1
			if (t < 1 / 6) return p + (q - p) * 6 * t
			if (t < 1 / 2) return q
			if (t < 2 / 3) return p + (q - p) * (2 / 3 - t) * 6
			return p
		}

		const q = l < 0.5 ? l * (1 + s) : l + s - l * s
		const p = 2 * l - q
		r = hue2rgb(p, q, h + 1 / 3)
		g = hue2rgb(p, q, h)
		b = hue2rgb(p, q, h - 1 / 3)
	}

	const toHex = (c: number) => {
		const hex = Math.round(c * 255).toString(16)
		return hex.length === 1 ? '0' + hex : hex
	}

	return `#${toHex(r)}${toHex(g)}${toHex(b)}`
}

/**
 * Generates a text color with the same hue as the background but adjusted lightness for good contrast
 * @param backgroundColor Hex color string (e.g., "#FF0000" or "#F00")
 * @returns Hex color string with same hue but good contrast, or undefined if invalid
 */
export function getContrastTextColor(
	backgroundColor: string | null | undefined
): string | undefined {
	if (!backgroundColor || !isValidHexColor(backgroundColor)) {
		return undefined
	}

	// Normalize hex color
	let hex = backgroundColor.replace('#', '')
	if (hex.length === 3) {
		hex = hex
			.split('')
			.map((char) => char + char)
			.join('')
	}

	// Parse RGB components and calculate background luminance
	const r = parseInt(hex.substring(0, 2), 16)
	const g = parseInt(hex.substring(2, 4), 16)
	const b = parseInt(hex.substring(4, 6), 16)
	const bgLuminance = getRelativeLuminance(r, g, b)

	// Convert to HSL to extract hue
	const [hue] = hexToHsl(backgroundColor)

	// Determine if background is light or dark
	const isLightBackground = bgLuminance > 0.5

	// Use fixed saturation and lightness based on background lightness
	// For light backgrounds: use dark text (low lightness, high saturation)
	// For dark backgrounds: use light text (high lightness, high saturation)
	const saturation = 70 // Fixed saturation for good readability
	const lightness = isLightBackground ? 25 : 85 // Dark for light bg, light for dark bg

	// Generate the color with the same hue but adjusted saturation and lightness
	return hslToHex(hue, saturation, lightness)
}

export function sortObject<T>(o: T & object): T {
	return Object.keys(o)
		.sort()
		.reduce((obj, key) => {
			obj[key] = o[key]
			return obj
		}, {}) as T
}

export function sortArray<T>(array: T[], compareFn?: (a: T, b: T) => number): T[] {
	const arr = [...array]
	arr.sort(compareFn)
	return arr
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

export function urlParamsToObject(params: URLSearchParams): Record<string, string> {
	const result: Record<string, string> = {}
	params.forEach((value, key) => {
		result[key] = value
	})
	return result
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
export function roughSizeOfObject(object: object | string | any) {
	if (typeof object === 'string') {
		return object.length * 2
	}

	const visited = new Set<object>()
	const stack = [object]
	let bytes = 0

	while (stack.length) {
		const value = stack.pop()

		if (typeof value === 'boolean') {
			bytes += 4
		} else if (typeof value === 'string') {
			bytes += value.length * 2
		} else if (typeof value === 'number') {
			bytes += 8
		} else if (typeof value === 'object' && value !== null && !visited.has(value)) {
			visited.add(value)

			for (const key in value) {
				bytes += 2 * key.length
				stack.push(value[key])
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
	return replaceFalseWithUndefinedRec(structuredClone(stateSnapshot(obj)))
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
				newObj[key] = structuredClone(stateSnapshot(obj[key]))
			}
		}
		return newObj
	}
}

export function hasUnsavedChanges(saved: Value, modified: Value): boolean {
	const normalizedSaved = cleanValueProperties({ ...saved, path: undefined })
	const normalizedModified = cleanValueProperties({ ...modified, path: undefined })
	return (
		orderedJsonStringify(replaceFalseWithUndefined(normalizedSaved)) !==
		orderedJsonStringify(replaceFalseWithUndefined(normalizedModified))
	)
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
	return job_kind !== 'flow' && job_kind !== 'singlestepflow' && !isFlowPreview(job_kind)
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
	hrefTarget?: '_blank' | '_self' | '_parent' | '_top'
	disabled?: boolean
	type?: 'action' | 'delete'
	hide?: boolean | undefined
	extra?: Snippet
	id?: string
	tooltip?: string
	separatorTop?: boolean
	submenuItems?: Item[]
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

export function formatDateRange(
	start: string | Date | undefined,
	end: string | Date | undefined
): string {
	if (typeof start === 'string') start = new Date(start)
	if (typeof end === 'string') end = new Date(end)

	if (start && end) {
		const differentDays =
			start.getFullYear() !== end.getFullYear() ||
			start.getMonth() !== end.getMonth() ||
			start.getDate() !== end.getDate()

		const differentYears = start.getFullYear() !== end.getFullYear()

		if (differentDays || differentYears) {
			// Clone to avoid mutating originals
			start = new Date(start)
			end = new Date(end)
			// Zero out time for display
			start.setHours(0, 0, 0, 0)
			end.setHours(0, 0, 0, 0)
		}

		if (differentYears) {
			// Zero out month and day for display
			start.setMonth(0, 1)
			end.setMonth(0, 1)
		}

		return `${formatDatePretty(start)} to ${formatDatePretty(end)}`
	}

	if (!end && start) return `After ${formatDatePretty(start)}`
	if (!start && end) return `Before ${formatDatePretty(end)}`
	return 'No input'
}

export function toJsonStr(result: any) {
	try {
		// console.log(result)
		return JSON.stringify(result ?? null, null, 4) ?? 'null'
	} catch (e) {
		return 'error stringifying object: ' + e.toString()
	}
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

import type { Component, Snippet } from 'svelte'
import { OpenAPIV2, type OpenAPI, type OpenAPIV3, type OpenAPIV3_1 } from 'openapi-types'
import type { IPosition } from 'monaco-editor'

export type StateStore<T> = {
	val: T
}

export type ReadFieldsRecursivelyOptions = {
	excludeField?: string[]
}

export function readFieldsRecursively(obj: any, options: ReadFieldsRecursivelyOptions = {}): void {
	if (Array.isArray(obj)) {
		// <= in case a new object is added. should read as undefined
		for (let i = 0; i <= obj.length; i++) {
			if (obj[i] && typeof obj[i] === 'object') {
				readFieldsRecursively(obj[i])
			}
		}
	} else if (obj !== null && typeof obj === 'object') {
		Object.keys(obj).forEach((key) => {
			if (!options.excludeField?.includes(key)) readFieldsRecursively(obj[key], options)
		})
	}
}

export function reorder<T>(items: T[], oldIndex: number, newIndex: number): T[] {
	const updatedItems = [...items]
	const [removedItem] = updatedItems.splice(oldIndex, 1)
	updatedItems.splice(newIndex, 0, removedItem)
	return updatedItems
}

export function scroll_into_view_if_needed_polyfill(elem: Element, centerIfNeeded: boolean = true) {
	const observer = new IntersectionObserver(
		function ([entry]) {
			const ratio = entry.intersectionRatio
			if (ratio < 1) {
				const place = ratio <= 0 && centerIfNeeded ? `center` : `nearest`
				elem.scrollIntoView({
					block: place,
					inline: place
				})
			}
			observer.disconnect()
		},
		{
			root: null, // or specify a scrolling parent if needed
			rootMargin: '0px 1000px', // Essentially making horizontal checks irrelevant
			threshold: 0.1 // Adjust threshold to control when observer should trigger
		}
	)
	observer.observe(elem)

	return observer // return for testing
}

// Structured clone raises an error on $state values and some stuff like Window
// $state.snapshot clones everything but prints warnings for some values (e.g. functions)
import _clone from 'clone'
export function clone<T>(t: T): T {
	return _clone(t)
}

export const editorPositionMap: Record<string, IPosition> = {}

export type S3Uri = `s3://${string}/${string}`
export type S3Object =
	| S3Uri
	| {
			s3: string
			storage?: string
	  }

export function parseS3Object(s3Object: S3Object): { s3: string; storage?: string } {
	if (typeof s3Object === 'object') return s3Object
	const match = s3Object.match(/^s3:\/\/([^/]*)\/(.*)$/)
	return { storage: match?.[1] || undefined, s3: match?.[2] ?? '' }
}

export function formatS3Object(s3Object: S3Object): S3Uri {
	if (typeof s3Object === 'object') return `s3://${s3Object.storage ?? ''}/${s3Object.s3}`
	return s3Object
}

export function isS3Uri(uri: string): uri is S3Uri {
	const match = uri.match(/^s3:\/\/([^/]*)\/(.*)$/)
	return !!match && match.length === 3
}

export function uniqueBy<T>(array: T[], key: (t: T) => any): T[] {
	const seen = new Set()
	return array.filter((item) => {
		const value = key(item)
		if (seen.has(value)) {
			return false
		} else {
			seen.add(value)
			return true
		}
	})
}

export function pruneNullishArray<T>(array: (T | null | undefined)[]): T[] {
	return array.filter((item): item is T => item !== null && item !== undefined)
}

export function assert(msg: string, condition: boolean, value?: any) {
	if (!condition) {
		let m = 'Assertion failed: ' + msg
		if (value) m += '\nValue: ' + JSON.stringify(value, null, 2)
		m += '\nPlease alert the Windmill team about this'
		sendUserToast(m, true)
		console.error(m)
	}
}

export function createCache<Keys extends Record<string, any>, T, InitialKeys extends Keys = Keys>(
	compute: (keys: Keys) => T,
	params?: { maxSize?: number; initial?: InitialKeys; invalidateMs?: number }
): (keys: Keys) => T {
	let cache = new Map<string, { value: T; timestamp: number }>()
	const maxSize = params?.maxSize ?? 15

	if (params?.initial) {
		let key = JSON.stringify(params.initial, Object.keys(params.initial).sort())
		let value = compute(params.initial)
		cache.set(key, { value, timestamp: Date.now() })
	}

	return (keys: Keys) => {
		if (typeof params?.invalidateMs === 'number') {
			for (const [key, entry] of cache.entries()) {
				if (Date.now() - entry.timestamp > params.invalidateMs) {
					cache.delete(key)
				}
			}
		}

		let key = JSON.stringify(keys, Object.keys(keys).sort())
		if (!cache.get(key)) {
			let value = compute(keys)
			cache.set(key, { value, timestamp: Date.now() })

			if (cache.size > maxSize) {
				// remove the oldest entry (first inserted)
				const oldestKey = cache.keys().next().value!
				cache.delete(oldestKey)
			}
		}
		return cache.get(key)!.value
	}
}

export async function wait(ms: number) {
	return new Promise((resolve) => setTimeout(() => resolve(undefined), ms))
}

export function validateRetryConfig(retry: Retry | undefined): string | null {
	if (retry?.exponential?.seconds !== undefined) {
		const seconds = retry.exponential.seconds
		if (typeof seconds !== 'number' || !Number.isInteger(seconds) || seconds < 0) {
			return 'Exponential backoff base (seconds) must be an integer ≥ 0'
		}
	}
	return null
}
export type CssColor = keyof (typeof tokensFile)['tokens']['light']
import tokensFile from './assets/tokens/tokens.json'
import { darkModeName, lightModeName } from './assets/tokens/colorTokensConfig'
import BarsStaggered from './components/icons/BarsStaggered.svelte'
import { GitIcon } from './components/icons'
import { Bot, Code, Package } from 'lucide-svelte'
import type { DbInput } from './components/dbTypes'
export function getCssColor(
	color: CssColor,
	{
		alpha = 1,
		format = 'css-var'
	}: {
		alpha?: number
		format?: 'css-var' | 'hex-dark' | 'hex-light'
	}
): string {
	if (format === 'hex-light') {
		return tokensFile.tokens[lightModeName][color]
	}
	if (format === 'hex-dark') {
		return tokensFile.tokens[darkModeName][color]
	}
	return `rgb(var(--color-${color}) / ${alpha})`
}

export type IconType = Component<{ size?: number }> | typeof import('lucide-svelte').Dot

export function getJobKindIcon(jobKind: Job['job_kind']) {
	if (jobKind === 'flow' || isFlowPreview(jobKind) || jobKind === 'unassigned_flow') {
		return BarsStaggered
	} else if (jobKind === 'deploymentcallback') {
		return GitIcon
	} else if (
		jobKind === 'dependencies' ||
		jobKind === 'appdependencies' ||
		jobKind === 'flowdependencies'
	) {
		return Package
	} else if (
		jobKind === 'script' ||
		isScriptPreview(jobKind) ||
		jobKind === 'script_hub' ||
		jobKind === 'singlestepflow' ||
		jobKind === 'unassigned_script' ||
		jobKind === 'unassigned_singlestepflow'
	) {
		return Code
	} else if (jobKind === 'aiagent') {
		return Bot
	} else if (jobKind) return Code
}

export function chunkBy<T>(array: T[], getKey: (key: T) => string): T[][] {
	const chunks: T[][] = []

	for (const item of array) {
		const key = getKey(item)
		let lastChunk = chunks[chunks.length - 1]

		if (!lastChunk || getKey(lastChunk[0]) !== key) {
			lastChunk = []
			chunks.push(lastChunk)
		}

		lastChunk.push(item)
	}

	return chunks
}

// AI generated
export function getQueryStmtCountHeuristic(query: string): number {
	// Handle empty or whitespace-only strings
	if (query.trim() === '') {
		return 0
	}

	let count = 0
	let currState: 'normal' | 'single-quote' | 'double-quote' | 'line-comment' | 'block-comment' =
		'normal'
	let hasContentAfterLastSemicolon = false

	for (let i = 0; i < query.length; i++) {
		const char = query[i]
		const nextChar = query[i + 1]

		switch (currState) {
			case 'normal':
				if (char === "'") {
					currState = 'single-quote'
					hasContentAfterLastSemicolon = true
				} else if (char === '"') {
					currState = 'double-quote'
					hasContentAfterLastSemicolon = true
				} else if (char === '-' && nextChar === '-') {
					currState = 'line-comment'
					i++ // skip next char
				} else if (char === '/' && nextChar === '*') {
					currState = 'block-comment'
					hasContentAfterLastSemicolon = true
					i++ // skip next char
				} else if (char === ';') {
					count++
					hasContentAfterLastSemicolon = false
				} else if (char !== ' ' && char !== '\t' && char !== '\n' && char !== '\r') {
					// Non-whitespace character means we have content
					hasContentAfterLastSemicolon = true
				}
				break

			case 'single-quote':
				if (char === "'") {
					// In SQL, '' is an escaped single quote
					if (nextChar === "'") {
						i++ // skip the escaped quote
					} else {
						currState = 'normal'
					}
				}
				break

			case 'double-quote':
				if (char === '"') {
					// In SQL, "" is an escaped double quote
					if (nextChar === '"') {
						i++ // skip the escaped quote
					} else {
						currState = 'normal'
					}
				}
				break

			case 'line-comment':
				if (char === '\n') {
					currState = 'normal'
				}
				break

			case 'block-comment':
				if (char === '*' && nextChar === '/') {
					currState = 'normal'
					i++ // skip next char
				}
				break
		}
	}

	// Count implicit final statement if:
	// 1. We're in normal state and query doesn't end with semicolon, OR
	// 2. We're in a quote state (unclosed quote) - there's an implicit statement
	// 3. We're in a block comment state - there's an implicit statement before the unclosed comment
	// 4. We're in a line comment state and we had content after the last semicolon before entering the comment
	const trimmedQuery = query.trimEnd()
	if (currState === 'normal' && trimmedQuery !== '' && !trimmedQuery.endsWith(';')) {
		count++
	} else if (
		currState === 'single-quote' ||
		currState === 'double-quote' ||
		currState === 'block-comment'
	) {
		// Unclosed quote or unclosed block comment means there's an implicit statement
		count++
	} else if (currState === 'line-comment' && hasContentAfterLastSemicolon) {
		// Line comment with content before it means there's an implicit statement
		count++
	}

	return count
}

export function countChars(str: string, char: string): number {
	let count = 0
	for (let i = 0; i < str.length; i++) {
		if (str[i] === char) {
			count++
		}
	}
	return count
}

export function onlyAlphaNumAndUnderscore(str: string): string {
	return str.replace(/[^a-zA-Z0-9_]/g, '')
}

export function buildReactiveObj<T extends object>(fields: {
	[name in keyof T]: [() => T[name], (v: T[name]) => void]
}): T {
	const obj = {} as T
	for (const key in fields) {
		Object.defineProperty(obj, key, {
			get: fields[key][0],
			set: fields[key][1],
			enumerable: true,
			configurable: true
		})
	}
	return obj
}

export function pick<T extends object, K extends keyof T>(obj: T, keys: readonly K[]): Pick<T, K> {
	const result = {} as Pick<T, K>
	for (const key of keys) {
		if (key in obj) {
			result[key] = obj[key]
		}
	}
	return result
}

export function parseDbInputFromAssetSyntax(path: string): DbInput | null {
	const [p1, _p2] = path.split('://')
	const [p2, _p3] = _p2.split('/')
	const [p3, p4] = _p3.split('.')
	return p1 === 'ducklake'
		? { type: 'ducklake', ducklake: p2 || 'main', specificTable: p4 ?? p3 }
		: p1 === 'datatable'
			? {
					type: 'database',
					resourcePath: `datatable://${p2 || 'main'}`,
					resourceType: 'postgresql',
					specificTable: p4 ?? p3,
					specificSchema: p4 ? p3 : undefined
				}
			: null
}

/**
 * Formats memory size in KB to human-readable format with appropriate units
 * @param sizeInKb - Memory size in kilobytes
 * @param includeTooltip - Whether to return tooltip data with precise values
 * @returns Formatted string with appropriate unit (KB, MB, GB) or object with display and tooltip
 */
export function formatMemory(sizeInKb: number): string
export function formatMemory(
	sizeInKb: number,
	includeTooltip: true
): { display: string; tooltip: string }
export function formatMemory(
	sizeInKb: number,
	includeTooltip = false
): string | { display: string; tooltip: string } {
	const precise = `${sizeInKb.toLocaleString()}KB`

	let display: string
	if (sizeInKb >= 1024 * 1024) {
		// Convert to GB for values >= 1GB
		display = `${(sizeInKb / (1024 * 1024)).toFixed(0)}GB`
	} else if (sizeInKb >= 1024) {
		// Convert to MB for values >= 1MB
		display = `${(sizeInKb / 1024).toFixed(0)}MB`
	} else {
		// Keep as KB for smaller values
		display = `${sizeInKb.toFixed(0)}KB`
	}

	if (includeTooltip) {
		return {
			display,
			tooltip: `${precise} (${(sizeInKb / 1024).toFixed(2)}MB)`
		}
	}

	return display
}

export function assignObjInPlace(
	target: Record<string, any>,
	source: Record<string, any>,
	options: { onDelete?: 'Delete' | 'SetNull' } = { onDelete: 'Delete' }
) {
	for (const key in target) {
		if (!(key in source)) {
			if (options?.onDelete === 'Delete') delete target[key]
			else if (options?.onDelete === 'SetNull') target[key] = null
		}
	}
	for (const key in source) target[key] = source[key]
}

export function isUSLocale(): boolean {
	try {
		const locale = Intl.DateTimeFormat().resolvedOptions().locale
		return locale.startsWith('en-US')
	} catch {
		return false
	}
}

export function formatDatePretty(date: Date): string {
	if (!date || isNaN(date.getTime())) return ''

	const now = new Date()
	const year = date.getFullYear()
	const month = date.getMonth() + 1
	const day = date.getDate()
	const hours = date.getHours()
	const minutes = date.getMinutes()

	const isCurrentYear = year === now.getFullYear()
	const isToday = isCurrentYear && month === now.getMonth() + 1 && day === now.getDate()
	const isOnlyYear = month === 1 && day === 1 && hours === 0 && minutes === 0
	const hasTime = hours !== 0 || minutes !== 0

	// If only year is defined (rest is 01/01 00:00)
	if (isOnlyYear) {
		return String(year)
	}

	// Format month/day depending on locale: MM/DD for US, DD/MM otherwise
	const mm = String(month).padStart(2, '0')
	const dd = String(day).padStart(2, '0')
	const monthDay = isUSLocale() ? `${mm}/${dd}` : `${dd}/${mm}`
	// Format time if present (12-hour format with AM/PM)
	let timeStr = ''
	if (hasTime) {
		const isPM = hours >= 12
		const displayHours = hours % 12 || 12
		const displayMinutes = String(minutes).padStart(2, '0')
		timeStr = ` ${displayHours}:${displayMinutes} ${isPM ? 'PM' : 'AM'}`
	}

	// If today and same year, only show time (if present)
	if (isToday) {
		return timeStr ? timeStr.trim() : monthDay
	}

	// If same year, show month/day and time (if present)
	if (isCurrentYear) {
		return `${monthDay}${timeStr}`
	}

	// Otherwise, show full date with year and time (if present)
	return `${monthDay}/${year}${timeStr}`
}

export function parsePrettyDate(text: string): Date | null {
	if (!text) return null

	const now = new Date()
	const currentYear = now.getFullYear()

	// Try parsing as year-only (e.g., "2025")
	if (/^\d{4}$/.test(text)) {
		const year = parseInt(text)
		return new Date(year, 0, 1, 0, 0, 0)
	}

	// Try parsing time-only (e.g., "11:02 AM") - assumes today
	const timeOnlyMatch = text.match(/^(\d{1,2}):(\d{2})\s+(AM|PM)$/i)
	if (timeOnlyMatch) {
		const [, hourStr, minuteStr, meridiem] = timeOnlyMatch
		let hours = parseInt(hourStr)
		const minutes = parseInt(minuteStr)

		if (meridiem.toUpperCase() === 'PM' && hours !== 12) hours += 12
		if (meridiem.toUpperCase() === 'AM' && hours === 12) hours = 0

		return new Date(currentYear, now.getMonth(), now.getDate(), hours, minutes, 0)
	}

	const usLocale = isUSLocale()

	// Parse a "first/second" pair as month/day (US) or day/month (non-US)
	function parseFirstSecond(firstStr: string, secondStr: string): { month: number; day: number } {
		const first = parseInt(firstStr)
		const second = parseInt(secondStr)
		return usLocale ? { month: first - 1, day: second } : { month: second - 1, day: first }
	}

	// Try parsing NN/NN (e.g., "01/04") - assumes current year, no time
	// US: MM/DD, non-US: DD/MM
	const monthDayMatch = text.match(/^(\d{2})\/(\d{2})$/)
	if (monthDayMatch) {
		const { month, day } = parseFirstSecond(monthDayMatch[1], monthDayMatch[2])
		return new Date(currentYear, month, day, 0, 0, 0)
	}

	// Try parsing NN/NN TIME (e.g., "01/04 11:02 AM") - assumes current year with time
	// US: MM/DD TIME, non-US: DD/MM TIME
	const monthDayTimeMatch = text.match(/^(\d{2})\/(\d{2})\s+(\d{1,2}):(\d{2})\s+(AM|PM)$/i)
	if (monthDayTimeMatch) {
		const { month, day } = parseFirstSecond(monthDayTimeMatch[1], monthDayTimeMatch[2])
		let hours = parseInt(monthDayTimeMatch[3])
		const minutes = parseInt(monthDayTimeMatch[4])
		const meridiem = monthDayTimeMatch[5]

		if (meridiem.toUpperCase() === 'PM' && hours !== 12) hours += 12
		if (meridiem.toUpperCase() === 'AM' && hours === 12) hours = 0

		return new Date(currentYear, month, day, hours, minutes, 0)
	}

	// Try parsing NN/NN/YYYY (e.g., "01/04/2025") - no time
	// US: MM/DD/YYYY, non-US: DD/MM/YYYY
	const fullDateMatch = text.match(/^(\d{2})\/(\d{2})\/(\d{4})$/)
	if (fullDateMatch) {
		const { month, day } = parseFirstSecond(fullDateMatch[1], fullDateMatch[2])
		const year = parseInt(fullDateMatch[3])
		return new Date(year, month, day, 0, 0, 0)
	}

	// Try parsing NN/NN/YYYY TIME (e.g., "01/04/2025 3:00 PM")
	// US: MM/DD/YYYY TIME, non-US: DD/MM/YYYY TIME
	const fullDateTimeMatch = text.match(/^(\d{2})\/(\d{2})\/(\d{4})\s+(\d{1,2}):(\d{2})\s+(AM|PM)$/i)
	if (fullDateTimeMatch) {
		const { month, day } = parseFirstSecond(fullDateTimeMatch[1], fullDateTimeMatch[2])
		const year = parseInt(fullDateTimeMatch[3])
		let hours = parseInt(fullDateTimeMatch[4])
		const minutes = parseInt(fullDateTimeMatch[5])
		const meridiem = fullDateTimeMatch[6]

		if (meridiem.toUpperCase() === 'PM' && hours !== 12) hours += 12
		if (meridiem.toUpperCase() === 'AM' && hours === 12) hours = 0

		return new Date(year, month, day, hours, minutes, 0)
	}

	// Fallback to standard Date parsing (e.g., ISO strings)
	const date = new Date(text)
	return isNaN(date.getTime()) ? null : date
}
