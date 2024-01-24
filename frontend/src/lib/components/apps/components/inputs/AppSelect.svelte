<script lang="ts">
	import { getContext, onDestroy } from 'svelte'
	import Select from '../../svelte-select/lib/index'

	import type {
		AppViewerContext,
		ComponentCustomCSS,
		ListContext,
		ListInputs,
		RichConfigurations
	} from '../../types'
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

	const {
		app,
		worldStore,
		connectingInput,
		selectedComponent,
		runnableComponents,
		componentControl,
		darkMode
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

	let resolvedConfig = initConfig(
		components['selectcomponent'].initialData.configuration,
		configuration
	)

	let outputs = initOutput($worldStore, id, {
		result: undefined as string | undefined
	})

	let value: string | undefined = noDefault ? undefined : outputs?.result.peak()

	$: resolvedConfig.items && handleItems()

	let listItems: { label: string; value: string; created?: boolean }[] = []

	function handleItems() {
		listItems = Array.isArray(resolvedConfig.items)
			? resolvedConfig.items.map((item) => {
					return {
						label: item?.label ?? 'undefined',
						value: item?.value != undefined ? JSON.stringify(item.value) : 'undefined'
					}
			  })
			: []
		let rawValue
		if (resolvedConfig.defaultValue !== undefined) {
			rawValue = resolvedConfig.defaultValue
		} else if (listItems.length > 0 && resolvedConfig?.preselectFirst) {
			rawValue = resolvedConfig.items[0].value
		}
		if (rawValue !== undefined) {
			value = JSON.stringify(rawValue)
			outputs?.result.set(rawValue)
		}
		if (iterContext && listInputs) {
			listInputs.set(id, rawValue)
		}
		if (rowContext && rowInputs) {
			rowInputs.set(id, rawValue)
		}
	}

	onDestroy(() => {
		listInputs?.remove(id)
		rowInputs?.remove(id)
	})

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
	}

	function setValue(nvalue: any) {
		let result: any = undefined
		try {
			result = JSON.parse(nvalue)
		} catch (_) {}
		value = nvalue
		outputs?.result.set(result)
		if (iterContext && listInputs) {
			listInputs.set(id, result)
		}
		if (rowContext && rowInputs) {
			rowInputs.set(id, result)
		}
		if (recomputeIds) {
			recomputeIds.forEach((id) => $runnableComponents?.[id]?.cb?.forEach((f) => f()))
		}
	}

	function onClear() {
		value = undefined
		outputs?.result.set(undefined, true)
		if (iterContext && listInputs) {
			listInputs.set(id, undefined)
		}
	}

	let css = initCss($app.css?.selectcomponent, customCss)

	function handleFilter(e) {
		if (resolvedConfig.create) {
			if (e.detail.length === 0 && filterText.length > 0) {
				const prev = listItems.filter((i) => !i.created)
				listItems = [
					...prev,
					{ value: JSON.stringify(filterText), label: filterText, created: true }
				]
			}
		}
	}

	$: resolvedConfig.defaultValue && handleDefault()

	function handleDefault() {
		if (resolvedConfig.defaultValue) {
			value = JSON.stringify(resolvedConfig.defaultValue)
			outputs?.result.set(resolvedConfig.defaultValue)
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
		{#if Array.isArray(listItems) && listItems.every((x) => typeof x == 'object' && typeof x['label'] == 'string' && `value` in x)}
			<Select
				inAppEditor={true}
				--border-radius="0"
				--border-color="#999"
				bind:filterText
				on:filter={handleFilter}
				on:clear={onClear}
				on:change={onChange}
				items={listItems}
				listAutoWidth={resolvedConfig.fullWidth}
				inputStyles={SELECT_INPUT_DEFAULT_STYLE.inputStyles}
				containerStyles={'border-color: #999;' +
					($darkMode
						? SELECT_INPUT_DEFAULT_STYLE.containerStylesDark
						: SELECT_INPUT_DEFAULT_STYLE.containerStyles) +
					css?.input?.style}
				{value}
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
