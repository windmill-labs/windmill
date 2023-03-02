<script lang="ts">
	import { getContext } from 'svelte'
	import { getDocument } from 'pdfjs-dist'
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

	$: canvas && loadDocument(source)

	async function loadDocument(src: string | ArrayBuffer | undefined) {
		console.log(canvas, source)
		const context = canvas?.getContext('2d')
		if (!(src && context)) {
			return
		}
		const doc = await getDocument(src).promise
		const page = await doc.getPage(1)
		page.render({
			canvasContext: context,
			viewport: page.getViewport({ scale: 1 })
		})
	}

	$: css = concatCustomCss($app.css?.pdfcomponent, customCss)
</script>

<InputValue {id} input={configuration.source} bind:value={source} />

<canvas
	bind:this={canvas}
	class={css?.container?.class ?? ''}
	style={css?.container?.style ?? ''}
/>
