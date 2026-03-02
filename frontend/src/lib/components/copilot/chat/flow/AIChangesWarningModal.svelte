<script lang="ts">
	import { Button } from '$lib/components/common'
	import Modal from '../../../common/modal/Modal.svelte'
	import { AlertTriangleIcon } from 'lucide-svelte'

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
	<div slot="title" class="flex items-center gap-2">
		<AlertTriangleIcon size={20} class="text-orange-500" />
		AI Changes Will Be Lost
	</div>
	<div class="py-4">
		<p class="text-gray-700 dark:text-gray-300">
			You have pending AI changes that will be rejected when saving. Do you want to continue?
		</p>
	</div>
	<div slot="actions">
		<div class="flex gap-2 justify-end">
			<Button variant="contained" color="red" on:click={handleConfirm}>
				Reject changes and save
			</Button>
		</div>
	</div>
</Modal>
