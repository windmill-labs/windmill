<script lang="ts">
	import type ShareModal from '$lib/components/ShareModal.svelte'
	import ScriptRow from './ScriptRow.svelte'

	export let items: Array<any>
	export let shareModal: ShareModal
</script>

<div class="flex flex-col">
	<div class="overflow-hidden rounded-md border">
		<table class="min-w-full divide-y divide-gray-300">
			<tbody class="divide-y divide-gray-200 bg-white">
				{#each items as item (item.type + item.path)}
					<ScriptRow
						starred={item.starred}
						marked={item.marked}
						on:change
						script={item}
						{shareModal}
					/>
				{/each}
			</tbody>
		</table>
	</div>
</div>
