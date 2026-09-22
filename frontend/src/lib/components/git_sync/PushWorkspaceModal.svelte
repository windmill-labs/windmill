<script lang="ts">
	import Modal from '$lib/components/common/modal/Modal.svelte'
	import { Button } from '$lib/components/common'
	import { Loader2 } from 'lucide-svelte'
	import GitPushPreview from './GitPushPreview.svelte'
	import type { SettingsObject } from '$lib/git-sync'

	interface Props {
		open: boolean
		gitRepoResourcePath: string
		uiState: SettingsObject
		onSuccess?: () => void
		isNewConnection?: boolean
		onSaveWithoutInit?: () => void
	}

	let {
		open = $bindable(false),
		gitRepoResourcePath,
		uiState,
		onSuccess,
		isNewConnection = false,
		onSaveWithoutInit
	}: Props = $props()

	let push: GitPushPreview | undefined = $state()
	const status = $derived(push?.status() ?? { previewing: true, applying: false, canApply: false })
</script>

<Modal bind:open title="Push Workspace to Git Repository" class="sm:max-w-4xl">
	{#if open}
		{#key gitRepoResourcePath}
			<GitPushPreview bind:this={push} {gitRepoResourcePath} {uiState} {onSuccess} />
		{/key}
	{/if}

	<!-- Reversed row: whatever is rendered first lands rightmost, so the primary action ends up at
	     the far right and Cancel on the left. -->
	{#snippet actions()}
		<Button
			size="sm"
			variant="accent"
			onclick={() => push?.apply()}
			disabled={!status.canApply}
			startIcon={{
				icon: status.applying ? Loader2 : undefined,
				classes: status.applying ? 'animate-spin' : ''
			}}
		>
			{status.applying
				? isNewConnection
					? 'Initializing...'
					: 'Pushing...'
				: isNewConnection
					? 'Initialize repo and save connection'
					: 'Push to repository'}
		</Button>
		{#if isNewConnection && onSaveWithoutInit}
			<Button
				size="sm"
				variant="default"
				onclick={onSaveWithoutInit}
				disabled={status.applying || status.previewing}
			>
				Save without initializing repo
			</Button>
		{/if}
	{/snippet}
</Modal>
