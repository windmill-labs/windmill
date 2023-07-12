<script lang="ts">
	import { getContext } from 'svelte'
	import Select from '../../svelte-select/lib/index'

	import type {
		AppViewerContext,
		ComponentCustomCSS,
		ListContext,
		ListInputs,
		RichConfigurations
	} from '../../types'
	import { concatCustomCss } from '../../utils'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import { SELECT_INPUT_DEFAULT_STYLE } from '../../../../defaults'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import { components } from '../../editor/component'

	export let id: string
	export let configuration: RichConfigurations
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let customCss: ComponentCustomCSS<'selectcomponent'> | undefined = undefined
	export let render: boolean
	export let extraKey: string | undefined = undefined
	export let preclickAction: (() => Promise<void>) | undefined = undefined
	export let recomputeIds: string[] | undefined = undefined
	export let controls: { left: () => boolean; right: () => boolean | string } | undefined =
		undefined

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

	let value: string | undefined = outputs?.result.peak()

	$: resolvedConfig.items && handleItems()

	let listItems: { label: string; value: string; created?: boolean }[] = []

	function handleItems() {
		listItems = Array.isArray(resolvedConfig.items)
			? resolvedConfig.items.map((item) => {
					return {
						label: item.label,
						value: JSON.stringify(item.value)
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
			listInputs(id, rawValue)
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
	}

	function setValue(nvalue: any) {
		let result: any = undefined
		try {
			result = JSON.parse(nvalue)
		} catch (_) {}
		value = nvalue
		outputs?.result.set(result)
		if (iterContext && listInputs) {
			listInputs(id, result)
		}
		if (recomputeIds) {
			recomputeIds.forEach((id) => $runnableComponents?.[id]?.cb?.forEach((f) => f()))
		}
	}

	function onClear() {
		value = undefined
		outputs?.result.set(undefined)
		if (iterContext && listInputs) {
			listInputs(id, undefined)
		}
	}

	$: css = concatCustomCss($app.css?.selectcomponent, customCss)

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
<InitializeComponent {id} />

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
		<Select
			--border-radius="0"
			--border-color="#999"
			bind:filterText
			on:filter={handleFilter}
			on:clear={onClear}
			on:change={onChange}
			inAppEditor={true}
			items={listItems}
			listAutoWidth={resolvedConfig.fullWidth}
			inputStyles={SELECT_INPUT_DEFAULT_STYLE.inputStyles}
			containerStyles={'border-color: #999;' +
				SELECT_INPUT_DEFAULT_STYLE.containerStyles +
				css?.input?.style}
			{value}
			placeholder={resolvedConfig.placeholder}
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
	</div>
</AlignWrapper>

<style global>
	.app-select .value-container {
		padding: 0 !important;
	}
	.svelte-select-list {
		z-index: 5000 !important;
	}
</style>
