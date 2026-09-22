<!--
@component
The bar every route wears, above the page content and beside the sidebar: the sidebar toggle, the
workspace picker, then whatever the route registered — an item's breadcrumb and summary, or a
section name — and the route's own buttons at the far end.

The row's height matches the sidebar's own header row, so the two read as one band.
-->
<script lang="ts">
	import { Menubar } from '$lib/components/meltComponents'
	import WorkspaceMenu from '$lib/components/sidebar/WorkspaceMenu.svelte'
	import BreadcrumbSegment from '$lib/components/BreadcrumbSegment.svelte'
	import EditorHeader from '$lib/components/EditorHeader.svelte'
	import { pageHeader } from './pageHeaderRegistry.svelte'
	import { kindKey, KIND_LABEL_LOWER } from '$lib/components/workspacePicker'

	interface Props {
		/** The sidebar's show/hide control, placed at the very start of the band. */
		toggle?: import('svelte').Snippet
	}

	let { toggle }: Props = $props()

	const content = $derived(pageHeader.content)
	const item = $derived(content?.item)
	const section = $derived(content?.section)
</script>

<div class="flex items-center gap-1 h-12 px-2 border-b shrink-0 min-w-0">
	{@render toggle?.()}

	<Menubar>
		{#snippet children({ createMenu })}
			<WorkspaceMenu {createMenu} strictWorkspaceSelect={false} />
		{/snippet}
	</Menubar>

	{#if item}
		<EditorHeader
			inline
			kind={item.kind}
			raw_app={item.raw_app}
			savedPath={item.savedPath}
			workspaceId={item.workspaceId}
			onBehalfOfEmail={item.onBehalfOfEmail}
			pathEditable={item.pathEditable ?? true}
			summaryEditable={item.summaryEditable ?? true}
			onNavigate={item.onNavigate}
			bind:path={() => item.path, (v) => item.onPathChange?.(v ?? '')}
			bind:summary={() => item.summary, (v) => item.onSummaryChange?.(v ?? '')}
		/>
	{:else if section}
		<nav aria-label="Breadcrumb" class="contents">
			<BreadcrumbSegment
				label={section.kind ? KIND_LABEL_LOWER[section.kind] : section.label}
				isCurrent
				initialHighlight={section.kind ? kindKey(section.kind) : undefined}
				initialScope={section.kind ? { kind: section.kind } : undefined}
				onPick={() => {}}
			/>
		</nav>
	{/if}

	{#if content?.actions}
		<div class="ml-auto flex items-center gap-2 shrink-0">
			{@render content.actions()}
		</div>
	{/if}
</div>
