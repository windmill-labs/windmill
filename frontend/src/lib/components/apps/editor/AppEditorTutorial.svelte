<script lang="ts">
	import AppTutorials from '../../AppTutorials.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'

	let appTutorials: AppTutorials | undefined = $state(undefined)
	let targetTutorial: string | undefined = $state(undefined)

	export function runTutorialById(id: string, options?: { skipStepsCount?: number }) {
		appTutorials?.runTutorialById(id, options)
	}
</script>

<AppTutorials
	bind:this={appTutorials}
	on:reload
	on:error={(event: CustomEvent<{ detail: string }>) => {
		targetTutorial = event.detail.detail
	}}
/>

<ConfirmationModal
	open={targetTutorial !== undefined}
	title="Tutorial error"
	confirmationText="Open new tab"
	on:canceled={() => {
		targetTutorial = undefined
	}}
	on:confirmed={async () => {
		window.open(`/apps/add?tutorial=${targetTutorial}`, '_blank')
	}}
>
	<div class="flex flex-col w-full space-y-4">
		<span> This tutorial can only be run on a new app.</span>
	</div>
</ConfirmationModal>
