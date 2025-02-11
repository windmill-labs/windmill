<script lang="ts">
	import { getContext, onMount } from 'svelte'
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

	export let id: string
	export let componentContainerHeight: number
	export let customCss: ComponentCustomCSS<'containercomponent'> | undefined = undefined
	export let render: boolean
	export let groupFields: RichConfigurations | undefined = undefined

	const { app, focusedGrid, selectedComponent, worldStore, connectingInput, componentControl } =
		getContext<AppViewerContext>('AppViewerContext')

	let everRender = render
	$: render && !everRender && (everRender = true)

	let groupContext = writable({})

	let outputs = initOutput($worldStore, id, { group: $groupContext })

	$: outputs.group.set($groupContext, true)

	function onFocus() {
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

	let css = initCss($app.css?.containercomponent, customCss)
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
