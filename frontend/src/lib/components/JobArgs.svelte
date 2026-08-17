<script lang="ts">
	import { ChevronRightSquare, ClipboardCopy, Download, Expand } from 'lucide-svelte'
	import ArgInfo from './ArgInfo.svelte'
	import { Badge, Button, Drawer, DrawerContent, Skeleton } from './common'

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
	import { downloadViaClient, shouldDownloadViaClient } from '$lib/utils/downloadFile'
	import { appendViewToken } from '$lib/viewToken'

	interface Props {
		id?: string | undefined
		args: any
		argLabel?: string | undefined
		workspace?: string | undefined
	}

	let { id = undefined, args, argLabel = undefined, workspace = undefined }: Props = $props()

	// Internal flag injected by "test this step" runs to suppress the asset
	// dispatcher. Not a real input: shown as a badge instead of a table row,
	// but kept in the expanded JSON drawer and download.
	const SKIP_ASSET_DISPATCH_ARG = '_wmill_skip_asset_dispatch'
	// Internal map (path -> content digest) injected by local-dev previews so
	// relative imports resolve from not-yet-deployed local content. Same
	// treatment: badge, not a noisy table row.
	const TEMP_SCRIPT_REFS_ARG = '_TEMP_SCRIPT_REFS'
	const INTERNAL_ARGS = [SKIP_ASSET_DISPATCH_ARG, TEMP_SCRIPT_REFS_ARG]

	let skippedAssetDispatch = $derived(
		args != undefined && typeof args === 'object' && args[SKIP_ASSET_DISPATCH_ARG] === true
	)
	let hasTempScriptRefs = $derived(
		args != undefined && typeof args === 'object' && args[TEMP_SCRIPT_REFS_ARG] != undefined
	)
	let displayArgs = $derived(
		args != undefined && typeof args === 'object'
			? Object.fromEntries(Object.entries(args).filter(([k]) => !INTERNAL_ARGS.includes(k)))
			: args
	)

	let jsonViewer: Drawer | undefined = $state()
	let runLocally: Drawer | undefined = $state()
	let jsonStr = $state('')

	const argsDownloadName = 'windmill-args.json'
	let argsApiPath = $derived(
		id && workspace ? appendViewToken(`/w/${workspace}/jobs_u/get_args/${id}`) : undefined
	)
	let argsDataHref = $derived(`data:text/json;charset=utf-8,${encodeURIComponent(jsonStr)}`)

	function pythonCode() {
		return `
if __name__ == "__main__":
${Object.entries(displayArgs)
	.map(([arg, value]) => {
		return `    ${arg} = ${JSON.stringify(value)}`
	})
	.join('\n')}

	main(${Object.keys(displayArgs)
		.map((x) => `${x} = ${x}`)
		.join(', ')})
`
	}

	function typescriptCode() {
		return `
if (import.meta.main) {
${Object.entries(displayArgs)
	.map(([arg, value]) => {
		return `  let ${arg} = ${JSON.stringify(value)}`
	})
	.join('\n')}

  await main(...)
}
`
	}
</script>

{#if args && typeof args === 'object' && deepEqual( Object.keys(args ?? {}), ['reason'] ) && args['reason'] == 'PREPROCESSOR_ARGS_ARE_DISCARDED'}
	Preprocessor args are discarded
{:else if id && workspace && args && typeof args === 'object' && deepEqual( Object.keys(args ?? {}), ['reason'] ) && args['reason'] == 'WINDMILL_TOO_BIG'}
	The args are too big in size to be able to fetch alongside job. Please {#if shouldDownloadViaClient()}<button
			class="text-blue-500 hover:underline"
			onclick={() => downloadViaClient(argsApiPath!, argsDownloadName)}
			>download the JSON file to view them</button
		>{:else}<a href="/api{argsApiPath}" target="_blank">download the JSON file to view them</a
		>{/if}.
{:else}
	<div class="relative">
		<DataTable size="sm" containerClass="bg-surface-tertiary">
			<Head>
				<tr class="w-full">
					<Cell head first>
						{argLabel ?? 'Input'}
						{#if skippedAssetDispatch}
							<Badge
								color="gray"
								verySmall
								title="This run was started with {SKIP_ASSET_DISPATCH_ARG} and did not auto-trigger downstream asset consumers"
							>
								asset dispatch skipped
							</Badge>
						{/if}
						{#if hasTempScriptRefs}
							<Badge
								color="gray"
								verySmall
								title="This preview carried {TEMP_SCRIPT_REFS_ARG} so relative imports resolve from local (not-yet-deployed) script content"
							>
								local import refs
							</Badge>
						{/if}
					</Cell>
					<Cell head last>Value</Cell>
				</tr>
				{#snippet headerAction()}
					<div class="center-center -m-1">
						<Button
							unifiedSize="md"
							variant="subtle"
							onClick={() => {
								jsonStr = JSON.stringify(args, null, 4)
								jsonViewer?.openDrawer()
							}}
							iconOnly
							startIcon={{ icon: Expand }}
						></Button>
					</div>
				{/snippet}
			</Head>

			<tbody class="divide-y w-full">
				{#if displayArgs && typeof displayArgs === 'object' && Object.keys(displayArgs ?? {}).length > 0}
					{#each Object.entries(displayArgs ?? {}).sort( (a, b) => a?.[0]?.localeCompare(b?.[0]) ) as [arg, value]}
						<Row>
							<Cell first>{arg}</Cell>
							<Cell><ArgInfo {value} /></Cell>
						</Row>
					{/each}
				{:else if displayArgs && typeof displayArgs !== 'object'}
					<Row><Cell>Argument is not an object (type: {typeof displayArgs})</Cell></Row>
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
			{#if argsApiPath && shouldDownloadViaClient()}
				<Button
					on:click={() => downloadViaClient(argsApiPath!, argsDownloadName)}
					startIcon={{ icon: Download }}
					size="xs"
					color="light"
				>
					Download
				</Button>
			{:else}
				<Button
					download={argsDownloadName}
					href={argsApiPath ? `/api${argsApiPath}` : argsDataHref}
					startIcon={{ icon: Download }}
					size="xs"
					color="light"
				>
					Download
				</Button>
			{/if}
			<Button
				on:click={() => runLocally?.openDrawer()}
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
			<div class="text-sm mb-2 text-primary">
				{#if argsApiPath && shouldDownloadViaClient()}
					<button
						class="underline"
						onclick={() => downloadViaClient(argsApiPath!, argsDownloadName)}
					>
						JSON is too large to be displayed in full.
					</button>
				{:else}
					<a download={argsDownloadName} href={argsApiPath ? `/api${argsApiPath}` : argsDataHref}>
						JSON is too large to be displayed in full.
					</a>
				{/if}
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
