<script lang="ts">
	import Badge from '$lib/components/common/badge/Badge.svelte'
	// import { ClipboardCopy, Download } from 'lucide-svelte'
	// import Drawer from '../common/drawer/Drawer.svelte'
	// import DrawerContent from '../common/drawer/DrawerContent.svelte'
	// import { Highlight } from 'svelte-highlight'
	// import Button from '$lib/components/common/button/Button.svelte'
	// import { json } from 'svelte-highlight/languages'
	// import { copyToClipboard } from '$lib/utils'
	// import { deepEqual } from 'fast-equals'

	export let flowStatus: any

	// $: args =
	// 	'_metadata' in flowStatus && 'original_args' in flowStatus['_metadata']
	// 		? flowStatus['_metadata']['original_args']
	// 		: undefined

	$: hasPreprocessedArgs =
		'_metadata' in flowStatus && !!flowStatus['_metadata']['preprocessed_args']

	// $: argsStr = args !== undefined ? JSON.stringify(args, null, 4) : undefined

	// let jsonViewer: Drawer | undefined = undefined
</script>

{#if hasPreprocessedArgs}
	<div>
		<Badge color="yellow">preprocessed args</Badge>
	</div>
{/if}

<!-- 
{#if args !== undefined && argsStr !== undefined}
	<Drawer bind:this={jsonViewer} size="900px">
		<DrawerContent title="Original args" on:close={jsonViewer.closeDrawer}>
			<svelte:fragment slot="actions">
				<Button
					download="windmill-args.json"
					href={`data:text/json;charset=utf-8,${encodeURIComponent(argsStr)}`}
					startIcon={{ icon: Download }}
					size="xs"
					color="light"
				>
					Download
				</Button>
				<Button
					on:click={() => copyToClipboard(argsStr)}
					color="light"
					size="xs"
					startIcon={{ icon: ClipboardCopy }}
				>
					Copy to clipboard
				</Button>
			</svelte:fragment>
			{#if args.length > 100000 || (args && typeof args === 'object' && deepEqual( Object.keys(args), ['reason'] ) && args['reason'] == 'WINDMILL_TOO_BIG')}
				<div class="text-sm mb-2 text-tertiary">
					<a
						download="windmill-args.json"
						href={`data:text/json;charset=utf-8,${encodeURIComponent(args)}`}
					>
						JSON is too large to be displayed in full.
					</a></div
				>
			{:else}
				<Highlight language={json} code={argsStr.replace(/\\n/g, '\n')} />
			{/if}
		</DrawerContent>
	</Drawer>
	<button
		on:click={() => {
			jsonViewer?.openDrawer()
		}}
	>
		<Badge color="yellow">preprocessed args</Badge>
	</button>
{/if} -->
