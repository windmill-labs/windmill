<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
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
	let wrapper: HTMLDivElement | undefined = undefined
	let error: string | undefined = undefined

	$: if (!source) {
		error = 'Set the "Source" of the PDF in the right panel'
	}
	$: loadDocument(source)

	async function loadDocument(src: string | ArrayBuffer | undefined) {
		if (!src) {
			return
		}
		try {
			const doc = await getDocument(src).promise
			while (wrapper?.firstChild) {
				wrapper.removeChild(wrapper.firstChild)
			}
			for (let i = 0; i < doc.numPages; i++) {
				const canvas = document.createElement('canvas')
				const canvasContext = canvas.getContext('2d')
				if (!(canvasContext && wrapper)) {
					console.error('Could not get canvas context for page ' + i)
					continue
				}
				const page = await doc.getPage(i + 1)
				const viewport = page.getViewport({ scale: 1 })
				canvas.height = viewport.height
				canvas.width = viewport.width
				canvas.classList.add('mx-auto', 'my-4')
				page.render({ canvasContext, viewport })
				wrapper.appendChild(canvas)
			}
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
		<div
			bind:this={wrapper}
			class={twMerge(
				'w-full h-full overflow-auto bg-gray-100',
				css?.container?.class ?? '',
				error ? 'hidden' : ''
			)}
			style={css?.container?.style ?? ''}
		/>
	{/if}
	{#if error}
		<div class="absolute inset-0 z-10 center-center text-center text-gray-600 text-sm">
			{error}
		</div>
	{/if}
</div>
