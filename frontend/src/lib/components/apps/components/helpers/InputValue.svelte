<script lang="ts">
	import { createEventDispatcher, getContext, onDestroy, tick } from 'svelte'
	import { get } from 'svelte/store'
	import type {
		AppInput,
		EvalAppInput,
		EvalV2AppInput,
		TemplateV2Input,
		UploadAppInput,
		UploadS3AppInput
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
	import { deepMergeWithPriority, isCodeInjection } from '$lib/utils'
	import sum from 'hash-sum'
	import { createDispatcherIfMounted } from '$lib/createDispatcherIfMounted'

	type T = string | number | boolean | Record<string | number, any> | undefined

	export let input: AppInput | RichConfiguration
	export let value: T
	export let id: string | undefined = undefined
	export let error: string = ''
	export let key: string = ''
	export let field: string = key
	export let onDemandOnly: boolean = false
	export let exportValueFunction: boolean = false

	const { componentControl, runnableComponents, recomputeAllContext } =
		getContext<AppViewerContext>('AppViewerContext')

	const editorContext = getContext<AppEditorContext>('AppEditorContext')
	const iterContext = getContext<ListContext>('ListWrapperContext')
	const rowContext = getContext<ListContext>('RowWrapperContext')
	const groupContext = getContext<GroupContext>('GroupContext')

	let previousConnectedValue: any | undefined = undefined

	let previousConnectedValues: Record<string, any> = {}

	let groupStore = groupContext?.context

	$: fullContext = {
		iter: iterContext ? $iterContext : undefined,
		row: rowContext ? $rowContext : undefined,
		group: groupStore ? $groupStore : undefined
	}

	$: lastInput?.type == 'evalv2' &&
		!onDemandOnly &&
		(fullContext.iter != undefined ||
			fullContext.row != undefined ||
			fullContext.group != undefined) &&
		lastInput.connections?.some(
			(x) => x.componentId == 'row' || x.componentId == 'iter' || x.componentId == 'group'
		) &&
		debounceEval()

	$: lastInput &&
		lastInput.type == 'templatev2' &&
		isCodeInjection(lastInput.eval) &&
		(fullContext.iter != undefined ||
			fullContext.row != undefined ||
			fullContext.group != undefined) &&
		lastInput.connections?.some(
			(x) => x.componentId == 'row' || x.componentId == 'iter' || x.componentId == 'group'
		) &&
		debounceTemplate()

	const dispatch = createEventDispatcher()
	const dispatchIfMounted = createDispatcherIfMounted(dispatch)

	if (input == undefined) {
		// How did this ever do anything at the top level in svelte 4 if
		// events were not being picked up before the component fully mounted?
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

	const { worldStore, state: stateStore, mode } = getContext<AppViewerContext>('AppViewerContext')

	$: stateId = $worldStore?.stateId

	let timeout: NodeJS.Timeout | undefined = undefined

	let firstDebounce = true
	const debounce_ms = 50

	export async function computeExpr(args?: Record<string, any>) {
		return await evalExpr(lastInput as EvalAppInput, args)
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
			// console.log('template')
			value = nvalue
		}
	}

	$: lastInput &&
		lastInput.type == 'template' &&
		isCodeInjection(lastInput.eval) &&
		$stateId &&
		$stateStore &&
		debounce(debounceTemplate)

	let lastExprHash: any = undefined

	const debounceEval = async (s?: string) => {
		let args = s == 'exprChanged' ? { file: { name: 'example.png' } } : undefined
		let nvalue = await evalExpr(lastInput as EvalAppInput, args)

		if (field) {
			editorContext?.evalPreview.update((x) => {
				x[`${id}.${field}`] = nvalue
				return x
			})
		}

		if (!onDemandOnly) {
			let nhash = typeof nvalue != 'object' ? nvalue : sum(nvalue)
			if (lastExprHash != nhash) {
				// console.log('eval changed', field, nvalue)
				value = nvalue
				lastExprHash = nhash
			}
		}
	}

	$: lastInput && lastInput.type == 'eval' && $stateId && $stateStore && debounce2(debounceEval)

	$: lastInput?.type == 'evalv2' && lastInput.expr && debounceEval('exprChanged')
	$: lastInput?.type == 'templatev2' && lastInput.eval && debounceTemplate()

	async function handleConnection() {
		// console.log('handleCon')
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
			await debounceTemplate()
		} else if (lastInput?.type == 'eval') {
			value = await evalExpr(lastInput as EvalAppInput)
		} else if (lastInput?.type == 'evalv2') {
			// console.log('evalv2', onDemandOnly, field)
			if (onDemandOnly && exportValueFunction) {
				value = (args?: any) => {
					return evalExpr(lastInput as EvalV2AppInput, args)
				}
				return
			}
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
		} else if (lastInput?.type == 'uploadS3') {
			value = (lastInput as UploadS3AppInput).value
		} else {
			value = undefined
		}

		await tick()
		dispatchIfMounted('done')
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

	async function evalExpr(
		input: EvalAppInput | EvalV2AppInput,
		args?: Record<string, any>
	): Promise<any> {
		if (iterContext && $iterContext.disabled) return
		try {
			const context = computeGlobalContext(
				$worldStore,
				id,
				deepMergeWithPriority(fullContext, args ?? {})
			)
			const r = await eval_like(
				input.expr,
				context,
				$stateStore,
				$mode == 'dnd',
				$componentControl,
				$worldStore,
				$runnableComponents,
				false,
				groupContext?.id,
				get(recomputeAllContext)?.onRefresh
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
					'`' + input.eval.replaceAll('`', '\\`') + '`',
					computeGlobalContext($worldStore, id, fullContext),
					$stateStore,
					$mode == 'dnd',
					$componentControl,
					$worldStore,
					$runnableComponents,
					false,
					groupContext?.id,
					get(recomputeAllContext)?.onRefresh
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

	function onValueChange(newValue: any, force?: boolean): void {
		if (iterContext && $iterContext.disabled) return
		if (
			lastInput?.type === 'connected' &&
			((newValue !== undefined && newValue !== null) || force)
		) {
			const { connection } = lastInput
			if (!connection) {
				// No connection
				return
			}

			// console.log('onValueChange', newValue, connection, previousConnectedValue)

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
