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
	<!-- The setup dialog shows the filters on its own step; opened on its own, this is the only
	     place that says what the filters let through. -->
	<div class="flex flex-col gap-1 text-xs text-secondary mb-4">
		<div><span class="font-semibold text-emphasis">Include paths:</span> {uiState.include_path?.join(', ') || 'None'}</div>
		<div><span class="font-semibold text-emphasis">Exclude paths:</span> {uiState.exclude_path?.join(', ') || 'None'}</div>
		{#if uiState.extra_include_path?.length}
			<div><span class="font-semibold text-emphasis">Extra include paths:</span> {uiState.extra_include_path.join(', ')}</div>
		{/if}
		<div><span class="font-semibold text-emphasis">Included types:</span> {uiState.include_type?.join(', ') || 'None'}</div>
		<p class="mt-2">
			This does not update the git sync settings in <span class="font-mono">wmill.yaml</span>: they
			can only be pulled from the repository, which is their source of truth. To change what is
			pushed, cancel and edit the filters in the workspace settings.
		</p>
	</div>

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
