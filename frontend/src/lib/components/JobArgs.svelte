<script lang="ts">
	import { ChevronRightSquare, ClipboardCopy, Download, Expand } from 'lucide-svelte'
	import ArgInfo from './ArgInfo.svelte'
	import { Button, Drawer, DrawerContent, Skeleton } from './common'

	import { Highlight } from 'svelte-highlight'
	import { copyToClipboard } from '$lib/utils'
	import { json } from 'svelte-highlight/languages'
	import DataTable from './table/DataTable.svelte'
	import Cell from './table/Cell.svelte'
	import Head from './table/Head.svelte'
	import Row from './table/Row.svelte'

	export let args: any

	let jsonViewer: Drawer
	let runLocally: Drawer
	let jsonStr = ''

	function pythonCode() {
		return `
if __name__ == 'main':
${Object.entries(args)
	.map(([arg, value]) => {
		return `  ${arg} = ${JSON.stringify(value)}`
	})
	.join('\n')}

  main(${Object.keys(args)
		.map((x) => `${x} = ${x}`)
		.join(', ')})
`
	}

	function typescriptCode() {
		return `
if (import.meta.main) {
${Object.entries(args)
	.map(([arg, value]) => {
		return `  let ${arg} = ${JSON.stringify(value)}`
	})
	.join('\n')}

  await main(...)
}
`
	}
</script>

<div class="relative">
	<DataTable size="sm">
		<Head>
			<tr>
				<Cell head first>Argument</Cell>
				<Cell head last>Value</Cell>
			</tr>
			<svelte:fragment slot="headerAction">
				<button
					on:click={() => {
						jsonStr = JSON.stringify(args, null, 4)
						jsonViewer.openDrawer()
					}}
				>
					<Expand size={18} />
				</button>
			</svelte:fragment>
		</Head>

		<tbody class="divide-y">
			{#if args && Object.keys(args).length > 0}
				{#each Object.entries(args).sort((a, b) => a[0].localeCompare(b[0])) as [arg, value]}
					<Row>
						<Cell first>{arg}</Cell>
						<Cell last><ArgInfo {value} /></Cell>
					</Row>
				{/each}
			{:else if args}
				<Row><Cell>No arguments</Cell></Row>
			{:else}
				<Row>
					<Cell first>
						<Skeleton layout={[[1], 0.5, [1]]} />
					</Cell>
					<Cell last>
						<Skeleton layout={[[1], 0.5, [1]]} />
					</Cell>
				</Row>
			{/if}
		</tbody>
	</DataTable>
</div>

<Drawer bind:this={jsonViewer} size="900px">
	<DrawerContent title="Expanded Args" on:close={jsonViewer.closeDrawer}>
		<svelte:fragment slot="actions">
			<a
				class="text-sm text-secondary mr-2 inline-flex gap-2 items-center py-2 px-2 hover:bg-surface-hover rounded-lg"
				download="windmill-args.json"
				href="data:text/json;charset=utf-8,{encodeURIComponent(jsonStr)}"
				>Download <Download size={14} /></a
			>
			<Button on:click={runLocally.openDrawer} color="light" size="xs">
				<div class="flex gap-2 items-center">Use in a local script<ChevronRightSquare /> </div>
			</Button>
			<Button on:click={() => copyToClipboard(jsonStr)} color="light" size="xs">
				<div class="flex gap-2 items-center">Copy to clipboard <ClipboardCopy /> </div>
			</Button>
		</svelte:fragment>
		{#if jsonStr.length > 100000}
			<div class="text-sm mb-2 text-tertiary">
				<a
					download="windmill-args.json"
					href="data:text/json;charset=utf-8,{encodeURIComponent(jsonStr)}">Download</a
				>
				JSON is too large to be displayed in full.
			</div>
		{:else}
			<Highlight language={json} code={jsonStr.replace(/\\n/g, '\n')} />
		{/if}
	</DrawerContent>
</Drawer>
<Drawer bind:this={runLocally} size="900px">
	<DrawerContent title="Run locally" on:close={runLocally.closeDrawer}>
		<h3 class="mb-2">Envs</h3>
		If using the wmill client in your code, set the following env variables:
		<pre
			><code
				>BASE_URL="{window.location.origin}"
WM_TOKEN="{'<TOKEN>'}"</code
			></pre
		>
		<h3 class="mt-8">TypeScript</h3>
		<pre><code>{typescriptCode()}</code></pre>
		<h3 class="mt-8">Python</h3>
		<pre><code>{pythonCode()}</code></pre>
	</DrawerContent>
</Drawer>
