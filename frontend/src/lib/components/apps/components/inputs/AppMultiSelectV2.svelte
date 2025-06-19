<script lang="ts">
	import { getContext, onDestroy } from 'svelte'
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
	import MultiSelect from '$lib/components/multiselect/MultiSelect.svelte'
	import { deepEqual } from 'fast-equals'

	export let id: string
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'multiselectcomponent'> | undefined = undefined
	export let render: boolean
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined

	const iterContext = getContext<ListContext>('ListWrapperContext')
	const listInputs: ListInputs | undefined = getContext<ListInputs>('ListInputs')

	const [floatingRef, floatingContent] = createFloatingActions({
		strategy: 'absolute',
		middleware: [offset(5), flip(), shift()]
	})

	const { app, worldStore, selectedComponent, componentControl } =
		getContext<AppViewerContext>('AppViewerContext')
	let items: (string | { value: string; label: any })[] = []

	const resolvedConfig = initConfig(
		components['multiselectcomponent'].initialData.configuration,
		configuration
	)

	const outputs = initOutput($worldStore, id, {
		result: [] as string[]
	})

	let selectedItems: (number | string | { value: string; label: any })[] | undefined = [
		...new Set(outputs?.result.peak())
	] as (number | string | { value: string; label: any })[]

	function setResultsFromSelectedItems() {
		const value = [
			...(selectedItems?.map((item) => {
				if (typeof item == 'number') {
					return item.toString()
				} else if (typeof item == 'object' && item.value != undefined && item.label != undefined) {
					return item?.value ?? `NOT_STRING`
				} else if (typeof item == 'string') {
					return item
				} else if (typeof item == 'object' && item.label != undefined) {
					return item.label
				} else {
					return 'NOT_STRING'
				}
			}) ?? [])
		]
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

	$: resolvedConfig.items && handleItems()

	function handleItems() {
		if (Array.isArray(resolvedConfig.items)) {
			items = resolvedConfig.items?.map((item) => {
				if (typeof item == 'object' && item.value != undefined && item.label != undefined) {
					return item
				}
				if (typeof item == 'number') {
					return item.toString()
				}
				return typeof item === 'string' ? item : `NOT_STRING`
			})
		}
	}

	$: resolvedConfig.defaultItems && setSelectedItemsFromValues(resolvedConfig.defaultItems)

	function setSelectedItemsFromValues(values: any[]) {
		if (Array.isArray(values)) {
			const nvalue = values
				.map((value) => {
					return (
						items.find((item) => {
							if (typeof item == 'object' && item.value != undefined && item.label != undefined) {
								return deepEqual(item.value, value)
							}
							return item == value
						}) ??
						(typeof value == 'string' ? value : undefined) ??
						(typeof value == 'number' ? value.toString() : undefined)
					)
				})
				.filter((item) => item != undefined)
			selectedItems = [...new Set(nvalue)]
			setResultsFromSelectedItems()
		}
	}

	let css = initCss(app.val.css?.multiselectcomponent, customCss)

	function setOuterDivStyle(outerDiv: HTMLDivElement, portalRef: HTMLDivElement, style: string) {
		outerDiv.setAttribute('style', style)
		// find ul in portalRef and set style
		const ul = portalRef.querySelector('ul')
		ul?.setAttribute('style', extractCustomProperties(style))
	}

	$: outerDiv &&
		portalRef &&
		css?.multiselect?.style &&
		setOuterDivStyle(outerDiv, portalRef, css?.multiselect?.style)

	let outerDiv: HTMLDivElement | undefined = undefined
	let portalRef: HTMLDivElement | undefined = undefined

	function moveOptionsToPortal() {
		// Find ul element with class 'options' within the outerDiv
		const ul = outerDiv?.querySelector('.options')

		if (ul) {
			// Move the ul element to the portal
			portalRef?.appendChild(ul)
		}
	}

	$: if (render && portalRef && outerDiv && items?.length > 0) {
		tick().then(() => {
			moveOptionsToPortal()
		})
	}
	let w = 0
	let open: boolean = false
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
		componentStyle={app.val.css?.multiselectcomponent}
	/>
{/each}

<InitializeComponent {id} />

<AlignWrapper {render} hFull {verticalAlignment}>
	<div
		class="w-full app-editor-input"
		on:pointerdown={(e) => {
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
				bind:outerDiv
				outerDivClass={`${resolvedConfig.allowOverflow ? '' : 'h-full'}`}
				ulSelectedClass={`${resolvedConfig.allowOverflow ? '' : 'overflow-auto max-h-full'} `}
				--sms-border={'none'}
				--sms-min-height={'32px'}
				--sms-focus-border={'none'}
				bind:selected={selectedItems}
				options={items}
				placeholder={resolvedConfig.placeholder}
				allowUserOptions={resolvedConfig.create}
				on:change={(event) => {
					if (event?.detail?.type === 'removeAll') {
						outputs?.result.set([])
						setContextValue([])
					} else {
						setResultsFromSelectedItems()
					}
				}}
				on:open={() => {
					$selectedComponent = [id]
					open = true
				}}
				on:close={() => {
					open = false
				}}
				let:option
			>
				<!-- needed because portal doesn't work for mouseup event en mobile -->
				<!-- svelte-ignore a11y-no-static-element-interactions -->
				<div
					class="w-full"
					on:mouseup|stopPropagation
					on:pointerdown|stopPropagation={(e) => {
						let newe = new MouseEvent('mouseup')
						e.target?.['parentElement']?.dispatchEvent(newe)
					}}
				>
					{typeof option == 'object' ? (option?.label ?? 'NO_LABEL') : option}
				</div>
			</MultiSelect>
			<Portal name="app-multiselect-v2">
				<div use:floatingContent class="z5000" hidden={!open}>
					<!-- svelte-ignore a11y-click-events-have-key-events -->
					<!-- svelte-ignore a11y-no-static-element-interactions -->
					<div
						bind:this={portalRef}
						class="multiselect"
						style={`min-width: ${w}px;`}
						on:click|stopPropagation
					></div>
				</div>
			</Portal>
		{:else}
			Value {selectedItems} is not an array
		{/if}
	</div>
</AlignWrapper>
