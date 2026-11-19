<!--
@component
A shared session's preview panel: the artifacts the reader opened, one tab each, framed like a
live session's preview panel. A share carries nothing else a tab could show.
-->
<script lang="ts">
	import { PanelRightClose } from 'lucide-svelte'
	import type { Snippet } from 'svelte'
	import DraggableTabs, { type TabItem } from '$lib/components/common/tabs/DraggableTabs.svelte'
	import ArtifactViewer from '$lib/components/copilot/chat/artifacts/ArtifactViewer.svelte'
	import type { PersistedArtifact } from '$lib/components/copilot/chat/artifacts/artifactsDB'

	interface Props {
		tabs: PersistedArtifact[]
		activeId: string | undefined
		onSelect: (id: string) => void
		onClose: (id: string) => void
		onReorder: (ids: string[]) => void
		onCollapse: () => void
		/** Floats over the strip's right end, where a live session keeps its preview controls. */
		actions?: Snippet
	}

	let { tabs, activeId, onSelect, onClose, onReorder, onCollapse, actions }: Props = $props()

	const items = $derived<TabItem[]>(tabs.map((a) => ({ id: a.id, label: a.name, title: a.name })))
	// Measured so the strip reserves their width and no tab scrolls underneath them.
	let actionsWidth = $state(0)
</script>

<div class="flex min-h-0 flex-1 flex-col p-2 pl-0">
	<div
		class="relative flex min-h-0 flex-1 flex-col overflow-hidden rounded-md border border-light bg-surface"
	>
		<button
			type="button"
			onclick={onCollapse}
			title="Collapse preview"
			aria-label="Collapse preview"
			class="absolute left-1 top-1 z-30 inline-flex h-6 w-6 items-center justify-center rounded text-tertiary hover:bg-surface-hover hover:text-primary"
		>
			<PanelRightClose size={14} />
		</button>
		{#if actions}
			<div
				bind:clientWidth={actionsWidth}
				class="absolute right-1 top-0 z-30 flex h-8 items-center gap-1"
			>
				{@render actions()}
			</div>
		{/if}
		<div class="shrink-0" style="--preview-actions-w: {actions ? actionsWidth + 8 : 8}px">
			<DraggableTabs
				tabs={items}
				activeId={activeId ?? ''}
				{onSelect}
				{onClose}
				onReorder={(order) => onReorder(order.map((t) => t.id))}
				class="session-preview-tab-strip h-8 border-b border-light bg-surface-secondary/50 pl-9 pr-[var(--preview-actions-w)]"
			/>
		</div>
		<!-- One viewer per tab, visibility-toggled as the live panel does, so switching tabs keeps
		     each one's scroll position and source/preview choice. -->
		<div class="relative min-h-0 flex-1">
			{#each tabs as artifact (artifact.id)}
				<div class="absolute inset-0 {artifact.id === activeId ? '' : 'invisible'}">
					<ArtifactViewer {artifact} sharedCopy onPin={() => {}} />
				</div>
			{/each}
		</div>
	</div>
</div>
