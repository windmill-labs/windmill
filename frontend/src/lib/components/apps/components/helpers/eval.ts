import { sendUserToast } from '$lib/utils'
import { isPlainObject } from 'lodash'
import type { World } from '../../rx'

export function computeGlobalContext(
	world: World | undefined,
	id: string | undefined,
	extraContext: any = {}
) {
	return {
		...Object.fromEntries(
			Object.entries(world?.outputsById ?? {})
				.filter(([k, _]) => k != id && k != 'state')
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

function create_context_function_template(eval_string, context, noReturn: boolean) {
	return `
return async function (context, state, goto, setTab) {
"use strict";
${
	Object.keys(context).length > 0
		? `let ${Object.keys(context).map((key) => ` ${key} = context['${key}']`)};`
		: ``
}
${noReturn ? `return ${eval_string}` : eval_string}
}                                                                                                                   
`
}

function make_context_evaluator(
	eval_string,
	context,
	noReturn: boolean
): (context, state, goto, setTab) => Promise<any> {
	let template = create_context_function_template(eval_string, context, noReturn)
	let functor = Function(template)
	return functor()
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

export async function eval_like(
	text: string,
	context = {},
	noReturn: boolean,
	state: Record<string, any>,
	editor: boolean,
	controlComponents: Record<string, { setTab?: (index: number) => void }>,
	worldStore: World | undefined
) {
	const proxiedState = new Proxy(state, {
		set(target, key, value) {
			if (typeof key !== 'string') {
				throw new Error('Invalid key')
			}
			let o = worldStore?.newOutput('state', key, value)
			if (isSerializable(value)) {
				o?.set(value, true)
			} else {
				o?.set('Not serializable object usable only by frontend scripts', true)
			}
			target[key] = value
			return true
		}
	})
	let evaluator = make_context_evaluator(text, context, noReturn)
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
		}
	)
}
