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
	import MultiSelect from 'svelte-multiselect'
	import Portal from '$lib/components/Portal.svelte'

	import { createFloatingActions } from 'svelte-floating-ui'
	import { extractCustomProperties } from '$lib/utils'
	import { tick } from 'svelte'
	import { offset, flip, shift } from 'svelte-floating-ui/dom'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	export let id: string
	export let configuration: RichConfigurations
	export let customCss: ComponentCustomCSS<'multiselectcomponent'> | undefined = undefined
	export let render: boolean
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined

	const [floatingRef, floatingContent] = createFloatingActions({
		strategy: 'absolute',
		middleware: [offset(5), flip(), shift()]
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
			<MultiSelect
				bind:outerDiv
				outerDivClass={`${resolvedConfig.allowOverflow ? '' : 'h-full'}`}
				ulSelectedClass={`${resolvedConfig.allowOverflow ? '' : 'overflow-auto max-h-full'} `}
				ulOptionsClass={'p-2 !bg-surface-secondary'}
				--sms-border={'none'}
				--sms-min-height={'32px'}
				--sms-focus-border={'none'}
				bind:selected={value}
				options={Array.isArray(items) ? items : []}
				placeholder={resolvedConfig.placeholder}
				allowUserOptions={resolvedConfig.create}
				on:change={(event) => {
					if (event?.detail?.type === 'removeAll') {
						outputs?.result.set([])
					} else {
						outputs?.result.set([...(value ?? [])])
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
					{option}
				</div>
			</MultiSelect>
			<Portal name="app-multiselect">
				<div use:floatingContent class="z5000" hidden={!open}>
					<!-- svelte-ignore a11y-no-static-element-interactions -->
					<!-- svelte-ignore a11y-click-events-have-key-events -->
					<div
						bind:this={portalRef}
						class="multiselect"
						style={`min-width: ${w}px;`}
						on:click|stopPropagation
					></div>
				</div>
			</Portal>
		{:else}
			Value {value} is not an array
		{/if}
	</div>
</AlignWrapper>
