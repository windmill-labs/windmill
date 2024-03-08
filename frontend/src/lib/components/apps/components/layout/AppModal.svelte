<script lang="ts">
	import { getContext } from 'svelte'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import { Button } from '$lib/components/common'
	import { twMerge } from 'tailwind-merge'
	import { AlignWrapper } from '../helpers'
	import { initConfig, initOutput } from '../../editor/appUtils'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import Portal from 'svelte-portal'
	import { clickOutside } from '$lib/utils'
	import { X } from 'lucide-svelte'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	export let customCss: ComponentCustomCSS<'modalcomponent'> | undefined = undefined
	export let id: string
	export let configuration: RichConfigurations
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let noWFull = false
	export let render: boolean

	const {
		app,
		focusedGrid,
		selectedComponent,
		worldStore,
		connectingInput,
		mode,
		componentControl
	} = getContext<AppViewerContext>('AppViewerContext')

	//used so that we can count number of outputs setup for first refresh
	initOutput($worldStore, id, {})

	let css = initCss($app.css?.modalcomponent, customCss)
	let open = false

	function handleKeyUp(event: KeyboardEvent): void {
		const key = event.key
		if (key === 'Escape' || key === 'Esc') {
			if (open) {
				event.preventDefault()
				closeDrawer()
			}
		}
	}

	function closeDrawer(): void {
		console.log('Close drawer')
		open = false
	}

	let resolvedConfig = initConfig(
		components['modalcomponent'].initialData.configuration,
		configuration
	)

	let unclickableOutside = false
	function unclosableModal() {
		unclickableOutside = true
		setTimeout(() => {
			unclickableOutside = false
		}, 1000)
	}
	$componentControl[id] = {
		openModal: () => {
			unclosableModal()
			open = true
		},
		closeModal: () => {
			open = false
		},
		open: () => {
			unclosableModal()
			open = true
		},
		close: () => {
			open = false
		}
	}
</script>

<svelte:window on:keyup={handleKeyUp} />

<InitializeComponent {id} />

{#each Object.keys(components['modalcomponent'].initialData.configuration) as key (key)}
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
		componentStyle={$app.css?.modalcomponent}
	/>
{/each}

{#if render}
	<div class="h-full w-full">
		<AlignWrapper {noWFull} {horizontalAlignment} {verticalAlignment}>
			<Button
				btnClasses={twMerge(css?.button?.class, 'wm-button', 'wm-modal-button')}
				wrapperClasses={twMerge(
					resolvedConfig?.buttonFillContainer ? 'w-full h-full' : '',
					css?.buttonContainer?.class,
					'wm-button-container',
					'wm-modal-button-container',
					resolvedConfig?.hideButtonOnView && $mode == 'preview'
						? 'invisible h-0 overflow-hidden'
						: ''
				)}
				style={css?.button?.style}
				wrapperStyle={css?.buttonContainer?.style}
				disabled={resolvedConfig.buttonDisabled}
				on:pointerdown={(e) => {
					e?.stopPropagation()
				}}
				on:click={async (e) => {
					$focusedGrid = {
						parentComponentId: id,
						subGridIndex: 0
					}
					open = true
				}}
				size={resolvedConfig.buttonSize}
				color={resolvedConfig.buttonColor}
			>
				<div>{resolvedConfig.buttonLabel}</div>
			</Button>
		</AlignWrapper>
	</div>
{/if}

<Portal target="#app-editor-top-level-drawer">
	<div
		class={twMerge(
			`${
				$mode == 'dnd' ? 'absolute' : 'fixed'
			} top-0 bottom-0 left-0 right-0 transition-all duration-50`,
			open ? ' bg-black bg-opacity-60' : 'h-0 overflow-hidden invisible',
			$mode === 'dnd' ? 'z-[1000]' : 'z-[1100]'
		)}
	>
		<div
			style={css?.popup?.style}
			class={twMerge('mx-24 mt-8 bg-surface rounded-lg relative', css?.popup?.class)}
			use:clickOutside={false}
			on:click_outside={(e) => {
				if ($mode !== 'dnd' && !unclickableOutside) {
					closeDrawer()
				}
			}}
		>
			<div class="px-4 py-2 border-b flex justify-between items-center">
				<div>{resolvedConfig.modalTitle}</div>
				<div class="w-8">
					<button
						on:click|stopPropagation={() => {
							open = false
						}}
						class="hover:bg-surface-hover bg-surface-secondary rounded-full w-8 h-8 flex items-center justify-center transition-all"
					>
						<X class="text-tertiary" />
					</button>
				</div>
			</div>
			<div
				class="wm-modal"
				on:pointerdown={(e) => {
					e?.stopPropagation()
					if (!$connectingInput.opened) {
						$selectedComponent = [id]
						$focusedGrid = {
							parentComponentId: id,
							subGridIndex: 0
						}
					}
				}}
			>
				{#if $app.subgrids?.[`${id}-0`]}
					<SubGridEditor
						visible={open && render}
						{id}
						noPadding
						subGridId={`${id}-0`}
						on:focus={() => {
							if (!$connectingInput.opened) {
								$selectedComponent = [id]
								$focusedGrid = {
									parentComponentId: id,
									subGridIndex: 0
								}
							}
						}}
					/>
				{/if}
			</div>
		</div>
	</div>
</Portal>
