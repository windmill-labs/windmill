<script lang="ts">
	import ConfirmationModal from './ConfirmationModal.svelte'
	import { beforeNavigate, goto } from '$app/navigation'
	import { onDestroy } from 'svelte/internal'
	import { dirtyStore } from './dirtyStore'

	let navigationState: { from: URL | undefined; to: URL | null; cancel: () => void } | undefined =
		undefined
	$: open = Boolean(navigationState)

	beforeNavigate((newNavigationState) => {
		if (
			!navigationState &&
			$dirtyStore &&
			newNavigationState.to &&
			newNavigationState.to.url.pathname !== newNavigationState.from?.url.pathname
		) {
			navigationState = {
				to: newNavigationState.to.url,
				from: newNavigationState.from?.url,
				cancel: newNavigationState.cancel
			}
			newNavigationState.cancel()
		}
	})

	onDestroy(() => {
		$dirtyStore = false
	})
</script>

<ConfirmationModal
	{open}
	title="Unsaved changes detected"
	confirmationText="Discard changes"
	on:canceled={() => {
		if (navigationState) {
			navigationState.cancel()
		}
		navigationState = undefined
	}}
	on:confirmed={() => {
		if (navigationState?.to) {
			goto(navigationState.to)
		}
		$dirtyStore = false
		navigationState = undefined
	}}
>
	<div class="flex flex-col w-full space-y-4">
		<span>
			Are you sure you want to discard change you have made? (A draft has been temporarily and
			locally saved)
		</span>
	</div>
</ConfirmationModal>
