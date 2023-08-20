<script lang="ts">
	import { Highlight } from 'svelte-highlight'
	import { json } from 'svelte-highlight/languages'
	import TableCustom from './TableCustom.svelte'
	import { copyToClipboard, truncate } from '$lib/utils'
	import { Button, Drawer, DrawerContent } from './common'
	import { ClipboardCopy, Download, Expand } from 'lucide-svelte'
	import Portal from 'svelte-portal'
	import ObjectViewer from './propertyPicker/ObjectViewer.svelte'

	export let result: any
	export let requireHtmlApproval = false
	export let filename: string | undefined = undefined
	export let disableExpand = false
	export let jobId: string | undefined = undefined
	export let workspaceId: string | undefined = undefined

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
		| undefined

	$: resultKind = inferResultKind(result)

	let forceJson = false
	let enableHtml = false

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

	function inferResultKind(result: any) {
		if (result) {
			try {
				let keys = Object.keys(result)
				if (isRectangularArray(result)) {
					return 'table-row'
				} else if (isObjectOfArray(result, keys)) {
					return 'table-col'
				} else if (keys.length == 1 && keys[0] == 'html') {
					return 'html'
				} else if (keys.length == 1 && keys[0] == 'png') {
					return 'png'
				} else if (keys.length == 1 && keys[0] == 'svg') {
					return 'svg'
				} else if (keys.length == 1 && keys[0] == 'jpeg') {
					return 'jpeg'
				} else if (keys.length == 1 && keys[0] == 'file') {
					return 'file'
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
						console.log('autodownload', result.file, result.filename)
					}
					return 'file'
				} else if (
					keys.length == 3 &&
					keys.includes('resume') &&
					keys.includes('cancel') &&
					keys.includes('approvalPage')
				) {
					return 'approval'
				}
			} catch (err) {}
		}
		return 'json'
	}

	let jsonViewer: Drawer
	$: jsonStr = JSON.stringify(result, null, 4)

	function contentOrRootString(obj: string | { filename: string; content: string }) {
		if (typeof obj === 'string') {
			return obj
		} else {
			return obj.content
		}
	}
</script>

<div class="inline-highlight">
	{#if result != undefined}
		{#if resultKind && resultKind != 'json'}
			<div class="flex flex-row w-full justify-between items-center">
				<div class="mb-2 text-tertiary text-sm">
					as JSON&nbsp;<input class="windmillapp" type="checkbox" bind:checked={forceJson} /></div
				>
				<slot name="copilot-fix" />
			</div>
		{/if}
		{#if typeof result == 'object' && Object.keys(result).length > 0}<div
				class="mb-2 w-full text-sm relative"
				>The result keys are: <b>{truncate(Object.keys(result).join(', '), 50)}</b>
				{#if !disableExpand}
					<div class="text-tertiary text-xs absolute top-5.5 right-0 inline-flex gap-2">
						<button on:click={() => copyToClipboard(jsonStr)}><ClipboardCopy size={16} /></button>
						<button on:click={jsonViewer.openDrawer}><Expand size={16} /></button>
					</div>
				{/if}
			</div>{/if}{#if !forceJson && resultKind == 'table-col'}<div
				class="grid grid-flow-col-dense border border-gray-200 rounded-md"
			>
				{#each Object.keys(result) as col}
					<div class="flex flex-col max-h-40 min-w-full">
						<div class="px-12 text-left uppercase border-b bg-gray-50 overflow-hidden rounded-t-md">
							{col}
						</div>
						{#if Array.isArray(result[col])}
							{#each result[col] as item}
								<div class="px-12 text-left text-xs whitespace-nowrap overflow-auto pb-2">
									{typeof item === 'string' ? item : JSON.stringify(item)}
								</div>
							{/each}
						{/if}
					</div>
				{/each}
			</div>
		{:else if !forceJson && resultKind == 'table-row'}<div
				class="grid grid-flow-col-dense border border-gray-200"
			>
				<TableCustom>
					<tbody slot="body">
						{#each asListOfList(result) as row}
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
		{:else if !forceJson && resultKind == 'file'}
			<div
				><a
					download={result.filename ?? 'windmill.file'}
					href="data:application/octet-stream;base64,{contentOrRootString(result.file)}">Download</a
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
		{:else if !forceJson && resultKind == 'approval'}<div class="flex flex-col gap-3 mt-8 mx-4">
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
				<Button color="red" variant="border" on:click={() => fetch(result['cancel'])}>Cancel</Button
				>
				<div class="center-center"
					><a rel="noreferrer" target="_blank" href={result['approvalPage']}>Approval Page</a></div
				>
			</div>
		{:else}
			{#if jsonStr.length > 10000}
				<div class="text-sm mb-2 text-tertiary">
					<a
						download="{filename ?? 'result'}.json"
						href={workspaceId && jobId
							? `/api/w/${workspaceId}/jobs_u/completed/get_result/${jobId}`
							: `data:text/json;charset=utf-8,${encodeURIComponent(jsonStr)}`}>Download</a
					>
					JSON is too large to be displayed in full.
				</div>
				<ObjectViewer json={result} />
			{:else if typeof result == 'string' && result.length > 0}
				<pre class="text-sm">{result}</pre>
				<div class="flex">
					<Button on:click={() => copyToClipboard(result)} color="light" size="xs">
						<div class="flex gap-2 items-center">Copy <ClipboardCopy size={12} /> </div>
					</Button>
				</div>
			{:else}
				<Highlight language={json} code={jsonStr.replace(/\\n/g, '\n')} />
			{/if}
		{/if}
	{:else}
		<div class="text-tertiary text-sm">No result: {jsonStr}</div>
	{/if}
</div>

{#if !disableExpand}
	<Portal>
		<Drawer bind:this={jsonViewer} size="900px">
			<DrawerContent title="Expanded Result" on:close={jsonViewer.closeDrawer}>
				<svelte:fragment slot="actions">
					<a
						class="text-sm text-secondary mr-2 inline-flex gap-2 items-center py-2 px-2 hover:bg-gray-100 rounded-lg"
						download="{filename ?? 'result'}.json"
						href={workspaceId && jobId
							? `/api/w/${workspaceId}/jobs_u/completed/get_result/${jobId}`
							: `data:text/json;charset=utf-8,${encodeURIComponent(jsonStr)}`}
						>Download <Download size={14} /></a
					>
					<Button on:click={() => copyToClipboard(jsonStr)} color="light" size="xs">
						<div class="flex gap-2 items-center">Copy to clipboard <ClipboardCopy /> </div>
					</Button>
				</svelte:fragment>
				{#if jsonStr.length > 100000}
					<div class="text-sm mb-2 text-tertiary">
						<a
							download="{filename ?? 'result'}.json"
							href="data:text/json;charset=utf-8,{encodeURIComponent(jsonStr)}">Download</a
						>
						JSON is too large to be displayed in full.
					</div>
				{:else if typeof result == 'string' && result.length > 0}
					<pre class="text-sm">{result}</pre>
					<div class="flex">
						<Button on:click={() => copyToClipboard(result)} color="light" size="xs">
							<div class="flex gap-2 items-center">Copy <ClipboardCopy size={12} /> </div>
						</Button>
					</div>
				{:else}
					<Highlight language={json} code={jsonStr.replace(/\\n/g, '\n')} />
				{/if}
			</DrawerContent>
		</Drawer>
	</Portal>
{/if}
