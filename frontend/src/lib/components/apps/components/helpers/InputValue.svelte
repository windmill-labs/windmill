<script lang="ts">
	import { isCodeInjection } from '$lib/components/flows/utils'
	import { getContext } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { AppEditorContext } from '../../types'
	import { accessPropertyByPath } from '../../utils'

	type T = string | number | boolean | Record<string | number, any> | undefined

	export let input: AppInput
	export let value: T
	export let id: string | undefined = undefined

	const { worldStore } = getContext<AppEditorContext>('AppEditorContext')

	$: input && setDefault()
	$: state = $worldStore?.state
	$: input && $worldStore && handleConnection()
	$: input && $state && input.type == 'template' && setValue()

	function setDefault() {
		if (!value && input.defaultValue) {
			value = input.defaultValue
		}
	}

	function handleConnection() {
		if (input.type === 'connected') {
			$worldStore?.connect<any>(input, onValueChange, value)
		} else if (input.type === 'static' || input.type == 'template') {
			setValue()
		} else {
			value = undefined
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

	function setValue() {
		if (input.type === 'template' && isCodeInjection(input.eval)) {
			try {
				value = eval_like('`' + input.eval + '`', computeGlobalContext())
			} catch (e) {
				value = e.message
			}
		} else if (input.type === 'static') {
			value = input.value
		} else if (input.type === 'template') {
			value = input.eval
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
				// Must remove top level property from path
				// Which was manually added, i.e. result
				const realPath = path.split('.').slice(1).join('.')

				value = accessPropertyByPath<T>(newValue, realPath)
			} else {
				value = newValue
			}
		} else {
			// TODO: handle disconnect
		}
	}
</script>
