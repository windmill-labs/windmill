<script lang="ts">
	import { createEventDispatcher, getContext, onDestroy, tick } from 'svelte'
	import type {
		AppInput,
		EvalAppInput,
		EvalV2AppInput,
		TemplateV2Input,
		UploadAppInput
	} from '../../inputType'
	import type {
		AppEditorContext,
		AppViewerContext,
		GroupContext,
		ListContext,
		RichConfiguration
	} from '../../types'
	import { accessPropertyByPath } from '../../utils'
	import { computeGlobalContext, eval_like } from './eval'
	import deepEqualWithOrderedArray from './deepEqualWithOrderedArray'
	import { deepEqual } from 'fast-equals'
	import { isCodeInjection } from '$lib/utils'

	type T = string | number | boolean | Record<string | number, any> | undefined

	export let input: AppInput | RichConfiguration
	export let value: T
	export let id: string | undefined = undefined
	export let error: string = ''
	export let key: string = ''
	export let field: string = key

	const { componentControl, runnableComponents } = getContext<AppViewerContext>('AppViewerContext')
	const editorContext = getContext<AppEditorContext>('AppEditorContext')

	const iterContext = getContext<ListContext>('ListWrapperContext')
	const rowContext = getContext<ListContext>('RowWrapperContext')
	const groupContext = getContext<GroupContext>('GroupContext')

	let previousConnectedValue: any | undefined = undefined

	let previousConnectedValues: Record<string, any> = {}

	$: fullContext = {
		iter: iterContext ? $iterContext : undefined,
		row: rowContext ? $rowContext : undefined,
		group: groupContext ? $groupContext : undefined
	}

	$: lastInput?.type == 'evalv2' &&
		(fullContext.iter != undefined ||
			fullContext.row != undefined ||
			fullContext.group != undefined) &&
		lastInput.connections.some(
			(x) => x.componentId == 'row' || x.componentId == 'iter' || x.componentId == 'group'
		) &&
		debounceEval()

	$: lastInput &&
		lastInput.type == 'templatev2' &&
		isCodeInjection(lastInput.eval) &&
		(fullContext.iter != undefined ||
			fullContext.row != undefined ||
			fullContext.group != undefined) &&
		lastInput.connections.some(
			(x) => x.componentId == 'row' || x.componentId == 'iter' || x.componentId == 'group'
		) &&
		debounceTemplate()

	const dispatch = createEventDispatcher()

	if (input == undefined) {
		dispatch('done')
	}

	let lastInput: AppInput | undefined = input ? JSON.parse(JSON.stringify(input)) : undefined

	onDestroy(() => (lastInput = undefined))

	$: if (input && !deepEqualWithOrderedArray(input, lastInput)) {
		lastInput = JSON.parse(JSON.stringify(input))
		// Needed because of file uploads
		if (lastInput && input?.['value'] instanceof ArrayBuffer) {
			// @ts-ignore
			lastInput.value = input?.['value']
		}
	}

	const { worldStore, state, mode } = getContext<AppViewerContext>('AppViewerContext')

	$: stateId = $worldStore?.stateId

	let timeout: NodeJS.Timeout | undefined = undefined

	let firstDebounce = true
	const debounce_ms = 50

	export async function computeExpr() {
		const nvalue = await evalExpr(lastInput as EvalAppInput)
		if (!deepEqual(nvalue, value)) {
			value = nvalue
		}
		return nvalue
	}

	function debounce(cb: () => Promise<void>) {
		if (firstDebounce) {
			firstDebounce = false
			cb()
			return
		}
		if (timeout) {
			clearTimeout(timeout)
		}
		timeout = setTimeout(cb, debounce_ms)
	}

	function debounce2(cb: () => Promise<void>) {
		if (firstDebounce) {
			firstDebounce = false
			cb()
			return
		}
		if (timeout) {
			clearTimeout(timeout)
		}
		timeout = setTimeout(cb, 50)
	}

	$: lastInput && $worldStore && debounce(handleConnection)

	const debounceTemplate = async () => {
		let nvalue = await getValue(lastInput as EvalAppInput)
		if (!deepEqual(nvalue, value)) {
			value = nvalue
		}
	}

	$: lastInput &&
		lastInput.type == 'template' &&
		isCodeInjection(lastInput.eval) &&
		$stateId &&
		$state &&
		debounce(debounceTemplate)

	let lastExpr: any = undefined

	const debounceEval = async () => {
		let nvalue = await evalExpr(lastInput as EvalAppInput)
		if (field) {
			editorContext?.evalPreview.update((x) => {
				x[`${id}.${field}`] = nvalue
				return x
			})
		}
		if (!deepEqual(nvalue, value)) {
			if (
				typeof nvalue == 'string' ||
				typeof nvalue == 'number' ||
				typeof nvalue == 'boolean' ||
				typeof nvalue == 'bigint'
			) {
				if (nvalue != lastExpr) {
					lastExpr = nvalue
					value = nvalue as T
				}
			} else {
				lastExpr = nvalue
				value = nvalue
			}
		}
	}

	$: lastInput && lastInput.type == 'eval' && $stateId && $state && debounce2(debounceEval)

	$: lastInput?.type == 'evalv2' && lastInput.expr && debounceEval()
	$: lastInput?.type == 'templatev2' && lastInput.eval && debounceTemplate()

	async function handleConnection() {
		if (lastInput?.type === 'connected') {
			if (lastInput.connection) {
				const { path, componentId } = lastInput.connection
				const [p] = path ? path.split('.')[0].split('[') : [undefined]
				if (p) {
					const skey = `${id}-${key}-${rowContext ? $rowContext.index : 0}-${
						iterContext ? $iterContext.index : 0
					}`
					$worldStore?.connect<any>(
						{ componentId: componentId, id: p },
						onValueChange,
						skey,
						previousConnectedValue
					)
				} else {
					console.debug('path was invalid for connection', lastInput.connection)
				}
			}
		} else if (lastInput?.type === 'static' || lastInput?.type == 'template') {
			value = await getValue(lastInput)
		} else if (lastInput?.type == 'eval') {
			value = await evalExpr(lastInput as EvalAppInput)
		} else if (lastInput?.type == 'evalv2') {
			const skey = `${id}-${key}-${rowContext ? $rowContext.index : 0}-${
				iterContext ? $iterContext.index : 0
			}`
			const input = lastInput as EvalV2AppInput
			for (const c of input.connections ?? []) {
				const previousValueKey = `${c.componentId}-${c.id}`
				$worldStore?.connect<any>(
					c,
					onEvalChange(previousValueKey),
					skey,
					previousConnectedValues[previousValueKey]
				)
			}
		} else if (lastInput?.type == 'templatev2') {
			const input = lastInput as TemplateV2Input
			const skey = `${id}-${key}-${rowContext ? $rowContext.index : 0}-${
				iterContext ? $iterContext.index : 0
			}`
			for (const c of input.connections ?? []) {
				const previousValueKey = `${c.componentId}-${c.id}`
				$worldStore?.connect<any>(
					c,
					onTemplateChange(previousValueKey),
					skey,
					previousConnectedValues[previousValueKey]
				)
			}
		} else if (lastInput?.type == 'upload') {
			value = (lastInput as UploadAppInput).value
		} else {
			value = undefined
		}

		await tick()
		dispatch('done')
	}

	function onEvalChange(previousValueKey: string) {
		return (newValue) => {
			previousConnectedValues[previousValueKey] = newValue
			debounceEval()
		}
	}

	function onTemplateChange(previousValueKey: string) {
		return (newValue) => {
			previousConnectedValues[previousValueKey] = newValue
			debounceTemplate()
		}
	}

	async function evalExpr(input: EvalAppInput | EvalV2AppInput): Promise<any> {
		if (iterContext && $iterContext.disabled) return
		try {
			const r = await eval_like(
				input.expr,
				computeGlobalContext($worldStore, fullContext),
				true,
				$state,
				$mode == 'dnd',
				$componentControl,
				$worldStore,
				$runnableComponents
			)
			error = ''
			return r
		} catch (e) {
			error = e.message
			console.warn("Eval error in app input '" + id + "' with key '" + key + "'", e)
			return value
		}
	}

	async function getValue(input: AppInput) {
		if (iterContext && $iterContext.disabled) return

		if (!input) return
		if ((input.type === 'template' || input.type == 'templatev2') && isCodeInjection(input.eval)) {
			try {
				const r = await eval_like(
					'`' + input.eval + '`',
					computeGlobalContext($worldStore, fullContext),
					true,
					$state,
					$mode == 'dnd',
					$componentControl,
					$worldStore,
					$runnableComponents
				)
				error = ''
				return r
			} catch (e) {
				console.warn("Eval error in app input '" + id + "' with key '" + key + "'", e)
				return e.message
			}
		} else if (input.type === 'static') {
			return input.value
		} else if (input.type === 'template' || input.type == 'templatev2') {
			return input.eval
		}
	}

	function onValueChange(newValue: any): void {
		if (iterContext && $iterContext.disabled) return

		if (lastInput?.type === 'connected' && newValue !== undefined && newValue !== null) {
			const { connection } = lastInput
			if (!connection) {
				// No connection
				return
			}

			previousConnectedValue = newValue

			let { path }: { path: string } = connection

			path = path.replace(/\[(\d+)\]/g, '.$1').replace(/\[\"(.*)\"\]/g, '.$1')
			let splitPoint = path.indexOf('.')
			if (splitPoint != -1) {
				const realPath = path.substring(splitPoint + 1)
				value = accessPropertyByPath<T>(newValue, realPath)
			} else {
				value = newValue
			}
		} else {
			// TODO: handle disconnect
		}
	}
</script>
