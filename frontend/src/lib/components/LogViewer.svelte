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
	export let tag: string | undefined
	export let small = false

	// @ts-ignore
	const ansi_up = new AnsiUp()

	let scroll = true
	let div: HTMLElement | null = null

	let LOG_INC = 10000
	let LOG_LIMIT = LOG_INC

	let lastJobId = jobId
	$: if (jobId !== lastJobId) {
		lastJobId = jobId
		LOG_LIMIT = LOG_INC
	}
	$: truncatedContent = content
		? (content.length ?? 0) > LOG_LIMIT
			? content?.slice(-LOG_LIMIT)
			: content
		: ''

	$: if (content != undefined && isLoading) {
		isLoading = false
	}

	$: truncatedContent && scrollToBottom()
	$: html = ansi_up.ansi_to_html(truncatedContent ?? '')
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
				>{#if content}{#if content?.length > LOG_LIMIT}(truncated to the last {LOG_LIMIT} characters)... <button
							on:click={() => {
								scroll = false
								LOG_LIMIT = LOG_LIMIT + Math.min(LOG_INC, content?.length ?? 0 - LOG_LIMIT)
							}}>Show more</button
						>
					{/if}{@html html}{:else if isLoading}Waiting for job to start...{:else}No logs are available yet{/if}</pre
			>
		</div>
	</DrawerContent>
</Drawer>

<div class="relative w-full h-full {wrapperClass}">
	<div bind:this={div} class="w-full h-full overflow-auto relative bg-surface-secondary">
		<div class="sticky z-10 top-0 right-0 w-full flex flex-row-reverse justify-between text-sm">
			<div class="flex gap-1 pl-0.5 bg-surface-secondary">
				<button on:click={logViewer.openDrawer}>Expand</button>
				<div
					class="{small ? '' : 'py-2'} pr-2 {small
						? '!text-2xs'
						: '!text-xs'} flex gap-2 items-center"
				>
					Auto scroll
					<input class="windmillapp" type="checkbox" bind:checked={scroll} />
				</div>
			</div>
		</div>
		{#if isLoading}
			<div class="flex gap-2 absolute top-2 left-2 items-center">
				<Loader2 class="animate-spin" />
				{#if tag}
					<div class="text-secondary {small ? '!text-2xs' : '!text-xs'}">tag: {tag}</div>
				{/if}
			</div>
		{:else if duration}
			<span
				class="absolute {small ? '!text-2xs' : '!text-xs'} text-tertiary dark:text-gray-400 {small
					? 'top-0'
					: 'top-2'} left-2">took {duration}ms</span
			>
		{/if}
		{#if mem}
			<span
				class="absolute {small ? '!text-2xs' : '!text-xs'} text-tertiary dark:text-gray-400 {small
					? 'top-0'
					: 'top-2'}  left-36">mem peak: {(mem / 1024).toPrecision(4)}MB</span
			>
		{/if}
		<pre class="whitespace-pre-wrap break-words {small ? '!text-2xs' : '!text-xs'} w-full p-2"
			>{#if content}{#if content?.length > LOG_LIMIT}(truncated to the last {LOG_LIMIT} characters)... <button
						on:click={() => {
							scroll = false
							LOG_LIMIT = LOG_LIMIT + Math.min(LOG_INC, content?.length ?? 0 - LOG_LIMIT)
						}}>Show more</button
					>
				{/if}<span>{@html html}</span>{:else if !isLoading}<span>No logs are available yet</span
				>{/if}</pre
		>
	</div>
</div>
