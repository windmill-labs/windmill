<script lang="ts">
	import { getContext, onMount, untrack } from 'svelte'
	import { initOutput } from '../../editor/appUtils'
	import SubGridEditor from '../../editor/SubGridEditor.svelte'
	import type { AppViewerContext, ComponentCustomCSS, RichConfigurations } from '../../types'
	import { initCss } from '../../utils'
	import InitializeComponent from '../helpers/InitializeComponent.svelte'
	import { twMerge } from 'tailwind-merge'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	// import type { EvalV2AppInput, StaticAppInput } from '../../inputType'
	import { writable } from 'svelte/store'
	import GroupWrapper from '../GroupWrapper.svelte'
	import InputValue from '../helpers/InputValue.svelte'

	interface Props {
		id: string
		componentContainerHeight: number
		customCss?: ComponentCustomCSS<'containercomponent'> | undefined
		render: boolean
		groupFields?: RichConfigurations | undefined
	}

	let {
		id,
		componentContainerHeight,
		customCss = undefined,
		render,
		groupFields = undefined
	}: Props = $props()

	const { app, focusedGrid, selectedComponent, worldStore, connectingInput, componentControl } =
		getContext<AppViewerContext>('AppViewerContext')

	let everRender = $state(render)
	$effect.pre(() => {
		render && !everRender && (everRender = true)
	})

	let groupContext = writable({})

	let outputs = initOutput($worldStore, id, { group: $groupContext })

	$effect.pre(() => {
		$groupContext
		untrack(() => {
			outputs.group.set($groupContext, true)
		})
	})

	function onFocus() {
		console.log('focus 2', id)
		$focusedGrid = {
			parentComponentId: id,
			subGridIndex: 0
		}
	}

	onMount(() => {
		$componentControl[id] = {
			setGroupValue: (field: string, value: any) => {
				groupContext.update((group) => {
					group[field] = value
					return group
				})
			}
		}
	})

	let css = $state(initCss($app.css?.containercomponent, customCss))
</script>

<InitializeComponent {id} />

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.containercomponent}
	/>
{/each}

{#each Object.keys(groupFields ?? {}) as field}
	{#if groupFields && field in groupFields}
		<InputValue key={field} {id} input={groupFields[field]} bind:value={$groupContext[field]} />
	{/if}
{/each}

{#if everRender}
	<div class="w-full h-full">
		{#if $app.subgrids?.[`${id}-0`]}
			<GroupWrapper {id} context={groupContext}>
				<SubGridEditor
					visible={render}
					{id}
					class={twMerge(css?.container?.class, 'wm-container')}
					style={css?.container?.style}
					subGridId={`${id}-0`}
					containerHeight={componentContainerHeight}
					on:focus={() => {
						if (!$connectingInput.opened) {
							$selectedComponent = [id]
						}
						onFocus()
					}}
				/>
			</GroupWrapper>
		{/if}
	</div>
{:else if $app.subgrids?.[`${id}-0`]}
	<GroupWrapper {id} context={groupContext}>
		<SubGridEditor visible={false} {id} subGridId={`${id}-0`} />
	</GroupWrapper>
{/if}
