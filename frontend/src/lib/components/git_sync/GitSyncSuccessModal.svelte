<script lang="ts">
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import GitSyncSuccessContent from './GitSyncSuccessContent.svelte'

	interface Props {
		open: boolean
		savedWithoutInit?: boolean
		autoPullOn?: boolean
		explainAutoPullOff?: boolean
		webhookError?: string
	}

	let {
		open = $bindable(false),
		savedWithoutInit = false,
		autoPullOn = false,
		explainAutoPullOff = false,
		webhookError = undefined
	}: Props = $props()
</script>

<Modal bind:open title="Git Sync Connection Saved" class="sm:max-w-4xl" cancelText="Close">
	<GitSyncSuccessContent {savedWithoutInit} {autoPullOn} {explainAutoPullOff} {webhookError} />
</Modal>
