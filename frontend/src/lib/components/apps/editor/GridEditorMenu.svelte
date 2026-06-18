<script module lang="ts">
	import { writable } from 'svelte/store'
	import Popover from '../../Popover.svelte'

	import { handleCut, handleCopy } from './component/componentCallbacks.svelte'
	interface ContextMenuRegistry {
		id: string
		close: () => void
	}

	// Using a Svelte store for global state management
	const openedContextMenus = writable<Set<ContextMenuRegistry>>(new Set())
</script>

<script lang="ts">
	import { getModifierKey } from '$lib/utils'

	import {
		Anchor,
		ArrowDownFromLine,
		Copy,
		Expand,
		ExternalLink,
		Paintbrush2,
		Scissors,
		Trash
	} from 'lucide-svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import type { AppEditorContext, AppViewerContext } from '../types'
	import DeleteComponent from './settingsPanel/DeleteComponent.svelte'
	import { secondaryMenuLeft } from './settingsPanel/secondaryMenu'
	import { clickOutside } from '$lib/utils'
	import Portal from '$lib/components/Portal.svelte'

	import { twMerge } from 'tailwind-merge'

	let contextMenuVisible = $state(false)
	let menuX = $state(0)
	let menuY = $state(0)

	function handleRightClick(event: MouseEvent) {
		event.preventDefault()
		contextMenuVisible = true
		menuX = event.clientX
		menuY = event.clientY

		openedContextMenus.update((menus) => {
			menus.forEach((menu) => menu.id !== id && menu.close())
			menus.clear()
			menus.add({ id, close: () => (contextMenuVisible = false) })
			return menus
		})
	}

	function closeContextMenu() {
		contextMenuVisible = false
		openedContextMenus.update((menus) => {
			menus.clear()
			return menus
		})
	}

	function handleClickOutside(event: MouseEvent) {
		if (contextMenuVisible) {
			closeContextMenu()
		}
	}

	interface Props {
		locked?: boolean
		fullHeight?: boolean
		id: string
		children?: import('svelte').Snippet
	}

	let { locked = false, fullHeight = false, id, children }: Props = $props()

	const { selectedComponent, focusedGrid, componentControl, app } = getContext<AppViewerContext>(
		'AppViewerContext'
	) as AppViewerContext
	const { movingcomponents, stylePanel, jobsDrawerOpen, history } = getContext<AppEditorContext>(
		'AppEditorContext'
	) as AppEditorContext
	const ctx = {
		history,
		app,
		selectedComponent,
		focusedGrid,
		componentControl,
		movingcomponents,
		jobsDrawerOpen
	}
	const dispatch = createEventDispatcher()

	let deleteComponent: DeleteComponent | undefined = $state(undefined)

	const menuItems = [
		{
			label: () => 'Cut',
			onClick: () => {
				handleCut(new KeyboardEvent('keydown'), ctx)
			},
			icon: Scissors,
			shortcut: `${getModifierKey()}X`,
			disabled: $movingcomponents?.includes($selectedComponent?.[0] ?? '')
		},
		{
			label: () => 'Copy',
			onClick: () => {
				handleCopy(new KeyboardEvent('keydown'), ctx)
			},
			icon: Copy,
			shortcut: `${getModifierKey()}C`
		},
		{
			label: () => (fullHeight ? 'Undo fill height' : 'Fill height'),
			onClick: () => {
				dispatch('fillHeight')
			},
			icon: ArrowDownFromLine,
			tooltip: {
				text: 'When set to full height, a component will extend its height to fill the entire parent container (or canvas).',
				link: 'https://www.windmill.dev/docs/apps/app_configuration_settings/app_styling#full-height'
			}
		},
		{
			label: () => 'Expand',
			onClick: () => {
				dispatch('expand')
			},
			icon: Expand,
			tooltip: {
				text: "Clicking the expand button maximizes the component's width and height, respecting other components' position.",
				link: 'https://www.windmill.dev/docs/apps/canvas#expand-a-component'
			}
		},
		{
			label: () => (locked ? 'Unlock' : 'Lock'),
			onClick: () => {
				dispatch('lock')
			},
			icon: Anchor,
			tooltip: {
				text: 'Lock the component to prevent it from being repositioned by other components.',
				link: 'https://www.windmill.dev/docs/apps/canvas#lock-the-position-of-a-component'
			}
		},
		{
			label: () => 'Show style panel',
			onClick: () => {
				secondaryMenuLeft?.toggle(stylePanel(), { type: 'style' })
			},
			icon: Paintbrush2,
			disabled: $secondaryMenuLeft.isOpen,
			tooltip: {
				text: 'Use style panel to define custom CSS and Tailwind classes for the components.',
				link: 'https://www.windmill.dev/docs/apps/app_configuration_settings/app_styling'
			}
		},

		{
			label: () => 'Delete',
			onClick: () => {
				deleteComponent?.removeGridElement()
			},
			icon: Trash,
			shortcut: `Del`,
			color: 'red'
		}
	]
</script>

<DeleteComponent bind:this={deleteComponent} />

<svelte:window onclick={handleClickOutside} />

<!-- svelte-ignore a11y_click_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div onclick={closeContextMenu} oncontextmenu={handleRightClick} class="h-full w-full">
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	{@render children?.()}

	{#if contextMenuVisible}
		<Portal name="grid-editor">
			<div style="position: fixed; top: {menuY}px; left: {menuX}px; z-index:6000;">
				<div class="rounded-md bg-surface border shadow-md divide-y w-64">
					<div class="p-1" use:clickOutside={false}>
						<!-- svelte-ignore a11y_click_events_have_key_events -->
						{#each menuItems as item}
							<!-- svelte-ignore a11y_no_static_element_interactions -->
							<!-- svelte-ignore a11y_click_events_have_key_events -->

							<Popover
								notClickable
								placement="right"
								popupClass="z-[7000]"
								disablePopup={!item.tooltip}
								appearTimeout={800}
							>
								{#snippet text()}
									{item.tooltip?.text}
									{#if item.tooltip?.link}
										<a href={item.tooltip.link} target="_blank" class="text-blue-300 text-xs">
											<div class="flex flex-row gap-2 mt-4">
												See documentation
												<ExternalLink size="16" />
											</div>
										</a>
									{/if}
								{/snippet}

								<button
									class={twMerge(
										'flex items-center p-2 hover:bg-surface-hover cursor-pointer transition-all rounded-md  w-full',
										item.color === 'red' && 'text-red-500',
										item.color === 'green' && 'text-green-500',
										item.color === 'blue' && 'text-blue-500',
										item.disabled && 'opacity-50 cursor-not-allowed'
									)}
									onclick={() => {
										item.onClick()
										closeContextMenu()
									}}
									disabled={item.disabled}
								>
									<item.icon class="w-4 h-4" />

									<span class="ml-2 text-xs">{item.label()}</span>
									{#if item.shortcut}
										<span class="ml-auto text-xs text-gray-400">
											{item.shortcut}
										</span>
									{/if}
								</button>
							</Popover>
						{/each}
					</div>
				</div>
			</div>
		</Portal>
	{/if}
</div>
