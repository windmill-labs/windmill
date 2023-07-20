<script lang="ts">
	import { getContext } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { concatCustomCss } from '../../utils'
	import AlignWrapper from '../helpers/AlignWrapper.svelte'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	// @ts-ignore
	import MultiSelect from 'svelte-multiselect'
	import Portal from 'svelte-portal'
	import { createFloatingActions } from 'svelte-floating-ui'
	import { extractCustomProperties } from '$lib/utils'
	import { tick } from 'svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'multiselectcomponent'> | undefined = undefined
	export let render: boolean

	const [floatingRef, floatingContent] = createFloatingActions({
		strategy: 'absolute'
	})

	const { app, worldStore, selectedComponent, componentControl } =
		getContext<AppViewerContext>('AppViewerContext')
	let items: string[]

	const resolvedConfig = initConfig(
		components['multiselectcomponent'].initialData.configuration,
		configuration
	)

	const outputs = initOutput($worldStore, id, {
		result: [] as string[]
	})

	let value: string[] | undefined = [...new Set(outputs?.result.peak())] as string[]

	$componentControl[id] = {
		setValue(nvalue: string[]) {
			value = [...new Set(nvalue)]
			outputs?.result.set([...(value ?? [])])
		}
	}

	$: resolvedConfig.items && handleItems()

	function handleItems() {
		if (Array.isArray(resolvedConfig.items)) {
			items = resolvedConfig.items?.map((label) => {
				return typeof label === 'string' ? label : `NOT_STRING`
			})
		}
	}

	$: resolvedConfig.defaultItems && handleDefaultItems()

	function handleDefaultItems() {
		if (Array.isArray(resolvedConfig.defaultItems)) {
			const nvalue = resolvedConfig.defaultItems?.map((label) => {
				return typeof label === 'string' ? label : `NOT_STRING`
			})
			value = [...new Set(nvalue)]
			outputs?.result.set([...(value ?? [])])
		}
	}

	$: css = concatCustomCss($app.css?.multiselectcomponent, customCss)

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

	$: if (render) {
		tick().then(() => {
			moveOptionsToPortal()
		})
	}

	let w = 0
	let h = 0
	let open = false
</script>

{#each Object.keys(components['multiselectcomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

<InitializeComponent {id} />

<AlignWrapper {render} hFull>
	<div
		class="app-select w-full h-full"
		on:pointerdown={(e) => {
			if (!e.shiftKey) {
				e.stopPropagation()
			}
		}}
		use:floatingRef
		bind:clientWidth={w}
		bind:clientHeight={h}
	>
		{#if !value || Array.isArray(value)}
			<div style={`height:${h}px;`}>
				<MultiSelect
					bind:outerDiv
					outerDivClass={`${resolvedConfig.allowOverflow ? '' : 'h-full'}`}
					ulSelectedClass={`${resolvedConfig.allowOverflow ? '' : 'overflow-auto max-h-full'} `}
					ulOptionsClass={'p-2'}
					bind:selected={value}
					on:change={() => {
						outputs?.result.set([...(value ?? [])])
					}}
					options={Array.isArray(items) ? items : []}
					placeholder={resolvedConfig.placeholder}
					allowUserOptions={resolvedConfig.create}
					on:open={() => {
						$selectedComponent = [id]
						open = true
					}}
					on:close={() => {
						open = false
					}}
				>
					<div slot="option" let:option>
						{option}
					</div>
				</MultiSelect>
				<Portal>
					<div use:floatingContent class="z5000" hidden={!open}>
						<div bind:this={portalRef} class="multiselect" style={`min-width: ${w}px;`} />
					</div>
				</Portal>
			</div>
		{:else}
			Value {value} is not an array
		{/if}
	</div>
</AlignWrapper>

<style global>
	.app-select .value-container {
		padding: 0 !important;
		overflow: auto;
	}

	.z5000 {
		z-index: 5000 !important;
	}
</style>
