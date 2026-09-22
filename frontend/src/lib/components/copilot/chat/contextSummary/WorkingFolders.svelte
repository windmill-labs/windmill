<script lang="ts">
	import { Button } from '$lib/components/common'
	import type { FolderAccessState } from './folderAccessState.svelte'

	let {
		state: accessState,
		onChange,
		disabled = false
	}: {
		state: FolderAccessState
		onChange: () => void
		disabled?: boolean
	} = $props()
	let selectedNames = $derived(accessState.selectedScopes.map((scope) => scope.name).join(', '))
	let selectionSummary = $derived(
		accessState.selectedScopes.length === accessState.scopes.length
			? 'All folders, personal and workspace-wide context'
			: accessState.selectedScopes.length === 0
				? 'No folders selected'
				: accessState.selectedScopes
						.slice(0, 2)
						.map((scope) => scope.name)
						.join(', ') +
					(accessState.selectedScopes.length > 2
						? ` +${accessState.selectedScopes.length - 2}`
						: '')
	)
</script>

<div class="flex min-w-0 shrink-0 flex-col gap-2">
	<div class="flex items-center justify-between gap-3">
		<div class="min-w-0">
			<p class="text-xs text-primary">Working folders</p>
			<p class="mt-1 truncate text-xs text-secondary" title={selectedNames || selectionSummary}
				>{selectionSummary}</p
			>
		</div>
		<Button variant="subtle" unifiedSize="sm" {disabled} onclick={onChange}>Change</Button>
	</div>
</div>
