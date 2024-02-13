<script lang="ts">
	import type { Tweened } from 'svelte/motion'
	import QueuePopover from './QueuePopover.svelte'
	import Popup from '../common/popup/Popup.svelte'

	export let queue_count: Tweened<number> | undefined = undefined
	export let allWorkspaces: boolean = false
</script>

<div class="flex gap-1 relative max-w-36 min-w-[50px] items-baseline">
	<div class="text-xs absolute -top-4 truncate">Jobs waiting for a worker</div>
	<div class="mt-1">{queue_count ? ($queue_count ?? 0).toFixed(0) : '...'}</div>
	<div class="truncate text-2xs text-blue-400">
		{#if queue_count && ($queue_count ?? 0) > 0}
			<Popup>
				<svelte:fragment slot="button">
					<span class="text-2xs truncate">jobs</span>
				</svelte:fragment>
				<QueuePopover {allWorkspaces} />
			</Popup>
		{/if}
	</div>
</div>
