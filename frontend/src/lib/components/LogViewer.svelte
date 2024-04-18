<script lang="ts" context="module">
	const s3LogPrefix = '[windmill] Previous logs have been saved to object storage at logs/'
</script>

<script lang="ts">
	import { ClipboardCopy, Download, Expand, Loader2 } from 'lucide-svelte'
	import { Button, Drawer, DrawerContent } from './common'
	import { copyToClipboard } from '$lib/utils'
	import { workspaceStore } from '$lib/stores'
	import AnsiUp from 'ansi_up'
	import NoWorkerWithTagWarning from './runs/NoWorkerWithTagWarning.svelte'
	import { JobService } from '$lib/gen'

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

	// let downloadStartUrl: string | undefined = undefined

	let LOG_INC = 10000
	let LOG_LIMIT = LOG_INC

	let lastJobId = jobId

	let loadedFromObjectStore = ''
	$: if (jobId !== lastJobId) {
		lastJobId = jobId
		loadedFromObjectStore = ''
		LOG_LIMIT = LOG_INC
	}

	$: downloadStartUrl = truncatedContent.startsWith(s3LogPrefix)
		? truncatedContent.substring(s3LogPrefix.length, truncatedContent.indexOf('\n'))
		: undefined

	$: truncatedContent = truncateContent(content, loadedFromObjectStore, LOG_LIMIT)

	function truncateContent(
		jobContent: string | undefined,
		loadedFromObjectStore: string,
		limit: number
	) {
		let content = loadedFromObjectStore + jobContent ?? ''
		if (content.length > limit) {
			return content.substring(content.length - limit)
		}
		return content
	}

	$: if (content != undefined && isLoading) {
		isLoading = false
	}

	$: truncatedContent && scrollToBottom()

	$: html = ansi_up.ansi_to_html(
		downloadStartUrl
			? truncatedContent.substring(truncatedContent.indexOf('\n') + 2, truncatedContent.length)
			: truncatedContent
	)
	export function scrollToBottom() {
		scroll && setTimeout(() => div?.scroll({ top: div?.scrollHeight, behavior: 'smooth' }), 100)
	}

	let logViewer: Drawer

	async function getStoreLogs() {
		if (downloadStartUrl) {
			scroll = false
			let res = (await JobService.getLogFileFromStore({
				workspace: $workspaceStore ?? '',
				path: downloadStartUrl
			})) as string
			downloadStartUrl = undefined
			LOG_LIMIT += Math.min(LOG_INC, res.length)
			loadedFromObjectStore = res + loadedFromObjectStore
			let newC = truncateContent(content, loadedFromObjectStore, LOG_LIMIT)
			LOG_LIMIT -= newC.indexOf('\n') + 1
		} else {
			console.error('No file detected to download from')
		}
	}

	function showMoreTruncate(len: number) {
		scroll = false
		LOG_LIMIT += LOG_INC
		console.log(LOG_INC, len, LOG_LIMIT)
		let newC = truncateContent(content, loadedFromObjectStore, LOG_LIMIT)
		LOG_LIMIT -= newC.indexOf('\n') + 1
	}
</script>

<Drawer bind:this={logViewer} size="900px">
	<DrawerContent title="Expanded Logs" on:close={logViewer.closeDrawer}>
		<svelte:fragment slot="actions">
			<Button
				href="/api/w/{$workspaceStore}/jobs_u/get_logs/{jobId}"
				download="windmill_logs_{jobId}.txt"
				color="light"
				size="xs"
				startIcon={{
					icon: Download
				}}
			>
				Download
			</Button>

			<Button
				on:click={() => copyToClipboard(content)}
				color="light"
				size="xs"
				startIcon={{
					icon: ClipboardCopy
				}}
			>
				Copy to clipboard
			</Button>
		</svelte:fragment>
		<div>
			<pre
				class="bg-surface-secondary text-secondary text-xs w-full p-2 whitespace-pre-wrap border rounded-md"
				>{#if content}{@const len =
						(content?.length ?? 0) +
						(loadedFromObjectStore?.length ?? 0)}{#if downloadStartUrl}<button
							on:click={getStoreLogs}>Show more...</button
						><br
						/>{:else if len > LOG_LIMIT}(truncated to the last {LOG_LIMIT} characters)... <button
							on:click={() => showMoreTruncate(len)}>Show more</button
						>
					{/if}{@html html}{:else if isLoading}Waiting for job to start...{:else}No logs are available yet{/if}</pre
			>
		</div>
	</DrawerContent>
</Drawer>

<div class="relative w-full h-full {wrapperClass}">
	<div
		bind:this={div}
		class="w-full h-full overflow-auto relative bg-surface-secondary max-h-screen"
	>
		<div class="sticky z-10 top-0 right-0 w-full flex flex-row-reverse justify-between text-sm">
			<div class="flex gap-2 pl-0.5 bg-surface-secondary">
				<div class="flex items-center">
					<a
						class="text-primary pb-0.5"
						target="_blank"
						href="/api/w/{$workspaceStore}/jobs_u/get_logs/{jobId}"
						download="windmill_logs_{jobId}.txt"
						><Download size="14" />
					</a>
				</div>
				<button on:click={logViewer.openDrawer}><Expand size="12" /></button>
				<div
					class="{small ? '' : 'py-2'} pr-2 {small
						? '!text-2xs'
						: '!text-xs'} flex gap-2 text-tertiary items-center"
				>
					Auto scroll
					<input class="windmillapp" type="checkbox" bind:checked={scroll} />
				</div>
			</div>
		</div>
		{#if isLoading}
			<div class="flex gap-2 absolute top-2 left-2 items-center z-10">
				<Loader2 class="animate-spin" />
				{#if tag}
					<div class="flex flex-row items-center gap-1">
						<div class="text-secondary {small ? '!text-2xs' : '!text-xs'}">tag: {tag}</div>
						<NoWorkerWithTagWarning {tag} />
					</div>
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
			>{#if content}{@const len =
					(content?.length ?? 0) +
					(loadedFromObjectStore?.length ?? 0)}{#if downloadStartUrl}<button on:click={getStoreLogs}
						>Show more...</button
					><br />{:else if len > LOG_LIMIT}(truncated to the last {LOG_LIMIT} characters)<br
					/><button on:click={() => showMoreTruncate(len)}>Show more..</button><br />{/if}<span
					>{@html html}</span
				>{:else if !isLoading}<span>No logs are available yet</span>{/if}</pre
		>
	</div>
</div>
