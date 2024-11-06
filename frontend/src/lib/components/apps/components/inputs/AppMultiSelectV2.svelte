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

	export let id: string
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'multiselectcomponent'> | undefined = undefined
	export let render: boolean
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined

	// every option is labeled, or no one is.
	type Options = ObjectOption[] | string[]

	const [floatingRef, floatingContent] = createFloatingActions({
		strategy: 'absolute',
		middleware: [offset(5), flip(), shift()]
	})

	const { app, worldStore, selectedComponent, componentControl } =
		getContext<AppViewerContext>('AppViewerContext')
	let items: Options = []

	const resolvedConfig = initConfig(
		components['multiselectcomponentv2'].initialData.configuration,
		configuration
	)

	$: handleItems(resolvedConfig.items)

	function handleItems(resolvedConfigItems) {
		if (!resolvedConfigItems) {
			return
		}
		items = []
		value = []
		if (isObjectOptionArray(resolvedConfigItems)) {
			items = parseLabeledItems(resolvedConfigItems)
		} else if (isStringArray(resolvedConfigItems)) {
			items = parseStringItems(resolvedConfigItems)
		}
	}

	const outputs = initOutput($worldStore, id, {
		result: isObjectOptionArray(items)
			? ([] as ObjectOption[])
			: isStringArray(items)
			? ([] as string[])
			: ([] as unknown[])
	})

	let value: Options | undefined = isObjectOptionArray(outputs?.result.peak())
		? ([...new Set(outputs?.result.peak())] as ObjectOption[])
		: ([...new Set(outputs?.result.peak())] as string[])

	$componentControl[id] = {
		setValue(nvalue: Options) {
			if (isObjectOptionArray(nvalue)) {
				value = [...new Set(nvalue)] as ObjectOption[]
				outputs?.result.set([...(value ?? [])])
			}
		}
	}

	function isObjectOption(item: any): item is ObjectOption {
		if (!item || item !== Object(item)) {
			return false
		}
		if (item.label === undefined || item.label === null) {
			return false
		}
		if (typeof item.label !== 'string' && typeof item.label !== 'number') {
			return false
		}
		return true
	}

	function isObjectOptionArray(arr: any | undefined): arr is ObjectOption[] {
		if (!arr || !Array.isArray(arr)) {
			return false
		}
		return arr.every((item) => isObjectOption(item))
	}

	function isStringArray(arr: any | undefined): arr is string[] {
		if (!arr || !Array.isArray(arr)) {
			return false
		}
		return arr.every((item) => typeof item === 'string')
	}

	function parseLabeledItems(resolvedConfigItems: ObjectOption[]) {
		return resolvedConfigItems?.map((item) => {
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

	function parseStringItems(resolvedConfigItems: string[]) {
		return resolvedConfigItems?.map((item) => {
			if (item === null || item === undefined || typeof item !== 'string') {
				console.error(
					'When not labeled, MultiSelect component items should be an array of strings.'
				)
				return 'not string'
			}
			if (item === '') {
				return 'empty string'
			}
			return item
		})
	}

	$: resolvedConfig.defaultItems && handleDefaultItems()

	// todo
	function handleDefaultItems() {
		let nvalue: typeof items
		if (!Array.isArray(resolvedConfig.defaultItems)) {
			nvalue = []
			outputs?.result.set([])
			return
		}
		let rawNvalue = new Set(
			resolvedConfig.defaultItems?.filter((v) => typeof v === 'string' || typeof v === 'number')
		)
		if (isObjectOptionArray(items)) {
			nvalue = items?.filter((item) => rawNvalue.has(item.label)) as ObjectOption[]
			value = [...new Set(nvalue)]
			outputs?.result.set([...(value ?? [])])
		} else if (isStringArray(items)) {
			nvalue = items?.filter((label) => rawNvalue.has(label)) as string[]
			value = [...new Set(nvalue)]
			outputs?.result.set([...(value ?? [])])
		}
	}

	let css = initCss($app.css?.multiselectcomponent, customCss)

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

	const multiSelectProps = {
		outerDivClass: resolvedConfig.allowOverflow ? '' : 'h-full',
		ulSelectedClass: resolvedConfig.allowOverflow ? '' : 'overflow-auto max-h-full',
		'--sms-border': 'none',
		'--sms-min-height': '32px',
		'--sms-focus-border': 'none',
		placeholder: resolvedConfig.placeholder,
		allowUserOptions: resolvedConfig.create,
		onOpen: () => {
			$selectedComponent = [id]
			open = true
		},
		onClose: () => {
			open = false
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
		{#if !value || Array.isArray(value)}
			{#if isObjectOptionArray(value) && isObjectOptionArray(items)}
				<MultiSelect
					bind:selected={value}
					options={items}
					on:change={(event) => {
						if (event?.detail?.type === 'removeAll') {
							outputs?.result.set([])
						} else {
							outputs?.result.set([...(value ?? [])])
						}
					}}
					{...multiSelectProps}
					bind:outerDiv
					on:open={multiSelectProps.onOpen}
					on:close={multiSelectProps.onClose}
					let:option
					id="objectoption-multiselect"
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
			{:else if isStringArray(value) && isStringArray(items)}
				<MultiSelect
					bind:selected={value}
					options={items}
					on:change={(event) => {
						if (event?.detail?.type === 'removeAll') {
							outputs?.result.set([])
						} else {
							outputs?.result.set([...(value ?? [])])
						}
					}}
					{...multiSelectProps}
					bind:outerDiv
					on:open={multiSelectProps.onOpen}
					on:close={multiSelectProps.onClose}
					let:option
					id="simplestring-multiselect"
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
						{option}
					</div>
				</MultiSelect>
			{/if}
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
			Value {value} is not an array
		{/if}
	</div>
</AlignWrapper>
