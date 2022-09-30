<script lang="ts">
	import ConfirmationModal from './ConfirmationModal.svelte'
	import { beforeNavigate, goto } from '$app/navigation'

	let navigationState: { from: URL; to: URL | null; cancel: () => void } | undefined = undefined
	$: open = Boolean(navigationState)

	beforeNavigate((newNavigationState) => {
		navigationState = newNavigationState
	})
</script>

<ConfirmationModal
	{open}
	title="Un"
	confirmationText="Discard changes"
	on:canceled={() => {
		if (navigationState) {
			navigationState.cancel()
		}
		navigationState = undefined
	}}
	on:confirmed={() => {
		open = false
		if (navigationState?.to) {
			goto(navigationState.to)
		}
		navigationState = undefined
	}}
>
	<div class="flex flex-col w-full space-y-4">
		<span>Are you sure you want to remove this step?</span>
	</div>
</ConfirmationModal>
