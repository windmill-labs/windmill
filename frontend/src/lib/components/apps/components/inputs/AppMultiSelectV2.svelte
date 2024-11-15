<script lang="ts">
	import { getContext } from 'svelte'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
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
	import type { ObjectOption } from '../../../multiselect/types'
	import { parseConfigOptions } from './utils'

	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'multiselectcomponent'> | undefined = undefined
	export let id: string
	export let render: boolean
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined

	const [floatingRef, floatingContent] = createFloatingActions({
		strategy: 'absolute',
		middleware: [offset(5), flip(), shift()]
	})

	const { app, worldStore, selectedComponent, componentControl } =
		getContext<AppViewerContext>('AppViewerContext')

	const resolvedConfig = initConfig(
		components['multiselectcomponentv2'].initialData.configuration,
		configuration
	)
	const outputs = initOutput($worldStore, id, { result: [] as any[] })
	let options: ObjectOption[] = []
	let selectedOptions: ObjectOption[] = [...new Set(findOptionsByValue(outputs?.result.peak()))]

	let css = initCss($app.css?.multiselectcomponent, customCss)
	let outerDiv: HTMLDivElement | undefined = undefined
	let portalRef: HTMLDivElement | undefined = undefined
	let w = 0
	let open: boolean = false

	$: if (resolvedConfig.items) {
		options = parseConfigOptions(resolvedConfig.items)
	}

	$componentControl[id] = {
		setValue(newValues: any[]) {
			selectOptionsByValue(newValues)
		}
	}

	$: if (resolvedConfig.defaultItems) {
		selectOptionsByValue(resolvedConfig.defaultItems)
	}

	$: outerDiv &&
		portalRef &&
		css?.multiselect?.style &&
		setOuterDivStyle(outerDiv, portalRef, css?.multiselect?.style)

	$: if (render && portalRef && outerDiv && options?.length > 0) {
		tick().then(() => {
			moveOptionsToPortal()
		})
	}

	function findOptionsByValue(valuesArray: any[]) {
		const values = new Set(valuesArray)
		return options?.filter((item) => values.has(item.value)) as ObjectOption[]
	}

	function selectOptionsByValue(values: any[]) {
		if (!Array.isArray(values)) {
			outputs?.result.set([])
		}

		selectedOptions = findOptionsByValue(values)
		outputs?.result.set([...toValues(selectedOptions)])
	}

	function toValues(options: ObjectOption[] | undefined) {
		if (!options) {
			return []
		}

		return options.map((o) => {
			let val = o.value
			try {
				if (typeof val === 'string') val = JSON.parse(val)
			} catch (_) {}
			return val
		})
	}

	function setOuterDivStyle(outerDiv: HTMLDivElement, portalRef: HTMLDivElement, style: string) {
		outerDiv.setAttribute('style', style)
		// find ul in portalRef and set style
		const ul = portalRef.querySelector('ul')
		ul?.setAttribute('style', extractCustomProperties(style))
	}

	function moveOptionsToPortal() {
		// Find ul element with class 'options' within the outerDiv
		const ul = outerDiv?.querySelector('.options')

		if (ul) {
			// Move the ul element to the portal
			portalRef?.appendChild(ul)
		}
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
		{#if !selectedOptions || Array.isArray(selectedOptions)}
			<MultiSelect
				bind:outerDiv
				outerDivClass={`${resolvedConfig.allowOverflow ? '' : 'h-full'}`}
				ulSelectedClass={`${resolvedConfig.allowOverflow ? '' : 'overflow-auto max-h-full'} `}
				--sms-border={'none'}
				--sms-min-height={'32px'}
				--sms-focus-border={'none'}
				bind:selected={selectedOptions}
				{options}
				placeholder={resolvedConfig.placeholder}
				allowUserOptions={resolvedConfig.create}
				on:change={(event) => {
					if (event?.detail?.type === 'removeAll') {
						outputs?.result.set([])
					} else {
						outputs?.result.set([...toValues(selectedOptions)])
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
					{option.label}
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
					/>
				</div>
			</Portal>
		{:else}
			Value {selectedOptions} is not an array
		{/if}
	</div>
</AlignWrapper>
