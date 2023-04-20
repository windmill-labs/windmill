<script lang="ts">
	import { getContext } from 'svelte'
	import { initOutput } from '../../editor/appUtils'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, ComponentCustomCSS } from '../../types'
	import { concatCustomCss } from '../../utils'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let initializing: boolean | undefined = undefined
	export let customCss: ComponentCustomCSS<'htmlcomponent'> | undefined = undefined
	export let render: boolean

	const { app, worldStore, selectedComponent, mode } =
		getContext<AppViewerContext>('AppViewerContext')

	const outputs = initOutput($worldStore, id, {
		result: undefined,
		loading: false
	})

	let result: string | undefined = undefined
	let h: number | undefined = undefined
	let w: number | undefined = undefined

	$: css = concatCustomCss($app.css?.htmlcomponent, customCss)
</script>

<div
	on:pointerdown={(e) => {
		e?.preventDefault()
	}}
	class="h-full w-full"
	bind:clientHeight={h}
	bind:clientWidth={w}
>
	{#if $mode == 'dnd'}
		<button
			class="absolute bottom-0 left-0 text-xs border px-2 py-0.5 bg-white/80"
			on:click={() => ($selectedComponent = [id])}>Select</button
		>
	{/if}

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
			<iframe
				frameborder="0"
				style="height: {h}px; width: {w}px; {css?.container?.style ?? ''}"
				class="p-0 {css?.container?.class ?? ''}"
				title="sandbox"
				srcdoc={result
					? '<scr' + `ipt type="application/javascript" src="/tailwind.js"></script>` + result
					: ''}
			/>
		{/key}
		<!-- svelte-ignore a11y-click-events-have-key-events -->
		<div on:click|stopPropagation class="absolute top-0 h-full w-full" />
	</RunnableWrapper>
</div>
