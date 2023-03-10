<script lang="ts">
	import { getContext } from 'svelte'
	import type { AppInput } from '../../inputType'
	import type { AppViewerContext, ComponentCustomCSS } from '../../types'
	import { concatCustomCss } from '../../utils'
	import RunnableWrapper from '../helpers/RunnableWrapper.svelte'

	export let id: string
	export let componentInput: AppInput | undefined
	export let initializing: boolean | undefined = undefined
	export let customCss: ComponentCustomCSS<'container'> | undefined = undefined
	export let render: boolean

	export const staticOutputs: string[] = ['result', 'loading']
	const { app } = getContext<AppViewerContext>('AppViewerContext')

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
		{render}
		autoRefresh
		flexWrap
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
					: 'No html'}
			/>
		{/key}
	</RunnableWrapper>
</div>
