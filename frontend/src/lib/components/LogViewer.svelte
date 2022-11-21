<script lang="ts">
	export let content: string | undefined
	export let isLoading: boolean

	let scroll = true
	let div: HTMLElement | null = null

	$: if (content != undefined) {
		isLoading = false
	}

	$: content && scrollToBottom()

	export function scrollToBottom() {
		scroll && setTimeout(() => div?.scroll({ top: div?.scrollHeight, behavior: 'smooth' }), 100)
	}
</script>

<div class="relative w-full h-full">
	<div bind:this={div} class="w-full h-full overflow-auto bg-gray-50 relative">
		<div
			class="sticky top-0 right-0 w-full flex flex-row-reverse justify-between text-gray-500 text-sm bg-gray-50/20"
		>
			<div class="p-2 text-xs flex gap-2 items-center">
				Auto scroll
				<input type="checkbox" bind:checked={scroll} />
			</div>
		</div>
		<pre class="whitespace-pre-wrap break-words bg-gray-50 text-xs w-full p-2"
			>{#if content}{content}{:else if isLoading}Waiting for job to start...{:else}No logs are available yet{/if}</pre
		>
	</div>
</div>
