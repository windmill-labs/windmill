<script lang="ts">
	import { getContext } from 'svelte'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { concatCustomCss } from '../../utils'
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

	$: css = concatCustomCss($app.css?.modalcomponent, customCss)
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
		open = false
	}

	let resolvedConfig = initConfig(
		components['modalcomponent'].initialData.configuration,
		configuration
	)

	$componentControl[id] = {
		openModal: () => {
			open = true
		},
		closeModal: () => {
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
{#if render}
	<div class="h-full w-full">
		<AlignWrapper {noWFull} {horizontalAlignment} {verticalAlignment}>
			<Button
				btnClasses={twMerge(css?.button?.class, 'wm-modal-button')}
				wrapperClasses={twMerge(
					resolvedConfig?.buttonFillContainer ? 'w-full h-full' : '',
					css?.buttonContainer?.class,
					'wm-modal-button-container'
				)}
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
			open ? 'z-[1100] bg-black bg-opacity-60' : 'h-0 overflow-hidden'
		)}
	>
		<div
			style={css?.popup?.style}
			class={twMerge(
				'm-24 max-h-[80%] bg-surface overflow-y-auto rounded-lg relative',
				css?.popup?.class
			)}
			use:clickOutside={false}
			on:click_outside={() => {
				if ($mode !== 'dnd') {
					closeDrawer()
				}
			}}
		>
			<div class="px-4 py-2 border-b flex justify-between items-center">
				<div>{resolvedConfig.modalTitle}</div>
				<div class="w-8">
					<button
						on:click={() => {
							open = false
						}}
						style={css?.button?.style}
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
