<script lang="ts">
	import DisplayResult from './DisplayResult.svelte'

	import { Highlight } from 'svelte-highlight'
	import { json } from 'svelte-highlight/languages'
	import { copyToClipboard, parseS3Object, roughSizeOfObject } from '$lib/utils'
	import ExpandableImage from '$lib/components/common/image/ExpandableImage.svelte'
	import { base } from '$lib/base'
	import { downloadViaClient, shouldDownloadViaClient } from '$lib/utils/downloadFile'
	import { appendViewToken } from '$lib/viewToken'
	import { Badge, Button, Drawer, DrawerContent } from './common'
	import {
		ClipboardCopy,
		Download,
		PanelRightOpen,
		Table2,
		Braces,
		Highlighter,
		ArrowDownFromLine,
		Database,
		Loader2
	} from 'lucide-svelte'
	import DucklakeResultPreview from './assets/AssetGraph/DucklakeResultPreview.svelte'
	import DataTestsResult from './DataTestsResult.svelte'
	import Portal from '$lib/components/Portal.svelte'
	import DisplayResultControlBar from './DisplayResultControlBar.svelte'

	import ObjectViewer from './propertyPicker/ObjectViewer.svelte'
	import S3FilePicker from './S3FilePicker.svelte'
	import Alert from './common/alert/Alert.svelte'
	import AutoDataTable from './table/AutoDataTable.svelte'
	import Markdown from 'svelte-exmarkdown'
	import Toggle from './Toggle.svelte'
	import FileDownload from './common/fileDownload/FileDownload.svelte'

	import ParqetTableRenderer from './ParqetCsvTableRenderer.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import MapResult from './MapResult.svelte'
	import DownloadCsv from './table/DownloadCsv.svelte'
	import { convertJsonToCsv } from './table/tableUtils'
	import Tooltip from './Tooltip.svelte'
	import HighlightTheme from './HighlightTheme.svelte'
	import type { DisplayResultUi } from './custom_ui'
	import { getContext, hasContext, createEventDispatcher, onDestroy, untrack } from 'svelte'
	import { toJsonStr } from '$lib/utils'
	import { userStore } from '$lib/stores'
	import ResultStreamDisplay from './ResultStreamDisplay.svelte'
	import { twMerge } from 'tailwind-merge'
	import DOMPurify from 'dompurify'
	import MarkupApprovalGate from './MarkupApprovalGate.svelte'
	import type { MarkupTrust } from './apps/markupTrust'

	const TABLE_MAX_SIZE = 5000000
	const DISPLAY_MAX_SIZE = 100000

	const IMG_KINDS = ['png', 'svg', 'jpeg', 'gif']
	const NO_SIZE_LIMIT_KINDS = [...IMG_KINDS, 'pdf', 'file', 'filename', 'html']

	const dispatch = createEventDispatcher()

	type ResultKind =
		| 'json'
		| 'table-col'
		| 'table-row'
		| 'table-row-object'
		| 'html'
		| 'png'
		| 'file'
		| 'jpeg'
		| 'gif'
		| 'error'
		| 'approval'
		| 'svg'
		| 'filename'
		| 's3object'
		| 's3object-list'
		| 'materialized'
		| 'plain'
		| 'markdown'
		| 'map'
		| 'nondisplayable'
		| 'pdf'
		| undefined
	let resultKind: ResultKind = $state()
	let length = $state(1)

	let hasBigInt = $state(false)

	interface Props {
		result: any
		/** How to treat `result.html` / `result.svg`. Defaults to sanitizing, which is
		 * what every non-app caller wants. Low-code app display components pass
		 * `getAppMarkupTrust()`. */
		markupTrust?: MarkupTrust
		filename?: string | undefined
		disableExpand?: boolean
		jobId?: string | undefined
		workspaceId?: string | undefined
		hideAsJson?: boolean
		noControls?: boolean
		drawerOpen?: boolean
		nodeId?: string | undefined
		loading?: boolean | undefined
		language?: string | undefined
		appPath?: string | undefined
		customUi?: DisplayResultUi | undefined
		isTest?: boolean
		externalToolbarAvailable?: boolean
		forceJson?: boolean
		result_stream?: string | undefined
		fixTableSizingToParent?: boolean
		copilot_fix?: import('svelte').Snippet
		children?: import('svelte').Snippet
		growVertical?: boolean
	}

	let {
		result,
		markupTrust = 'sanitize',
		filename = undefined,
		disableExpand = false,
		jobId = undefined,
		workspaceId = undefined,
		hideAsJson = false,
		noControls = false,
		drawerOpen = $bindable(false),
		nodeId = undefined,
		language = undefined,
		appPath = undefined,
		customUi = undefined,
		isTest = true,
		externalToolbarAvailable = false,
		forceJson = $bindable(false),
		result_stream = undefined,
		fixTableSizingToParent = false,
		copilot_fix,
		children,
		loading = false,
		growVertical = false
	}: Props = $props()
	let s3FileDisplayRawMode = $state(false)

	// Build the image/PDF source URL for an S3 object. When `appPath` is set
	// (deployed app view) the read is authorized on-behalf of the app author via
	// the provenance-gated `apps_u/download_s3_file/{appPath}` endpoint; otherwise
	// (editor/preview) it uses the viewer-scoped `job_helpers/load_image_preview`.
	function s3DisplayUrl(s3object: { s3: string; storage?: string; presigned?: string }): string {
		const endpoint = appPath
			? `apps_u/download_s3_file/${appPath}`
			: 'job_helpers/load_image_preview'
		const keyParam = appPath ? 's3' : 'file_key'
		let url = `/api/w/${workspaceId}/${endpoint}?${keyParam}=${encodeURIComponent(s3object.s3)}`
		if (s3object.storage) {
			url += `&storage=${s3object.storage}`
		}
		if (appPath && s3object.presigned) {
			url += `&${s3object.presigned}`
		}
		return url
	}

	function isTableRow(result: any): boolean {
		return Array.isArray(result) && result.every((x) => Array.isArray(x))
	}

	function isTableCol(result: any, keys: string[]): boolean {
		return (
			!Array.isArray(result) &&
			keys.map((k) => Array.isArray(result[k])).reduce((a, b) => a && b, true)
		)
	}

	function isTableRowObject(json) {
		// check array of objects (with possible a first row of headers)
		const hasHeaders =
			Array.isArray(json[0]) &&
			json[0].length > 0 &&
			json[0].length <= 50 &&
			json[0].every((item) => typeof item === 'string')
		return isTableRowObjectInner(json, hasHeaders)
	}

	function isTableRowObjectInner(json: any, hasHeaders: boolean) {
		return (
			Array.isArray(json) &&
			json.length > (hasHeaders ? 1 : 0) &&
			json.every((item, index) => {
				if (hasHeaders && index === 0) {
					return true
				}
				if (item && typeof item === 'object') {
					let keys = Object.keys(item)
					if (keys.length > 0 && !Array.isArray(item)) {
						if (hasHeaders || keys.length <= 50) {
							return true
						}
					}
				}
				return false
			})
		)
	}

	let largeObject: boolean | undefined = $state(undefined)

	let resultApiPath = $derived(
		workspaceId && jobId
			? appendViewToken(
					nodeId
						? `/w/${workspaceId}/jobs/result_by_id/${jobId}/${nodeId}`
						: `/w/${workspaceId}/jobs_u/completed/get_result/${jobId}`
				)
			: undefined
	)
	let resultDownloadHref = $derived(
		resultApiPath
			? `${base}/api${resultApiPath}`
			: `data:text/json;charset=utf-8,${encodeURIComponent(toJsonStr(result))}`
	)
	let resultDownloadName = $derived(`${filename ?? 'result'}.json`)

	function checkIfS3(result: any, keys: string[]) {
		return keys.includes('s3') && typeof result.s3 === 'string'
	}

	// The materialize-run summary — `[{ materialized: 'ducklake://…', rows,
	// snapshot_id }]` or the bare object. The shape is narrow (a `ducklake://`
	// value plus a `snapshot_id` key) so an ordinary user result isn't hijacked.
	function parseMaterializedResult(
		res: any
	):
		| { materialized: string; partition?: string; rows?: number; snapshot_id?: number | null }
		| undefined {
		const obj = Array.isArray(res) && res.length === 1 ? res[0] : res
		if (
			obj &&
			typeof obj === 'object' &&
			typeof obj.materialized === 'string' &&
			obj.materialized.startsWith('ducklake://') &&
			'snapshot_id' in obj
		) {
			return obj
		}
		return undefined
	}

	let showMaterializedPreview = $state(true)
	let is_render_all = $state(false)
	let download_as_csv = $state(false)
	function inferResultKind(result: any) {
		if (typeof result === 'string' && result.match(/s3:\/\/[^/]*\/.+/)) {
			largeObject = true
			return 's3object'
		}
		try {
			if (result === 'WINDMILL_TOO_BIG') {
				largeObject = true
				return 'json'
			}
		} catch (err) {
			return 'nondisplayable'
		}

		if (result !== undefined) {
			download_as_csv = false
			if (typeof result === 'string') {
				length = 0
				largeObject = false
				is_render_all = false
				return 'plain'
			}
			try {
				let keys = result && typeof result === 'object' ? Object.keys(result) : []

				if (parseMaterializedResult(result)) {
					largeObject = false
					is_render_all = false
					return 'materialized'
				}

				is_render_all =
					keys.length == 1 && keys.includes('render_all') && Array.isArray(result['render_all'])

				let size = roughSizeOfObject(result)
				console.debug('size of object', size)
				// Otherwise, check if the result is too large (10kb) for json

				if (size > TABLE_MAX_SIZE) {
					largeObject = true
					if (Array.isArray(result) && isTableRowObject(result)) {
						download_as_csv = true
					}
					return 'json'
				} else {
					largeObject = size > DISPLAY_MAX_SIZE
				}

				if (!largeObject) {
					hasBigInt = checkIfHasBigInt(result)
					if (hasBigInt) {
						return 'json'
					}
				}

				if (Array.isArray(result)) {
					if (result.length === 0) {
						return 'json'
					} else if (
						result.every((elt) => typeof elt === 'object' && checkIfS3(elt, Object.keys(elt)))
					) {
						largeObject = result.length > 100
						if (largeObject) {
							return 'json'
						}
						return 's3object-list'
					} else if (isTableRow(result)) {
						return 'table-row'
					} else if (isTableRowObject(result)) {
						return 'table-row-object'
					} else {
						return 'json'
					}
				} else if (
					keys.length === 1 &&
					['table-row', 'table-row-object', 'table-col'].includes(keys[0])
				) {
					return keys[0] as 'table-row' | 'table-row-object' | 'table-col'
				}

				if (keys.length != 0) {
					if (
						keys.length == 1 &&
						[...IMG_KINDS, 'html', 'map', 'pdf', 'file', 'error'].includes(keys[0])
					) {
						return !largeObject || NO_SIZE_LIMIT_KINDS.includes(keys[0])
							? (keys[0] as ResultKind)
							: 'json'
					} else if (
						!largeObject &&
						keys.includes('windmill_content_type') &&
						result['windmill_content_type'].startsWith('text/')
					) {
						return 'plain'
					} else if (keys.length === 2 && keys.includes('file') && keys.includes('filename')) {
						return 'file'
					} else if (
						keys.length === 3 &&
						keys.includes('file') &&
						keys.includes('filename') &&
						keys.includes('autodownload')
					) {
						if (result.autodownload) {
							const a = document.createElement('a')

							a.href = 'data:application/octet-stream;base64,' + result.file
							a.download = result.filename
							a.click()
						}
						return 'file'
					} else if (
						!largeObject &&
						keys.includes('resume') &&
						keys.includes('cancel') &&
						keys.includes('approvalPage')
					) {
						return 'approval'
					} else if (!largeObject && checkIfS3(result, keys)) {
						return 's3object'
					} else if (
						!largeObject &&
						keys.length === 1 &&
						(keys[0] === 'md' || keys[0] === 'markdown')
					) {
						return 'markdown'
					} else if (!largeObject && isTableCol(result, keys)) {
						return 'table-col'
					} else if (keys.length < 1000 && keys.includes('wm_renderer')) {
						const renderer = result['wm_renderer']
						if (
							typeof renderer === 'string' &&
							(!largeObject || NO_SIZE_LIMIT_KINDS.includes(renderer)) &&
							[
								'json',
								'html',
								'png',
								'file',
								'jpeg',
								'gif',
								'svg',
								'filename',
								's3object',
								'plain',
								'markdown',
								'map',
								'pdf'
							].includes(renderer)
						) {
							return renderer as ResultKind
						}
					}
				}
			} catch (err) {}
		} else {
			largeObject = false
			is_render_all = false
		}
		return 'json'
	}

	let jsonViewer: Drawer | undefined = $state()
	let s3FileViewer: S3FilePicker | undefined = $state()

	function checkIfHasBigInt(result: any) {
		if (typeof result === 'number' && Number.isInteger(result) && !Number.isSafeInteger(result)) {
			return true
		}

		if (Array.isArray(result)) {
			return result.some(checkIfHasBigInt)
		}

		if (result && typeof result === 'object') {
			return Object.values(result).some(checkIfHasBigInt)
		}
		return false
	}

	function contentOrRootString(obj: string | { filename: string; content: string } | undefined) {
		if (obj == undefined || obj == null) {
			return ''
		}
		if (typeof obj === 'string') {
			return obj
		} else if (typeof obj === 'object') {
			return obj?.['content']
		} else {
			return ''
		}
	}

	// An html/svg result is attacker-authored: any member can return one and a
	// higher-privileged user may view it. Every `{@html}` of result markup must go
	// through here, or it becomes stored XSS (GHSA-gh2j-49rx-4464). `approval` renders
	// verbatim too, so it must stay behind MarkupApprovalGate at the call site.
	function renderResultMarkup(markup: string | { filename: string; content: string } | undefined) {
		const str = contentOrRootString(markup) || ''
		return markupTrust === 'sanitize' ? DOMPurify.sanitize(str) : str
	}

	function handleArrayOfObjectsHeaders(json: any) {
		// handle possible a first row of headers
		if (
			Array.isArray(json) &&
			json.length > 0 &&
			Array.isArray(json[0]) &&
			json[0].length > 0 &&
			json[0].every((item) => typeof item === 'string')
		) {
			const headers = json[0]
			const rows: { [key: string]: string }[] = new Array(json.length - 1)

			for (let i = 1; i < json.length; i++) {
				const obj: { [key: string]: string } = {}
				const row = json[i]

				for (let j = 0; j < headers.length; j++) {
					obj[headers[j]] = row[headers[j]]
				}

				rows[i - 1] = obj
			}

			return rows
		}

		return json
	}

	// The explicit column order from a leading header row (see
	// handleArrayOfObjectsHeaders). Returned as an array so the order survives:
	// baking it into object keys loses integer-like names like "1234", which JS
	// enumerates first in ascending numeric order.
	function getForcedColumnOrder(json: any): string[] | undefined {
		if (
			Array.isArray(json) &&
			json.length > 0 &&
			Array.isArray(json[0]) &&
			json[0].length > 0 &&
			json[0].every((item) => typeof item === 'string')
		) {
			return json[0]
		}
		return undefined
	}

	type InputObject = { [key: string]: number[] }

	function objectOfArraysToObjects(input: InputObject): any[] {
		const maxLength = Math.max(...Object.values(input).map((arr) => arr.length))
		const result: Array<{
			[key: string]: number | null
		}> = []

		for (let i = 0; i < maxLength; i++) {
			const obj: { [key: string]: number | null } = {}

			for (const key of Object.keys(input)) {
				if (i < input[key].length) {
					obj[key] = input[key][i]
				} else {
					obj[key] = null
				}
			}

			result.push(obj)
		}

		return result
	}

	function arrayOfRowsToObjects(input: any) {
		if (Array.isArray(input) && input.length > 0) {
			// handle possible first row of headers
			if (
				input.length > 1 &&
				Array.isArray(input[0]) &&
				input[0].length > 0 &&
				input[0].every((item) => typeof item === 'string')
			) {
				const headers = input[0]
				const rows = input.slice(1)

				return rows.map((row) => {
					const obj: { [key: string]: string } = {}

					for (let i = 0; i < headers.length; i++) {
						obj[headers[i]] = row[i]
					}

					return obj
				})
			} else {
				return input
			}
		}
		return []
	}

	export function openDrawer() {
		jsonViewer?.openDrawer()
	}

	let globalForceJson: boolean = $state(false)

	let seeS3PreviewFileFromList = $state('')

	const disableTooltips = hasContext('disableTooltips')
		? getContext('disableTooltips') === true
		: false

	let resultHeaderHeight = $state(0)

	let toolbarLocation: 'self' | 'external' | undefined = $state(undefined)
	function chooseToolbarLocation(shouldShowToolbar: boolean, resultHeaderHeight: number) {
		if (!shouldShowToolbar) {
			toolbarLocation = undefined
		} else if (externalToolbarAvailable && resultHeaderHeight < 16) {
			toolbarLocation = 'external'
		} else {
			toolbarLocation = 'self'
		}
		dispatch('toolbar-location-changed', toolbarLocation)
	}

	export function getToolbarLocation() {
		return toolbarLocation
	}

	onDestroy(() => {
		dispatch('toolbar-location-changed', undefined)
	})

	$effect(() => {
		;[result]
		untrack(() => {
			resultKind = inferResultKind(result)
		})
	})
	$effect(() => {
		chooseToolbarLocation(
			!is_render_all &&
				resultKind != 'nondisplayable' &&
				result != undefined &&
				length != undefined &&
				largeObject != undefined &&
				!disableExpand &&
				!noControls,
			resultHeaderHeight
		)
	})

	// Per-test breakdown of a managed `// materialize` run, rendered as a
	// checklist above the raw result. On success it rides the result
	// (`data_tests: [{ test, violating }]`, a one-row array). On failure the job
	// result is the error, whose message is the worker's breakdown text — parsed
	// back into the same shape so the checklist shows on both outcomes. Both
	// formats are produced by this repo's worker (see duckdb_executor.rs); the
	// derivation is inert (undefined) for every other DisplayResult use.
	let dataTests = $derived.by(() => {
		// Both structured shapes carry `[{ test, violating, sample? }]`; the
		// sample (bounded violating-row rows) may arrive as a JSON string (the
		// worker keeps it string-typed through the summary row) and is optional
		// by contract — anything malformed degrades to no sample, never to a
		// dropped checklist.
		const normalize = (
			dt: any
		): Array<{ test: string; violating: number; sample?: Record<string, any>[] }> | undefined => {
			if (typeof dt === 'string') {
				try {
					dt = JSON.parse(dt)
				} catch {
					return undefined
				}
			}
			if (
				!Array.isArray(dt) ||
				dt.length === 0 ||
				!dt.every((x) => x && typeof x.test === 'string' && typeof x.violating === 'number')
			) {
				return undefined
			}
			return dt.map((x) => {
				let sample = x.sample
				if (typeof sample === 'string') {
					try {
						sample = JSON.parse(sample)
					} catch {
						sample = undefined
					}
				}
				if (!Array.isArray(sample) || !sample.every((r) => r && typeof r === 'object')) {
					sample = undefined
				}
				return { test: x.test, violating: x.violating, sample }
			})
		}
		// Success: structured column on the summary row.
		const row = Array.isArray(result) ? (result as any)?.[0] : (result as any)
		const fromRow = normalize(row?.data_tests)
		if (fromRow) return fromRow
		// Failure: the worker attaches the same structured breakdown (plus
		// per-failed-test samples) to the error payload.
		const fromError = normalize((result as any)?.error?.data_tests)
		if (fromError) return fromError
		// Failure fallback for results predating the structured error payload:
		// parse the worker's breakdown out of the error message.
		const msg = (result as any)?.error?.message
		if (typeof msg === 'string' && msg.includes('data tests failed on')) {
			const out: Array<{ test: string; violating: number }> = []
			for (const line of msg.split('\n')) {
				const fail = line.match(/^\s*✗\s*(.+?)\s*—\s*(\d+)\s+violating/)
				const pass = line.match(/^\s*✓\s*(.+?)\s*$/)
				if (fail) out.push({ test: fail[1], violating: parseInt(fail[2], 10) })
				else if (pass) out.push({ test: pass[1], violating: 0 })
			}
			if (out.length > 0) return out
		}
		return undefined
	})
</script>

<HighlightTheme />

{#if dataTests}
	<DataTestsResult tests={dataTests} />
{/if}

{#if result_stream && result == undefined}
	<div class="flex flex-col w-full gap-2">
		<div class="flex items-center gap-2 text-secondary text-xs">
			<Loader2 class="animate-spin" size={14} /> Streaming result
		</div>
		<ResultStreamDisplay {result_stream} />
	</div>
{:else if is_render_all}
	<div class="flex flex-col w-full gap-2">
		{#if !noControls}
			<div class="text-primary text-sm">
				<ToggleButtonGroup
					selected={globalForceJson ? 'json' : 'pretty'}
					on:selected={(ev) => {
						globalForceJson = ev.detail === 'json'
					}}
				>
					{#snippet children({ item })}
						<ToggleButton value="pretty" label="Pretty" icon={Highlighter} {item} />
						<ToggleButton value="json" label="JSON" icon={Braces} {item} />
					{/snippet}
				</ToggleButtonGroup>
			</div>
		{/if}
		<div class="flex flex-col w-full gap-10">
			{#each result['render_all'] as res}
				<DisplayResult
					{noControls}
					result={res}
					{markupTrust}
					{filename}
					{disableExpand}
					{jobId}
					{nodeId}
					{workspaceId}
					{appPath}
					forceJson={globalForceJson}
					hideAsJson={true}
				/>
			{/each}
		</div>
	</div>
{:else if resultKind === 'nondisplayable'}
	<div class="text-red-400">Non displayable object</div>
{:else}
	<div
		class={twMerge(
			'inline-highlight relative grow flex flex-col',
			['plain', 'markdown'].includes(resultKind ?? '') ? 'min-h-0' : 'min-h-[160px]',
			growVertical ? '' : 'h-full'
		)}
	>
		{#if result != undefined && length != undefined && largeObject != undefined}
			<div class="flex justify-between items-center w-full">
				<div
					class="text-primary text-sm flex flex-row gap-2 items-center"
					bind:clientHeight={resultHeaderHeight}
				>
					{#if !hideAsJson && !['json', 's3object'].includes(resultKind ?? '') && typeof result === 'object'}<ToggleButtonGroup
							selected={forceJson ? 'json' : resultKind?.startsWith('table-') ? 'table' : 'pretty'}
							on:selected={(ev) => {
								forceJson = ev.detail === 'json'
							}}
						>
							{#snippet children({ item })}
								{#if ['table-col', 'table-row', 'table-row-object'].includes(resultKind ?? '')}
									<ToggleButton size="sm" value="table" label="Table" icon={Table2} {item} />
								{:else}
									<ToggleButton size="sm" value="pretty" label="Pretty" icon={Highlighter} {item} />
								{/if}
								<ToggleButton size="sm" value="json" label="JSON" icon={Braces} {item} />
							{/snippet}
						</ToggleButtonGroup>
					{/if}
				</div>

				<div class="text-secondary text-xs flex gap-2.5 z-10 items-center">
					{#if customUi?.disableAiFix !== true}
						{@render copilot_fix?.()}
					{/if}
					{#if toolbarLocation === 'self'}
						<!-- TODO : When svelte 5 is released, use a snippet to pass the toolbar to a parent -->
						<DisplayResultControlBar
							{customUi}
							{filename}
							{workspaceId}
							{jobId}
							{nodeId}
							{base}
							{result}
							{disableTooltips}
							on:open-drawer={() => openDrawer()}
						/>
					{/if}
				</div>
			</div>
			<div class="grow relative">
				{#if !forceJson && resultKind === 'table-col'}
					{@const data =
						typeof result === 'object' && 'table-col' in result ? result['table-col'] : result}
					<AutoDataTable
						class={fixTableSizingToParent
							? 'absolute inset-0 [&>div]:h-full [&>div]:min-h-[10rem]'
							: ''}
						objects={objectOfArraysToObjects(data)}
					/>
				{:else if !forceJson && resultKind === 'table-row'}
					{@const data =
						typeof result === 'object' && 'table-row' in result ? result['table-row'] : result}
					<AutoDataTable
						class={fixTableSizingToParent
							? 'absolute inset-0 [&>div]:h-full [&>div]:min-h-[10rem]'
							: ''}
						objects={arrayOfRowsToObjects(data)}
					/>
				{:else if !forceJson && resultKind === 'table-row-object'}
					{@const data =
						typeof result === 'object' && 'table-row-object' in result
							? result['table-row-object']
							: result}
					<AutoDataTable
						class={fixTableSizingToParent
							? 'absolute inset-0 [&>div]:h-full [&>div]:min-h-[10rem]'
							: ''}
						objects={handleArrayOfObjectsHeaders(data)}
						headerOrder={getForcedColumnOrder(data)}
					/>
				{:else if !forceJson && resultKind === 'html'}
					<div class="h-full">
						{#if markupTrust === 'approval'}
							<MarkupApprovalGate>
								{#snippet children()}
									<!-- eslint-disable-next-line svelte/no-at-html-tags -->
									{@html renderResultMarkup(result.html)}
								{/snippet}
							</MarkupApprovalGate>
						{:else}
							<!-- eslint-disable-next-line svelte/no-at-html-tags -->
							{@html renderResultMarkup(result.html)}
						{/if}
					</div>
				{:else if !forceJson && resultKind === 'map'}
					<div class="h-full" data-interactive>
						<MapResult
							lat={result.map.lat}
							lon={result.map.lon}
							zoom={result.map.zoom}
							markers={result.map.markers}
						/>
					</div>
				{:else if !forceJson && resultKind === 'png'}
					<div class="h-full">
						<ExpandableImage
							alt="png rendered"
							class="w-auto h-full"
							src="data:image/png;base64,{contentOrRootString(result.png)}"
						/>
					</div>
				{:else if !forceJson && resultKind === 'jpeg'}
					<div class="h-full">
						<ExpandableImage
							alt="jpeg rendered"
							class="w-auto h-full"
							src="data:image/jpeg;base64,{contentOrRootString(result.jpeg)}"
						/>
					</div>
				{:else if !forceJson && resultKind === 'svg'}
					{@const svgMarkup = contentOrRootString(result.svg) || ''}
					<div>
						<a
							download="windmill.svg"
							href="data:image/svg+xml;charset=utf-8,{encodeURIComponent(svgMarkup)}">Download</a
						>
					</div>
					<div class="h-full overflow-auto">
						{#if markupTrust === 'approval'}
							<MarkupApprovalGate>
								{#snippet children()}
									<!-- eslint-disable-next-line svelte/no-at-html-tags -->
									{@html renderResultMarkup(result.svg)}
								{/snippet}
							</MarkupApprovalGate>
						{:else}
							<!-- eslint-disable-next-line svelte/no-at-html-tags -->
							{@html renderResultMarkup(result.svg)}
						{/if}
					</div>
				{:else if !forceJson && resultKind === 'gif'}
					<div class="h-full">
						<ExpandableImage
							alt="gif rendered"
							class="w-auto h-full"
							src="data:image/gif;base64,{contentOrRootString(result.gif)}"
						/>
					</div>
				{:else if !forceJson && resultKind === 'pdf'}
					<div class="h-96 mt-2 border">
						{#await import('$lib/components/display/PdfViewer.svelte')}
							<Loader2 class="animate-spin" />
						{:then Module}
							<Module.default
								allowFullscreen
								source="data:application/pdf;base64,{contentOrRootString(result.pdf)}"
							/>
						{/await}
					</div>
				{:else if !forceJson && resultKind === 'plain'}<div class="h-full text-2xs"
						><pre class="whitespace-pre-wrap"
							>{typeof result === 'string' ? result : result?.['result']}</pre
						>{#if !noControls && !loading}
							<div class="flex">
								<Button
									on:click={() =>
										copyToClipboard(typeof result === 'string' ? result : result?.['result'])}
									variant="subtle"
									unifiedSize="sm"
									endIcon={{ icon: ClipboardCopy }}
								>
									Copy
								</Button>
							</div>
						{/if}
					</div>
				{:else if !forceJson && resultKind === 'file'}
					<div>
						<a
							download={result.filename ?? result.file?.filename ?? 'windmill.file'}
							href="data:application/octet-stream;base64,{contentOrRootString(result.file)}"
							>Download</a
						>
					</div>
				{:else if !forceJson && resultKind === 'error' && result?.error}
					<div class="flex flex-col items-start">
						<span class="text-red-500 pt-2 font-semibold !text-xs whitespace-pre-wrap"
							>{#if result.error.name || result.error.message}{result.error.name}: {result.error
									.message}{:else}{JSON.stringify(result.error, null, 4)}{/if}</span
						>
						<pre class="text-xs pt-2 whitespace-pre-wrap text-primary"
							>{result.error.stack ?? ''}</pre
						>
						{#if result.error?.extra}
							<pre class="text-xs pt-2 whitespace-pre-wrap text-primary"
								>{JSON.stringify(result.error.extra, null, 4)}</pre
							>
						{/if}
						{@render children?.()}
					</div>
					{#if !isTest && language === 'bun'}
						<div class="pt-20"></div>
						<Alert size="xs" type="info" title="Seeing an odd error?">
							Bun script are bundled for performance reasons. If you see an odd error that doesn't
							appear when testing (which doesn't use bundling), try putting <code>//nobundling</code
							> at the top of your script to disable bundling and feel free to mention it to the Windmill's
							team.
						</Alert>
					{/if}
					{#if language === 'python3' && result?.error?.message?.includes('ImportError: cannot import name')}
						<Alert size="xs" type="info" title="Seeing an odd import error?">
							Python requirements inference may be inaccurate. This is due to the fact that
							requirement names can vary from package names they provide. Try to <a
								href="https://www.windmill.dev/docs/advanced/dependencies_in_python#pinning-dependencies-and-requirements"
								target="_blank"
								rel="noopener noreferrer">manually pin requirements</a
							>
						</Alert>
					{/if}
					{#if language === 'python3' && result?.error?.message?.startsWith('execution error:\npip compile failed')}
						<Alert size="xs" type="info" title="Seeing an odd resolution error?">
							Python requirements inference may be inaccurate. This is due to the fact that
							requirement names can vary from package names they provide. Try to <a
								href="https://www.windmill.dev/docs/advanced/dependencies_in_python#pinning-dependencies-and-requirements"
								target="_blank"
								rel="noopener noreferrer">manually pin requirements</a
							>
						</Alert>
					{/if}
				{:else if !forceJson && resultKind === 'approval'}<div
						class="flex flex-col gap-3 mt-2 mx-4"
					>
						<Button
							variant="accent"
							on:click={() =>
								fetch(result['resume'], {
									method: 'POST',
									body: JSON.stringify({}),
									headers: { 'Content-Type': 'application/json' }
								})}
						>
							Resume</Button
						>
						<Button variant="default" destructive on:click={() => fetch(result['cancel'])}
							>Cancel</Button
						>
						<div class="center-center"
							><a rel="noreferrer" target="_blank" href={result['approvalPage']}>Approval Page</a
							></div
						>
					</div>
				{:else if !forceJson && resultKind === 'materialized'}
					{@const m = parseMaterializedResult(result)}
					{#if m}
						<div class="flex flex-col gap-2 w-full">
							<div class="flex items-center gap-2 flex-wrap text-xs">
								<Database size={14} class="text-tertiary shrink-0" />
								<span class="font-mono text-emphasis break-all">{m.materialized}</span>
								{#if m.partition}
									<Badge color="blue">partition {m.partition}</Badge>
								{/if}
								{#if typeof m.rows === 'number'}
									<Badge color="green">
										{m.rows}
										{m.rows === 1 ? 'row' : 'rows'}{m.partition ? ' in partition' : ''}
									</Badge>
								{/if}
								{#if m.snapshot_id != null}
									<Badge color="gray">snapshot {m.snapshot_id}</Badge>
								{/if}
							</div>
							<Toggle
								class="flex"
								bind:checked={showMaterializedPreview}
								size="xs"
								options={{ right: 'Preview rows' }}
							/>
							{#if showMaterializedPreview}
								<div class="border rounded-md h-80 min-h-0 overflow-hidden">
									<DucklakeResultPreview
										assetUri={m.materialized}
										partition={m.partition}
										class="h-full"
									/>
								</div>
							{/if}
						</div>
					{/if}
				{:else if !forceJson && resultKind === 's3object'}
					{@const s3object = parseS3Object(result) as typeof result}
					<div
						class="h-full w-full {typeof s3object?.s3 === 'string' &&
						s3object?.s3?.endsWith('.parquet')
							? 'h-min-[600px]'
							: ''}"
					>
						<div class="flex flex-col gap-2">
							<Toggle
								class="flex"
								bind:checked={s3FileDisplayRawMode}
								size="xs"
								options={{ right: 'Raw S3 object' }}
							/>

							{#if s3FileDisplayRawMode}
								<Highlight
									class=""
									language={json}
									code={toJsonStr(s3object).replace(/\\n/g, '\n')}
								/>
								{#if $userStore}
									<button
										class="text-secondary underline text-2xs whitespace-nowrap"
										onclick={() => {
											s3FileViewer?.open?.(s3object)
										}}
										><span class="flex items-center gap-1"
											><PanelRightOpen size={12} />object store explorer<Tooltip
												>Require admin privilege or "S3 resource details and content can be accessed
												by all users of this workspace" of S3 Storage to be set in the workspace
												settings</Tooltip
											></span
										>
									</button>
								{/if}
							{:else if !s3object?.disable_download}
								<FileDownload {workspaceId} {s3object} {appPath} />
								{#if $userStore}
									<button
										class="text-secondary underline text-2xs whitespace-nowrap"
										onclick={() => {
											s3FileViewer?.open?.(s3object)
										}}
										><span class="flex items-center gap-1"
											><PanelRightOpen size={12} />object store explorer<Tooltip
												>Require admin privilege or "S3 resource details and content can be accessed
												by all users of this workspace" of S3 Storage to be set in the workspace
												settings</Tooltip
											></span
										>
									</button>
								{/if}
							{/if}
						</div>
						{#if typeof s3object?.s3 === 'string'}
							{#if s3object?.s3?.endsWith('.parquet') || s3object?.s3?.endsWith('.csv')}
								{#key s3object.s3}
									<ParqetTableRenderer
										disable_download={s3object?.disable_download}
										{workspaceId}
										{appPath}
										s3resource={s3object?.s3}
										storage={s3object?.storage}
										presigned={s3object?.presigned}
									/>
								{/key}
							{:else if s3object?.s3?.endsWith('.png') || s3object?.s3?.endsWith('.jpeg') || s3object?.s3?.endsWith('.jpg') || s3object?.s3?.endsWith('.webp')}
								<div class="h-full mt-2">
									<ExpandableImage
										alt="preview rendered"
										title={s3object?.s3}
										class="w-auto h-full"
										src={s3DisplayUrl(s3object)}
									/>
								</div>
							{:else if s3object?.s3?.endsWith('.pdf')}
								<div class="h-96 mt-2 border">
									{#await import('$lib/components/display/PdfViewer.svelte')}
										<Loader2 class="animate-spin" />
									{:then Module}
										<Module.default allowFullscreen source={s3DisplayUrl(s3object)} />
									{/await}
								</div>
							{/if}
						{/if}
					</div>
				{:else if !forceJson && resultKind === 's3object-list'}
					<div class="h-full w-full">
						<div class="flex flex-col gap-2">
							<Toggle
								class="flex mt-1"
								bind:checked={s3FileDisplayRawMode}
								size="xs"
								options={{ right: 'Raw S3 object' }}
							/>
							{#each result as _s3object}
								{@const s3object = parseS3Object(_s3object) as typeof _s3object}
								{#if s3FileDisplayRawMode}
									<Highlight
										class=""
										language={json}
										code={toJsonStr(s3object).replace(/\\n/g, '\n')}
									/>
									<button
										class="text-primary text-2xs whitespace-nowrap"
										onclick={() => {
											s3FileViewer?.open?.(s3object)
										}}
										><span class="flex items-center gap-1"
											><PanelRightOpen size={12} />open preview</span
										>
									</button>
								{:else if !s3object?.disable_download}
									<FileDownload {workspaceId} {s3object} {appPath} />
								{:else}
									<div class="flex text-primary pt-2">{s3object?.s3} (download disabled)</div>
								{/if}
								{#if s3object?.s3?.endsWith('.parquet') || s3object?.s3?.endsWith('.csv')}
									{#if seeS3PreviewFileFromList == s3object?.s3}
										<ParqetTableRenderer
											disable_download={s3object?.disable_download}
											{workspaceId}
											{appPath}
											s3resource={s3object?.s3}
											storage={s3object?.storage}
											presigned={s3object?.presigned}
										/>{:else}
										<button
											class="text-primary whitespace-nowrap flex gap-2 items-center"
											onclick={() => {
												seeS3PreviewFileFromList = s3object?.s3
											}}
											>open table preview <ArrowDownFromLine />
										</button>
									{/if}
								{:else if s3object?.s3?.endsWith('.png') || s3object?.s3?.endsWith('.jpeg') || s3object?.s3?.endsWith('.jpg') || s3object?.s3?.endsWith('.webp')}
									{#if seeS3PreviewFileFromList == s3object?.s3}
										<div class="h-full mt-2">
											<ExpandableImage
												alt="preview rendered"
												title={s3object?.s3}
												class="w-auto h-full"
												src={s3DisplayUrl(s3object)}
											/>
										</div>
									{:else}
										<button
											class="text-primary whitespace-nowrap flex gap-2 items-center"
											onclick={() => {
												seeS3PreviewFileFromList = s3object?.s3
											}}
											>open image preview <ArrowDownFromLine />
										</button>
									{/if}
								{:else if s3object?.s3?.endsWith('.pdf')}
									<div class="h-96 mt-2 border" data-interactive>
										{#await import('$lib/components/display/PdfViewer.svelte')}
											<Loader2 class="animate-spin" />
										{:then Module}
											<Module.default allowFullscreen source={s3DisplayUrl(s3object)} />
										{/await}
									</div>
								{/if}
							{/each}
						</div>
					</div>
				{:else if !forceJson && resultKind === 'markdown'}
					<div class="prose-xs dark:prose-invert !list-disc !list-outside">
						<Markdown md={result?.md ?? result?.markdown} />
					</div>
				{:else if largeObject || hasBigInt}
					{#if result && typeof result === 'object' && 'file' in result}
						<div
							><a
								download={result.filename ?? result.file?.filename ?? 'windmill.file'}
								href="data:application/octet-stream;base64,{contentOrRootString(result.file)}"
								>Download</a
							>
						</div>
					{:else}
						{#if largeObject}
							<div class="text-xs text-emphasis"
								>{#if resultApiPath && shouldDownloadViaClient()}
									<button onclick={() => downloadViaClient(resultApiPath!, resultDownloadName)}>
										Download {filename ? '' : 'as JSON'}
									</button>
								{:else}
									<a download={resultDownloadName} href={resultDownloadHref}>
										Download {filename ? '' : 'as JSON'}
									</a>
								{/if}
								{#if download_as_csv}
									<DownloadCsv
										getContent={() => convertJsonToCsv(result)}
										customText="Download as CSV"
									/>
								{/if}
							</div>

							<div class="mt-1 mb-2">
								<Alert
									size="xs"
									title="Large result detected"
									type="warning"
									tooltip="We recommend using persistent object storage for large result. See docs for setting up an object storage service integration using s3 or any other s3 compatible services."
									documentationLink="https://www.windmill.dev/docs/core_concepts/persistent_storage#object-storage-for-large-data-s3-r2-minio-azure-blob"
								/>
							</div>
						{/if}
						{#if result && result != 'WINDMILL_TOO_BIG'}
							<ObjectViewer json={result} />
						{/if}
					{/if}
				{:else if typeof result === 'string' && result.length > 0}
					<pre class="text-sm">{result}</pre>{#if !noControls}<div class="flex">
							<Button
								on:click={() => copyToClipboard(result)}
								variant="subtle"
								unifiedSize="sm"
								endIcon={{ icon: ClipboardCopy }}
							></Button>
						</div>
					{/if}
				{:else}
					<Highlight
						class={twMerge(forceJson ? 'pt-1' : 'h-full w-full', '!bg-surface-primary')}
						language={json}
						code={toJsonStr(result).replace(/\\n/g, '\n')}
					/>
				{/if}
			</div>
		{:else if typeof result === 'string' && resultKind === 'plain'}
			<div class="h-full text-xs">
				<pre>{result}</pre>
				{#if !noControls}
					<div class="flex">
						<Button
							on:click={() => copyToClipboard(result)}
							variant="subtle"
							unifiedSize="md"
							endIcon={{ icon: ClipboardCopy }}
						></Button>
					</div>
				{/if}
			</div>
		{:else}
			<div class="text-primary text-xs">No result: {toJsonStr(result)}</div>
		{/if}
	</div>

	{#if !disableExpand && !noControls}
		<Drawer bind:this={jsonViewer} bind:open={drawerOpen} size="900px">
			<DrawerContent title="Expanded Result" on:close={jsonViewer.closeDrawer}>
				{#snippet actions()}
					{#if customUi?.disableDownload !== true}
						{#if resultApiPath && shouldDownloadViaClient()}
							<Button
								on:click={() => downloadViaClient(resultApiPath!, resultDownloadName)}
								startIcon={{ icon: Download }}
								variant="subtle"
								unifiedSize="md"
							>
								Download
							</Button>
						{:else}
							<Button
								download={resultDownloadName}
								href={resultDownloadHref}
								startIcon={{ icon: Download }}
								variant="subtle"
								unifiedSize="md"
							>
								Download
							</Button>
						{/if}
					{/if}
					<Button
						on:click={() => copyToClipboard(toJsonStr(result))}
						variant="subtle"
						unifiedSize="md"
						startIcon={{
							icon: ClipboardCopy
						}}
					>
						Copy to clipboard
					</Button>
				{/snippet}
				<DisplayResult
					{noControls}
					{result}
					{markupTrust}
					{filename}
					{jobId}
					{nodeId}
					{workspaceId}
					{appPath}
					{hideAsJson}
					{forceJson}
					disableExpand={true}
				/>
			</DrawerContent>
		</Drawer>

		<Portal name="s3filepicker">
			<S3FilePicker bind:this={s3FileViewer} readOnlyMode={true} />
		</Portal>
	{/if}
{/if}
