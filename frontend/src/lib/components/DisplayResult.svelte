<script lang="ts">
	import { Highlight } from 'svelte-highlight'
	import { json } from 'svelte-highlight/languages'
	import TableCustom from './TableCustom.svelte'
	import { copyToClipboard, truncate } from '$lib/utils'
	import { Button, Drawer, DrawerContent } from './common'
	import autosize from 'svelte-autosize'
	import { ClipboardCopy } from 'lucide-svelte'
	import Portal from 'svelte-portal'

	export let result: any
	export let requireHtmlApproval = false
	export let filename: string | undefined = undefined

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

	function inferResultKind(result: any) {
		if (result) {
			try {
				let keys = Object.keys(result)
				if (isRectangularArray(result)) {
					return 'table-row'
				} else if (keys.map((k) => Array.isArray(result[k])).reduce((a, b) => a && b)) {
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
					return 'filename'
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
	let payload = ''

	let jsonViewer: Drawer
</script>

<div class="inline-highlight">
	{#if result != undefined}
		{#if resultKind && resultKind != 'json'}
			<div class="mb-2 text-gray-500 text-sm bg-gray-50/20">
				as JSON&nbsp;<input class="windmillapp" type="checkbox" bind:checked={forceJson} /></div
			>{/if}{#if typeof result == 'object' && Object.keys(result).length > 0}<div
				class="mb-2 w-full text-sm text-gray-700 relative"
				>The result keys are: <b>{truncate(Object.keys(result).join(', '), 50)}</b>
				<div class="text-gray-500 text-xs absolute top-5 right-0">
					<button on:click={jsonViewer.openDrawer}>Expand</button>
				</div></div
			>{/if}{#if !forceJson && resultKind == 'table-col'}<div
				class="grid grid-flow-col-dense border border-gray-200 rounded-md"
			>
				{#each Object.keys(result) as col}
					<div class="flex flex-col max-h-40 min-w-full">
						<div class="px-12 text-left uppercase border-b bg-gray-50 overflow-hidden rounded-t-md">
							{col}
						</div>
						{#if Array.isArray(result[col])}
							{#each result[col] as item}
								<div class="px-12 text-left text-xs whitespace-nowrap">
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
								class="text-gray-600 mb-2 text-left border-2 !border-t-0 rounded-b border-red-400 overflow-auto p-1"
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
			<div class="h-full"
				><img alt="png rendered" class="w-auto h-full" src="data:image/png;base64,{result.png}" />
			</div>
		{:else if !forceJson && resultKind == 'jpeg'}
			<div class="h-full"
				><img
					alt="jpeg rendered"
					class="w-auto h-full"
					src="data:image/jpeg;base64,{result.jpeg}"
				/>
			</div>
		{:else if !forceJson && resultKind == 'svg'}
			<div
				><a download="windmill.svg" href="data:text/plain;base64,{btoa(result.svg)}">Download</a>
			</div>
			<div class="h-full overflow-auto">{@html result.svg} </div>
		{:else if !forceJson && resultKind == 'gif'}
			<div class="h-full"
				><img alt="gif rendered" class="w-auto h-full" src="data:image/gif;base64,{result.gif}" />
			</div>
		{:else if !forceJson && resultKind == 'file'}
			<div
				><a download="windmill.file" href="data:application/octet-stream;base64,{result.file}"
					>Download</a
				>
			</div>
		{:else if !forceJson && resultKind === 'filename'}
			<div>
				<a download={result.filename} href="data:application/octet-stream;base64,{result.file}">
					Download
				</a>
			</div>
		{:else if !forceJson && resultKind == 'error'}<div>
				<span class="text-red-500 font-semibold text-sm whitespace-pre-wrap"
					>{result.error.name}: {result.error.message}</span
				>
				<pre class="text-sm whitespace-pre-wrap text-gray-900">{result.error.stack ?? ''}</pre>
			</div>
		{:else if !forceJson && resultKind == 'approval'}<div class="flex flex-col gap-1 mx-4">
				<Button
					color="green"
					variant="border"
					on:click={() =>
						fetch(result['resume'], {
							method: 'POST',
							body: JSON.stringify(payload),
							headers: { 'Content-Type': 'application/json' }
						})}
				>
					Resume</Button
				>
				<Button color="red" variant="border" on:click={() => fetch(result['cancel'])}>Cancel</Button
				>
				<div>
					<h3>Payload</h3>
					<div class="border border-black">
						<input type="text" bind:value={payload} use:autosize />
					</div>
				</div>
				<div class="center-center"
					><a rel="noreferrer" target="_blank" href={result['approvalPage']}>Approval Page</a></div
				>
			</div>
		{:else}
			{@const jsonStr = JSON.stringify(result, null, 4).replace(/\\n/g, '\n')}
			{#if jsonStr.length > 10000}
				JSON too large. <a
					download="{filename ?? 'result'}.json"
					href="data:text/json;charset=utf-8,{encodeURIComponent(jsonStr)}">Download</a
				>
			{:else}
				<Highlight language={json} code={jsonStr} />
			{/if}
		{/if}
	{:else}
		<div class="text-gray-500 text-sm">No result: {JSON.stringify(result)}</div>
	{/if}
</div>

<Portal>
	<Drawer bind:this={jsonViewer} size="900px">
		<DrawerContent title="Expanded Result" on:close={jsonViewer.closeDrawer}>
			<svelte:fragment slot="actions">
				<Button
					on:click={() => copyToClipboard(JSON.stringify(result, null, 4))}
					color="light"
					size="xs"
				>
					<div class="flex gap-2 items-center">Copy to clipboard <ClipboardCopy /> </div>
				</Button>
			</svelte:fragment>
			<Highlight language={json} code={JSON.stringify(result, null, 4).replace(/\\n/g, '\n')} />
		</DrawerContent>
	</Drawer>
</Portal>
