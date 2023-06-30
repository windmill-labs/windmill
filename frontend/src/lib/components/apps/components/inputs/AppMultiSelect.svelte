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

	export let id: string
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'multiselectcomponent'> | undefined = undefined
	export let render: boolean

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

	let value: string[] | undefined = outputs?.result.peak()

	$componentControl[id] = {
		setValue(nvalue: string[]) {
			value = nvalue
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
			value = resolvedConfig.defaultItems?.map((label) => {
				return typeof label === 'string' ? label : `NOT_STRING`
			})
			outputs?.result.set([...(value ?? [])])
		}
	}

	$: css = concatCustomCss($app.css?.multiselectcomponent, customCss)

	$: outerDiv && css?.multiselect?.style && outerDiv.setAttribute('style', css?.multiselect?.style)

	let outerDiv: HTMLDivElement | undefined = undefined
	let portalRef: HTMLDivElement | undefined = undefined

	function moveOptionsUlToPortal() {
		if (!outerDiv || !portalRef) return

		const { x, y, width, height } = outerDiv.getBoundingClientRect()

		portalRef.setAttribute(
			'style',
			`position: absolute; top: ${y}px; left: ${x}px; width: ${width}px; height: ${height}px; z-index: 1000;`
		)

		portalRef.appendChild(outerDiv)
	}
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
	>
		{#if !value || Array.isArray(value)}
			<div>
				<MultiSelect
					bind:outerDiv
					outerDivClass={`${resolvedConfig.allowOverflow ? '' : 'h-full'}`}
					ulSelectedClass={`${resolvedConfig.allowOverflow ? '' : 'overflow-auto max-h-full'} `}
					bind:selected={value}
					on:change={() => {
						outputs?.result.set([...(value ?? [])])
					}}
					options={Array.isArray(items) ? items : []}
					placeholder={resolvedConfig.placeholder}
					allowUserOptions={resolvedConfig.create}
					on:open={() => {
						$selectedComponent = [id]
						moveOptionsUlToPortal()
					}}
				/>
				<Portal>
					<div bind:this={portalRef} />
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
	.svelte-select-list {
		z-index: 1000 !important;
	}
</style>
