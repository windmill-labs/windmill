<script lang="ts" module>
	let outTimeout: NodeJS.Timeout | undefined = undefined
</script>

<script lang="ts">
	import { stopPropagation } from 'svelte/legacy'

	import { getContext, onMount, untrack } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import type { AppEditorContext, AppViewerContext } from '../../types'
	import ComponentHeader from '../ComponentHeader.svelte'
	import type { AppComponent } from './components'

	import { Anchor } from 'lucide-svelte'
	import { findGridItemParentGrid, isContainer } from '../appUtils'
	import ComponentInner from './ComponentInner.svelte'

	interface Props {
		component: AppComponent
		selected: boolean
		locked?: boolean
		fullHeight: boolean
		overlapped?: string | undefined
		moveMode?: string | undefined
		componentDraggedId?: string | undefined
		render?: boolean
	}

	let {
		component,
		selected,
		locked = false,
		fullHeight,
		overlapped = undefined,
		moveMode = undefined,
		componentDraggedId = undefined,
		render = false
	}: Props = $props()

	let initializing: boolean | undefined = $state()

	const { mode, app, hoverStore, connectingInput } =
		getContext<AppViewerContext>('AppViewerContext')

	const editorContext = getContext<AppEditorContext>('AppEditorContext')
	const componentActive = editorContext?.componentActive

	const movingcomponents = editorContext?.movingcomponents
	let ismoving = $derived(
		movingcomponents != undefined && $mode == 'dnd' && $movingcomponents?.includes(component.id)
	)

	let errorHandledByComponent: boolean = $state(false)
	let componentContainerHeight: number = $state(0)
	let componentContainerWidth: number = $state(0)

	let inlineEditorOpened: boolean = $state(false)
	let showSkeleton = $state(false)

	onMount(() => {
		setTimeout(() => {
			showSkeleton = true
		}, 100)
	})

	function mouseOut() {
		outTimeout && clearTimeout(outTimeout)
		outTimeout = setTimeout(() => {
			if ($hoverStore !== undefined) {
				// In order to avoid flickering when hovering over table actions,
				// we leave the actions to manage the hover state
				if ($hoverStore.startsWith(`${component.id}_`)) {
					return
				}

				$hoverStore = undefined
			}
		}, 50)
	}

	function componentDraggedIsNotChild(componentDraggedId: string, componentId: string) {
		let parentGrid = findGridItemParentGrid($app, componentDraggedId)

		return !parentGrid?.startsWith(`${componentId}-`)
	}

	function areOnTheSameSubgrid(componentDraggedId: string, componentId: string) {
		return (
			findGridItemParentGrid($app, componentDraggedId) === findGridItemParentGrid($app, componentId)
		)
	}

	let cachedComponentDraggedIsNotChild: boolean | undefined = $state()
	let cachedAreOnTheSameSubgrid: boolean | undefined = $state()

	function updateCache(componentDraggedId: string | undefined) {
		if (componentDraggedId) {
			cachedComponentDraggedIsNotChild = componentDraggedIsNotChild(
				componentDraggedId,
				component.id
			)
			cachedAreOnTheSameSubgrid = areOnTheSameSubgrid(componentDraggedId, component.id)
		} else {
			cachedComponentDraggedIsNotChild = undefined
			cachedAreOnTheSameSubgrid = undefined
		}
	}

	$effect(() => {
		componentDraggedId
		untrack(() => {
			updateCache(componentDraggedId)
		})
	})
</script>

<!-- svelte-ignore a11y_mouse_events_have_key_events -->
<!-- svelte-ignore a11y_no_static_element_interactions -->
<div
	onmouseover={stopPropagation(() => {
		outTimeout && clearTimeout(outTimeout)
		if (component.id !== $hoverStore) {
			$hoverStore = component.id
		}
	})}
	onmouseout={stopPropagation(mouseOut)}
	class={twMerge(
		'h-full flex flex-col w-full component relative',
		initializing ? 'overflow-hidden h-0' : ''
	)}
	data-connection-button
>
	{#if render}
		{#if locked && componentActive && $componentActive && moveMode === 'move' && componentDraggedId && componentDraggedId !== component.id && cachedAreOnTheSameSubgrid}
			<div
				class={twMerge('absolute inset-0 bg-locked center-center flex-col z-50', 'bg-locked-hover')}
			>
				<div class="bg-surface p-2 shadow-sm rounded-md flex center-center flex-col gap-2">
					<Anchor size={24} class="text-primary " />
					<div class="text-xs"> Anchored: The component cannot be moved. </div>
				</div>
			</div>
		{:else if moveMode === 'insert' && isContainer(component.type) && componentDraggedId && componentDraggedId !== component.id && cachedComponentDraggedIsNotChild}
			<div
				class={twMerge(
					'absolute inset-0  flex-col rounded-md bg-blue-100 dark:bg-gray-800 bg-opacity-50',
					'outline-dashed outline-offset-2 outline-2 outline-blue-300 dark:outline-blue-700',
					overlapped === component?.id ? 'bg-draggedover dark:bg-draggedover-dark' : ''
				)}
			></div>
		{/if}
		{#if $mode !== 'preview'}
			<ComponentHeader
				on:mouseover={() => {
					outTimeout && clearTimeout(outTimeout)

					if (component.id !== $hoverStore) {
						$hoverStore = component.id
					}
				}}
				hover={$hoverStore === component.id}
				{component}
				{selected}
				{fullHeight}
				connecting={$connectingInput.opened}
				on:lock
				on:expand
				on:fillHeight
				{locked}
				{inlineEditorOpened}
				hasInlineEditor={component.type === 'textcomponent' &&
					component.componentInput &&
					component.componentInput.type !== 'connected'}
				on:triggerInlineEditor={() => {
					inlineEditorOpened = !inlineEditorOpened
				}}
				{errorHandledByComponent}
				{componentContainerWidth}
			/>
		{/if}

		{#if ismoving}
			<div class="absolute -top-8 w-40">
				<button
					class="border p-0.5 text-xs"
					onclick={() => {
						$movingcomponents = undefined
					}}
				>
					Cancel move
				</button>
			</div>
		{/if}
	{/if}
	<div
		class={twMerge(
			render ? 'h-full outline-1' : 'h-0 overflow-hidden',
			$mode === 'dnd' ? 'bg-surface/40' : '',
			$hoverStore === component.id && $mode !== 'preview'
				? $connectingInput.opened
					? 'outline outline-[#f8aa4b]'
					: 'outline outline-blue-400'
				: '',
			selected && $mode !== 'preview' ? 'outline outline-blue-600' : '',
			$mode != 'preview' ? 'cursor-pointer' : '',
			'relative z-auto',
			$app.css?.['app']?.['component']?.class,
			'wm-app-component',
			ismoving ? 'animate-pulse' : ''
		)}
		style={$app.css?.['app']?.['component']?.style}
		bind:clientHeight={componentContainerHeight}
		bind:clientWidth={componentContainerWidth}
	>
		<ComponentInner
			{component}
			{render}
			{componentContainerHeight}
			bind:initializing
			bind:errorHandledByComponent
			{inlineEditorOpened}
		/>
	</div>
</div>
{#if initializing && render && showSkeleton}
	<!-- svelte-ignore a11y_mouse_events_have_key_events -->
	<!-- svelte-ignore a11y_no_static_element_interactions -->
	<div
		onmouseover={stopPropagation(() => {
			if (component.id !== $hoverStore) {
				$hoverStore = component.id
			}
		})}
		onmouseout={stopPropagation(() => {
			if ($hoverStore !== undefined) {
				$hoverStore = undefined
			}
		})}
		class="absolute inset-0 center-center flex-col border animate-skeleton dark:bg-frost-900/50 [animation-delay:1000ms]"
	></div>
{/if}
