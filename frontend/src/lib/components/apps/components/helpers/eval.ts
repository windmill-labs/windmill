import { isPlainObject } from 'lodash'
import type { World } from '../../rx'
import { sendUserToast } from '$lib/toast'

export function computeGlobalContext(world: World | undefined, extraContext: any = {}) {
	return {
		...Object.fromEntries(
			Object.entries(world?.outputsById ?? {})
				.filter(([k, _]) => k != 'state')
				.map(([key, value]) => {
					return [
						key,
						Object.fromEntries(Object.entries(value ?? {}).map((x) => [x[0], x[1].peak()]))
					]
				})
		),
		...extraContext
	}
}

function create_context_function_template(
	eval_string: string,
	contextKeys: string[],
	noReturn: boolean
) {
	let hasReturnAsLastLine = noReturn || eval_string.split('\n').some((x) => x.startsWith('return '))
	return `
return async function (context, state, goto, setTab, recompute, getAgGrid, setValue, setSelectedIndex, openModal, closeModal, open, close, validate, invalidate, validateAll, clearFiles, showToast) {
"use strict";
${
	contextKeys && contextKeys.length > 0
		? `let ${contextKeys.map((key) => ` ${key} = context['${key}']`)};`
		: ``
}
${
	hasReturnAsLastLine
		? eval_string
		: `
return ${eval_string.startsWith('return ') ? eval_string.substring(7) : eval_string}`
}

}                                                                                                                   
`
}

type WmFunctor = (
	context,
	state,
	goto,
	setTab,
	recompute,
	getAgGrid,
	setValue,
	setSelectedIndex,
	openModal,
	closeModal,
	open,
	close,
	validate,
	invalidate,
	validateAll,
	clearFiles,
	showToast
) => Promise<any>

let functorCache: Record<number, WmFunctor> = {}
function make_context_evaluator(eval_string, contextKeys: string[], noReturn: boolean): WmFunctor {
	let cacheKey = hashCode(JSON.stringify({ eval_string, contextKeys, noReturn }))
	if (functorCache[cacheKey]) {
		return functorCache[cacheKey]
	}
	let template = create_context_function_template(eval_string, contextKeys, noReturn)
	let functor = Function(template)
	let r = functor()
	functorCache[cacheKey] = r
	return r
}

function isSerializable(obj) {
	var isNestedSerializable
	function isPlain(val) {
		return (
			val == null ||
			typeof val === 'undefined' ||
			typeof val === 'string' ||
			typeof val === 'boolean' ||
			typeof val === 'number' ||
			Array.isArray(val) ||
			isPlainObject(val)
		)
	}
	if (!isPlain(obj)) {
		return false
	}
	for (var property in obj) {
		if (obj.hasOwnProperty(property)) {
			if (!isPlain(obj[property])) {
				return false
			}
			if (typeof obj[property] == 'object') {
				isNestedSerializable = isSerializable(obj[property])
				if (!isNestedSerializable) {
					return false
				}
			}
		}
	}
	return true
}

function hashCode(s: string): number {
	var hash = 0,
		i,
		chr
	if (s.length === 0) return hash
	for (i = 0; i < s.length; i++) {
		chr = s.charCodeAt(i)
		hash = (hash << 5) - hash + chr
		hash |= 0 // Convert to 32bit integer
	}
	return hash
}
export async function eval_like(
	text: string,
	context = {},
	state: Record<string, any>,
	editor: boolean,
	controlComponents: Record<
		string,
		{
			setTab?: (index: number) => void
			agGrid?: { api: any; columnApi: any }
			setValue?: (value: any) => void
			setSelectedIndex?: (index: number) => void
			openModal?: () => void
			closeModal?: () => void
			open?: () => void
			close?: () => void
			validate?: (key: string) => void
			invalidate?: (key: string, error: string) => void
			validateAll?: () => void
			clearFiles?: () => void
		}
	>,
	worldStore: World | undefined,
	runnableComponents: Record<string, { cb?: (() => void)[] }>,
	noReturn: boolean
) {
	const proxiedState = new Proxy(state, {
		set(target, key, value) {
			if (typeof key !== 'string') {
				throw new Error('Invalid key')
			}
			target[key] = value
			let o = worldStore?.newOutput('state', key, value)
			if (isSerializable(value)) {
				o?.set(value, true)
			} else {
				o?.set('Not serializable object usable only by frontend scripts', true)
			}
			return true
		}
	})

	let evaluator = make_context_evaluator(text, Object.keys(context ?? {}), noReturn)
	// console.log(i, j)
	return await evaluator(
		context,
		proxiedState,
		async (x, newTab) => {
			if (newTab || editor) {
				if (!newTab) {
					sendUserToast(
						'In editor mode, `goto` opens a new tab to prevent losing your work. To test the redirection , use the preview mode.'
					)
				}
				window.open(x, '_blank')
			} else {
				await newTab(x)
			}
		},
		(id, index) => {
			controlComponents[id]?.setTab?.(index)
		},
		(id) => {
			runnableComponents[id]?.cb?.forEach((f) => f())
		},
		(id) => {
			return controlComponents[id]?.agGrid
		},
		(id, value) => {
			controlComponents[id]?.setValue?.(value)
		},
		(id, index) => {
			controlComponents[id]?.setSelectedIndex?.(index)
		},
		(id) => {
			controlComponents[id]?.openModal?.()
		},
		(id) => {
			controlComponents[id]?.closeModal?.()
		},
		(id) => {
			controlComponents[id]?.open?.()
		},
		(id) => {
			controlComponents[id]?.close?.()
		},
		(id, key) => {
			controlComponents[id]?.validate?.(key)
		},
		(id, key, error) => {
			controlComponents[id]?.invalidate?.(key, error)
		},
		(id) => {
			controlComponents[id]?.validateAll?.()
		},
		(id) => {
			controlComponents[id]?.clearFiles?.()
		},
		(message, error) => {
			sendUserToast(message, error)
		}
	)
}
