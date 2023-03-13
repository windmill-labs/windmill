import { goto } from '$app/navigation'
import type { World } from '../../rx'

export function computeGlobalContext(
	world: World | undefined,
	id: string | undefined,
	extraContext: any = {}
) {
	return {
		...Object.fromEntries(
			Object.entries(world?.outputsById ?? {})
				.filter(([k, _]) => k != id)
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
return async function (context, state, goto) {
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
): (context, state, goto) => Promise<any> {
	let template = create_context_function_template(eval_string, context, noReturn)
	let functor = Function(template)
	return functor()
}

export async function eval_like(text, context = {}, noReturn: boolean = true, state: any = {}) {
	let evaluator = make_context_evaluator(text, context, noReturn)
	return await evaluator(context, state, async (x) => {
		await goto(x)
	})
}
