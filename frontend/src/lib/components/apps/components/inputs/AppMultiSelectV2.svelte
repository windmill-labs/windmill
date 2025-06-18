<script lang="ts">
	import { createBubbler, stopPropagation } from 'svelte/legacy'

	const bubble = createBubbler()
	import { getContext, onDestroy, untrack } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type {
		AppViewerContext,
		ComponentCustomCSS,
		ListContext,
		ListInputs,
		RichConfigurations
	} from '../../types'
	import { initCss } from '../../utils'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	// @ts-ignore
	import Portal from '$lib/components/Portal.svelte'

	import { createFloatingActions } from 'svelte-floating-ui'
	import { extractCustomProperties } from '$lib/utils'
	import { tick } from 'svelte'
	import { offset, flip, shift } from 'svelte-floating-ui/dom'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import { deepEqual } from 'fast-equals'
	import MultiSelect from '$lib/components/select/MultiSelect.svelte'

	interface Props {
		id: string
		configuration: RichConfigurations
		customCss?: ComponentCustomCSS<'multiselectcomponent'> | undefined
		render: boolean
		verticalAlignment?: 'top' | 'center' | 'bottom' | undefined
	}

	let {
		id,
		configuration,
		customCss = undefined,
		render,
		verticalAlignment = undefined
	}: Props = $props()

	const iterContext = getContext<ListContext>('ListWrapperContext')
	const listInputs: ListInputs | undefined = getContext<ListInputs>('ListInputs')

	const [floatingRef, floatingContent] = createFloatingActions({
		strategy: 'absolute',
		middleware: [offset(5), flip(), shift()]
	})

	const { app, worldStore, selectedComponent, componentControl } =
		getContext<AppViewerContext>('AppViewerContext')

	let items: { value: string; label?: any }[] = $state([])

	const resolvedConfig = $state(
		initConfig(components['multiselectcomponent'].initialData.configuration, configuration)
	)

	const outputs = initOutput($worldStore, id, {
		result: [] as string[]
	})

	let selectedItems: string[] = $state([...new Set(outputs?.result.peak())].map(convertToValue))
	$effect(() => setResultsFromSelectedItems(selectedItems))

	let customItems: string[] = $state([])

	function setResultsFromSelectedItems(value: string[]) {
		outputs?.result.set(value)
		setContextValue(value)
	}

	$componentControl[id] = {
		setValue(nvalue: string[]) {
			if (Array.isArray(nvalue)) {
				setSelectedItemsFromValues(nvalue)
			} else {
				console.error('Invalid value for multiselect component, expected array', nvalue)
			}
		}
	}

	onDestroy(() => {
		listInputs?.remove(id)
	})

	function setContextValue(value: any) {
		if (iterContext && listInputs) {
			listInputs.set(id, value)
		}
	}

	function handleItems() {
		if (Array.isArray(resolvedConfig.items)) {
			items = resolvedConfig.items?.map(convertToItem)
		}
	}

	function convertToItem(v: any) {
		if (typeof v == 'object' && v.value != undefined && v.label != undefined) {
			return v as { value: any; label?: string }
		}
		if (typeof v == 'number') return { value: v.toString() }
		return { value: typeof v === 'string' ? v : `NOT_STRING` }
	}
	function convertToValue(item: any): string {
		if (typeof item == 'object' && item.value != undefined) return item.value
		if (typeof item == 'number') return item.toString()
		if (typeof item == 'string') return item
		return item?.toString?.() ?? 'NOT_STRING'
	}

	function setSelectedItemsFromValues(values: any[]) {
		if (Array.isArray(values)) {
			const nvalue = values
				.map((value) => {
					const x = items.find((item) => {
						if (typeof item == 'object' && item.value != undefined && item.label != undefined) {
							return deepEqual(item.value, value)
						}
						return item == value
					})
					return (
						(typeof x === 'object' ? x.value : x) ??
						(typeof value == 'string' ? value : undefined) ??
						(typeof value == 'number' ? value.toString() : undefined)
					)
				})
				.filter((item) => item != undefined)
			selectedItems = [...new Set(nvalue)]
			setResultsFromSelectedItems(selectedItems)
		}
	}

	let css = $state(initCss($app.css?.multiselectcomponent, customCss))

	function setOuterDivStyle(outerDiv: HTMLDivElement, portalRef: HTMLDivElement, style: string) {
		outerDiv.setAttribute('style', style)
		// find ul in portalRef and set style
		const ul = portalRef.querySelector('ul')
		ul?.setAttribute('style', extractCustomProperties(style))
	}

	let outerDiv: HTMLDivElement | undefined = undefined
	let portalRef: HTMLDivElement | undefined = $state(undefined)

	function moveOptionsToPortal() {
		// Find ul element with class 'options' within the outerDiv
		const ul = outerDiv?.querySelector('.options')

		if (ul) {
			// Move the ul element to the portal
			portalRef?.appendChild(ul)
		}
	}

	let w = $state(0)
	let open: boolean = $state(false)
	$effect(() => {
		resolvedConfig.items && untrack(() => handleItems())
	})
	$effect(() => {
		resolvedConfig.defaultItems &&
			untrack(() => setSelectedItemsFromValues(resolvedConfig.defaultItems))
	})
	$effect(() => {
		outerDiv &&
			portalRef &&
			css?.multiselect?.style &&
			untrack(() => setOuterDivStyle(outerDiv, portalRef!, css.multiselect!.style))
	})
	$effect(() => {
		if (render && portalRef && outerDiv && items?.length > 0) {
			tick().then(() => {
				moveOptionsToPortal()
			})
		}
	})
</script>

{#each Object.keys(components['multiselectcomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
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
		componentStyle={$app.css?.multiselectcomponent}
	/>
{/each}

<InitializeComponent {id} />

<AlignWrapper {render} hFull {verticalAlignment}>
	<div
		class="w-full"
		onpointerdown={(e) => {
			$selectedComponent = [id]

			if (!e.shiftKey) {
				e.stopPropagation()
			}
			selectedComponent.set([id])
		}}
		use:floatingRef
		bind:clientWidth={w}
	>
		{#if !selectedItems || Array.isArray(selectedItems)}
			<MultiSelect
				items={[...items, ...customItems.map((value) => ({ value }))]}
				placeholder={resolvedConfig.placeholder}
				bind:value={selectedItems}
				onCreateItem={resolvedConfig.create
					? (item) => {
							customItems.push(item)
							selectedItems.push(item)
							customItems = customItems
							selectedItems = selectedItems
						}
					: undefined}
				onOpen={() => (($selectedComponent = [id]), (open = true))}
			/>

			<Portal name="app-multiselect-v2">
				<div use:floatingContent class="z5000" hidden={!open}>
					<!-- svelte-ignore a11y_click_events_have_key_events -->
					<!-- svelte-ignore a11y_no_static_element_interactions -->
					<div
						bind:this={portalRef}
						class="multiselect"
						style={`min-width: ${w}px;`}
						onclick={stopPropagation(bubble('click'))}
					></div>
				</div>
			</Portal>
		{:else}
			Value {selectedItems} is not an array
		{/if}
	</div>
</AlignWrapper>
