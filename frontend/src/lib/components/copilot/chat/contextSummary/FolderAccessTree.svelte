<script lang="ts">
	import { Button } from '$lib/components/common'
	import Checkbox from '$lib/components/common/checkbox/Checkbox.svelte'
	import { Building, ChevronRight, Folder, UserRound } from 'lucide-svelte'
	import AccessManifest from './AccessManifest.svelte'
	import { scopeRecap, type AccessScope } from './folderAccess'
	import type { FolderAccessState } from './folderAccessState.svelte'

	let { state, title = 'Context' }: { state: FolderAccessState; title?: string } = $props()

	function scopeIcon(scope: AccessScope) {
		if (scope.kind === 'personal') return UserRound
		if (scope.kind === 'workspace') return Building
		return Folder
	}
</script>

<div class="flex flex-col w-full min-w-0">
	<div class="flex items-center gap-1 px-2 pb-1">
		<span class="text-2xs text-secondary grow min-w-0">{title}</span>
		<div class="flex items-center gap-1">
			<Button
				variant="subtle"
				unifiedSize="2xs"
				disabled={state.selectedScopes.length === state.scopes.length}
				onclick={() => state.setAll(true)}>All</Button
			>
			<Button
				variant="subtle"
				unifiedSize="2xs"
				disabled={state.selectedScopes.length === 0}
				onclick={() => state.setAll(false)}>None</Button
			>
		</div>
	</div>

	<div class="flex flex-col">
		{#each state.scopes as scope (scope.id)}
			{@const Icon = scopeIcon(scope)}
			{@const recap = scopeRecap(scope)}
			<div
				class={state.isSelected(scope.id) ? 'transition-opacity' : 'opacity-60 transition-opacity'}
			>
				<div
					class="grid grid-cols-[minmax(0,1fr)_auto] items-center gap-2 px-2 py-0.5 min-w-0 rounded-md hover:bg-surface-hover"
				>
					<button
						type="button"
						class="flex items-center gap-2 text-left min-w-0 w-full rounded-md py-0.5 font-normal"
						aria-expanded={state.expanded[scope.id] ?? false}
						onclick={() => state.toggleExpanded(scope.id)}
					>
						<ChevronRight
							size={12}
							class="shrink-0 text-hint transition-transform {state.expanded[scope.id]
								? 'rotate-90'
								: ''}"
						/>
						<Icon
							size={12}
							class="shrink-0 {state.isSelected(scope.id) ? 'text-tertiary' : 'text-hint'}"
						/>
						<span
							title={scope.name}
							class="text-xs font-normal truncate {state.isSelected(scope.id)
								? 'text-emphasis'
								: 'text-secondary'}">{scope.name}</span
						>
						<span
							title={recap}
							class="ml-auto min-w-0 max-w-[65%] truncate text-right text-2xs font-normal text-secondary"
							>{recap}</span
						>
					</button>
					<label
						aria-label={scope.name}
						class="flex items-center justify-end shrink-0 px-2 py-0.5 cursor-pointer"
					>
						<Checkbox
							checked={state.isSelected(scope.id)}
							onClick={(event) => state.setSelected(scope.id, event.currentTarget.checked)}
						/>
					</label>
				</div>
				{#if state.expanded[scope.id]}
					<div class="pl-8 pr-12 pb-1">
						<AccessManifest {scope} dense />
					</div>
				{/if}
			</div>
		{/each}
	</div>
</div>
