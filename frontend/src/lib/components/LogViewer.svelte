<script lang="ts">
	import { ClipboardCopy, Download, Loader2 } from 'lucide-svelte'
	import { Button, Drawer, DrawerContent } from './common'
	import { copyToClipboard } from '$lib/utils'
	import { workspaceStore } from '$lib/stores'
	import AnsiUp from 'ansi_up'

	export let content: string | undefined
	export let isLoading: boolean
	export let duration: number | undefined = undefined
	export let mem: number | undefined = undefined
	export let wrapperClass = ''
	export let jobId: string | undefined = undefined

	// @ts-ignore
	const ansi_up = new AnsiUp()

	let scroll = true
	let div: HTMLElement | null = null

	$: if (content != undefined) {
		isLoading = false
	}

	$: content && scrollToBottom()
	$: html = ansi_up.ansi_to_html(content ?? '')
	export function scrollToBottom() {
		scroll && setTimeout(() => div?.scroll({ top: div?.scrollHeight, behavior: 'smooth' }), 100)
	}

	let logViewer: Drawer
</script>

<Drawer bind:this={logViewer} size="900px">
	<DrawerContent title="Expanded Logs" on:close={logViewer.closeDrawer}>
		<svelte:fragment slot="actions">
			<a
				class="text-sm text-secondary mr-2 inline-flex gap-2 items-center py-2 px-2 hover:bg-surface-hover rounded-lg"
				download="windmill-logs.json"
				href="/api/w/{$workspaceStore}/jobs_u/get_logs/{jobId}">Download <Download size={14} /></a
			>
			<Button on:click={() => copyToClipboard(content)} color="light" size="xs">
				<div class="flex gap-2 items-center">Copy to clipboard <ClipboardCopy /> </div>
			</Button>
		</svelte:fragment>
		<div>
			<pre class="bg-surface-secondary text-secondary text-xs w-full p-2"
				>{#if content}{content}{:else if isLoading}Waiting for job to start...{:else}No logs are available yet{/if}</pre
			>
		</div>
	</DrawerContent>
</Drawer>

<div class="relative w-full h-full {wrapperClass}">
	<div bind:this={div} class="w-full h-full overflow-auto relative secondaryBackground">
		<div class="sticky top-0 right-0 w-full flex flex-row-reverse justify-between text-sm">
			<div class="flex gap-1">
				<button on:click={logViewer.openDrawer}>Expand</button>
				<div class="py-2 pr-2 text-xs flex gap-2 items-center">
					Auto scroll
					<input class="windmillapp" type="checkbox" bind:checked={scroll} />
				</div>
			</div>
		</div>
		{#if isLoading}
			<Loader2 class="animate-spin absolute top-2 left-2" />
		{:else if duration}
			<span class="absolute text-xs text-tertiary dark:text-gray-400 top-2 left-2"
				>took {duration}ms</span
			>
		{/if}
		{#if mem}
			<span class="absolute text-xs text-tertiary dark:text-gray-400 top-2 left-36"
				>mem peak: {(mem / 1024).toPrecision(4)}MB</span
			>
		{/if}
		<pre class="whitespace-pre-wrap break-words text-xs w-full p-2"
			>{#if content}<span>{@html html}</span>{:else if !isLoading}<span
					>No logs are available yet</span
				>{/if}</pre
		>
	</div>
</div>
