<script lang="ts">
	import { Highlight } from 'svelte-highlight'
	import { json } from 'svelte-highlight/languages'
	import { copyToClipboard, roughSizeOfObject } from '$lib/utils'
	import { base } from '$lib/base'
	import { Button, Drawer, DrawerContent } from './common'
	import {
		ClipboardCopy,
		Download,
		PanelRightOpen,
		Table2,
		Braces,
		Highlighter,
		ArrowDownFromLine
	} from 'lucide-svelte'
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
	import PdfViewer from './display/PdfViewer.svelte'
	import type { DisplayResultUi } from './custom_ui'
	import { getContext, hasContext, createEventDispatcher, onDestroy } from 'svelte'
	import { toJsonStr } from '$lib/utils'
	import { createDispatcherIfMounted } from '$lib/createDispatcherIfMounted'

	export let result: any
	export let requireHtmlApproval = false
	export let filename: string | undefined = undefined
	export let disableExpand = false
	export let jobId: string | undefined = undefined
	export let workspaceId: string | undefined = undefined
	export let hideAsJson: boolean = false
	export let noControls: boolean = false
	export let drawerOpen = false
	export let nodeId: string | undefined = undefined
	export let language: string | undefined = undefined
	export let appPath: string | undefined = undefined
	export let customUi: DisplayResultUi | undefined = undefined
	export let isTest: boolean = true
	export let externalToolbarAvailable: boolean = false

	const IMG_MAX_SIZE = 10000000
	const TABLE_MAX_SIZE = 5000000
	const DISPLAY_MAX_SIZE = 100000

	const dispatch = createEventDispatcher()
	const dispatchIfMounted = createDispatcherIfMounted(dispatch)

	let resultKind:
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
		| 'plain'
		| 'markdown'
		| 'map'
		| 'nondisplayable'
		| 'pdf'
		| undefined

	let hasBigInt = false
	$: resultKind = inferResultKind(result)

	export let forceJson = false
	let enableHtml = false
	let s3FileDisplayRawMode = false

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
		return (
			Array.isArray(json) &&
			json.length > 0 &&
			(json.every(
				(item) =>
					item && typeof item === 'object' && Object.keys(item).length > 0 && !Array.isArray(item)
			) ||
				(Array.isArray(json[0]) &&
					json[0].every((item) => typeof item === 'string') &&
					json
						.slice(1)
						.every(
							(item) =>
								item &&
								typeof item === 'object' &&
								Object.keys(item).length > 0 &&
								!Array.isArray(item)
						)))
		)
	}

	let largeObject: boolean | undefined = undefined

	function checkIfS3(result: any, keys: string[]) {
		return keys.includes('s3') && typeof result.s3 === 'string'
	}

	let is_render_all = false
	let download_as_csv = false
	function inferResultKind(result: any) {
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
				is_render_all =
					keys.length == 1 && keys.includes('render_all') && Array.isArray(result['render_all'])

				// Check if the result is an image
				if (['png', 'svg', 'jpeg', 'html', 'gif'].includes(keys[0]) && keys.length == 1) {
					// Check if the image is too large (10mb)
					largeObject = roughSizeOfObject(result) > IMG_MAX_SIZE

					return keys[0] as 'png' | 'svg' | 'jpeg' | 'html' | 'gif'
				}

				let size = roughSizeOfObject(result)
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

				if (largeObject) {
					return 'json'
				}

				if (keys.length != 0) {
					if (keys.length == 1 && keys[0] === 'html') {
						return 'html'
					} else if (keys.length == 1 && keys[0] === 'map') {
						return 'map'
					} else if (keys.length == 1 && keys[0] === 'pdf') {
						return 'pdf'
					} else if (keys.length == 1 && keys[0] === 'file') {
						return 'file'
					} else if (
						keys.includes('windmill_content_type') &&
						result['windmill_content_type'].startsWith('text/')
					) {
						return 'plain'
					} else if (keys.length == 1 && keys[0] === 'error') {
						return 'error'
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
						keys.includes('resume') &&
						keys.includes('cancel') &&
						keys.includes('approvalPage')
					) {
						return 'approval'
					} else if (checkIfS3(result, keys)) {
						return 's3object'
					} else if (keys.length === 1 && (keys.includes('md') || keys.includes('markdown'))) {
						return 'markdown'
					} else if (isTableCol(result, keys)) {
						return 'table-col'
					}
				}
			} catch (err) {}
		} else {
			largeObject = false
			is_render_all = false
		}
		return 'json'
	}

	let jsonViewer: Drawer
	let s3FileViewer: S3FilePicker

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

	function handleArrayOfObjectsHeaders(json: any) {
		// handle possible a first row of headers
		if (
			Array.isArray(json) &&
			json.length > 0 &&
			Array.isArray(json[0]) &&
			json[0].length > 0 &&
			json[0].every((item) => typeof item === 'string') &&
			json
				.slice(1)
				.every(
					(item) =>
						item && typeof item === 'object' && Object.keys(item).length > 0 && !Array.isArray(item)
				)
		) {
			const headers = json[0]
			const rows = json.slice(1)

			const result = rows.map((row) => {
				const obj: { [key: string]: string } = {}

				for (const header of headers) {
					obj[header] = row[header]
				}

				return obj
			})

			return result
		}

		return json
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
		jsonViewer.openDrawer()
	}

	let globalForceJson: boolean = false

	let seeS3PreviewFileFromList = ''

	const disableTooltips = hasContext('disableTooltips')
		? getContext('disableTooltips') === true
		: false

	let resultHeaderHeight = 0

	let toolbarLocation: 'self' | 'external' | undefined = undefined
	function chooseToolbarLocation(shouldShowToolbar: boolean, resultHeaderHeight: number) {
		if (!shouldShowToolbar) {
			toolbarLocation = undefined
		} else if (externalToolbarAvailable && resultHeaderHeight < 16) {
			toolbarLocation = 'external'
		} else {
			toolbarLocation = 'self'
		}
		dispatchIfMounted('toolbar-location-changed', toolbarLocation)
	}

	export function getToolbarLocation() {
		return toolbarLocation
	}

	$: chooseToolbarLocation(
		!is_render_all &&
			resultKind != 'nondisplayable' &&
			result != undefined &&
			length != undefined &&
			largeObject != undefined &&
			!disableExpand &&
			!noControls,
		resultHeaderHeight
	)

	onDestroy(() => {
		dispatch('toolbar-location-changed', undefined)
	})
</script>

<HighlightTheme />
{#if is_render_all}
	<div class="flex flex-col w-full gap-2">
		{#if !noControls}
			<div class="text-tertiary text-sm">
				<ToggleButtonGroup
					class="h-6"
					selected={globalForceJson ? 'json' : 'pretty'}
					on:selected={(ev) => {
						globalForceJson = ev.detail === 'json'
					}}
					let:item
				>
					<ToggleButton class="px-1.5" value="pretty" label="Pretty" icon={Highlighter} {item} />

					<ToggleButton class="px-1.5" value="json" label="JSON" icon={Braces} {item} />
				</ToggleButtonGroup>
			</div>
		{/if}
		<div class="flex flex-col w-full gap-10">
			{#each result['render_all'] as res}
				<svelte:self
					{noControls}
					result={res}
					{requireHtmlApproval}
					{filename}
					{disableExpand}
					{jobId}
					{nodeId}
					{workspaceId}
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
		class="inline-highlight relative grow {['plain', 'markdown'].includes(resultKind ?? '')
			? ''
			: 'min-h-[160px]'}"
	>
		{#if result != undefined && length != undefined && largeObject != undefined}
			<div class="flex justify-between items-center w-full">
				<div
					class="text-tertiary text-sm flex flex-row gap-2 items-center"
					bind:clientHeight={resultHeaderHeight}
				>
					{#if !hideAsJson && !['json', 's3object'].includes(resultKind ?? '') && typeof result === 'object'}<ToggleButtonGroup
							class="h-6"
							selected={forceJson ? 'json' : resultKind?.startsWith('table-') ? 'table' : 'pretty'}
							let:item
							on:selected={(ev) => {
								forceJson = ev.detail === 'json'
							}}
						>
							{#if ['table-col', 'table-row', 'table-row-object'].includes(resultKind ?? '')}
								<ToggleButton class="px-1.5" value="table" label="Table" icon={Table2} {item} />
							{:else}
								<ToggleButton
									class="px-1.5"
									value="pretty"
									label="Pretty"
									icon={Highlighter}
									{item}
								/>
							{/if}
							<ToggleButton class="px-1.5" value="json" label="JSON" icon={Braces} {item} />
						</ToggleButtonGroup>
					{/if}
				</div>

				<div class="text-secondary text-xs flex gap-2.5 z-10 items-center">
					{#if customUi?.disableAiFix !== true}
						<slot name="copilot_fix" />
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
			<div class="grow">
				{#if !forceJson && resultKind === 'table-col'}
					{@const data = 'table-col' in result ? result['table-col'] : result}
					<AutoDataTable objects={objectOfArraysToObjects(data)} />
				{:else if !forceJson && resultKind === 'table-row'}
					{@const data = 'table-row' in result ? result['table-row'] : result}
					<AutoDataTable objects={arrayOfRowsToObjects(data)} />
				{:else if !forceJson && resultKind === 'table-row-object'}
					{@const data = 'table-row-object' in result ? result['table-row-object'] : result}
					<AutoDataTable objects={handleArrayOfObjectsHeaders(data)} />
				{:else if !forceJson && resultKind === 'html'}
					<div class="h-full">
						{#if !requireHtmlApproval || enableHtml}
							{@html result.html}
						{:else}
							<div class="font-main text-sm">
								<div class="flex flex-col">
									<div class="bg-red-400 py-1 rounded-t text-white font-bold text-center">
										Warning
									</div>
									<p
										class="text-tertiary mb-2 text-left border-2 !border-t-0 rounded-b border-red-400 overflow-auto p-1"
										>Rendering HTML can expose you to <a
											href="https://owasp.org/www-community/attacks/xss/"
											target="_blank"
											rel="noreferrer"
											class="hover:underline">XSS attacks</a
										>. Only enable it if you trust the author of the script.
									</p>
								</div>
								<div class="center-center">
									<Button size="sm" color="dark" on:click={() => (enableHtml = true)}>
										Enable HTML rendering
									</Button>
								</div>
							</div>
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
						<img
							alt="png rendered"
							class="w-auto h-full"
							src="data:image/png;base64,{contentOrRootString(result.png)}"
						/>
					</div>
				{:else if !forceJson && resultKind === 'jpeg'}
					<div class="h-full">
						<img
							alt="jpeg rendered"
							class="w-auto h-full"
							src="data:image/jpeg;base64,{contentOrRootString(result.jpeg)}"
						/>
					</div>
				{:else if !forceJson && resultKind === 'svg'}
					<div
						><a download="windmill.svg" href="data:text/plain;base64,{btoa(result.svg)}">Download</a
						>
					</div>
					<div class="h-full overflow-auto">{@html result.svg} </div>
				{:else if !forceJson && resultKind === 'gif'}
					<div class="h-full">
						<img
							alt="gif rendered"
							class="w-auto h-full"
							src="data:image/gif;base64,{contentOrRootString(result.gif)}"
						/>
					</div>
				{:else if !forceJson && resultKind === 'pdf'}
					<div class="h-96 mt-2 border">
						<PdfViewer
							allowFullscreen
							source="data:application/pdf;base64,{contentOrRootString(result.pdf)}"
						/>
					</div>
				{:else if !forceJson && resultKind === 'plain'}<div class="h-full text-2xs"
						><pre class="whitespace-pre-wrap"
							>{typeof result === 'string' ? result : result?.['result']}</pre
						>{#if !noControls}
							<div class="flex">
								<Button
									on:click={() =>
										copyToClipboard(typeof result === 'string' ? result : result?.['result'])}
									color="light"
									size="xs"
								>
									<div class="flex gap-2 items-center">Copy <ClipboardCopy size={12} /> </div>
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
						<slot />
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
							color="green"
							variant="border"
							on:click={() =>
								fetch(result['resume'], {
									method: 'POST',
									body: JSON.stringify({}),
									headers: { 'Content-Type': 'application/json' }
								})}
						>
							Resume</Button
						>
						<Button color="red" variant="border" on:click={() => fetch(result['cancel'])}
							>Cancel</Button
						>
						<div class="center-center"
							><a rel="noreferrer" target="_blank" href={result['approvalPage']}>Approval Page</a
							></div
						>
					</div>
				{:else if !forceJson && resultKind === 's3object'}
					<div
						class="h-full w-full {typeof result?.s3 === 'string' && result?.s3?.endsWith('.parquet')
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
									code={toJsonStr(result).replace(/\\n/g, '\n')}
								/>
								<button
									class="text-secondary underline text-2xs whitespace-nowrap"
									on:click={() => {
										s3FileViewer?.open?.(result)
									}}
									><span class="flex items-center gap-1"
										><PanelRightOpen size={12} />object store explorer<Tooltip
											>Require admin privilege or "S3 resource details and content can be accessed
											by all users of this workspace" of S3 Storage to be set in the workspace
											settings</Tooltip
										></span
									>
								</button>
							{:else if !result?.disable_download}
								<FileDownload {workspaceId} s3object={result} {appPath} />
								<button
									class="text-secondary underline text-2xs whitespace-nowrap"
									on:click={() => {
										s3FileViewer?.open?.(result)
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
						</div>
						{#if typeof result?.s3 === 'string'}
							{#if !appPath && (result?.s3?.endsWith('.parquet') || result?.s3?.endsWith('.csv'))}
								{#key result.s3}
									<ParqetTableRenderer
										disable_download={result?.disable_download}
										{workspaceId}
										s3resource={result?.s3}
										storage={result?.storage}
									/>
								{/key}
							{:else if result?.s3?.endsWith('.png') || result?.s3?.endsWith('.jpeg') || result?.s3?.endsWith('.jpg') || result?.s3?.endsWith('.webp')}
								<div class="h-full mt-2">
									<img
										alt="preview rendered"
										class="w-auto h-full"
										src="{`/api/w/${workspaceId}/${
											appPath
												? 'apps_u/download_s3_file/' + appPath
												: 'job_helpers/load_image_preview'
										}?${appPath ? 's3' : 'file_key'}=${encodeURIComponent(result.s3)}` +
											(result.storage ? `&storage=${result.storage}` : '')}{appPath &&
										result.presigned
											? `&${result.presigned}`
											: ''}"
									/>
								</div>
							{:else if result?.s3?.endsWith('.pdf')}
								<div class="h-96 mt-2 border">
									<PdfViewer
										allowFullscreen
										source="{`/api/w/${workspaceId}/${
											appPath
												? 'apps_u/download_s3_file/' + appPath
												: 'job_helpers/load_image_preview'
										}?${appPath ? 's3' : 'file_key'}=${encodeURIComponent(result.s3)}` +
											(result.storage ? `&storage=${result.storage}` : '')}{appPath &&
										result.presigned
											? `&${result.presigned}`
											: ''}"
									/>
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
							{#each result as s3object}
								{#if s3FileDisplayRawMode}
									<Highlight
										class=""
										language={json}
										code={toJsonStr(s3object).replace(/\\n/g, '\n')}
									/>
									<button
										class="text-secondary text-2xs whitespace-nowrap"
										on:click={() => {
											s3FileViewer?.open?.(s3object)
										}}
										><span class="flex items-center gap-1"
											><PanelRightOpen size={12} />open preview</span
										>
									</button>
								{:else if !s3object?.disable_download}
									<FileDownload {s3object} />
								{:else}
									<div class="flex text-secondary pt-2">{s3object?.s3} (download disabled)</div>
								{/if}
								{#if s3object?.s3?.endsWith('.parquet') || s3object?.s3?.endsWith('.csv')}
									{#if seeS3PreviewFileFromList == s3object?.s3}
										<ParqetTableRenderer
											disable_download={s3object?.disable_download}
											{workspaceId}
											s3resource={s3object?.s3}
											storage={s3object?.storage}
										/>{:else}
										<button
											class="text-secondary whitespace-nowrap flex gap-2 items-center"
											on:click={() => {
												seeS3PreviewFileFromList = s3object?.s3
											}}
											>open table preview <ArrowDownFromLine />
										</button>
									{/if}
								{:else if s3object?.s3?.endsWith('.png') || s3object?.s3?.endsWith('.jpeg') || s3object?.s3?.endsWith('.jpg') || s3object?.s3?.endsWith('.webp')}
									{#if seeS3PreviewFileFromList == s3object?.s3}
										<div class="h-full mt-2">
											<img
												alt="preview rendered"
												class="w-auto h-full"
												src={`/api/w/${workspaceId}/job_helpers/load_image_preview?file_key=${encodeURIComponent(
													s3object.s3
												)}` + (s3object.storage ? `&storage=${s3object.storage}` : '')}
											/>
										</div>
									{:else}
										<button
											class="text-secondary whitespace-nowrap flex gap-2 items-center"
											on:click={() => {
												seeS3PreviewFileFromList = s3object?.s3
											}}
											>open image preview <ArrowDownFromLine />
										</button>
									{/if}
								{:else if s3object?.s3?.endsWith('.pdf')}
									<div class="h-96 mt-2 border" data-interactive>
										<PdfViewer
											allowFullscreen
											source={`/api/w/${workspaceId}/job_helpers/load_image_preview?file_key=${encodeURIComponent(
												s3object.s3
											)}` + (s3object.storage ? `&storage=${s3object.storage}` : '')}
										/>
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
							<div class="text-sm text-tertiary"
								><a
									download="{filename ?? 'result'}.json"
									href={workspaceId && jobId
										? nodeId
											? `${base}/api/w/${workspaceId}/jobs/result_by_id/${jobId}/${nodeId}`
											: `${base}/api/w/${workspaceId}/jobs_u/completed/get_result/${jobId}`
										: `data:text/json;charset=utf-8,${encodeURIComponent(toJsonStr(result))}`}
								>
									Download {filename ? '' : 'as JSON'}
								</a>
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
							<Button on:click={() => copyToClipboard(result)} color="light" size="xs">
								<div class="flex gap-2 items-center">Copy <ClipboardCopy size={12} /> </div>
							</Button>
						</div>
					{/if}
				{:else}
					<Highlight
						class={forceJson ? 'pt-1' : 'h-full w-full'}
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
						<Button on:click={() => copyToClipboard(result)} color="light" size="xs">
							<div class="flex gap-2 items-center">Copy <ClipboardCopy size={12} /> </div>
						</Button>
					</div>
				{/if}
			</div>
		{:else}
			<div class="text-tertiary text-sm">No result: {toJsonStr(result)}</div>
		{/if}
	</div>

	{#if !disableExpand && !noControls}
		<Drawer bind:this={jsonViewer} bind:open={drawerOpen} size="900px">
			<DrawerContent title="Expanded Result" on:close={jsonViewer.closeDrawer}>
				<svelte:fragment slot="actions">
					{#if customUi?.disableDownload !== true}
						<Button
							download="{filename ?? 'result'}.json"
							href={workspaceId && jobId
								? nodeId
									? `${base}/api/w/${workspaceId}/jobs/result_by_id/${jobId}/${nodeId}`
									: `${base}/api/w/${workspaceId}/jobs_u/completed/get_result/${jobId}`
								: `data:text/json;charset=utf-8,${encodeURIComponent(toJsonStr(result))}`}
							startIcon={{ icon: Download }}
							color="light"
							size="xs"
						>
							Download
						</Button>
					{/if}
					<Button
						on:click={() => copyToClipboard(toJsonStr(result))}
						color="light"
						size="xs"
						startIcon={{
							icon: ClipboardCopy
						}}
					>
						Copy to clipboard
					</Button>
				</svelte:fragment>
				<svelte:self
					{noControls}
					{result}
					{requireHtmlApproval}
					{filename}
					{jobId}
					{nodeId}
					{workspaceId}
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
