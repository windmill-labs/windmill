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
	import HighlightTheme from './HighlightTheme.svelte'
	import { deepEqual } from 'fast-equals'
	import { isWindmillTooBigObject } from './job_args'

	export let id: string | undefined = undefined
	export let args: any
	export let argLabel: string | undefined = undefined
	export let workspace: string | undefined = undefined

	let jsonViewer: Drawer
	let runLocally: Drawer
	let jsonStr = ''

	function pythonCode() {
		return `
if __name__ == "__main__":
${Object.entries(args)
	.map(([arg, value]) => {
		return `    ${arg} = ${JSON.stringify(value)}`
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

{#if args && typeof args === 'object' && deepEqual( Object.keys(args), ['reason'] ) && args['reason'] == 'PREPROCESSOR_ARGS_ARE_DISCARDED'}
	Preprocessor args are discarded
{:else if id && workspace && args && typeof args === 'object' && deepEqual( Object.keys(args), ['reason'] ) && args['reason'] == 'WINDMILL_TOO_BIG'}
	The args are too big in size to be able to fetch alongside job. Please <a
		href="/api/w/{workspace}/jobs_u/get_args/{id}"
		target="_blank">download the JSON file to view them</a
	>.
{:else}
	<div class="relative">
		<DataTable size="sm">
			<Head>
				<tr class="w-full">
					<Cell head first>{argLabel ?? 'Arg'}</Cell>
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

			<tbody class="divide-y w-full">
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
{/if}

<HighlightTheme />

<Drawer bind:this={jsonViewer} size="900px">
	<DrawerContent title="Expanded Args" on:close={jsonViewer.closeDrawer}>
		{#snippet actions()}
			<Button
				download="windmill-args.json"
				href={id && workspace
					? `/api/w/${workspace}/jobs_u/get_args/${id}`
					: `data:text/json;charset=utf-8,${encodeURIComponent(jsonStr)}`}
				startIcon={{ icon: Download }}
				size="xs"
				color="light"
			>
				Download
			</Button>
			<Button
				on:click={runLocally.openDrawer}
				color="light"
				size="xs"
				startIcon={{ icon: ChevronRightSquare }}
			>
				Use in a local script
			</Button>
			<Button
				on:click={() => copyToClipboard(jsonStr)}
				color="light"
				size="xs"
				startIcon={{ icon: ClipboardCopy }}
			>
				Copy to clipboard
			</Button>
		{/snippet}
		{#if jsonStr.length > 100000 || (id && workspace && args && isWindmillTooBigObject(args))}
			<div class="text-sm mb-2 text-tertiary">
				<a
					download="windmill-args.json"
					href={id && workspace
						? `/api/w/${workspace}/jobs_u/get_args/${id}`
						: `data:text/json;charset=utf-8,${encodeURIComponent(jsonStr)}`}
				>
					JSON is too large to be displayed in full.
				</a></div
			>
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
