<script lang="ts">
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import GitSyncSuccessContent from './GitSyncSuccessContent.svelte'

	interface Props {
		open: boolean
		savedWithoutInit?: boolean
		autoPullOn?: boolean
	}

	let { open = $bindable(false), savedWithoutInit = false, autoPullOn = false }: Props = $props()
</script>

<Modal bind:open title="Git Sync Connection Saved" class="sm:max-w-4xl" cancelText="Close">
	<GitSyncSuccessContent {savedWithoutInit} {autoPullOn} />
</Modal>
