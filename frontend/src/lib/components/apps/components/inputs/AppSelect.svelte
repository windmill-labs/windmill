<script lang="ts">
	import { getContext } from 'svelte'
	import Select from '../../svelte-select/lib/index'

	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import { SELECT_INPUT_DEFAULT_STYLE } from '../../../../defaults'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { components } from '../../editor/component'
	import { Alert } from '$lib/components/common'
	import { classNames } from '$lib/utils'
	import { Bug } from 'lucide-svelte'
	import Popover from '$lib/components/Popover.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let customCss: ComponentCustomCSS<'selectcomponent'> | undefined = undefined
	export let render: boolean
	export let extraKey: string | undefined = undefined
	export let preclickAction: (() => Promise<void>) | undefined = undefined
	export let recomputeIds: string[] | undefined = undefined
	export let noInitialize = false
	export let controls: { left: () => boolean; right: () => boolean | string } | undefined =
		undefined
	export let noDefault = false
	export let onSelect: string[] | undefined = undefined

	const {
		app,
		worldStore,
		connectingInput,
		selectedComponent,
		runnableComponents,
		componentControl,
		darkMode
	} = getContext<AppViewerContext>('AppViewerContext')

	$componentControl[id] = {
		setValue(nvalue: string) {
			setValue(JSON.stringify(nvalue))
		}
	}

	if (controls) {
		$componentControl[id] = { ...$componentControl[id], ...controls }
	}

	let resolvedConfig = initConfig(
		components['selectcomponent'].initialData.configuration,
		configuration
	)

	let outputs = initOutput($worldStore, id, {
		result: undefined as string | undefined
	})

	// The library expects double quotes around the value
	let value: string | undefined = noDefault
		? undefined
		: outputs?.result.peak()
		? JSON.stringify(outputs?.result.peak())
		: undefined

	$: resolvedConfig.items && handleItems()

	let listItems: { label: string; value: string; created?: boolean }[] = []

	function handleItems() {
		listItems = Array.isArray(resolvedConfig.items)
			? resolvedConfig.items.map((item) => {
					if (!item || typeof item !== 'object') {
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
			rawValue = resolvedConfig.items[0].value
		}
		if (rawValue !== undefined && rawValue !== null) {
			value = JSON.stringify(rawValue)
			outputs?.result.set(rawValue)
		}
	}

	function onChange(e: CustomEvent) {
		e?.stopPropagation()

		if (resolvedConfig.create) {
			listItems = listItems.map((i) => {
				delete i.created
				return i
			})
		}
		preclickAction?.()
		setValue(e.detail?.['value'])
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

		if (recomputeIds) {
			recomputeIds.forEach((id) => $runnableComponents?.[id]?.cb?.forEach((f) => f()))
		}
	}

	function onClear() {
		value = undefined
		outputs?.result.set(undefined, true)
	}

	let css = initCss($app.css?.selectcomponent, customCss)

	let previsousFilter = ''

	function handleFilter(e) {
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

	$: resolvedConfig.defaultValue != undefined && handleDefault()

	function handleDefault() {
		if (resolvedConfig.defaultValue != undefined) {
			const nvalue = resolvedConfig.defaultValue
			value = JSON.stringify(nvalue)
			outputs?.result.set(nvalue)
		}
	}
	let filterText = ''
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
		componentStyle={$app.css?.selectcomponent}
	/>
{/each}

{#if !noInitialize}
	<InitializeComponent {id} />
{/if}

<AlignWrapper {render} {verticalAlignment}>
	<div
		class="app-select w-full"
		style="height: 34px;"
		on:pointerdown={(e) => {
			if (!e.shiftKey) {
				e.stopPropagation()
			}
		}}
	>
		{#if Array.isArray(listItems) && listItems.every((x) => x && typeof x == 'object' && typeof x['label'] == 'string' && `value` in x)}
			{#if resolvedConfig.nativeHtmlSelect}
				<select class={css?.input?.class} style={css?.input?.style} on:change={onNativeChange}>
					{#if resolvedConfig.placeholder}
						<option value="" disabled selected>{resolvedConfig.placeholder}</option>
					{/if}
					{#each listItems as item (item.value)}
						<option value={item.value} selected={item.value === value}>{item.label}</option>
					{/each}
				</select>
			{:else}
				<Select
					inAppEditor={true}
					--border-radius="0.250rem"
					--clear-icon-color="#6b7280"
					--border={$darkMode ? '1px solid #6b7280' : '1px solid #d1d5db'}
					bind:filterText
					on:filter={handleFilter}
					on:clear={onClear}
					on:change={onChange}
					items={listItems}
					listAutoWidth={resolvedConfig.fullWidth}
					inputStyles={SELECT_INPUT_DEFAULT_STYLE.inputStyles}
					containerStyles={($darkMode
						? SELECT_INPUT_DEFAULT_STYLE.containerStylesDark
						: SELECT_INPUT_DEFAULT_STYLE.containerStyles) + css?.input?.style}
					{value}
					class={css?.input?.class}
					placeholder={resolvedConfig.placeholder}
					disabled={resolvedConfig.disabled}
					on:focus={() => {
						if (!$connectingInput.opened) {
							$selectedComponent = [id]
						}
					}}
				>
					<svelte:fragment slot="item" let:item
						>{#if resolvedConfig.create}{item.created ? 'Add new: ' : ''}{/if}{item.label}
					</svelte:fragment>
				</Select>
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
				<span slot="text">
					<div class="bg-surface">
						<Alert title="Incorrect options" type="error" size="xs" class="h-full w-full ">
							The selectable items should be an array of {'{"value": any, "label":}'}. Found:
							<pre class="w-full bg-surface p-2 rounded-md whitespace-pre-wrap"
								>{JSON.stringify(listItems, null, 4)}</pre
							>
						</Alert>
					</div>
				</span>
			</Popover>
		{/if}
	</div>
</AlignWrapper>
