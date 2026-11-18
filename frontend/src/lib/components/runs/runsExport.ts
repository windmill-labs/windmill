import type { Job } from '$lib/gen'
import { Parser } from '@json2csv/plainjs'
import { download } from '$lib/utils'

export type RunsExportFormat = 'csv' | 'json' | 'xlsx'

type Cell = string | number | boolean | null

function runStatus(job: Job): string {
	if (job.type === 'CompletedJob') {
		if (job.is_skipped) return 'skipped'
		if (job.canceled) return 'canceled'
		return job.success ? 'success' : 'failure'
	}
	if (job.canceled) return 'canceling'
	const running = 'running' in job && job.running
	if (running && 'suspend' in job && job.suspend) return 'suspended'
	return running ? 'running' : 'queued'
}

// One flat row per run, every field the list returns that is worth analysing, whatever
// columns the table shows.
export function runsExportRows(jobs: Job[]): Record<string, Cell>[] {
	return jobs.map((job) => ({
		id: job.id,
		workspace_id: job.workspace_id ?? null,
		status: runStatus(job),
		resolved: job.type === 'CompletedJob' ? (job.resolved ?? false) : null,
		path: job.script_path ?? null,
		job_kind: job.job_kind,
		created_at: job.created_at ?? null,
		scheduled_for: job.type === 'QueuedJob' ? (job.scheduled_for ?? null) : null,
		started_at: job.started_at ?? null,
		completed_at: job.type === 'CompletedJob' ? (job.completed_at ?? null) : null,
		duration_ms: job.type === 'CompletedJob' ? job.duration_ms : null,
		triggered_by: job.created_by ?? null,
		trigger_kind: job.trigger_kind ?? null,
		trigger_path: job.trigger_path ?? null,
		permissioned_as: job.permissioned_as,
		email: job.email ?? null,
		tag: job.tag ?? null,
		worker: job.worker ?? null,
		labels: ('labels' in job ? job.labels?.join(', ') : undefined) ?? null,
		parent_job: job.parent_job ?? null,
		is_flow_step: job.is_flow_step ?? null,
		priority: job.priority ?? null,
		mem_peak: job.mem_peak ?? null
	}))
}

// Labels, tags and paths are set by whoever runs a job, and a spreadsheet evaluates a cell
// starting with one of these as a formula, so they are quoted as text.
const FORMULA_PREFIX = /^[=+\-@\t\r]/

export function toCsv(rows: Record<string, Cell>[]): string {
	const fields = Object.keys(rows[0] ?? {})
	const safe = rows.map((row) =>
		Object.fromEntries(
			Object.entries(row).map(([k, v]) => [
				k,
				typeof v === 'string' && FORMULA_PREFIX.test(v) ? `'${v}` : v
			])
		)
	)
	return new Parser({ fields }).parse(safe)
}

export async function exportRuns(jobs: Job[], format: RunsExportFormat): Promise<void> {
	const rows = runsExportRows(jobs)
	const stamp = new Date().toISOString().replace(/[:.]/g, '-')
	const filename = `runs-${stamp}.${format}`
	if (format === 'json') {
		download(filename, JSON.stringify(rows, null, 2), 'application/json')
	} else if (format === 'csv') {
		download(filename, toCsv(rows), 'text/csv;charset=utf-8')
	} else {
		const blob = await toXlsx(rows)
		const url = URL.createObjectURL(blob)
		const a = document.createElement('a')
		a.href = url
		a.download = filename
		a.click()
		setTimeout(() => URL.revokeObjectURL(url), 100)
	}
}

function escapeXml(s: string): string {
	return s
		.replace(/[\u0000-\u0008\u000B\u000C\u000E-\u001F]/g, '')
		.replace(/&/g, '&amp;')
		.replace(/</g, '&lt;')
		.replace(/>/g, '&gt;')
		.replace(/"/g, '&quot;')
}

function columnName(i: number): string {
	let name = ''
	for (let n = i + 1; n > 0; n = Math.floor((n - 1) / 26)) {
		name = String.fromCharCode(65 + ((n - 1) % 26)) + name
	}
	return name
}

function xlsxCell(value: Cell, ref: string): string {
	if (value === null || value === '') return ''
	if (typeof value === 'number') return `<c r="${ref}"><v>${value}</v></c>`
	if (typeof value === 'boolean') return `<c r="${ref}" t="b"><v>${value ? 1 : 0}</v></c>`
	return `<c r="${ref}" t="inlineStr"><is><t xml:space="preserve">${escapeXml(value)}</t></is></c>`
}

// A single-sheet workbook with inline strings: the smallest part set Excel, Numbers and
// LibreOffice all open.
export async function toXlsx(rows: Record<string, Cell>[]): Promise<Blob> {
	const { default: JSZip } = await import('jszip')
	const headers = Object.keys(rows[0] ?? {})
	const sheetRows = [headers as Cell[], ...rows.map((r) => headers.map((h) => r[h]))].map(
		(cells, r) =>
			`<row r="${r + 1}">${cells.map((v, c) => xlsxCell(v, `${columnName(c)}${r + 1}`)).join('')}</row>`
	)
	const zip = new JSZip()
	zip.file(
		'[Content_Types].xml',
		'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Types xmlns="http://schemas.openxmlformats.org/package/2006/content-types"><Default Extension="rels" ContentType="application/vnd.openxmlformats-package.relationships+xml"/><Default Extension="xml" ContentType="application/xml"/><Override PartName="/xl/workbook.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.sheet.main+xml"/><Override PartName="/xl/worksheets/sheet1.xml" ContentType="application/vnd.openxmlformats-officedocument.spreadsheetml.worksheet+xml"/></Types>'
	)
	zip.file(
		'_rels/.rels',
		'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/officeDocument" Target="xl/workbook.xml"/></Relationships>'
	)
	zip.file(
		'xl/workbook.xml',
		'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><workbook xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main" xmlns:r="http://schemas.openxmlformats.org/officeDocument/2006/relationships"><sheets><sheet name="Runs" sheetId="1" r:id="rId1"/></sheets></workbook>'
	)
	zip.file(
		'xl/_rels/workbook.xml.rels',
		'<?xml version="1.0" encoding="UTF-8" standalone="yes"?><Relationships xmlns="http://schemas.openxmlformats.org/package/2006/relationships"><Relationship Id="rId1" Type="http://schemas.openxmlformats.org/officeDocument/2006/relationships/worksheet" Target="worksheets/sheet1.xml"/></Relationships>'
	)
	zip.file(
		'xl/worksheets/sheet1.xml',
		`<?xml version="1.0" encoding="UTF-8" standalone="yes"?><worksheet xmlns="http://schemas.openxmlformats.org/spreadsheetml/2006/main"><sheetData>${sheetRows.join('')}</sheetData></worksheet>`
	)
	return zip.generateAsync({
		type: 'blob',
		compression: 'DEFLATE',
		mimeType: 'application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'
	})
}
