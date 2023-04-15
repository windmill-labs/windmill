<script lang="ts">
	import { Loader2 } from 'lucide-svelte'
	import { Drawer, DrawerContent } from './common'

	export let content: string | undefined
	export let isLoading: boolean
	export let duration: number | undefined = undefined
	export let mem: number | undefined = undefined
	export let wrapperClass = ''

	let scroll = true
	let div: HTMLElement | null = null

	$: if (content != undefined) {
		isLoading = false
	}

	$: content && scrollToBottom()

	export function scrollToBottom() {
		scroll && setTimeout(() => div?.scroll({ top: div?.scrollHeight, behavior: 'smooth' }), 100)
	}

	let logViewer: Drawer
</script>

<Drawer bind:this={logViewer} size="900px">
	<DrawerContent title="Expanded Logs" on:close={logViewer.closeDrawer}>
		<div>
			<pre class="bg-gray-50 text-xs w-full p-2"
				>{#if content}{content}{:else if isLoading}Waiting for job to start...{:else}No logs are available yet{/if}</pre
			>
		</div>
	</DrawerContent>
</Drawer>

<div class="relative w-full h-full {wrapperClass}">
	<div bind:this={div} class="w-full h-full overflow-auto bg-gray-50 relative">
		<div
			class="sticky top-0 right-0 w-full flex flex-row-reverse justify-between text-gray-500 text-sm bg-gray-50/20"
		>
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
			<span class="absolute text-xs text-gray-500 top-2 left-2">took {duration}ms</span>
		{/if}
		{#if mem}
			<span class="absolute text-xs text-gray-500 top-2 left-36"
				>mem peak: {(mem / 1024).toPrecision(4)}MB</span
			>
		{/if}
		<pre class="whitespace-pre-wrap break-words bg-gray-50 text-xs w-full p-2"
			>{#if content}<span>{content}</span>{:else if !isLoading}<span class="text-gray-600"
					>No logs are available yet</span
				>{/if}</pre
		>
	</div>
</div>
