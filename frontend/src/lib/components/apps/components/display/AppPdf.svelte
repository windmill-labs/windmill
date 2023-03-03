<script lang="ts">
	import { getContext, tick } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { getDocument, type PDFDocumentProxy, type PDFPageProxy } from 'pdfjs-dist'
	import 'pdfjs-dist/build/pdf.worker.entry'
	import type { AppInput } from '../../inputType'
	import type { AppEditorContext, ComponentCustomCSS } from '../../types'
	import { concatCustomCss } from '../../utils'
	import InputValue from '../helpers/InputValue.svelte'
	import { throttle } from '../../../../utils'
	import { Button } from '../../../common'
	import { Download, Loader2, ZoomIn, ZoomOut } from 'lucide-svelte'

	export let id: string
	export let configuration: Record<string, AppInput>
	export const staticOutputs: string[] = ['loading']
	export let customCss: ComponentCustomCSS<'container'> | undefined = undefined

	const { app } = getContext<AppEditorContext>('AppEditorContext')

	let source: string | ArrayBuffer | undefined = undefined
	let wrapper: HTMLDivElement | undefined = undefined
	let error: string | undefined = undefined
	let doc: PDFDocumentProxy | undefined = undefined
	let pages: PDFPageProxy[] = []
	let zoom = 1
	let controlsWidth: number | undefined = undefined
	let controlsHeight: number | undefined = undefined
	let pageNumber = 1

	$: if (!source) {
		resetDoc()
		error = 'Set the "Source" attribute of the PDF component'
	}
	$: wrapper && loadDocument(source)
	$: console.log(wrapper)
	$: wideView = controlsWidth && controlsWidth > 450

	async function resetDoc() {
		await doc?.destroy()
		doc = undefined
	}

	async function loadDocument(src: string | ArrayBuffer | undefined) {
		if (!src) {
			return
		}
		try {
			await resetDoc()
			doc = await getDocument(src).promise
			pageNumber = zoom = 1
			await tick()
			await renderPdf()
			error = undefined
		} catch (err) {
			await resetDoc()
			error = err?.message ?? (typeof err === 'string' ? err : 'Error loading PDF')
			console.log(err)
		}
	}

	async function renderPdf(scaleToViewport = true) {
		if (!(doc && wrapper)) {
			return
		}
		while (wrapper?.firstChild) {
			wrapper.removeChild(wrapper.firstChild)
		}
		pages = []
		for (let i = 0; i < doc.numPages; i++) {
			const canvas = document.createElement('canvas')
			const canvasContext = canvas.getContext('2d')
			if (!canvasContext) {
				console.warn('Could not get canvas context for PDF page ' + i)
				continue
			}
			const page = await doc.getPage(i + 1)
			pages.push(page)
			let viewport = page.getViewport({ scale: zoom ?? 1 })
			if (scaleToViewport) {
				const width = wrapper.getBoundingClientRect().width
				console.log(wrapper.clientWidth, wrapper.getBoundingClientRect())
				if (width && viewport.width > width) {
					viewport = page.getViewport({
						scale: width / viewport.width
					})
				}
			}
			canvas.height = viewport.height
			canvas.width = viewport.width
			console.log(viewport.width, viewport.height, canvas.width, canvas.height)
			canvas.classList.add('mx-auto', 'my-4', 'shadow-sm')
			await page.render({ canvasContext, viewport }).promise
			wrapper.appendChild(canvas)
		}
		pages = pages
	}

	function scrollToPage(page: number) {
		page = pageNumber = minMax(page, 1, pages.length)
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

	async function zoomPdf(dir?: 'in' | 'out') {
		if (!dir) {
			zoom = 1
		} else {
			const value = dir === 'in' ? zoom + 0.1 : zoom - 0.1
			zoom = minMax(value, 0.3, 5)
		}
		await renderPdf(false)
	}

	async function downloadPdf() {
		if (!doc) {
			return
		}
		const data = await doc.saveDocument()
		const url = URL.createObjectURL(new Blob([data.buffer]))
		const link = document.createElement('a')
		link.href = url
		link.download = 'document.pdf'
		link.click()
		URL.revokeObjectURL(url)
	}

	function minMax(value: number, min: number, max: number) {
		if (value < min) {
			return min
		} else if (value > max) {
			return max
		}
		return value
	}

	$: css = concatCustomCss($app.css?.pdfcomponent, customCss)
</script>

<InputValue {id} input={configuration.source} bind:value={source} />

<div class="relative w-full h-full bg-gray-100">
	{#if source}
		{#if pages?.length}
			<div
				bind:clientWidth={controlsWidth}
				bind:clientHeight={controlsHeight}
				class="sticky w-full top-0 flex {wideView
					? 'justify-center gap-14'
					: '!justify-between'} overflow-x-auto bg-white border mx-auto py-1"
			>
				<div class="flex justify-start items-center px-2 text-gray-600 text-sm">
					<Button
						on:click={() => zoomPdf('out')}
						disabled={!doc}
						size="xs"
						color="light"
						variant="border"
						title="Zoom out"
						aria-label="Zoom out"
						btnClasses="!rounded-r-none !px-2"
					>
						<ZoomOut size={16} />
					</Button>
					{#if wideView}
						<Button
							on:click={() => zoomPdf()}
							disabled={!doc}
							size="xs"
							color="light"
							variant="border"
							title="Reset zoom"
							aria-label="Reset zoom"
							btnClasses="!w-[50px] !font-medium !rounded-none !border-l-0 !px-1"
						>
							{(zoom * 100).toFixed(0)}%
						</Button>
					{/if}
					<Button
						on:click={() => zoomPdf('in')}
						disabled={!doc}
						size="xs"
						color="light"
						variant="border"
						title="Zoom in"
						aria-label="Zoom in"
						btnClasses="!rounded-l-none !px-2 !border-l-0"
					>
						<ZoomIn size={16} />
					</Button>
				</div>
				<div class="center-center px-2 text-gray-600 text-sm">
					<input
						on:input|stopPropagation={({ currentTarget }) => {
							scrollToPage(currentTarget.valueAsNumber)
						}}
						min="1"
						max={pages.length}
						value={pageNumber}
						disabled={!doc}
						type="number"
						class="!w-[45px] !px-1 !py-0"
					/>
					<span class="whitespace-nowrap pl-1">
						/ {pages.length}
					</span>
				</div>
				<div class="flex justify-end items-center px-2 text-gray-600 text-sm">
					<Button
						on:click={downloadPdf}
						disabled={!doc}
						size="xs"
						color="light"
						variant="border"
						title="Download PDF"
						aria-label="Download PDF"
						btnClasses="!font-medium !px-2"
					>
						{#if wideView}
							<span class="mr-1"> Download </span>
						{/if}
						<Download size={16} />
					</Button>
				</div>
			</div>
		{:else}
			<div class="center-center flex-col w-full h-full text-center text-sm text-gray-600">
				<Loader2 class="animate-spin mb-2" />
				Loading PDF
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
		<div class="absolute inset-0 z-10 bg-gray-100 center-center text-center text-gray-600 text-sm">
			{error}
		</div>
	{/if}
</div>
