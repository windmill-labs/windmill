<script lang="ts">
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { createEventDispatcher } from 'svelte'

	export let open: boolean
	const dispatch = createEventDispatcher()
</script>

<ConfirmationModal
	{open}
	title="Remove step"
	confirmationText="Remove"
	on:canceled={() => {
		open = false
		dispatch('canceled')
	}}
	on:confirmed={() => {
		open = false
		dispatch('confirmed')
	}}
>
	<div class="flex flex-col w-full space-y-4">
		<span>Are you sure you want to remove this step?</span>
		<Alert type="info" title="Bypass confirmation">
			<div>
				You can press
				<kbd color="dark-gray">SHIFT</kbd>
				while removing a step to bypass confirmation.
			</div>
		</Alert>
	</div>
</ConfirmationModal>
