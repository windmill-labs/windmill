<script lang="ts">
	import { getContext } from 'svelte'
	import { getDocument } from 'pdfjs-dist'
	import 'pdfjs-dist/build/pdf.worker.entry'
	import type { AppInput } from '../../inputType'
	import type { AppEditorContext, ComponentCustomCSS } from '../../types'
	import { concatCustomCss } from '../../utils'
	import InputValue from '../helpers/InputValue.svelte'

	export let id: string
	export let configuration: Record<string, AppInput>
	export const staticOutputs: string[] = ['loading']
	export let customCss: ComponentCustomCSS<'container'> | undefined = undefined

	const { app } = getContext<AppEditorContext>('AppEditorContext')

	let source: string | ArrayBuffer | undefined = undefined
	let canvas: HTMLCanvasElement | undefined = undefined
	let error: string | undefined = undefined

	$: if (!source) error = 'Set the "Source" of the PDF in the right panel'
	$: canvas && loadDocument(source)

	async function loadDocument(src: string | ArrayBuffer | undefined) {
		const context = canvas?.getContext('2d')
		if (!(src && context)) {
			return
		}
		try {
			const doc = await getDocument(src).promise
			const page = await doc.getPage(1)
			page.render({
				canvasContext: context,
				viewport: page.getViewport({ scale: 1 })
			})
			error = undefined
		} catch (err) {
			error = err?.message ?? (typeof err === 'string' ? err : 'Error loading PDF')
			console.log(err)
		}
	}

	$: css = concatCustomCss($app.css?.pdfcomponent, customCss)
</script>

<InputValue {id} input={configuration.source} bind:value={source} />

<div class="relative w-full h-full">
	{#if source}
		<canvas
			bind:this={canvas}
			hidden={!!error}
			class={css?.container?.class ?? ''}
			style={css?.container?.style ?? ''}
		/>
	{/if}
	{#if error}
		<div class="absolute inset-0 z-10 center-center text-center text-gray-600 text-sm">
			{error}
		</div>
	{/if}
</div>
