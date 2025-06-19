<script lang="ts">
	import { getContext, onDestroy, untrack } from 'svelte'

	import type {
		AppViewerContext,
		ComponentCustomCSS,
		ListContext,
		ListInputs,
		RichConfigurations
	} from '../../types'
	import { initCss } from '../../utils'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { components } from '../../editor/component'
	import { Alert } from '$lib/components/common'
	import { classNames } from '$lib/utils'
	import { Bug } from 'lucide-svelte'
	import Popover from '$lib/components/Popover.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import Select from '$lib/components/Select.svelte'

	interface Props {
		id: string
		configuration: RichConfigurations
		verticalAlignment?: 'top' | 'center' | 'bottom' | undefined
		customCss?: ComponentCustomCSS<'selectcomponent'> | undefined
		render: boolean
		extraKey?: string | undefined
		preclickAction?: (() => Promise<void>) | undefined
		recomputeIds?: string[] | undefined
		noInitialize?: boolean
		controls?: { left: () => boolean; right: () => boolean | string } | undefined
		noDefault?: boolean
		onSelect?: string[] | undefined
	}

	let {
		id,
		configuration,
		verticalAlignment = undefined,
		customCss = undefined,
		render,
		extraKey = undefined,
		preclickAction = undefined,
		recomputeIds = undefined,
		noInitialize = false,
		controls = undefined,
		noDefault = false,
		onSelect = undefined
	}: Props = $props()

	const {
		app,
		worldStore,
		connectingInput,
		selectedComponent,
		runnableComponents,
		componentControl
	} = getContext<AppViewerContext>('AppViewerContext')

	const iterContext = getContext<ListContext>('ListWrapperContext')
	const listInputs: ListInputs | undefined = getContext<ListInputs>('ListInputs')
	const rowContext = getContext<ListContext>('RowWrapperContext')
	const rowInputs: ListInputs | undefined = getContext<ListInputs>('RowInputs')

	$componentControl[id] = {
		setValue(nvalue: string) {
			setValue(JSON.stringify(nvalue))
		}
	}

	if (controls) {
		$componentControl[id] = { ...$componentControl[id], ...controls }
	}

	let resolvedConfig = $state(
		initConfig(components['selectcomponent'].initialData.configuration, configuration)
	)

	let outputs = initOutput($worldStore, id, {
		result: undefined as string | undefined
	})

	// The library expects double quotes around the value
	let value: string | undefined = $state(
		noDefault
			? undefined
			: outputs?.result.peak()
				? JSON.stringify(outputs?.result.peak())
				: undefined
	)

	let listItems: { label: string; value: string; created?: boolean }[] = $state([])

	function setContextValue(value: any) {
		if (iterContext && listInputs) {
			listInputs.set(id, value)
		}
		if (rowContext && rowInputs) {
			rowInputs.set(id, value)
		}
	}
	function handleItems() {
		listItems = Array.isArray(resolvedConfig.items)
			? resolvedConfig.items.map((item) => {
					if (typeof item == 'string') {
						return {
							label: item,
							value: JSON.stringify(item)
						}
					} else if (!item || typeof item !== 'object') {
						console.error('Select component items should be an array of objects')
						return {
							label: 'not object',
							value: 'not object'
						}
					}
					return {
						label: item?.label ?? 'undefined',
						value: item?.value != undefined ? JSON.stringify(item.value) : 'undefined'
					}
				})
			: []

		if (value != undefined && listItems.some((x) => x.value === value)) {
			return
		}
		let rawValue
		if (resolvedConfig.defaultValue !== undefined) {
			rawValue = resolvedConfig.defaultValue
		} else if (listItems.length > 0 && resolvedConfig?.preselectFirst) {
			let firstItem = resolvedConfig.items[0]
			if (typeof firstItem === 'string') {
				rawValue = firstItem
			} else {
				rawValue = resolvedConfig.items[0].value
			}
		}
		if (rawValue !== undefined && rawValue !== null) {
			value = JSON.stringify(rawValue)
			outputs?.result.set(rawValue)
		}
		setContextValue(rawValue)
	}

	onDestroy(() => {
		listInputs?.remove(id)
		rowInputs?.remove(id)
	})

	function onChange(nvalue: any) {
		if (resolvedConfig.create) {
			listItems = listItems.map((i) => {
				delete i.created
				return i
			})
		}
		preclickAction?.()
		setValue(nvalue)
		if (onSelect) {
			onSelect.forEach((id) => $runnableComponents?.[id]?.cb?.forEach((f) => f()))
		}
	}

	function onNativeChange(e: Event) {
		const target = e.target as HTMLSelectElement
		const value = target.value
		setValue(value)
	}

	function setValue(nvalue: any) {
		let result: any = undefined
		try {
			result = JSON.parse(nvalue)
		} catch (_) {}
		value = nvalue
		outputs?.result.set(result)
		setContextValue(result)
		if (recomputeIds) {
			recomputeIds.forEach((id) => $runnableComponents?.[id]?.cb?.forEach((f) => f()))
		}
	}

	function onClear() {
		value = undefined
		outputs?.result.set(undefined, true)
		setContextValue(undefined)
	}

	let css = $state(initCss(app.val.css?.selectcomponent, customCss))

	let previsousFilter = ''

	function onFilter(newFilterText: string) {
		filterText = newFilterText
		if (resolvedConfig.create && filterText !== previsousFilter) {
			previsousFilter = filterText
			if (filterText.length > 0) {
				const prev = listItems?.filter((i) => !i.created)

				const exists = listItems?.some(
					(item) => item.label.toLowerCase() === filterText.toLowerCase()
				)
				if (!exists) {
					listItems = [
						...prev,
						{ value: JSON.stringify(filterText), label: filterText, created: true }
					]
				}
			}
		}
	}

	function handleDefault() {
		if (resolvedConfig.defaultValue != undefined) {
			const nvalue = resolvedConfig.defaultValue
			value = JSON.stringify(nvalue)
			outputs?.result.set(nvalue)
			setContextValue(nvalue)
		}
	}
	let filterText = $state('')
	$effect(() => {
		resolvedConfig.items && untrack(() => handleItems())
	})
	$effect(() => {
		resolvedConfig.defaultValue != undefined && untrack(() => handleDefault())
	})
</script>

{#each Object.keys(components['selectcomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{extraKey}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={app.val.css?.selectcomponent}
	/>
{/each}

{#if !noInitialize}
	<InitializeComponent {id} />
{/if}

<AlignWrapper {render} {verticalAlignment}>
	<div
		class="app-select w-full"
		style="height: 34px;"
		onpointerdown={(e) => {
			if (!e.shiftKey) {
				e.stopPropagation()
			}
		}}
	>
		{#if Array.isArray(listItems) && listItems.every((x) => x && typeof x == 'object' && typeof x['label'] == 'string' && `value` in x)}
			{#if resolvedConfig.nativeHtmlSelect}
				<select class={css?.input?.class} style={css?.input?.style} onchange={onNativeChange}>
					{#if resolvedConfig.placeholder}
						<option value="" disabled selected>{resolvedConfig.placeholder}</option>
					{/if}
					{#each listItems as item (item.value)}
						<option value={item.value} selected={item.value === value}>{item.label}</option>
					{/each}
				</select>
			{:else}
				<Select
					bind:filterText={() => filterText, onFilter}
					{onClear}
					items={listItems.map((item) =>
						item.created && item.label === filterText
							? { ...item, label: 'Add new: ' + item.label }
							: item
					)}
					listAutoWidth={resolvedConfig.fullWidth}
					containerStyle={css?.input?.style}
					bind:value={() => value, onChange}
					class={css?.input?.class}
					placeholder={resolvedConfig.placeholder}
					disabled={resolvedConfig.disabled}
					clearable
					onFocus={() => !$connectingInput.opened && ($selectedComponent = [id])}
				/>
			{/if}
		{:else}
			<Popover notClickable placement="bottom" popupClass="!bg-surface border w-96">
				<div
					class={classNames(
						'bg-red-100 w-full h-full flex items-center justify-center text-red-500'
					)}
				>
					<Bug size={14} />
				</div>
				{#snippet text()}
					<span>
						<div class="bg-surface">
							<Alert title="Incorrect options" type="error" size="xs" class="h-full w-full ">
								The selectable items should be an array of {'{"value": any, "label":}'}. Found:
								<pre class="w-full bg-surface p-2 rounded-md whitespace-pre-wrap"
									>{JSON.stringify(listItems, null, 4)}</pre
								>
							</Alert>
						</div>
					</span>
				{/snippet}
			</Popover>
		{/if}
	</div>
</AlignWrapper>
