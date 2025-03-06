<script lang="ts">
	import { getContext } from 'svelte'
	import { initOutput } from '../../editor/appUtils'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, ComponentCustomCSS } from '../../types'
	import { initCss } from '../../utils'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let initializing: boolean | undefined = undefined
	export let customCss: ComponentCustomCSS<'htmlcomponent'> | undefined = undefined
	export let render: boolean

	const { app, worldStore, mode } = getContext<AppViewerContext>('AppViewerContext')

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})

	let result: string | undefined = undefined

	let css = initCss($app.css?.htmlcomponent, customCss)
</script>

{#each Object.keys(css ?? {}) as key (key)}
	<ResolveStyle
		{id}
		{customCss}
		{key}
		bind:css={css[key]}
		componentStyle={$app.css?.htmlcomponent}
	/>
{/each}

{#if render}
	<div
		on:pointerdown={(e) => {
			if ($mode !== 'preview') {
				e?.preventDefault()
			}
		}}
		class="h-full w-full"
	>
		<RunnableWrapper
			{outputs}
			{render}
			autoRefresh
			{componentInput}
			{id}
			bind:initializing
			bind:result
		>
			<div class="w-full h-full overflow-auto">
				{#key result}
					{@html result}
				{/key}
			</div>
		</RunnableWrapper>
	</div>
{:else}
	<RunnableWrapper {outputs} render={false} autoRefresh {componentInput} {id} />
{/if}
