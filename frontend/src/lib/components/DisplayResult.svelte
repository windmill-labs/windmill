<script lang="ts">
	import DisplayResult from './DisplayResult.svelte'

	import { Highlight } from 'svelte-highlight'
	import { json } from 'svelte-highlight/languages'
	import DbtRunResult from '$lib/components/dbt/DbtRunResult.svelte'
	import { parseDbtRun } from '$lib/components/dbt/parseDbtRun'
	import { copyToClipboard, parseS3Object, roughSizeOfObject } from '$lib/utils'
	import ExpandableImage from '$lib/components/common/image/ExpandableImage.svelte'
	import {
		downloadBlob,
		downloadViaClient,
		fetchViaClient,
		shouldDownloadViaClient
	} from '$lib/utils/downloadFile'
	import { sendUserToast } from '$lib/toast'
	import { appendViewToken } from '$lib/viewToken'
	import { Badge, Button, Drawer, DrawerContent } from './common'
	import {
		ClipboardCopy,
		Download,
		Table2,
		Braces,
		Highlighter,
		Database,
		Loader2
	} from 'lucide-svelte'
	import DataTestsResult from './DataTestsResult.svelte'
	import Portal from '$lib/components/Portal.svelte'
	import DisplayResultControlBar from './DisplayResultControlBar.svelte'

	import ObjectViewer from './propertyPicker/ObjectViewer.svelte'
	import S3FilePicker from './S3FilePicker.svelte'
	import Alert from './common/alert/Alert.svelte'
	import AutoDataTable from './table/AutoDataTable.svelte'
	import Markdown from 'svelte-exmarkdown'
	import { markdownProse } from './markdownProse'
	import Toggle from './Toggle.svelte'
	import S3ObjectHeader from './common/fileDownload/S3ObjectHeader.svelte'
	import S3TextPreview from './common/fileDownload/S3TextPreview.svelte'

	import S3ObjectTable from './common/fileDownload/S3ObjectTable.svelte'
	import { S3TableView } from './common/fileDownload/s3TableView.svelte'
	import { serverTableFormat } from './s3TableOps'
	import { isMarkdownFile } from './common/fileDownload/FileTextPreview.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import MapResult from './MapResult.svelte'
	import ChartResult from './display/ChartResult.svelte'
	import { isChartResult } from './display/displayCatalog'
	import DownloadCsv from './table/DownloadCsv.svelte'
	import { convertJsonToCsv } from './table/tableUtils'
	import HighlightTheme from './HighlightTheme.svelte'
	import type { DisplayResultUi } from './custom_ui'
	import { getContext, hasContext, createEventDispatcher, onDestroy, untrack } from 'svelte'
	import { toJsonStr } from '$lib/utils'
	import { userStore } from '$lib/stores'
	import { isOfflineReplay, isReplaying } from './recording/offlineReplay.svelte'
	import ResultStreamDisplay from './ResultStreamDisplay.svelte'
	import { twMerge } from 'tailwind-merge'
	import DOMPurify from 'dompurify'
	import MarkupApprovalGate from './MarkupApprovalGate.svelte'
	import type { MarkupTrust } from './apps/markupTrust'
	import AgentResultDisplay from './AgentResultDisplay.svelte'
	import AgentStreamDisplay from './AgentStreamDisplay.svelte'
	import AgentTrace from './AgentTrace.svelte'
	import { isAgentStream, parseAgentErrorMessages, parseAgentResult } from './aiAgentResult'
	import { buildAgentTrace } from './agentTrace'

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
		| 'aiagent'
		| 'chart'
		| undefined
	let resultKind: ResultKind = $state()
	/** Kinds whose renderer leaves the page: S3/ducklake previews fetch the file or
	 * table, and `approval` renders buttons that `fetch` URLs carried in the result.
	 * A recording is caller-supplied, so on the public page those would aim a
	 * credentialed request at an arbitrary origin. */
	const REPLAY_INERT_KINDS: ResultKind[] = ['s3object', 's3object-list', 'materialized', 'approval']
	/** Kinds whose markup pulls subresources: DOMPurify stops scripting but keeps
	 * `<img src>` and SVG `<image href>`, and `map` tiles are requests by
	 * construction. Kinds absent here carry their bytes as `data:` and reach nothing,
	 * or render through a component that is itself inert on the public page —
	 * `aiagent` is the second case, via `GfmMarkdown`, which is why it renders
	 * markdown yet is not listed while `markdown` still is.
	 * Inert only on the public page, which promises to issue no requests. */
	const OFFLINE_INERT_KINDS: ResultKind[] = ['markdown', 'html', 'svg', 'map']
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
		/**
		 * Which run this result belongs to. Separate from `jobId`, which the replay
		 * page withholds so nothing fetches: the agent views still have to tell one
		 * run from the next, or a second one continues the first one's fold.
		 */
		runKey?: string | undefined
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
		runKey = undefined,
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
	/** What a max-iterations failure got through before it gave up, if this is one.
	 *  Empty for a run that failed before the worker tagged anything, and for one
	 *  that predates the tags reaching this payload at all — in which case the
	 *  section is not rendered rather than heading an empty box. */
	let agentErrorTrace = $derived.by(() => {
		const messages = parseAgentErrorMessages(result)
		if (!messages) return undefined
		const entries = buildAgentTrace(messages)
		return entries.length > 0 ? entries : undefined
	})

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

	function s3PreviewKind(
		key: unknown
	): 'table' | 'image' | 'pdf' | 'text' | 'markdown' | undefined {
		if (typeof key !== 'string') return undefined
		// Previews read the file with the viewer's rights: a logged-out viewer outside an app (a
		// public run page) has none, and gets the download only.
		if (!$userStore && !appPath) return undefined
		if (serverTableFormat(key)) return 'table'
		if (['.png', '.jpeg', '.jpg', '.webp'].some((ext) => key.endsWith(ext))) return 'image'
		if (key.endsWith('.pdf')) return 'pdf'
		if (/\.(txt|log)$/i.test(key)) return 'text'
		if (isMarkdownFile(key)) return 'markdown'
		return undefined
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
			? `/api${resultApiPath}`
			: `data:text/json;charset=utf-8,${encodeURIComponent(toJsonStr(result))}`
	)
	let resultDownloadName = $derived(`${filename ?? 'result'}.json`)

	// A result over the backend's inline cap arrives as 'WINDMILL_TOO_BIG'. Ask the backend for
	// `file.filename` alone to tell a `{ file: { content, filename } }` result. The root
	// `{ file, filename }` shape is a file only when those are its sole keys, which cannot be
	// checked without reading the large value, so it keeps the JSON download.
	let tooBigFilename: string | undefined = $state(undefined)
	let downloadingTooBigFile = $state(false)
	async function fetchResultAt(apiPath: string, jsonPath: string): Promise<unknown> {
		const r = await fetchViaClient(
			`${apiPath}${apiPath.includes('?') ? '&' : '?'}json_path=${jsonPath}`
		)
		if (!r.ok) throw new Error(`${r.status} ${r.statusText}`)
		return r.json()
	}
	$effect(() => {
		tooBigFilename = undefined
		const apiPath = result === 'WINDMILL_TOO_BIG' ? resultApiPath : undefined
		if (!apiPath) return
		let stale = false
		fetchResultAt(apiPath, 'file.filename').then(
			(name) => {
				if (!stale && typeof name === 'string') tooBigFilename = name
			},
			() => {}
		)
		return () => (stale = true)
	})
	async function downloadTooBigFile(apiPath: string, name: string) {
		downloadingTooBigFile = true
		try {
			const content = await fetchResultAt(apiPath, 'file.content')
			if (typeof content !== 'string' || !content) throw new Error('the result has no file content')
			const bin = atob(content)
			// Filled byte by byte: a per-character array is several times the file's size
			const bytes = new Uint8Array(bin.length)
			for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i)
			// A blob, not a data: URL, which browsers cap well below a large file's size
			downloadBlob(new Blob([bytes]), name)
		} catch (e) {
			sendUserToast(`Download failed: ${e instanceof Error ? e.message : e}`, true)
		} finally {
			downloadingTooBigFile = false
		}
	}

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
		// The whole string, on one line: text that mentions an `s3://` URI is a plain string.
		if (typeof result === 'string' && result.match(/^s3:\/\/[^/]*\/.+$/)) {
			largeObject = false
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

				// Classified before the size caps below: an agent's answer stays small
				// however long its conversation grows, so a run with a long trace
				// must not fall back to the JSON tree that hides the answer inside it.
				// `largeObject` is still set honestly, so switching to JSON gets the
				// same too-big handling as any other oversized result.
				if (parseAgentResult(result)) {
					is_render_all = false
					largeObject = roughSizeOfObject(result) > DISPLAY_MAX_SIZE
					return 'aiagent'
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
					// Only a chart the catalog accepts: any other `{ chart }` result stays JSON. The
					// catalog caps its rows, so it is exempt from the display size limit.
					if (keys.length == 1 && keys[0] === 'chart' && isChartResult(result)) {
						return 'chart'
					} else if (
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
						// Not via REPLAY_INERT_KINDS: this download is a side effect *inside* kind
						// inference, already done by the time a caller could reclassify. Replaying
						// must never write caller-chosen bytes into the viewer's downloads.
						if (result.autodownload && !isReplaying()) {
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
	// The file browser is only mounted with the result's controls.
	let canExplore = $derived(!!$userStore && !disableExpand && !noControls)

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

	// Rows of an S3 list whose preview is open, and those opened once: a hidden preview stays
	// mounted, so a table keeps its rows, scroll and search. By row index, since a list can
	// repeat a key; tied to the result so another result starts closed.
	let s3ListPreviews: { result: unknown; open: number[]; mounted: number[] } = $state.raw({
		result: undefined,
		open: [],
		mounted: []
	})
	let s3ListOpen = $derived(s3ListPreviews.result === result ? s3ListPreviews.open : [])
	let s3ListMounted = $derived(s3ListPreviews.result === result ? s3ListPreviews.mounted : [])
	function toggleS3ListPreview(index: number) {
		const open = s3ListOpen.includes(index)
		s3ListPreviews = {
			result,
			open: open ? s3ListOpen.filter((i) => i !== index) : [...s3ListOpen, index],
			mounted: s3ListMounted.includes(index) ? s3ListMounted : [...s3ListMounted, index]
		}
	}
	// The two below are also tied to the result: a rerun that writes the same key is a new file,
	// and must not open with the previous run's filters or separator.
	/** Keys of the markdown files shown as their source rather than rendered. */
	let markdownSource: { result: unknown; keys: string[] } = $state.raw({
		result: undefined,
		keys: []
	})
	let markdownSourceKeys = $derived(markdownSource.result === result ? markdownSource.keys : [])
	// Not reactive: filled while rendering, each entry holding its own state.
	let s3TableViews: { result: unknown; views: Map<string, S3TableView> } = {
		result: undefined,
		views: new Map()
	}
	// Keyed by storage too: the same key in two storages is two files.
	function s3TableView(s3object: { s3: string; storage?: string }) {
		if (s3TableViews.result !== result) s3TableViews = { result, views: new Map() }
		const key = `${s3object.storage ?? ''}\u0000${s3object.s3}`
		let view = s3TableViews.views.get(key)
		if (!view) s3TableViews.views.set(key, (view = new S3TableView()))
		return view
	}
	function setMarkdownSource(key: string, source: boolean) {
		markdownSource = {
			result,
			keys: source ? [...markdownSourceKeys, key] : markdownSourceKeys.filter((k) => k !== key)
		}
	}

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
		const replaying = isReplaying()
		const offlineReplay = isOfflineReplay()
		untrack(() => {
			resultKind = inferResultKind(result)
			// A recording carries the result JSON, nothing the result points at, and a
			// replay has no session to go get it: show the recorded value instead.
			const inert =
				(replaying && REPLAY_INERT_KINDS.includes(resultKind)) ||
				(offlineReplay && OFFLINE_INERT_KINDS.includes(resultKind))
			if (inert) {
				resultKind = 'json'
				largeObject = false
			}
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

	// Per-test breakdown of a managed `// materialize` run. On success it rides the
	// result as `data_tests`; on failure the job result is the error, whose message
	// is the worker's breakdown text, parsed back into the same shape so the
	// checklist shows either way. Inert for every other DisplayResult use.
	let dataTests = $derived.by(() => {
		// `DataTestsResult` renders an item per entry, and the message-derived branch
		// below builds them from *lines of text*, so no bound on the result's structure
		// can see them. A run with this many tests is unreadable anyway, and the cap has
		// to live where the parse happens rather than be predicted from the payload.
		const MAX_RENDERED = 1000
		// A per-test `sample` arrives as its own JSON string, so nothing that measures
		// the enclosing result's structure can see inside it — parsing an 8 MB string of
		// `{}` would allocate millions of objects before any row cap applied. Bound the
		// text first, then the rows.
		const MAX_SAMPLE_CHARS = 256 * 1024
		const MAX_SAMPLE_ROWS = 1000
		const capped = <T,>(tests: T[]): T[] =>
			tests.length > MAX_RENDERED ? tests.slice(0, MAX_RENDERED) : tests
		// Both shapes carry `[{ test, violating, sample? }]`. The sample may arrive as
		// a JSON string and is optional by contract, so anything malformed degrades to
		// no sample — never to a dropped checklist.
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
						sample = sample.length > MAX_SAMPLE_CHARS ? undefined : JSON.parse(sample)
					} catch {
						sample = undefined
					}
				}
				if (!Array.isArray(sample) || !sample.every((r) => r && typeof r === 'object')) {
					sample = undefined
				} else if (sample.length > MAX_SAMPLE_ROWS) {
					sample = sample.slice(0, MAX_SAMPLE_ROWS)
				}
				return { test: x.test, violating: x.violating, sample }
			})
		}
		// Success: structured column on the summary row.
		const row = Array.isArray(result) ? (result as any)?.[0] : (result as any)
		const fromRow = normalize(row?.data_tests)
		if (fromRow) return capped(fromRow)
		// Failure: the worker attaches the same structured breakdown (plus
		// per-failed-test samples) to the error payload.
		const fromError = normalize((result as any)?.error?.data_tests)
		if (fromError) return capped(fromError)
		// Failure fallback for results predating the structured error payload:
		// parse the worker's breakdown out of the error message.
		const msg = (result as any)?.error?.message
		if (typeof msg === 'string' && msg.includes('data tests failed on')) {
			const out: Array<{ test: string; violating: number }> = []
			for (const line of msg.split('\n')) {
				if (out.length >= MAX_RENDERED) break
				const fail = line.match(/^\s*✗\s*(.+?)\s*—\s*(\d+)\s+violating/)
				const pass = line.match(/^\s*✓\s*(.+?)\s*$/)
				if (fail) out.push({ test: fail[1], violating: parseInt(fail[2], 10) })
				else if (pass) out.push({ test: pass[1], violating: 0 })
			}
			if (out.length > 0) return out
		}
		return undefined
	})

	// Per-node breakdown of a dbt invocation, rendered above the raw result. On
	// success it IS the result; on failure the worker puts the same JSON in the
	// error message after the exit-status line, and that is the case worth
	// rendering — the failing node is what the user came for. Inert (undefined)
	// for every other DisplayResult use.
	let dbtRun = $derived(parseDbtRun(result))
</script>

{#snippet s3Preview(s3object: {
	s3: string
	storage?: string
	presigned?: string
	disable_download?: boolean
})}
	{@const kind = s3PreviewKind(s3object?.s3)}
	{#if kind === 'table'}
		<S3ObjectTable {s3object} {workspaceId} {appPath} view={s3TableView(s3object)} />
	{:else if kind === 'image'}
		<div class="h-full p-3">
			<ExpandableImage
				alt="Preview of {s3object?.s3}"
				title={s3object?.s3}
				class="w-auto h-full"
				src={s3DisplayUrl(s3object)}
			/>
		</div>
	{:else if kind === 'pdf'}
		<div class="h-96 overflow-hidden" data-interactive>
			{#await import('$lib/components/display/PdfViewer.svelte')}
				<Loader2 class="animate-spin" />
			{:then Module}
				<Module.default allowFullscreen source={s3DisplayUrl(s3object)} />
			{/await}
		</div>
	{:else if kind === 'text' || kind === 'markdown'}
		<S3TextPreview
			{s3object}
			{workspaceId}
			{appPath}
			markdown={kind === 'markdown'}
			markdownSource={markdownSourceKeys.includes(s3object.s3)}
		/>
	{/if}
{/snippet}

<HighlightTheme />

{#if dataTests}
	<DataTestsResult tests={dataTests} />
{/if}

{#if dbtRun}
	<DbtRunResult run={dbtRun} />
{/if}

{#if result_stream && result == undefined}
	<div class="flex flex-col w-full gap-2">
		<div class="flex items-center gap-2 text-secondary text-xs">
			<Loader2 class="animate-spin" size={14} /> Streaming result
		</div>
		{#if isAgentStream(result_stream)}
			<!-- An agent streams one JSON event per line, so the raw stream is a wall of
			     event objects rather than the answer being written. -->
			<AgentStreamDisplay raw={result_stream} streamKey={runKey ?? jobId} />
		{:else}
			<ResultStreamDisplay {result_stream} />
		{/if}
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
					{#if !hideAsJson && resultKind !== 'json' && (typeof result === 'object' || resultKind === 's3object')}<ToggleButtonGroup
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
				{:else if !forceJson && resultKind === 'chart'}
					<ChartResult chart={result.chart} />
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
					<div class="h-full pt-2">
						<ExpandableImage
							alt="png rendered"
							class="w-auto h-full"
							src="data:image/png;base64,{contentOrRootString(result.png)}"
						/>
					</div>
				{:else if !forceJson && resultKind === 'jpeg'}
					<div class="h-full pt-2">
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
					<div class="h-full pt-2">
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
					{#if agentErrorTrace}
						<!-- A run stopped by max_iterations fails, so the error above is what it
						     returned. What it managed to do rides inside that error and is the
						     whole reason to look at such a run, so it is added under the error
						     rather than replacing it. -->
						<div class="flex flex-col gap-1 pt-4 w-full min-w-0">
							<span class="text-emphasis text-xs font-semibold">Trace</span>
							<AgentTrace entries={agentErrorTrace} {workspaceId} />
						</div>
					{/if}
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
									<!-- Dynamic: the DuckDB preview reaches ~450 modules (data table editor, low-code
									     components), and every page that shows a job result imports this file. -->
									{#await import('./assets/AssetGraph/DucklakeResultPreview.svelte') then DucklakeResultPreview}
										<DucklakeResultPreview.default
											assetUri={m.materialized}
											partition={m.partition}
											class="h-full"
										/>
									{/await}
								</div>
							{/if}
						</div>
					{/if}
				{:else if !forceJson && resultKind === 's3object'}
					{@const s3object = parseS3Object(result) as typeof result}
					<div class="h-full w-full pt-2">
						{#if typeof s3object?.s3 === 'string'}
							<!-- One card: the header, then the preview flush below a separator, as in the S3
							     file browser. -->
							<div class="rounded-md border bg-surface overflow-hidden">
								<div class="px-3 py-2">
									<S3ObjectHeader
										{s3object}
										{workspaceId}
										{appPath}
										onExplore={canExplore ? () => s3FileViewer?.open?.(s3object) : undefined}
										markdownSource={s3PreviewKind(s3object.s3) === 'markdown'
											? markdownSourceKeys.includes(s3object.s3)
											: undefined}
										onMarkdownSource={(source) => setMarkdownSource(s3object.s3, source)}
									/>
								</div>
								{#if s3PreviewKind(s3object.s3)}
									<div class="border-t">
										{#key s3object.s3}
											{@render s3Preview(s3object)}
										{/key}
									</div>
								{/if}
							</div>
						{/if}
					</div>
				{:else if !forceJson && resultKind === 's3object-list'}
					<div class="h-full w-full pt-2">
						<div class="flex flex-col gap-2">
							<div class="rounded-md border bg-surface divide-y overflow-hidden">
								{#each result as _s3object, index}
									{@const s3object = parseS3Object(_s3object) as typeof _s3object}
									{@const kind = s3PreviewKind(s3object?.s3)}
									{@const collapsible = kind !== undefined && kind !== 'pdf'}
									{@const previewOpen = s3ListOpen.includes(index)}
									<div class="flex flex-col">
										<div class="px-3 py-2">
											<S3ObjectHeader
												{s3object}
												{workspaceId}
												{appPath}
												onExplore={canExplore ? () => s3FileViewer?.open?.(s3object) : undefined}
												previewOpen={collapsible ? previewOpen : undefined}
												onTogglePreview={() => toggleS3ListPreview(index)}
												markdownSource={kind === 'markdown' && previewOpen
													? markdownSourceKeys.includes(s3object.s3)
													: undefined}
												onMarkdownSource={(source) => setMarkdownSource(s3object.s3, source)}
											/>
										</div>
										{#if kind !== undefined && (!collapsible || s3ListMounted.includes(index))}
											<div class="border-t" class:hidden={collapsible && !previewOpen}>
												{@render s3Preview(s3object)}
											</div>
										{/if}
									</div>
								{/each}
							</div>
						</div>
					</div>
				{:else if !forceJson && resultKind === 'aiagent'}
					{@const agentResult = parseAgentResult(result)}
					{#if agentResult}
						<AgentResultDisplay result={agentResult} {workspaceId} runKey={runKey ?? jobId}>
							{#snippet structuredOutput(output)}
								<DisplayResult
									noControls
									hideAsJson
									result={output}
									{markupTrust}
									{filename}
									{disableExpand}
									{jobId}
									{nodeId}
									{workspaceId}
									{appPath}
									growVertical
								/>
							{/snippet}
						</AgentResultDisplay>
					{/if}
				{:else if !forceJson && resultKind === 'markdown'}
					<div class={markdownProse.sm}>
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
						{#if tooBigFilename && resultApiPath}
							<div class="flex">
								<Button
									onclick={() => downloadTooBigFile(resultApiPath!, tooBigFilename!)}
									disabled={downloadingTooBigFile}
									startIcon={{ icon: Download }}
									variant="subtle"
									unifiedSize="sm"
								>
									Download {tooBigFilename}
								</Button>
							</div>
						{/if}
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
