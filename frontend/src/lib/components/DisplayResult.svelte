<script lang="ts">
	import { Highlight } from 'svelte-highlight'
	import { json } from 'svelte-highlight/languages'
	import TableCustom from './TableCustom.svelte'
	import { truncate } from '$lib/utils'
	import { Button } from './common'
	import autosize from 'svelte-autosize'

	export let result: any

	let resultKind:
		| 'json'
		| 'table-col'
		| 'table-row'
		| 'png'
		| 'file'
		| 'jpeg'
		| 'gif'
		| 'error'
		| 'approval'
		| undefined
	$: resultKind = inferResultKind(result)

	let forceJson = false

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
				} else if (keys.length == 1 && keys[0] == 'png') {
					return 'png'
				} else if (keys.length == 1 && keys[0] == 'jpeg') {
					return 'jpeg'
				} else if (keys.length == 1 && keys[0] == 'file') {
					return 'file'
				} else if (keys.length == 1 && keys[0] == 'error') {
					return 'error'
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
</script>

<div class="inline-highlight">
	{#if result != undefined}
		{#if resultKind && resultKind != 'json'}
			<div class="mb-2 text-gray-500 text-sm bg-gray-50/20">
				as JSON&nbsp;<input type="checkbox" bind:checked={forceJson} /></div
			>{/if}{#if typeof result == 'object' && Object.keys(result).length > 0}<div
				class="mb-2 text-sm text-gray-700"
				>The result keys are: <b>{truncate(Object.keys(result).join(', '), 50)}</b></div
			>{/if}{#if !forceJson && resultKind == 'table-col'}<div
				class="grid grid-flow-col-dense border border-gray-200 rounded-md "
			>
				{#each Object.keys(result) as col}
					<div class="flex flex-col max-h-40 min-w-full">
						<div
							class="px-12 text-left uppercase border-b bg-gray-50 overflow-hidden rounded-t-md "
						>
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
				class="grid grid-flow-col-dense border border-gray-200 "
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
		{:else if !forceJson && resultKind == 'error'}<div>
				<span class="text-red-500 font-semibold text-sm"
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
		{:else}<Highlight
				language={json}
				code={JSON.stringify(result, null, 4).replace(/\\n/g, '\n')}
			/>
		{/if}
	{:else}
		<div class="text-gray-500 text-sm">No result</div>
	{/if}
</div>
