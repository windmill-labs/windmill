<script lang="ts">
	import { createEventDispatcher, getContext, onDestroy, tick, untrack } from 'svelte'
	import { get } from 'svelte/store'
	import type {
		AppInput,
		EvalAppInput,
		EvalV2AppInput,
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
	import { deepEqual } from 'fast-equals'
	import { deepMergeWithPriority, isCodeInjection } from '$lib/utils'
	import sum from 'hash-sum'
	import { createDispatcherIfMounted } from '$lib/createDispatcherIfMounted'

	type T = string | number | boolean | Record<string | number, any> | undefined

	interface Props {
		input: AppInput | RichConfiguration
		value: T
		id?: string | undefined
		error?: string
		key?: string
		field?: string
		onDemandOnly?: boolean
		exportValueFunction?: boolean
	}

	let {
		input,
		value = $bindable(),
		id = undefined,
		error = $bindable(''),
		key = '',
		field = key,
		onDemandOnly = false,
		exportValueFunction = false
	}: Props = $props()

	const { componentControl, runnableComponents, recomputeAllContext } =
		getContext<AppViewerContext>('AppViewerContext')

	const editorContext = getContext<AppEditorContext>('AppEditorContext')
	const iterContext = getContext<ListContext>('ListWrapperContext')
	const rowContext = getContext<ListContext>('RowWrapperContext')
	const groupContext = getContext<GroupContext>('GroupContext')

	let previousConnectedValue: any | undefined = undefined

	let previousConnectedValues: Record<string, any> = {}

	let groupStore = groupContext?.context

	const dispatch = createEventDispatcher()
	const dispatchIfMounted = createDispatcherIfMounted(dispatch)

	if (input == undefined) {
		// How did this ever do anything at the top level in svelte 4 if
		// events were not being picked up before the component fully mounted?
		dispatch('done')
	}

	const { worldStore, state: stateStore, mode } = getContext<AppViewerContext>('AppViewerContext')

	let timeout: NodeJS.Timeout | undefined = undefined

	let firstDebounce = true
	const debounce_ms = 50

	export async function computeExpr(args?: Record<string, any>) {
		return await evalExpr(input as EvalAppInput, args)
	}

	let destroyed = false
	onDestroy(() => {
		destroyed = true
		clearTimeout(timeout)
		timeout = undefined
	})

	function debounce(cb: () => Promise<void>) {
		if (destroyed) return
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
		if (destroyed) return
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

	const debounceTemplate = async () => {
		let nvalue = await getValue(input as EvalAppInput)
		if (!deepEqual(nvalue, value)) {
			// console.log('template')
			value = nvalue
		}
	}

	let lastExprHash: any = undefined

	const debounceEval = async (s?: string) => {
		let args = s == 'exprChanged' ? { file: { name: 'example.png' } } : undefined
		let nvalue = await evalExpr(input as EvalAppInput, args)
		if (field) {
			editorContext?.evalPreview.update((x) => {
				x[`${id}.${field}`] = nvalue
				return x
			})
		}

		if (!onDemandOnly) {
			let nhash = typeof nvalue != 'object' ? nvalue : sum(nvalue)
			if (lastExprHash != nhash) {
				value = nvalue
				lastExprHash = nhash
			}
		}
	}

	async function handleConnection() {
		if (destroyed) return
		if (input?.type === 'connected') {
			if (input.connection) {
				const { path, componentId } = input.connection
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
					console.debug('path was invalid for connection', input.connection)
				}
			}
		} else if (input?.type === 'static' || input?.type == 'template') {
			await debounceTemplate()
		} else if (input?.type == 'eval') {
			value = await evalExpr(input as EvalAppInput)
			let nhash = typeof value != 'object' ? value : sum(value)
			lastExprHash = nhash
		} else if (input?.type == 'evalv2') {
			// console.log('evalv2', onDemandOnly, field)
			if (onDemandOnly && exportValueFunction) {
				value = (args?: any) => {
					return evalExpr(input as EvalV2AppInput, args)
				}
				return
			}
			const skey = `${id}-${key}-${rowContext ? $rowContext.index : 0}-${
				iterContext ? $iterContext.index : 0
			}`
			for (const c of input.connections ?? []) {
				const previousValueKey = `${c.componentId}-${c.id}`
				$worldStore?.connect<any>(
					c,
					onEvalChange(previousValueKey),
					skey,
					previousConnectedValues[previousValueKey]
				)
			}
		} else if (input?.type == 'templatev2') {
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
		} else if (input?.type == 'upload') {
			value = (input as UploadAppInput).value
		} else if (input?.type == 'uploadS3') {
			value = (input as UploadS3AppInput).value
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
			// console.log('onTemplateChange', previousValueKey, newValue, id)
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
			try {
				console.warn("Eval error in app input '" + id + "' with key '" + key + "'", e)
			} catch (e) {
				console.warn('error warning', e)
			}
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
		if (input?.type === 'connected' && ((newValue !== undefined && newValue !== null) || force)) {
			const { connection } = input
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
	let fullContext = $derived({
		iter: iterContext ? $iterContext : undefined,
		row: rowContext ? $rowContext : undefined,
		group: groupStore ? $groupStore : undefined
	})

	$effect.pre(() => {
		input?.type == 'evalv2' &&
			!onDemandOnly &&
			(fullContext.iter != undefined ||
				fullContext.row != undefined ||
				fullContext.group != undefined) &&
			input.connections?.some(
				(x) => x.componentId == 'row' || x.componentId == 'iter' || x.componentId == 'group'
			) &&
			untrack(() => debounceEval())
	})
	$effect.pre(() => {
		input &&
			input.type == 'templatev2' &&
			isCodeInjection(input.eval) &&
			(fullContext.iter != undefined ||
				fullContext.row != undefined ||
				fullContext.group != undefined) &&
			input.connections?.some(
				(x) => x.componentId == 'row' || x.componentId == 'iter' || x.componentId == 'group'
			) &&
			untrack(() => debounceTemplate())
	})

	// $effect(() => {
	// 	console.log('handleConnection4', input)
	// })
	$effect(() => {
		input?.type == 'static' && input.value
		input && $worldStore && untrack(() => debounce(handleConnection))
	})
	$effect.pre(() => {
		input &&
			input.type == 'template' &&
			isCodeInjection(input.eval) &&
			$stateStore &&
			untrack(() => debounce(debounceTemplate))
	})
	$effect.pre(() => {
		input && input.type == 'eval' && $stateStore && untrack(() => debounce2(debounceEval))
	})

	if (input?.type == 'eval') {
		$worldStore?.stateId.subscribe((x) => {
			debounce2(debounceEval)
		})
	}

	if (input?.type == 'template') {
		$worldStore?.stateId.subscribe((x) => {
			debounce2(debounceTemplate)
		})
	}

	$effect.pre(() => {
		input?.type == 'evalv2' && input.expr && untrack(() => debounceEval('exprChanged'))
	})
	$effect.pre(() => {
		input?.type == 'templatev2' && input.eval && untrack(() => debounceTemplate())
	})
</script>

<!-- {JSON.stringify(input)} -->
<!-- 3{value} -->
