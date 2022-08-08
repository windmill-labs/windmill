<script lang="ts">
	import { Highlight } from 'svelte-highlight'
	import { json } from 'svelte-highlight/languages'
	import TableCustom from './TableCustom.svelte'
	import { truncate } from '$lib/utils'

	export let result: any

	let resultKind: 'json' | 'table-col' | 'table-row' | 'png' | 'file' | 'jpeg' | 'gif' | undefined =
		inferResultKind(result)

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
				}
			} catch (err) {}
		}
		return 'json'
	}
</script>

{#if result}
	{#if typeof result == 'object' && Object.keys(result).length > 0}<div>
			The result keys are: <b>{Object.keys(result).join(', ')}</b>
		</div>
	{/if}
	{#if resultKind == 'table-col'}
		<div class="grid grid-flow-col-dense border border-gray-200 rounded-md ">
			{#each Object.keys(result) as col}
				<div class="flex flex-col max-h-40 min-w-full overflow-auto">
					<div class="px-12 text-left uppercase border-b bg-gray-50 overflow-hidden rounded-t-md ">
						{col}
					</div>
					{#if Array.isArray(result[col])}
						{#each result[col] as item}
							<div class="px-12 text-left  whitespace-nowrap">
								{typeof item === 'string' ? item : JSON.stringify(item)}
							</div>
						{/each}
					{/if}
				</div>
			{/each}
		</div>
	{:else if resultKind == 'table-row'}<div
			class="grid grid-flow-col-dense border border-gray-200 rounded-md "
		>
			<TableCustom>
				<tbody slot="body">
					{#each asListOfList(result) as row}
						<tr>
							{#each row as v}
								<td>{truncate(JSON.stringify(v), 200) ?? ''}</td>
							{/each}
						</tr>
					{/each}
				</tbody>
			</TableCustom>
		</div>
	{:else if resultKind == 'png'}
		<div class="h-full">
			Result is an image: <img
				alt="png rendered"
				class="w-auto h-full"
				src="data:image/png;base64,{result.png}"
			/>
		</div>
	{:else if resultKind == 'jpeg'}
		<div class="h-full">
			Result is an image: <img
				alt="jpeg rendered"
				class="w-auto h-full"
				src="data:image/jpeg;base64,{result.jpeg}"
			/>
		</div>
	{:else if resultKind == 'gif'}
		<div class="h-full">
			Result is an image: <img
				alt="gif rendered"
				class="w-auto h-full"
				src="data:image/gif;base64,{result.gif}"
			/>
		</div>
	{:else if resultKind == 'file'}
		<div>
			Result is a file: <a
				download="windmill.file"
				href="data:application/octet-stream;base64,{result.file}">Download</a
			>
		</div>
	{:else}<Highlight language={json} code={JSON.stringify(result, null, 4).replace(/\\n/g, '\n')} />
	{/if}
{/if}
