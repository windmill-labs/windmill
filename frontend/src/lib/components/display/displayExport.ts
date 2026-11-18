import type { ChartBody, FenceResult, FenceTable } from './displayCatalog'
import { fenceItems, parseFence, tableRows } from './displayCatalog'
import { chartConfig } from './chartConfig'

async function chartImage(doc: Document, chart: ChartBody): Promise<HTMLElement> {
	const { Chart } = await import('chart.js/auto')
	const canvas = document.createElement('canvas')
	canvas.width = 800
	canvas.height = 400
	const config = chartConfig(chart, { dark: false, surface: '#ffffff' })
	const drawn = new Chart(canvas, {
		...config,
		options: { ...config.options, responsive: false, devicePixelRatio: 2 }
	})
	const src = drawn.toBase64Image()
	drawn.destroy()
	const figure = doc.createElement('figure')
	figure.className = 'wm-diagram'
	const img = doc.createElement('img')
	img.src = src
	img.alt = chart.title ?? `${chart.type} chart`
	img.style.maxWidth = '100%'
	figure.appendChild(img)
	return figure
}

function tableElement(doc: Document, item: FenceTable): HTMLElement {
	const { columns, rows } = tableRows(item)
	const table = doc.createElement('table')
	const head = table.createTHead().insertRow()
	for (const c of columns) head.appendChild(doc.createElement('th')).textContent = c
	const body = table.createTBody()
	for (const row of rows) {
		const tr = body.insertRow()
		for (const cell of row) tr.insertCell().textContent = cell === null ? '' : String(cell)
	}
	return table
}

async function displayElements(doc: Document, value: FenceResult): Promise<HTMLElement[]> {
	const out: HTMLElement[] = []
	for (const item of fenceItems(value)) {
		out.push('chart' in item ? await chartImage(doc, item.chart) : tableElement(doc, item))
	}
	return out
}

/**
 * Replaces each valid ```wm-display fence under `root` with what it renders as: a chart as a PNG,
 * a table as a plain HTML table, so a downloaded document carries no script. An invalid fence
 * stays as its source, as it does on screen.
 */
export async function drawDisplayFences(root: DocumentFragment | Element): Promise<void> {
	const doc = root.ownerDocument ?? document
	for (const code of Array.from(root.querySelectorAll('pre > code.language-wm-display'))) {
		const parsed = parseFence(code.textContent ?? '')
		if (!parsed.ok) continue
		code.parentElement!.replaceWith(...(await displayElements(doc, parsed.result)))
	}
}
