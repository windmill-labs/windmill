<script lang="ts">
	import { getContext } from 'svelte'
	import { initOutput } from '../../editor/appUtils'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, ComponentCustomCSS } from '../../types'
	import { concatCustomCss } from '../../utils'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'
	import { twMerge } from 'tailwind-merge'

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
				class={twMerge('p-0', css?.container?.class, 'wm-html')}
				title="sandbox"
				srcdoc={result
					? '<base target="_parent" /><scr' +
					  `ipt type="application/javascript" src="/tailwind.js"></script>` +
					  result
					: ''}
			/>
		{/key}
		<!-- svelte-ignore a11y-click-events-have-key-events -->
		{#if $mode == 'dnd'}
			<div on:click|stopPropagation class="absolute top-0 h-full w-full" />
		{/if}
	</RunnableWrapper>
</div>
