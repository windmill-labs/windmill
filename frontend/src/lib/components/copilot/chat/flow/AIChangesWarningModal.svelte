<script lang="ts">
	import { Button } from '$lib/components/common'
	import Modal from '../../../common/modal/Modal.svelte'

	let {
		open = $bindable(false),
		onConfirm
	}: {
		open: boolean
		onConfirm: () => void
	} = $props()

	function handleConfirm() {
		open = false
		onConfirm()
	}
</script>

<Modal bind:open title="AI Changes Will Be Lost">
	<div class="py-4">
		<p class="text-gray-700 dark:text-gray-300">
			You have pending AI changes that will be rejected when saving. Do you want to continue?
		</p>
	</div>
	{#snippet actions()}
		<div>
			<div class="flex gap-2 justify-end">
				<Button variant="contained" color="red" on:click={handleConfirm}>
					Reject changes and save
				</Button>
			</div>
		</div>
	{/snippet}
</Modal>
