<script lang="ts" module>
	const s3LogPrefixes = [
		'\n[windmill] Previous logs have been saved to object storage at logs/',
		'\n[windmill] Previous logs have been saved to disk at logs/',
		'\n[windmill] No object storage set in instance settings. Previous logs have been saved to disk at logs/'
	]
</script>

<script lang="ts">
	import { ClipboardCopy, Download, Expand, Loader2 } from 'lucide-svelte'
	import { Button, Drawer, DrawerContent } from './common'
	import { copyToClipboard } from '$lib/utils'
	import { base } from '$lib/base'
	import { workspaceStore } from '$lib/stores'
	import AnsiUp from 'ansi_up'
	import NoWorkerWithTagWarning from './runs/NoWorkerWithTagWarning.svelte'
	import { JobService } from '$lib/gen'
	import Tooltip from './Tooltip.svelte'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		content: string | undefined
		isLoading: boolean
		duration?: number | undefined
		mem?: number | undefined
		wrapperClass?: string
		jobId?: string | undefined
		tag: string | undefined
		small?: boolean
		drawerOpen?: boolean
		noMaxH?: boolean
		noAutoScroll?: boolean
		download?: boolean
		customEmptyMessage?: string
		tagLabel?: string
		noPadding?: boolean
	}

	let {
		content,
		isLoading,
		duration = undefined,
		mem = undefined,
		wrapperClass = '',
		jobId = undefined,
		tag,
		small = false,
		drawerOpen = $bindable(false),
		noMaxH = false,
		noAutoScroll = false,
		download = true,
		customEmptyMessage = 'No logs are available yet',
		tagLabel = undefined,
		noPadding = false
	}: Props = $props()

	// @ts-ignore
	const ansi_up = $state(new AnsiUp())

	ansi_up.use_classes = true

	let scroll = $state(true)
	let div: HTMLElement | null = $state(null)

	// let downloadStartUrl: string | undefined = undefined

	let LOG_INC = 10000
	let LOG_LIMIT = $state(LOG_INC)

	let lastJobId = $state(jobId)

	let loadedFromObjectStore = $state('')

	function findPrefixIndex(truncateContent: string): number | undefined {
		let index = s3LogPrefixes.findIndex((x) => truncateContent.startsWith(x))
		if (index == -1) {
			return undefined
		}
		return index
	}
	function findStartUrl(truncateContent: string, prefixIndex: number | undefined = undefined) {
		if (prefixIndex == undefined) {
			return undefined
		}
		const end = truncateContent.substring(1).indexOf('\n')
		return prefixIndex != undefined && truncateContent
			? truncateContent.substring(
					s3LogPrefixes[prefixIndex]?.length,
					end == -1 ? undefined : end + 1
				)
			: undefined
	}

	function tooltipText(prefixIndex: number | undefined) {
		if (prefixIndex == undefined) {
			return 'No path/file detected to download from'
		} else if (prefixIndex == 0) {
			return 'Download the previous logs from the instance configured object store'
		} else if (prefixIndex == 1) {
			return 'Attempt to download the logs from disk. Assume there is a shared disk between the workers and the server at /tmp/windmill/logs. Upgrade to EE to use an object store such as S3 instead of a shared volume.'
		} else if (prefixIndex == 2) {
			return 'Attempt to download the logs from disk. Assume there is a shared disk between the workers and the server at /tmp/windmill/logs. Since you are on EE, you can alternatively use an object store such as S3 configured in the instance settings instead of a shared volume..'
		}
	}

	function truncateContent(
		jobContent: string | undefined,
		loadedFromObjectStore: string,
		limit: number
	) {
		let content = loadedFromObjectStore + (jobContent ?? '')
		if (content.length > limit) {
			return content.substring(content.length - limit)
		}
		return content
	}

	export function scrollToBottom() {
		// console.log('scrollToBottom', scroll, div)
		scroll && setTimeout(() => div?.scroll({ top: div?.scrollHeight, behavior: 'smooth' }), 100)
	}

	let logViewer: Drawer | undefined = $state()

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
		let newC = truncateContent(content, loadedFromObjectStore, LOG_LIMIT)
		let newlineIndex = newC.indexOf('\n') + 1
		if (newlineIndex < LOG_INC / 2) {
			LOG_LIMIT -= newlineIndex
		}
	}
	$effect.pre(() => {
		if (jobId !== lastJobId) {
			lastJobId = jobId
			loadedFromObjectStore = ''
			LOG_LIMIT = LOG_INC
		}
	})
	let truncatedContent = $derived(truncateContent(content, loadedFromObjectStore, LOG_LIMIT))
	let prefixIndex = $derived(findPrefixIndex(truncatedContent))
	let downloadStartUrl = $derived(findStartUrl(truncatedContent, prefixIndex))
	$effect.pre(() => {
		truncatedContent && scrollToBottom()
	})
	let html = $derived(
		ansi_up.ansi_to_html(
			downloadStartUrl && prefixIndex != undefined
				? truncatedContent.substring(
						truncatedContent.substring(1).indexOf('\n') + 2,
						truncatedContent.length
					)
				: truncatedContent
		)
	)
</script>

<Drawer bind:this={logViewer} bind:open={drawerOpen} size="900px">
	<DrawerContent title="Expanded Logs" on:close={logViewer.closeDrawer}>
		{#snippet actions()}
			{#if jobId && download}
				<Button
					href="{base}/api/w/{$workspaceStore}/jobs_u/get_logs/{jobId}"
					download="windmill_logs_{jobId}.txt"
					color="light"
					size="xs"
					startIcon={{
						icon: Download
					}}
				>
					Download
				</Button>
			{/if}

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
		{/snippet}
		<div>
			<pre
				class="bg-surface-secondary text-secondary text-xs w-full p-2 whitespace-pre-wrap border rounded-md"
				>{#if content}{@const len =
						(content?.length ?? 0) +
						(loadedFromObjectStore?.length ?? 0)}{#if downloadStartUrl}<button
							onclick={getStoreLogs}
							>Show more... <Tooltip>{tooltipText(prefixIndex)}</Tooltip></button
						><br />{:else if len > LOG_LIMIT}(truncated to the last {LOG_LIMIT} characters)...<br
						/><button onclick={() => showMoreTruncate(len)}>Show more..</button><br
						/>{/if}{@html html}{:else if isLoading}Waiting for job to start...{:else}No logs are available yet{/if}</pre
			>
		</div>
	</DrawerContent>
</Drawer>

<div class="relative w-full h-full {wrapperClass}">
	<div
		bind:this={div}
		class="w-full h-full overflow-auto relative bg-surface-secondary {noMaxH ? '' : 'max-h-screen'}"
	>
		<div class="sticky z-10 top-0 right-0 w-full flex flex-row-reverse justify-between text-sm">
			<div class="flex gap-2 pl-0.5 bg-surface-secondary">
				{#if jobId && download}
					<div class="flex items-center">
						<a
							class="text-primary pb-0.5"
							target="_blank"
							href="{base}/api/w/{$workspaceStore}/jobs_u/get_logs/{jobId}"
							download="windmill_logs_{jobId}.txt"
							><Download size="14" />
						</a>
					</div>
				{/if}
				<button onclick={logViewer.openDrawer}><Expand size="12" /></button>
				{#if !noAutoScroll}
					<div
						class="{small ? '' : 'py-2'} pr-2 {small
							? '!text-2xs'
							: '!text-xs'} flex gap-2 text-tertiary items-center"
					>
						Auto scroll
						<input class="windmillapp" type="checkbox" bind:checked={scroll} />
					</div>
				{/if}
			</div>
		</div>
		{#if isLoading}
			<div class="flex gap-2 absolute top-2 left-2 items-center z-10">
				<Loader2 class="animate-spin" />
				{#if tag}
					<div class="flex flex-row items-center gap-1">
						<div class="text-secondary {small ? '!text-2xs' : '!text-xs'}"
							>{tagLabel ?? 'tag'}: {tag}</div
						>
						<NoWorkerWithTagWarning {tagLabel} {tag} />
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
		<pre
			class={twMerge(
				'whitespace-pre break-words w-full',
				small ? '!text-2xs' : '!text-xs',
				noPadding ? '' : 'p-2'
			)}
			>{#if content}{@const len =
					(content?.length ?? 0) +
					(loadedFromObjectStore?.length ?? 0)}{#if downloadStartUrl}<button onclick={getStoreLogs}
						>Show more... &nbsp;<Tooltip>{tooltipText(prefixIndex)}</Tooltip></button
					><br />{:else if len > LOG_LIMIT}(truncated to the last {LOG_LIMIT} characters)<br
					/><button onclick={() => showMoreTruncate(len)}>Show more..</button><br />{/if}<span
					>{@html html}</span
				>{:else if !isLoading}<span>{customEmptyMessage}</span>{/if}</pre
		>
	</div>
</div>

<style global>
	/* Foreground colors */
	.ansi-black-fg {
		color: rgb(0, 0, 0);
	}
	.ansi-red-fg {
		color: rgb(187, 0, 0);
	}
	.ansi-green-fg {
		color: rgb(0, 187, 0);
	}
	.ansi-yellow-fg {
		color: rgb(187, 187, 0);
	}
	.ansi-blue-fg {
		color: rgb(0, 0, 187);
	}
	.ansi-magenta-fg {
		color: rgb(187, 0, 187);
	}
	.ansi-cyan-fg {
		color: rgb(0, 187, 187);
	}
	.ansi-white-fg {
		color: rgb(255, 255, 255);
	}

	.ansi-bright-black-fg {
		color: rgb(85, 85, 85);
	}
	.ansi-bright-red-fg {
		color: rgb(255, 85, 85);
	}
	.ansi-bright-green-fg {
		color: rgb(0, 255, 0);
	}
	.ansi-bright-yellow-fg {
		color: rgb(255, 255, 85);
	}
	.ansi-bright-blue-fg {
		color: rgb(85, 85, 255);
	}
	.ansi-bright-magenta-fg {
		color: rgb(255, 85, 255);
	}
	.ansi-bright-cyan-fg {
		color: rgb(85, 255, 255);
	}
	.ansi-bright-white-fg {
		color: rgb(255, 255, 255);
	}

	/* Background colors */
	.ansi-black-bg {
		background-color: rgb(0, 0, 0);
	}
	.ansi-red-bg {
		background-color: rgb(187, 0, 0);
	}
	.ansi-green-bg {
		background-color: rgb(0, 187, 0);
	}
	.ansi-yellow-bg {
		background-color: rgb(187, 187, 0);
	}
	.ansi-blue-bg {
		background-color: rgb(0, 0, 187);
	}
	.ansi-magenta-bg {
		background-color: rgb(187, 0, 187);
	}
	.ansi-cyan-bg {
		background-color: rgb(0, 187, 187);
	}
	.ansi-white-bg {
		background-color: rgb(255, 255, 255);
	}

	.ansi-bright-black-bg {
		background-color: rgb(85, 85, 85);
	}
	.ansi-bright-red-bg {
		background-color: rgb(255, 85, 85);
	}
	.ansi-bright-green-bg {
		background-color: rgb(0, 255, 0);
	}
	.ansi-bright-yellow-bg {
		background-color: rgb(255, 255, 85);
	}
	.ansi-bright-blue-bg {
		background-color: rgb(85, 85, 255);
	}
	.ansi-bright-magenta-bg {
		background-color: rgb(255, 85, 255);
	}
	.ansi-bright-cyan-bg {
		background-color: rgb(85, 255, 255);
	}
	.ansi-bright-white-bg {
		background-color: rgb(255, 255, 255);
	}

	/* Foreground colors for dark mode (Nord theme) */
	.dark .ansi-black-fg {
		color: rgb(46, 52, 64);
	}
	.dark .ansi-red-fg {
		color: rgb(191, 97, 106);
	}
	.dark .ansi-green-fg {
		color: rgb(163, 190, 140);
	}
	.dark .ansi-yellow-fg {
		color: rgb(235, 203, 139);
	}
	.dark .ansi-blue-fg {
		color: rgb(94, 129, 172);
	}
	.dark .ansi-magenta-fg {
		color: rgb(180, 142, 173);
	}
	.dark .ansi-cyan-fg {
		color: rgb(136, 192, 208);
	}
	.dark .ansi-white-fg {
		color: rgb(216, 222, 233);
	}

	.dark .ansi-bright-black-fg {
		color: rgb(67, 76, 94);
	}
	.dark .ansi-bright-red-fg {
		color: rgb(191, 97, 106);
	}
	.dark .ansi-bright-green-fg {
		color: rgb(163, 190, 140);
	}
	.dark .ansi-bright-yellow-fg {
		color: rgb(235, 203, 139);
	}
	.dark .ansi-bright-blue-fg {
		color: rgb(94, 129, 172);
	}
	.dark .ansi-bright-magenta-fg {
		color: rgb(180, 142, 173);
	}
	.dark .ansi-bright-cyan-fg {
		color: rgb(136, 192, 208);
	}
	.dark .ansi-bright-white-fg {
		color: rgb(229, 233, 240);
	}

	/* Background colors for dark mode (Nord theme) */
	.dark .ansi-black-bg {
		background-color: rgb(46, 52, 64);
	}
	.dark .ansi-red-bg {
		background-color: rgb(191, 97, 106);
	}
	.dark .ansi-green-bg {
		background-color: rgb(163, 190, 140);
	}
	.dark .ansi-yellow-bg {
		background-color: rgb(235, 203, 139);
	}
	.dark .ansi-blue-bg {
		background-color: rgb(94, 129, 172);
	}
	.dark .ansi-magenta-bg {
		background-color: rgb(180, 142, 173);
	}
	.dark .ansi-cyan-bg {
		background-color: rgb(136, 192, 208);
	}
	.dark .ansi-white-bg {
		background-color: rgb(216, 222, 233);
	}

	.dark .ansi-bright-black-bg {
		background-color: rgb(67, 76, 94);
	}
	.dark .ansi-bright-red-bg {
		background-color: rgb(191, 97, 106);
	}
	.dark .ansi-bright-green-bg {
		background-color: rgb(163, 190, 140);
	}
	.dark .ansi-bright-yellow-bg {
		background-color: rgb(235, 203, 139);
	}
	.dark .ansi-bright-blue-bg {
		background-color: rgb(94, 129, 172);
	}
	.dark .ansi-bright-magenta-bg {
		background-color: rgb(180, 142, 173);
	}
	.dark .ansi-bright-cyan-bg {
		background-color: rgb(136, 192, 208);
	}
	.dark .ansi-bright-white-bg {
		background-color: rgb(229, 233, 240);
	}
</style>
