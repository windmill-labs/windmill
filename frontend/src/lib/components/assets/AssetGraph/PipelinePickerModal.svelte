<script lang="ts">
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import PipelineFolderList from './PipelineFolderList.svelte'

	interface Props {
		open: boolean
		// When provided, the current folder is dropped from the existing list
		// so users don't "switch" to the folder they're already on.
		currentFolder?: string | undefined
		// Carried over to the target URL so switching folders from the
		// editor keeps the user in edit mode; the default view mode needs
		// no param.
		mode?: 'view' | 'edit'
	}
	let { open = $bindable(), currentFolder = undefined, mode = 'view' }: Props = $props()
</script>

<Modal bind:open title="Open a pipeline">
	<PipelineFolderList {currentFolder} {mode} onOpen={() => (open = false)} />
</Modal>
