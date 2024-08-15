import type { World } from '../../rx'
import { sendUserToast } from '$lib/toast'
import { waitJob } from '$lib/components/waitJob'

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
return async function (context, state, createProxy, goto, setTab, recompute, getAgGrid, setValue, setSelectedIndex, openModal, closeModal, open, close, validate, invalidate, validateAll, clearFiles, showToast, waitJob, askNewResource) {
"use strict";
${
	contextKeys && contextKeys.length > 0
		? `let ${contextKeys.map((key) => ` ${key} = createProxy('${key}', context['${key}'])`)};`
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
	createProxy,
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
	showToast,
	waitJob,
	askNewResource
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
			showToast?: (message: string, error?: boolean) => void
			waitJob?: (jobId: string) => void
			askNewResource?: () => void
			setGroupValue?: (key: string, value: any) => void
		}
	>,
	worldStore: World | undefined,
	runnableComponents: Record<string, { cb?: (() => void)[] }>,
	noReturn: boolean,
	groupContextId: string | undefined
) {
	const createProxy = (name: string, obj: any) => {
		// console.log('Creating proxy', name, obj)
		if (obj != null && obj != undefined && typeof obj == 'object') {
			if (name == 'group' && groupContextId) {
				return createGroupProxy(groupContextId, obj)
			}
			return new Proxy(obj, {
				set(target, key, value) {
					if (name != 'state') {
						throw new Error(
							'Cannot set value on objects that are neither the global state or a container group field'
						)
					}
					if (typeof key !== 'string') {
						throw new Error('Invalid key')
					}
					target[key] = value
					let o = worldStore?.newOutput(name, key, value)
					o?.set(value, true)

					return true
				},
				get(obj, prop) {
					if (name != 'state' && prop == 'group') {
						return createGroupProxy(name, obj[prop])
					} else {
						return obj[prop]
					}
				}
			})
		} else {
			return obj
		}
	}

	const createGroupProxy = (name: string, obj: any) => {
		return new Proxy(obj, {
			set(target, key, value) {
				target[key] = value
				let o = worldStore?.newOutput(name, 'group', target)
				o?.set(target, true)
				if (typeof key !== 'string') {
					throw new Error('Invalid key')
				}
				controlComponents[name]?.setGroupValue?.(key, value)
				return true
			}
		})
	}

	const proxiedState = createProxy('state', state)

	let evaluator = make_context_evaluator(text, Object.keys(context ?? {}), noReturn)
	// console.log(i, j)
	return await evaluator(
		context,
		proxiedState,
		createProxy,
		async (x, newTab) => {
			if (newTab || editor) {
				if (!newTab) {
					sendUserToast(
						'In editor mode, `goto` opens a new tab to prevent losing your work. To test the redirection , use the preview mode.'
					)
				}
				window.open(x, '_blank')
			} else {
				window.location.href = x
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
		},
		async (id) => waitJob(id),
		(id) => {
			controlComponents[id]?.askNewResource?.()
		}
	)
}
