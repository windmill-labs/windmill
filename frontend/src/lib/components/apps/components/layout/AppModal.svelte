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
	import { getModal } from '$lib/components/common/modal/AlwaysMountedModal.svelte'
	import Portal from 'svelte-portal'
	import { clickOutside } from '$lib/utils'
	import { X } from 'lucide-svelte'
	import { components } from '../../editor/component'
	import ResolveConfig from '../helpers/ResolveConfig.svelte'

	export let customCss: ComponentCustomCSS<'drawercomponent'> | undefined = undefined
	export let id: string
	export let configuration: RichConfigurations
	export let horizontalAlignment: 'left' | 'center' | 'right' | undefined = undefined
	export let verticalAlignment: 'top' | 'center' | 'bottom' | undefined = undefined
	export let noWFull = false
	export let render: boolean

	const { app, focusedGrid, selectedComponent, worldStore, connectingInput, mode } =
		getContext<AppViewerContext>('AppViewerContext')

	//used so that we can count number of outputs setup for first refresh
	initOutput($worldStore, id, {})

	$: css = concatCustomCss($app.css?.containercomponent, customCss)
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
</script>

<div class="h-full w-full">
	<AlignWrapper {noWFull} {horizontalAlignment} {verticalAlignment}>
		<Button
			btnClasses={twMerge(
				$app.css?.['buttoncomponent']?.['button']?.class,
				resolvedConfig.buttonFillContainer ? 'w-full h-full' : ''
			)}
			disabled={resolvedConfig.buttonDisabled}
			on:pointerdown={(e) => {
				e?.stopPropagation()
			}}
			on:click={async (e) => {
				$focusedGrid = {
					parentComponentId: id,
					subGridIndex: 0
				}
				getModal(id)?.open()
				open = true
			}}
			size={resolvedConfig.buttonSize}
			color={resolvedConfig.buttonColor}
		>
			<div>{resolvedConfig.buttonLabel}</div>
		</Button>
	</AlignWrapper>
</div>

{#each Object.keys(components['imagecomponent'].initialData.configuration) as key (key)}
	<ResolveConfig
		{id}
		{key}
		bind:resolvedConfig={resolvedConfig[key]}
		configuration={configuration[key]}
	/>
{/each}

<InitializeComponent {id} />

<svelte:window on:keyup={handleKeyUp} />

<Portal target="#app-editor-top-level-drawer">
	<div
		class={twMerge(
			'absolute top-0 bottom-0 left-0 right-0 transition-all duration-50 overflow-hidden',
			open ? 'z-50 bg-black bg-opacity-60' : 'hidden'
		)}
	>
		<div
			class="m-24 h-[80%] bg-white overflow-y-auto rounded-lg relative"
			use:clickOutside
			on:click_outside={() => {
				if ($mode !== 'dnd') {
					closeDrawer()
				}
			}}
		>
			<div class="p-4 border-b flex justify-between items-center">
				<div>{resolvedConfig.modalTitle}</div>
				<div class="w-8">
					<button
						on:click={() => {
							open = false
						}}
						class="hover:bg-gray-200 bg-gray-100 rounded-full w-8 h-8 flex items-center justify-center transition-all"
					>
						<X class="text-gray-500" />
					</button>
				</div>
			</div>
			<div
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
						class={css?.container?.class}
						style={css?.container?.style}
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
