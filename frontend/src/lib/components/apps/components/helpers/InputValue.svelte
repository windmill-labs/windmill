<script lang="ts">
	import { isCodeInjection } from '$lib/components/flows/utils'
	import { createEventDispatcher, getContext, onMount } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { AppEditorContext } from '../../types'
	import { accessPropertyByPath } from '../../utils'

	type T = string | number | boolean | Record<string | number, any> | undefined

	export let input: AppInput
	export let value: T
	export let id: string | undefined = undefined
	export let row: Record<string, any> = {}

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')

	$: state = $worldStore?.state
	$: input && $worldStore && row && handleConnection()
	$: input && $state && input.type == 'template' && (value = getValue(input))

	function handleConnection() {
		if (input.type === 'connected') {
			$worldStore?.connect<any>(input, onValueChange)
		} else if (input.type === 'row') {
			setTimeout(() => (value = row[input['column']]), 0)
		} else if (input.type === 'static' || input.type == 'template') {
			setTimeout(() => (value = getValue(input)), 0)
		} else {
			setTimeout(() => (value = undefined), 0)
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
			console.log(computeGlobalContext())
			try {
				return eval_like('`' + input.eval + '`', computeGlobalContext())
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
		if (input.type === 'connected' && newValue !== undefined && newValue !== null) {
			const { connection } = input

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
