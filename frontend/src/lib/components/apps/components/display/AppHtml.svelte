<script lang="ts">
	import { getContext } from 'svelte'
	import { initOutput } from '../../editor/appUtils'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, ComponentCustomCSS } from '../../types'
	import { initCss } from '../../utils'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'

	interface Props {
		id: string
		componentInput: AppInput | undefined
		initializing?: boolean | undefined
		customCss?: ComponentCustomCSS<'htmlcomponent'> | undefined
		render: boolean
	}

	let {
		id,
		componentInput,
		initializing = $bindable(undefined),
		customCss = undefined,
		render
	}: Props = $props()

	const { app, worldStore, mode } = getContext<AppViewerContext>('AppViewerContext')

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})

	let result: string | undefined = $state(undefined)

	let css = $state(initCss($app.css?.htmlcomponent, customCss))
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
		onpointerdown={(e) => {
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
