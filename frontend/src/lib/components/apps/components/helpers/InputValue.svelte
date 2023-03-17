<script lang="ts">
	import { isCodeInjection } from '$lib/components/flows/utils'
	import { deepEqual } from 'fast-equals'
	import { createEventDispatcher, getContext } from 'svelte'
	import type { AppInput, EvalAppInput, RichAppInput, UploadAppInput } from '../../inputType'
	import type { AppViewerContext, RichConfiguration } from '../../types'
	import { accessPropertyByPath } from '../../utils'
	import { computeGlobalContext, eval_like } from './eval'

	type T = string | number | boolean | Record<string | number, any> | undefined

	export let input: AppInput | RichConfiguration
	export let value: T
	export let id: string | undefined = undefined
	export let error: string = ''
	export let extraContext: Record<string, any> = {}
	export let key: string = ''

	$: console.log(value)
	const { componentControl } = getContext<AppViewerContext>('AppViewerContext')

	const dispatch = createEventDispatcher()

	if (input == undefined) {
		dispatch('done')
	}

	let lastInput = input ? JSON.parse(JSON.stringify(input)) : undefined

	$: console.log(input)
	$: if (input && !deepEqual(input, lastInput)) {
		lastInput = JSON.parse(JSON.stringify(input))
		// Needed because of file uploads
		if (input?.['value'] instanceof ArrayBuffer) {
			lastInput.value = input?.['value']
		}
	}

	const { worldStore, state, mode } = getContext<AppViewerContext>('AppViewerContext')

	$: stateId = $worldStore?.stateId

	let timeout: NodeJS.Timeout | undefined = undefined
	const debounce_ms = 50
	function debounce(cb: () => Promise<void>) {
		if (timeout) {
			clearTimeout(timeout)
		}
		timeout = setTimeout(cb, debounce_ms)
	}

	$: lastInput && $worldStore && debounce(handleConnection)
	$: lastInput &&
		lastInput.type == 'template' &&
		$stateId &&
		$state &&
		debounce(async () => {
			value = await getValue(lastInput)
			dispatch('done')
		})
	$: lastInput &&
		lastInput.type == 'eval' &&
		$stateId &&
		$state &&
		debounce(async () => (value = await evalExpr(lastInput)))

	async function handleConnection() {
		console.log('handleConnection', lastInput)
		if (lastInput.type === 'connected') {
			$worldStore?.connect<any>(lastInput, onValueChange, `${id}-${key}`)
		} else if (lastInput.type === 'static' || lastInput.type == 'template') {
			value = await getValue(lastInput)
		} else if (lastInput.type == 'eval') {
			value = await evalExpr(lastInput as EvalAppInput)
		} else if (lastInput.type == 'upload') {
			value = (lastInput as UploadAppInput).value
		} else {
			value = undefined
		}
		dispatch('done')
	}

	async function evalExpr(input: EvalAppInput) {
		try {
			const r = await eval_like(
				input.expr,
				computeGlobalContext($worldStore, id, extraContext),
				true,
				$state,
				$mode == 'dnd',
				$componentControl,
				$worldStore
			)
			error = ''
			return r
		} catch (e) {
			error = e.message
			return value
		}
	}

	async function getValue(input: AppInput) {
		if (input.type === 'template' && isCodeInjection(input.eval)) {
			try {
				const r = await eval_like(
					'`' + input.eval + '`',
					computeGlobalContext($worldStore, id, extraContext),
					true,
					$state,
					$mode == 'dnd',
					$componentControl,
					$worldStore
				)
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

	function onValueChange(newValue: any): void {
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
