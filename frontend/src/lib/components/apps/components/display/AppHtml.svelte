<script lang="ts">
	import { getContext, untrack } from 'svelte'
	import { initOutput } from '../../editor/appUtils'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, ComponentCustomCSS } from '../../types'
	import { initCss } from '../../utils'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import ResolveStyle from '../helpers/ResolveStyle.svelte'
	import MarkupApprovalGate from '$lib/components/MarkupApprovalGate.svelte'
	import { getAppMarkupTrust } from '../../markupTrust'

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

	const markupTrust = getAppMarkupTrust()
	const { app, worldStore, mode } = getContext<AppViewerContext>('AppViewerContext')

	const outputs = initOutput(
		$worldStore,
		untrack(() => id),
		{
			result: undefined,
			loading: false
		}
	)

	let result: string | undefined = $state(undefined)

	let css = $state(
		initCss(
			$app.css?.htmlcomponent,
			untrack(() => customCss)
		)
	)
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
	<!-- svelte-ignore a11y_no_static_element_interactions -->
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
					{#if markupTrust === 'approval'}
						<MarkupApprovalGate>
							{#snippet children()}
								<!-- eslint-disable-next-line svelte/no-at-html-tags -->
								{@html result}
							{/snippet}
						</MarkupApprovalGate>
					{:else}
						<!-- eslint-disable-next-line svelte/no-at-html-tags -->
						{@html result}
					{/if}
				{/key}
			</div>
		</RunnableWrapper>
	</div>
{:else}
	<RunnableWrapper {outputs} render={false} autoRefresh {componentInput} {id} />
{/if}
