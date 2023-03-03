<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { getDocument, type PDFPageProxy } from 'pdfjs-dist'
	import 'pdfjs-dist/build/pdf.worker.entry'
	import type { AppInput } from '../../inputType'
	import type { AppEditorContext, ComponentCustomCSS } from '../../types'
	import { concatCustomCss } from '../../utils'
	import InputValue from '../helpers/InputValue.svelte'
	import { throttle } from '../../../../utils'

	export let id: string
	export let configuration: Record<string, AppInput>
	export const staticOutputs: string[] = ['loading']
	export let customCss: ComponentCustomCSS<'container'> | undefined = undefined

	const { app } = getContext<AppEditorContext>('AppEditorContext')

	let source: string | ArrayBuffer | undefined = undefined
	let wrapper: HTMLDivElement | undefined = undefined
	let error: string | undefined = undefined
	let pages: PDFPageProxy[] = []
	let firstPageWidth = 0
	let controlsHeight: number | undefined = undefined
	let pageNumber = 1

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
			pages = []
			pageNumber = 1
			for (let i = 0; i < doc.numPages; i++) {
				const canvas = document.createElement('canvas')
				const canvasContext = canvas.getContext('2d')
				if (!(canvasContext && wrapper)) {
					console.error('Could not get canvas context for page ' + i)
					continue
				}
				const page = await doc.getPage(i + 1)
				pages.push(page)
				let viewport = page.getViewport({ scale: 1 })
				const { width } = wrapper.getBoundingClientRect()
				if (viewport.width > width) {
					viewport = page.getViewport({
						scale: width / viewport.width
					})
				}
				canvas.height = viewport.height
				canvas.width = viewport.width
				if (i === 0) {
					firstPageWidth = viewport.width
				}
				canvas.classList.add('mx-auto', 'my-4', 'shadow-sm')
				page.render({ canvasContext, viewport })
				wrapper.appendChild(canvas)
			}
			pages = pages
			error = undefined
		} catch (err) {
			error = err?.message ?? (typeof err === 'string' ? err : 'Error loading PDF')
			console.log(err)
		}
	}

	function scrollToPage(page: number) {
		if (page < 1) {
			page = pageNumber = 1
		} else if (page > pages.length) {
			page = pageNumber = pages.length
		}
		const offset = (wrapper?.children.item(page - 1) as HTMLCanvasElement | null)?.offsetTop
		if (!offset) {
			return
		}
		const padding = (controlsHeight ? controlsHeight + 2 : 0) + 8
		wrapper?.scrollTo({
			top: offset - padding
		})
	}

	const throttledScroll = throttle(onScroll, 400)
	function onScroll() {
		if (!wrapper) {
			return
		}
		const THRESHOLD = 50
		let scrollPosition = wrapper.scrollTop + THRESHOLD + (controlsHeight ?? 0)
		let page = 1
		for (let i = 0; i < pages.length; i++) {
			const canvas = wrapper.children.item(i) as HTMLCanvasElement | null
			if (scrollPosition < (canvas?.offsetTop ?? wrapper.scrollHeight)) {
				break
			}
			page = i + 1
		}
		pageNumber = page
	}

	$: css = concatCustomCss($app.css?.pdfcomponent, customCss)
</script>

<InputValue {id} input={configuration.source} bind:value={source} />

<div class="relative w-full h-full bg-gray-100">
	{#if source}
		{#if pages[0]}
			<div
				bind:clientHeight={controlsHeight}
				class="sticky top-0 flex bg-white border mx-auto p-1"
				style="width: {firstPageWidth ? firstPageWidth + 2 + 'px' : '100%'};"
			>
				<div class="center-center grow px-2 text-gray-600 text-sm">
					<input
						on:input|stopPropagation={({ currentTarget }) => {
							scrollToPage(currentTarget.valueAsNumber)
						}}
						min="1"
						max={pages.length}
						value={pageNumber}
						type="number"
						class="max-w-[45px] !px-1 !py-0"
					/>
					<span class="pl-1">/ {pages.length}</span>
				</div>
			</div>
		{/if}
		<div
			bind:this={wrapper}
			on:scroll={throttledScroll}
			class={twMerge(
				'w-full h-full overflow-auto',
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
