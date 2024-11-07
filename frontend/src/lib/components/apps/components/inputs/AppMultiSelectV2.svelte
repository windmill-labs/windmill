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
	import { isObjectOptionArray, isStringArray, type ObjectOption } from '../../../multiselect/types'

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

	$: resolvedConfig.defaultItems && parseDefaultItems()

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
		selectedOptions = findOptionsByValue(values)
		outputs?.result.set([...(selectedOptions.map((option) => option.value) ?? [])])
	}

	function parseConfigOptions(resolvedConfigItems) {
		if (!resolvedConfigItems) {
			return []
		}
		if (isObjectOptionArray(resolvedConfigItems)) {
			return parseLabeledItems(resolvedConfigItems)
		} else if (isStringArray(resolvedConfigItems)) {
			return parseStringItems(resolvedConfigItems)
		}
		return []
	}

	function parseLabeledItems(resolvedConfigItems: ObjectOption[]) {
		return resolvedConfigItems?.map((item: ObjectOption) => {
			if (!item || typeof item !== 'object') {
				console.error(
					'When labeled, MultiSelect component items should be an array of { label: string, value: string }.'
				)
				return {
					label: 'not object',
					value: 'not object'
				}
			}
			return {
				label: item?.label ?? 'undefined',
				value:
					typeof item?.value === 'object' ? JSON.stringify(item.value) : item?.value ?? 'undefined'
			}
		})
	}

	function parseStringItems(resolvedConfigItems: any[]): ObjectOption[] {
		return resolvedConfigItems?.map((option: string) => {
			if (option === null || option === undefined || typeof option !== 'string') {
				console.error(
					'When not labeled, MultiSelect component items should be an array of strings.'
				)
				return { label: 'not string', value: 'not string' }
			}
			if (option === '') {
				return { label: 'empty string', value: 'empty string' }
			}
			return { label: option, value: option }
		})
	}

	function parseDefaultItems() {
		if (!Array.isArray(resolvedConfig.defaultItems)) {
			outputs?.result.set([])
			return
		}

		selectOptionsByValue(resolvedConfig.defaultItems)
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
						outputs?.result.set([...(selectedOptions.map((option) => option.value) ?? [])])
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
