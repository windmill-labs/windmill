<script lang="ts">
	import { stopPropagation } from 'svelte/legacy'

	import { getContext } from 'svelte'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppViewerContext, ComponentCustomCSS } from '../../types'
	import { initCss } from '../../utils'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { deepEqual } from 'fast-equals'
	import { initOutput } from '../../editor/appUtils'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { twMerge } from 'tailwind-merge'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	interface Props {
		id: string
		componentContainerHeight: number
		customCss?:
			| ComponentCustomCSS<'horizontalsplitpanescomponent' | 'verticalsplitpanescomponent'>
			| undefined
		horizontal?: boolean
		panes: number[]
		render: boolean
	}

	let {
		id,
		componentContainerHeight,
		customCss = undefined,
		horizontal = false,
		panes,
		render
	}: Props = $props()

	const { app, focusedGrid, selectedComponent, componentControl, worldStore, connectingInput } =
		getContext<AppViewerContext>('AppViewerContext')

	//used so that we can count number of outputs setup for first refresh
	initOutput($worldStore, id, {})

	let everRender = $state(render)

	$effect.pre(() => {
		render && !everRender && (everRender = true)
	})

	function onFocus() {
		$focusedGrid = {
			parentComponentId: id,
			subGridIndex: 0
		}
	}

	let css = $state(initCss($app.css?.containercomponent, customCss))

	$componentControl[id] = {
		left: () => {
			if ($focusedGrid?.subGridIndex) {
				const index = $focusedGrid?.subGridIndex ?? 0
				if (index > 0) {
					$focusedGrid.subGridIndex = index - 1
					return true
				}
			}
			return false
		},
		right: () => {
			// subGridIndex can be 0
			if ($focusedGrid?.subGridIndex !== undefined) {
				const index = $focusedGrid?.subGridIndex ?? 0

				if (index < panes.length - 1) {
					$focusedGrid.subGridIndex = index + 1
					return true
				}
			}
			return false
		}
	}

	let sumedup = $state(panes.map((x) => (x / panes.reduce((a, b) => a + b, 0)) * 100))
	$effect.pre(() => {
		let ns = panes.map((x) => (x / panes.reduce((a, b) => a + b, 0)) * 100)
		if (!deepEqual(ns, sumedup)) {
			sumedup = ns
		}
	})
</script>

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.[
			horizontal ? 'horizontalsplitpanescomponent' : 'verticalsplitpanescomponent'
		]}
	/>
{/each}

<InitializeComponent {id} />

{#if everRender}
	<div class="h-full w-full border" onpointerdown={onFocus}>
		{#key sumedup}
			<Splitpanes {horizontal}>
				{#each sumedup as paneSize, index (index)}
					<Pane size={paneSize} minSize={20}>
						<div
							class="w-full h-full"
							onpointerdown={stopPropagation(() => {
								$selectedComponent = [id]
								$focusedGrid = {
									parentComponentId: id,
									subGridIndex: index
								}
							})}
						>
							{#if $app.subgrids?.[`${id}-${index}`]}
								<SubGridEditor
									visible={render}
									{id}
									shouldHighlight={$focusedGrid?.subGridIndex === index}
									class={twMerge(
										css?.container?.class,
										horizontal ? 'wm-horizontal-split-panes' : 'wm-vertical-split-panes'
									)}
									style={css?.container?.style}
									subGridId={`${id}-${index}`}
									containerHeight={horizontal ? undefined : componentContainerHeight - 8}
									on:focus={() => {
										if (!$connectingInput.opened) {
											$selectedComponent = [id]
											$focusedGrid = {
												parentComponentId: id,
												subGridIndex: index
											}
										}
									}}
								/>
							{/if}
						</div>
					</Pane>
				{/each}
			</Splitpanes>
		{/key}
	</div>
{:else}
	{#each sumedup as _paneSize, index (index)}
		<SubGridEditor visible={false} {id} subGridId={`${id}-${index}`} />
	{/each}
{/if}
