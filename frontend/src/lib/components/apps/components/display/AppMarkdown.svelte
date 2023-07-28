<script lang="ts">
	import { getContext } from 'svelte'
	import { initOutput } from '../../editor/appUtils'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, ComponentCustomCSS } from '../../types'
	import { concatCustomCss } from '../../utils'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import SvelteMarkdown from 'svelte-markdown'
	import { classNames } from '$lib/utils'

	export let id: string
	export let componentInput: AppInput | undefined
	export let initializing: boolean | undefined = undefined
	export let customCss: ComponentCustomCSS<'mardowncomponent'> | undefined = undefined
	export let render: boolean

	const { app, worldStore, mode } = getContext<AppViewerContext>('AppViewerContext')

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})

	let result: string | undefined = undefined

	$: css = concatCustomCss($app.css?.mardowncomponent, customCss)
</script>

<div
	on:pointerdown={(e) => {
		e?.preventDefault()
	}}
	class={classNames('h-full w-full', css?.container?.class)}
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
		{#key result}
			<SvelteMarkdown source={result} />
		{/key}
		<!-- svelte-ignore a11y-click-events-have-key-events -->
		{#if $mode == 'dnd'}
			<div on:click|stopPropagation class="absolute top-0 h-full w-full" />
		{/if}
	</RunnableWrapper>
</div>
