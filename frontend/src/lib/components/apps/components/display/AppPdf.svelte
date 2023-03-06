<script lang="ts">
	import { getContext } from 'svelte'
	import { twMerge } from 'tailwind-merge'
	import { getDocument, type PDFDocumentProxy, type PDFPageProxy } from 'pdfjs-dist'
	import 'pdfjs-dist/build/pdf.worker.entry'
	import type { AppInput } from '../../inputType'
	import type { AppEditorContext, ComponentCustomCSS } from '../../types'
	import { concatCustomCss } from '../../utils'
	import InputValue from '../helpers/InputValue.svelte'
	import { throttle } from '../../../../utils'
	import { Button } from '../../../common'
	import { Download, Loader2, MoveHorizontal, ZoomIn, ZoomOut } from 'lucide-svelte'
	import { fade } from 'svelte/transition'
	import { findGridItem } from '../../editor/appUtils'

	export let id: string
	export let configuration: Record<string, AppInput>
	export const staticOutputs: string[] = ['loading']
	export let customCss: ComponentCustomCSS<'container'> | undefined = undefined

	const { app, mode, selectedComponent } = getContext<AppEditorContext>('AppEditorContext')

	let source: string | ArrayBuffer | undefined = undefined
	let wrapper: HTMLDivElement | undefined = undefined
	let error: string | undefined = undefined
	let doc: PDFDocumentProxy | undefined = undefined
	let pages: PDFPageProxy[] = []
	let zoom = 100
	let controlsWidth: number | undefined = undefined
	let controlsHeight: number | undefined = undefined
	let pageNumber = 1

	$: if (!source) {
		resetDoc()
		error = 'Set the "Source" attribute of the PDF component'
	}
	$: {
		zoom = minMax(zoom, 10, 500)
		renderPdf(false, true)
	}
	$: wrapper && loadDocument(source)
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
			pageNumber = 1
			zoom = 100
			await renderPdf()
			error = undefined
		} catch (err) {
			await resetDoc()
			error = err?.message ?? (typeof err === 'string' ? err : 'Error loading PDF')
			console.log(err)
		}
	}

	async function renderPdf(scaleToViewport = true, resizing = false) {
		if (!(doc && wrapper)) {
			return
		}
		const scrollPosition = wrapper.scrollTop / wrapper.scrollHeight
		if (!resizing) {
			pages = []
		}
		const nextPages: typeof pages = []
		const nextChildren: HTMLCanvasElement[] = []
		const { width } = wrapper.getBoundingClientRect()
		let scale = zoom / 100
		if (scaleToViewport) {
			const firstViewport = (await doc.getPage(1)).getViewport({ scale: 1 })
			// Rounded to the first integer that is a multiple of 10 and is less than the viewport width
			zoom = Math.floor((width / firstViewport.width) * 10) * 10
			scale = zoom / 100
		}

		for (let i = 0; i < doc.numPages; i++) {
			const canvas = document.createElement('canvas')
			const canvasContext = canvas.getContext('2d')
			if (!canvasContext) {
				console.warn('Could not get canvas context for PDF page ' + i)
				continue
			}
			const page = await doc.getPage(i + 1)
			nextPages.push(page)
			const viewport = page.getViewport({ scale })
			canvas.height = viewport.height
			canvas.width = viewport.width
			canvas.classList.add('mx-auto', 'my-4', 'shadow-sm')
			await page.render({ canvasContext, viewport }).promise
			nextChildren.push(canvas)
		}
		while (wrapper.firstChild) {
			wrapper.removeChild(wrapper.firstChild)
		}
		pages = [...nextPages]
		wrapper.append(...nextChildren)
		wrapper.scrollTo({
			top: scrollPosition * wrapper.scrollHeight
		})
	}

	function scrollToPage(page: number) {
		page = pageNumber = minMax(page, 1, pages.length)
		const offset = (wrapper?.children.item(page - 1) as HTMLCanvasElement | null)?.offsetTop
		if (!offset) {
			return
		}
		//                           controlsHeight + 2px border + half of the top margin
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

	async function syncZoomValue() {
		const gridItem = findGridItem($app, id)
		if (gridItem) {
			gridItem.data.configuration.zoom.value = zoom
		}
		console.log(gridItem?.data.configuration.zoom.value)
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
<InputValue {id} input={configuration.zoom} bind:value={zoom} />

<div class="relative w-full h-full bg-gray-100">
	{#if source}
		{#if pages?.length}
			<div
				bind:clientWidth={controlsWidth}
				bind:clientHeight={controlsHeight}
				class="fixed flex {$mode !== 'preview'
					? 'w-[calc(100%-2px)] top-[1px]'
					: 'w-full top-0'} {wideView
					? 'justify-center gap-14'
					: '!justify-between'} overflow-x-auto bg-white border mx-auto py-1"
			>
				<div class="flex justify-start items-center px-2 text-gray-600 text-sm">
					<Button
						on:click={() => (zoom -= 10)}
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
							on:click={() => (zoom = 100)}
							disabled={!doc}
							size="xs"
							color="light"
							variant="border"
							title="Reset zoom"
							aria-label="Reset zoom"
							btnClasses="!w-[50px] !font-medium !rounded-none !border-l-0 !px-1"
						>
							{zoom.toFixed(0)}%
						</Button>
					{/if}
					<Button
						on:click={() => renderPdf(true, true)}
						disabled={!doc}
						size="xs"
						color="light"
						variant="border"
						title="Scale to viewport"
						aria-label="Scale to viewport"
						btnClasses="!rounded-none !border-l-0 !px-2"
					>
						<MoveHorizontal size={16} />
					</Button>
					<Button
						on:click={() => (zoom += 10)}
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
			<div
				out:fade={{ duration: 200 }}
				class="absolute inset-0 center-center flex-col text-center text-sm bg-white text-gray-600"
			>
				<Loader2 class="animate-spin mb-2" />
				Loading PDF
			</div>
		{/if}
		<div
			bind:this={wrapper}
			on:scroll={throttledScroll}
			class={twMerge('w-full overflow-auto bg-gray-100', css?.container?.class ?? '')}
			style="padding-top: {controlsHeight ?? 0}px; {css?.container?.style ?? ''}"
		/>
	{/if}
	{#if $mode !== 'preview' && $selectedComponent === id}
		<button
			class="fixed z-10 bottom-0 left-0 px-2 py-0.5 bg-indigo-500/90 
			hover:bg-indigo-500 focus:bg-indigo-500 duration-200 text-white text-2xs"
			on:click={syncZoomValue}
		>
			Sync zoom value
		</button>
	{/if}
	{#if error}
		<div
			class="absolute inset-0 z-20 center-center 
		bg-gray-100 text-center text-gray-600 text-sm"
		>
			{error}
		</div>
	{/if}
</div>
