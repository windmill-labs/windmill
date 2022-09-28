<script lang="ts">
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { createEventDispatcher } from 'svelte'

	export let open: boolean
	const dispatch = createEventDispatcher()
</script>

<ConfirmationModal
	{open}
	title="Remove step"
	description="Are you sure you want to remove this step?"
	confirmationText="Remove"
	on:canceled={() => {
		open = false
		dispatch('confirmed')
	}}
	on:confirmed={() => {
		open = false
		dispatch('confirmed')
	}}
/>
