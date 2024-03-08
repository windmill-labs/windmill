<script lang="ts">
	import { Highlight } from 'svelte-highlight'
	import { json } from 'svelte-highlight/languages'
	import TableCustom from './TableCustom.svelte'
	import { copyToClipboard, roughSizeOfObject, truncate } from '$lib/utils'
	import { Button, Drawer, DrawerContent } from './common'
	import {
		ClipboardCopy,
		Download,
		Expand,
		PanelRightOpen,
		Table2,
		Braces,
		Highlighter
	} from 'lucide-svelte'
	import Portal from 'svelte-portal'
	import ObjectViewer from './propertyPicker/ObjectViewer.svelte'
	import S3FilePicker from './S3FilePicker.svelte'
	import Alert from './common/alert/Alert.svelte'
	import AutoDataTable from './table/AutoDataTable.svelte'
	import Markdown from 'svelte-exmarkdown'
	import Toggle from './Toggle.svelte'
	import FileDownload from './common/fileDownload/FileDownload.svelte'

	import ParqetTableRenderer from './ParqetTableRenderer.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'

	export let result: any
	export let requireHtmlApproval = false
	export let filename: string | undefined = undefined
	export let disableExpand = false
	export let jobId: string | undefined = undefined
	export let workspaceId: string | undefined = undefined
	export let hideAsJson: boolean = false

	let resultKind:
		| 'json'
		| 'table-col'
		| 'table-row'
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
		| undefined

	$: resultKind = inferResultKind(result)

	export let forceJson = false
	let enableHtml = false
	let s3FileDisplayRawMode = false

	function isRectangularArray(obj: any) {
		if (!Array.isArray(obj) || obj.length == 0) {
			return false
		}
		if (
			!Object.values(obj)
				.map(Array.isArray)
				.reduce((a, b) => a && b)
		) {
			return false
		}
		let innerSize = obj[0].length

		return Object.values(obj)
			.map((x: any) => x.length == innerSize)
			.reduce((a, b) => a && b)
	}

	function asListOfList(obj: any): ArrayLike<ArrayLike<any>> {
		return obj as ArrayLike<ArrayLike<any>>
	}

	function isObjectOfArray(result: any[], keys: string[]): boolean {
		return keys.map((k) => Array.isArray(result[k])).reduce((a, b) => a && b)
	}

	let largeObject: boolean | undefined = undefined

	let is_render_all = false
	function inferResultKind(result: any) {
		if (result == 'WINDMILL_TOO_BIG') {
			largeObject = true
			return 'json'
		}

		if (result !== undefined) {
			try {
				let keys = result && typeof result == 'object' ? Object.keys(result) : []
				is_render_all =
					keys.length == 1 && keys.includes('render_all') && Array.isArray(result['render_all'])

				// Check if the result is an image
				if (['png', 'svg', 'jpeg'].includes(keys[0]) && keys.length == 1) {
					// Check if the image is too large (10mb)
					largeObject = roughSizeOfObject(result) > 10000000

					return keys[0] as 'png' | 'svg' | 'jpeg'
				}

				let length = roughSizeOfObject(result)
				// Otherwise, check if the result is too large (10kb) for json
				largeObject = length > 10000

				if (largeObject) {
					return 'json'
				}

				if (keys.length != 0) {
					if (Array.isArray(result) && result.every((elt) => inferResultKind(elt) === 's3object')) {
						return 's3object-list'
					} else if (isRectangularArray(result)) {
						return 'table-col'
					} else if (keys.length == 1 && keys[0] == 'table-row') {
						return 'table-row'
					} else if (
						(keys.length == 1 && keys[0] == 'table-col') ||
						isObjectOfArray(result, keys)
					) {
						return 'table-col'
					} else if (keys.length == 1 && keys[0] == 'html') {
						return 'html'
					} else if (keys.length == 1 && keys[0] == 'file') {
						return 'file'
					} else if (
						keys.includes('windmill_content_type') &&
						result['windmill_content_type'].startsWith('text/')
					) {
						return 'plain'
					} else if (keys.length == 1 && keys[0] == 'error') {
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
					} else if (keys.length === 1 && keys.includes('s3')) {
						return 's3object'
					} else if (keys.length === 1 && (keys.includes('md') || keys.includes('markdown'))) {
						return 'markdown'
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

	function toJsonStr(result: any) {
		return JSON.stringify(result ?? null, null, 4) ?? 'null'
	}

	function contentOrRootString(obj: string | { filename: string; content: string }) {
		if (typeof obj === 'string') {
			return obj
		} else {
			return obj.content
		}
	}

	function isArrayWithObjects(json) {
		return (
			Array.isArray(json) &&
			json.length > 0 &&
			json.every((item) => item && typeof item === 'object' && Object.keys(item).length > 0)
		)
	}

	$: isTableDisplay = isArrayWithObjects(result)
	let richRender: boolean = true

	type InputObject = { [key: string]: number[] }

	function transform(input: InputObject): any[] {
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

	let globalForceJson: boolean = false
</script>

{#if is_render_all}
	<div class="flex flex-col w-full">
		<div class="mb-2 text-tertiary text-sm">
			as JSON&nbsp;<input class="windmillapp" type="checkbox" bind:checked={globalForceJson} />
		</div>
		{#each result['render_all'] as res}
			<svelte:self
				result={res}
				{requireHtmlApproval}
				{filename}
				{disableExpand}
				{jobId}
				{workspaceId}
				forceJson={globalForceJson}
				hideAsJson={true}
			/>
		{/each}</div
	>
{:else}
	<div class="inline-highlight relative grow min-h-[200px]">
		{#if result != undefined && length != undefined && largeObject != undefined}
			<div class="flex justify-between items-center w-full pb-1">
				<div class="text-tertiary text-sm flex items-center">
					{#if (resultKind && !['json', 's3object', 's3object-list', 'table-col', 'table-row'].includes(resultKind) && !hideAsJson) || isTableDisplay}
						<ToggleButtonGroup
							class="h-6"
							selected={isTableDisplay
								? richRender
									? 'table'
									: 'json'
								: forceJson
								? 'json'
								: 'pretty'}
							on:selected={(ev) => {
								if (isTableDisplay) {
									richRender = ev.detail === 'table'
								}
								forceJson = ev.detail === 'json'
							}}
						>
							{#if resultKind && !['json', 's3object', 's3object-list', 'table-col', 'table-row'].includes(resultKind) && !hideAsJson}
								<ToggleButton class="px-1.5" value="pretty" label="Pretty" icon={Highlighter} />
							{/if}
							{#if isTableDisplay}
								<ToggleButton class="px-1.5" value="table" label="Table" icon={Table2} />
							{/if}
							<ToggleButton class="px-1.5" value="json" label="JSON" icon={Braces} />
						</ToggleButtonGroup>
					{/if}
				</div>
				<div class="text-tertiary text-xs flex gap-2 z-10 items-center">
					<slot name="copilot-fix" />
					{#if !disableExpand}
						<button on:click={() => copyToClipboard(toJsonStr(result))}
							><ClipboardCopy size={16} /></button
						>
						<button on:click={jsonViewer.openDrawer}><Expand size={16} /></button>
					{/if}
				</div>
			</div>

			{#if !forceJson && resultKind == 'table-col'}
				{@const data = 'table-col' in result ? result['table-col'] : result}
				<AutoDataTable objects={transform(data)} />
			{:else if !forceJson && resultKind == 'table-row'}
				{@const data = 'table-row' in result ? result['table-row'] : result}
				<div class="grid grid-flow-col-dense border border-gray-200">
					<TableCustom>
						<tbody slot="body">
							{#each Array.isArray(asListOfList(data)) ? asListOfList(data) : [] as row}
								<tr>
									{#each row as v}
										<td class="!text-xs">{truncate(JSON.stringify(v), 200) ?? ''}</td>
									{/each}
								</tr>
							{/each}
						</tbody>
					</TableCustom>
				</div>
			{:else if !forceJson && resultKind == 'html'}
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
			{:else if !forceJson && resultKind == 'png'}
				<div class="h-full">
					<img
						alt="png rendered"
						class="w-auto h-full"
						src="data:image/png;base64,{contentOrRootString(result.png)}"
					/>
				</div>
			{:else if !forceJson && resultKind == 'jpeg'}
				<div class="h-full">
					<img
						alt="jpeg rendered"
						class="w-auto h-full"
						src="data:image/jpeg;base64,{contentOrRootString(result.jpeg)}"
					/>
				</div>
			{:else if !forceJson && resultKind == 'svg'}
				<div
					><a download="windmill.svg" href="data:text/plain;base64,{btoa(result.svg)}">Download</a>
				</div>
				<div class="h-full overflow-auto">{@html result.svg} </div>
			{:else if !forceJson && resultKind == 'gif'}
				<div class="h-full">
					<img
						alt="gif rendered"
						class="w-auto h-full"
						src="data:image/gif;base64,{contentOrRootString(result.gif)}"
					/>
				</div>
			{:else if !forceJson && resultKind == 'plain'}
				<div class="h-full text-2xs">
					<pre>{result?.['result']}</pre>
				</div>
			{:else if !forceJson && resultKind == 'file'}
				<div>
					<a
						download={result.filename ?? result.file?.filename ?? 'windmill.file'}
						href="data:application/octet-stream;base64,{contentOrRootString(result.file)}"
						>Download</a
					>
				</div>
			{:else if !forceJson && resultKind == 'error' && result?.error}
				<div class="flex flex-col items-start">
					<span class="text-red-500 font-semibold text-sm whitespace-pre-wrap"
						>{#if result.error.name || result.error.message}{result.error.name}: {result.error
								.message}{:else}{JSON.stringify(result.error, null, 4)}{/if}</span
					>
					<pre class="text-sm whitespace-pre-wrap text-primary">{result.error.stack ?? ''}</pre>
					<slot />
				</div>
			{:else if !forceJson && resultKind == 'approval'}<div class="flex flex-col gap-3 mt-2 mx-4">
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
			{:else if !forceJson && resultKind == 's3object'}
				<div
					class="h-full w-full {typeof result?.s3 == 'string' && result?.s3?.endsWith('.parquet')
						? 'h-min-[600px]'
						: ''}"
				>
					<div class="flex flex-col gap-2">
						<Toggle
							class="flex"
							bind:checked={s3FileDisplayRawMode}
							size="xs"
							options={{ right: 'Raw S3 object input' }}
						/>
						{#if s3FileDisplayRawMode}
							<Highlight class="" language={json} code={toJsonStr(result).replace(/\\n/g, '\n')} />
							<button
								class="text-secondary underline text-2xs whitespace-nowrap"
								on:click={() => {
									s3FileViewer?.open?.(result)
								}}
								><span class="flex items-center gap-1"
									><PanelRightOpen size={12} />open preview</span
								>
							</button>
						{:else}
							<FileDownload s3object={result} />
							<button
								class="text-secondary underline text-2xs whitespace-nowrap"
								on:click={() => {
									s3FileViewer?.open?.(result)
								}}
								><span class="flex items-center gap-1"
									><PanelRightOpen size={12} />open preview</span
								>
							</button>
						{/if}
					</div>
					{#if typeof result?.s3 == 'string' && result?.s3?.endsWith('.parquet')}
						<ParqetTableRenderer s3resource={result?.s3} />
					{/if}
				</div>
			{:else if !forceJson && resultKind == 's3object-list'}
				<div class="h-full w-full">
					<div class="flex flex-col gap-2">
						<Toggle
							class="flex"
							bind:checked={s3FileDisplayRawMode}
							size="xs"
							options={{ right: 'Raw S3 object input' }}
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
							{:else}
								<FileDownload {s3object} />
							{/if}
						{/each}
					</div>
				</div>
			{:else if !forceJson && resultKind == 'markdown'}
				<div class="prose dark:prose-invert">
					<Markdown md={result?.md ?? result?.markdown} />
				</div>
			{:else if !forceJson && isTableDisplay && richRender}
				<AutoDataTable objects={result} />
			{:else if largeObject}
				{#if result && typeof result == 'object' && 'filename' in result && 'file' in result}
					<div
						><a
							download={result.filename ?? result.file?.filename ?? 'windmill.file'}
							href="data:application/octet-stream;base64,{contentOrRootString(result.file)}"
							>Download</a
						>
					</div>
				{:else}
					<div class="text-sm text-tertiary"
						><a
							download="{filename ?? 'result'}.json"
							href={workspaceId && jobId
								? `/api/w/${workspaceId}/jobs_u/completed/get_result/${jobId}`
								: `data:text/json;charset=utf-8,${encodeURIComponent(toJsonStr(result))}`}
						>
							Download {filename ? '' : 'as JSON'}
						</a>
					</div>

					<div class="my-4">
						<Alert size="xs" title="Large file detected" type="warning">
							We recommend using persistent storage for large data files.
							<a
								href="https://www.windmill.dev/docs/core_concepts/persistent_storage#large-data-files-s3-r2-minio-azure-blob"
								target="_blank"
								rel="noreferrer"
								class="hover:underline"
							>
								See docs for setting up an object storage service integration using s3 or any other
								s3 compatible services
							</a>
						</Alert>
					</div>
				{/if}
				{#if result && result != 'WINDMILL_TOO_BIG'}
					<ObjectViewer json={result} />
				{/if}
			{:else if typeof result == 'string' && result.length > 0}
				<pre class="text-sm">{result}</pre>
				<div class="flex">
					<Button on:click={() => copyToClipboard(result)} color="light" size="xs">
						<div class="flex gap-2 items-center">Copy <ClipboardCopy size={12} /> </div>
					</Button>
				</div>
			{:else}
				<Highlight
					class={forceJson ? '' : 'h-full w-full'}
					language={json}
					code={toJsonStr(result).replace(/\\n/g, '\n')}
				/>
			{/if}
		{:else}
			<div class="text-tertiary text-sm">No result: {toJsonStr(result)}</div>
		{/if}
	</div>

	{#if !disableExpand}
		<Portal>
			<Drawer bind:this={jsonViewer} size="900px">
				<DrawerContent title="Expanded Result" on:close={jsonViewer.closeDrawer}>
					<svelte:fragment slot="actions">
						<Button
							download="{filename ?? 'result'}.json"
							href={workspaceId && jobId
								? `/api/w/${workspaceId}/jobs_u/completed/get_result/${jobId}`
								: `data:text/json;charset=utf-8,${encodeURIComponent(toJsonStr(result))}`}
							startIcon={{ icon: Download }}
							color="light"
							size="xs"
						>
							Download
						</Button>
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
					{#if largeObject}
						<div class="text-sm mb-2 text-tertiary">
							<a
								class="text-sm text-secondary mr-2 inline-flex gap-2 items-center py-2 px-2 hover:bg-gray-100 rounded-lg"
								download="{filename ?? 'result'}.json"
								href={workspaceId && jobId
									? `/api/w/${workspaceId}/jobs_u/completed/get_result/${jobId}`
									: `data:text/json;charset=utf-8,${encodeURIComponent(result)}`}
								>Download <Download size={14} /></a
							> JSON is too large to be displayed in full.
						</div>
					{:else if typeof result == 'string' && result.length > 0}
						<pre class="text-sm">{result}</pre>
						<div class="flex">
							<Button on:click={() => copyToClipboard(result)} color="light" size="xs">
								<div class="flex gap-2 items-center">Copy <ClipboardCopy size={12} /> </div>
							</Button>
						</div>
					{:else}
						<Highlight language={json} code={toJsonStr(result).replace(/\\n/g, '\n')} />
					{/if}
				</DrawerContent>
			</Drawer>
		</Portal>

		<Portal>
			<S3FilePicker bind:this={s3FileViewer} readOnlyMode={true} />
		</Portal>
	{/if}
{/if}
