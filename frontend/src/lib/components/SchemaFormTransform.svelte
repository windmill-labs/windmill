<script lang="ts">
	import { createBubbler } from 'svelte/legacy'

	const bubble = createBubbler()
	import type { Schema } from '$lib/common'
	import { allTrue, computeShow } from '$lib/utils'
	import { createEventDispatcher, untrack } from 'svelte'
	import { deepEqual } from 'fast-equals'
	import { dragHandleZone, type Options as DndOptions } from '@windmill-labs/svelte-dnd-action'
	import type { SchemaDiff } from '$lib/components/schema/schemaUtils.svelte'
	import ResizeTransitionWrapper from './common/ResizeTransitionWrapper.svelte'
	import { twMerge } from 'tailwind-merge'
	import type { InputTransform } from '$lib/gen'
	import InputTransformForm from './InputTransformForm.svelte'
	import type { PickableProperties } from './flows/previousResults'

	interface Props {
		schema: Schema | any
		hiddenArgs?: string[]
		args?: Record<string, InputTransform>
		disabled?: boolean
		isValid?: boolean
		autofocus?: boolean
		disablePortal?: boolean
		dndConfig?: DndOptions | undefined
		items?: { id: string; value: string }[] | undefined
		diff?: Record<string, SchemaDiff>
		nestedParent?: { label: string; nestedParent: any | undefined } | undefined
		shouldDispatchChanges?: boolean
		nestedClasses?: string
		largeGap?: boolean
		className?: string
		extraLib?: string
		previousModuleId: string | undefined
		pickableProperties?: PickableProperties | undefined
		enableAi?: boolean
		otherArgs?: Record<string, InputTransform>
		isAgentTool?: boolean
		s3StorageConfigured?: boolean
	}

	let {
		schema = $bindable(),
		hiddenArgs = [],
		args = $bindable(undefined),
		disabled = false,
		isValid = $bindable(true),
		autofocus = false,
		disablePortal = false,
		dndConfig = undefined,
		items = undefined,
		diff = {},
		nestedParent = undefined,
		shouldDispatchChanges = false,
		nestedClasses = '',
		largeGap = false,
		className = '',
		extraLib = $bindable('missing extraLib'),
		previousModuleId,
		pickableProperties = undefined,
		enableAi = false,
		otherArgs = {},
		isAgentTool = false,
		s3StorageConfigured = true
	}: Props = $props()

	const dispatch = createEventDispatcher()

	let inputCheck: { [id: string]: boolean } = $state({})

	let keys: string[] = $state([])

	function removeExtraKey() {
		const nargs = {}
		Object.keys(args ?? {}).forEach((key) => {
			if (keys.includes(key) && args) {
				nargs[key] = args[key]
			}
		})
		args = nargs
	}

	function hasExtraKeys() {
		return Object.keys(args ?? {}).some((x) => !keys.includes(x))
	}

	function reorder() {
		let lkeys = Object.keys(schema?.properties ?? {})
		if (!deepEqual(schema?.order, lkeys) || !deepEqual(keys, lkeys)) {
			if (schema?.order && Array.isArray(schema.order)) {
				const n = {}
				;(schema.order as string[]).forEach((x) => {
					if (schema.properties && schema.properties[x] != undefined) {
						n[x] = schema.properties[x]
					}
				})
				Object.keys(schema.properties ?? {})
					.filter((x) => !schema.order?.includes(x))
					.forEach((x) => {
						n[x] = schema.properties[x]
					})
				if (
					!deepEqual(schema.properties, n) ||
					!deepEqual(Object.keys(schema.properties), Object.keys(n))
				) {
					schema.properties = n
				}
			}
			let nkeys = Object.keys(schema.properties ?? {})

			if (!deepEqual(keys, nkeys)) {
				keys = nkeys
				dispatch('change')
			}
		}

		if (hasExtraKeys()) {
			removeExtraKey()
		}
	}

	let hidden: Record<string, boolean> = $state({})
	let fields = $derived(items ?? keys.map((x) => ({ id: x, value: x })))

	function handleHiddenFields(schema: Schema | any, args: Record<string, InputTransform>) {
		for (const x of fields) {
			if (schema?.properties?.[x.value]?.showExpr) {
				// For InputTransform, we need to check the actual value
				const argValue =
					args[x.value]?.type === 'static' ? args[x.value]?.value : args[x.value]?.expr
				const contextArgs = {}
				Object.keys(args ?? {}).forEach((key) => {
					const arg = args[key]
					contextArgs[key] = arg?.type === 'static' ? arg?.value : arg?.expr
				})

				if (computeShow(x.value, schema.properties?.[x.value]?.showExpr, contextArgs)) {
					hidden[x.value] = false
				} else if (!hidden[x.value]) {
					hidden[x.value] = true
					// remove arg
					delete args[x.value]
					// make sure it's made valid
					inputCheck[x.value] = true
				}
			}
		}
	}

	$effect.pre(() => {
		if (args == undefined || typeof args !== 'object') {
			args = {}
		}
	})

	$effect.pre(() => {
		schema?.order
		Object.keys(schema?.properties ?? {})
		schema && (untrack(() => reorder()), (hidden = {}))
	})

	$effect.pre(() => {
		;[schema, args]

		if (args && typeof args == 'object') {
			let oneShowExpr = false
			for (const key of fields) {
				if (schema?.properties?.[key.value]?.showExpr) {
					oneShowExpr = true
				}
			}
			if (!oneShowExpr) {
				return
			}
			for (const key in args) {
				args[key]
			}
		}
		untrack(() => handleHiddenFields(schema, args ?? {}))
	})

	$effect.pre(() => {
		isValid = allTrue(inputCheck ?? {})
	})
</script>

<div
	class="w-full {className} {nestedClasses}"
	use:dragHandleZone={dndConfig ?? { items: [], dragDisabled: true }}
	onfinalize={bubble('finalize')}
	onconsider={bubble('consider')}
>
	{#if keys.length > 0 && args}
		{#each fields as item, i (item.id)}
			{@const argName = item.value}
			<ResizeTransitionWrapper
				vertical
				class={twMerge(
					typeof diff[argName] === 'object' &&
						diff[argName].diff !== 'same' &&
						'bg-red-300 dark:bg-red-800 rounded-md'
				)}
				innerClass="w-full"
			>
				{#if !hiddenArgs.includes(argName) && keys.includes(argName)}
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<div
						class="flex flex-row items-center {largeGap ? 'pb-4' : 'pb-2'} "
						onclick={() => {
							dispatch('click', argName)
						}}
					>
						{#if args && typeof args == 'object' && schema?.properties[argName]}
							{#if !hidden[argName]}
								<InputTransformForm
									on:change={() => {
										dispatch('change')
									}}
									{schema}
									bind:arg={args[argName]}
									{argName}
									{extraLib}
									bind:inputCheck={inputCheck[argName]}
									{previousModuleId}
									{pickableProperties}
									{enableAi}
									{otherArgs}
									{isAgentTool}
									{s3StorageConfigured}
								/>
							{/if}
						{/if}
					</div>
				{/if}
			</ResizeTransitionWrapper>
		{/each}
	{:else}
		<div class="text-secondary text-xs">No inputs</div>
	{/if}
</div>
