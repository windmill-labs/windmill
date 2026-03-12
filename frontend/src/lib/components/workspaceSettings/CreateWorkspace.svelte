<script lang="ts">
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import CreateWorkspaceInner from './CreateWorkspaceInner.svelte'

	interface Props {
		isFork?: boolean
	}

	let { isFork = false }: Props = $props()
</script>

<CenteredModal title="{isFork ? 'Forking' : 'New'} Workspace" centerVertically={false}>
	<CreateWorkspaceInner {isFork} />
</CenteredModal>
