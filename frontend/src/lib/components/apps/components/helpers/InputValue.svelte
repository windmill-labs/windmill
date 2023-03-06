<script lang="ts">
	import { isCodeInjection } from '$lib/components/flows/utils'
	import { deepEqual } from 'fast-equals'
	import { getContext } from 'svelte'
	import type { AppInput, EvalAppInput, UploadAppInput } from '../../inputType'
	import type { AppEditorContext } from '../../types'
	import { accessPropertyByPath } from '../../utils'

	type T = string | number | boolean | Record<string | number, any> | undefined

	export let input: AppInput
	export let value: T
	export let id: string | undefined = undefined
	export let error: string = ''

	let lastInput = input ? JSON.parse(JSON.stringify(input)) : undefined

	$: if (input && !deepEqual(input, lastInput)) {
		lastInput = JSON.parse(JSON.stringify(input))
		// Needed because of file uploads
		if (input?.['value'] instanceof ArrayBuffer) {
			lastInput.value = input?.['value']
		}
	}

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')

	$: state = $worldStore?.state

	let timeout: NodeJS.Timeout | undefined = undefined
	const debounce_ms = 50
	function debounce(cb: () => void) {
		if (timeout) {
			clearTimeout(timeout)
		}
		timeout = setTimeout(cb, debounce_ms)
	}

	$: lastInput && $worldStore && debounce(handleConnection)
	$: lastInput &&
		lastInput.type == 'template' &&
		$state &&
		debounce(() => (value = getValue(lastInput)))
	$: lastInput &&
		lastInput.type == 'eval' &&
		$state &&
		debounce(() => (value = evalExpr(lastInput)))

	function handleConnection() {
		if (lastInput.type === 'connected') {
			$worldStore?.connect<any>(lastInput, onValueChange)
		} else if (lastInput.type === 'static' || lastInput.type == 'template') {
			value = getValue(lastInput)
		} else if (lastInput.type == 'eval') {
			value = evalExpr(lastInput as EvalAppInput)
		} else if (lastInput.type == 'upload') {
			value = (lastInput as UploadAppInput).value
		} else {
			value = undefined
		}
	}

	function evalExpr(input: EvalAppInput) {
		try {
			const r = eval_like(input.expr, computeGlobalContext())
			error = ''
			return r
		} catch (e) {
			error = e.message
			return value
		}
	}

	function computeGlobalContext() {
		return Object.fromEntries(
			Object.entries($worldStore?.outputsById ?? {})
				.filter(([k, _]) => k != id)
				.map(([key, value]) => {
					return [
						key,
						Object.fromEntries(Object.entries(value ?? {}).map((x) => [x[0], x[1].peak()]))
					]
				})
		)
	}

	export function getValue(input: AppInput) {
		if (input.type === 'template' && isCodeInjection(input.eval)) {
			try {
				const r = eval_like('`' + input.eval + '`', computeGlobalContext())
				error = ''
				return r
			} catch (e) {
				return e.message
			}
		} else if (input.type === 'static') {
			return input.value
		} else if (input.type === 'template') {
			return input.eval
		}
	}

	function create_context_function_template(eval_string, context) {
		return `
  return function (context) {
    "use strict";
    ${
			Object.keys(context).length > 0
				? `let ${Object.keys(context).map((key) => ` ${key} = context['${key}']`)};`
				: ``
		}
    return ${eval_string};
  }                                                                                                                   
  `
	}

	function make_context_evaluator(eval_string, context) {
		let template = create_context_function_template(eval_string, context)
		let functor = Function(template)
		return functor()
	}

	function eval_like(text, context = {}) {
		let evaluator = make_context_evaluator(text, context)
		return evaluator(context)
	}

	function onValueChange(newValue: any): void {
		console.log('newValue', newValue, id)
		if (lastInput.type === 'connected' && newValue !== undefined && newValue !== null) {
			const { connection } = lastInput

			if (!connection) {
				// No connection
				return
			}

			const { path } = connection

			const hasSubPath = ['.', '['].some((x) => path.includes(x))

			if (hasSubPath) {
				const realPath = path
					.replace(/\[(\w+)\]/g, '.$1')
					.split('.')
					.slice(1)
					.join('.')

				value = accessPropertyByPath<T>(newValue, realPath)
			} else {
				value = newValue
			}
		} else {
			// TODO: handle disconnect
		}
	}
</script>
