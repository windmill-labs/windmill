<script lang="ts">
	import { PanelRight } from 'lucide-svelte'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import { runToolDisplayAction } from './createdResourceActions.svelte'
	import { openItemPreviewAction, type PreviewCardKind } from './shared'

	interface Props {
		card: { kind: PreviewCardKind; path: string }
	}

	let { card }: Props = $props()

	// RowIcon has no 'pipeline' kind — a pipeline is a folder graph, shown with the
	// data-pipeline icon.
	const iconKind = $derived(card.kind === 'pipeline' ? 'data_pipeline' : card.kind)
	const kindLabel = $derived(
		card.kind === 'raw_app'
			? 'App'
			: card.kind === 'pipeline'
				? 'Pipeline'
				: card.kind.charAt(0).toUpperCase() + card.kind.slice(1)
	)

	let opening = $state(false)
	async function open() {
		if (opening) return
		opening = true
		try {
			await runToolDisplayAction(openItemPreviewAction(card.kind, card.path))
		} finally {
			opening = false
		}
	}
</script>

<button
	type="button"
	onclick={open}
	disabled={opening}
	title="Open {kindLabel.toLowerCase()} preview: {card.path}"
	class="group my-1 flex w-full items-center gap-2 rounded-md border border-light bg-surface px-2 py-1.5 text-left transition-colors hover:bg-surface-hover disabled:opacity-60"
>
	<span class="inline-flex shrink-0">
		<RowIcon kind={iconKind} size={14} />
	</span>
	<span class="min-w-0 flex-1">
		<span class="block truncate font-mono text-2xs text-primary">{card.path}</span>
		<span class="block text-[10px] uppercase tracking-wide text-tertiary">{kindLabel}</span>
	</span>
	<span
		class="inline-flex shrink-0 items-center gap-1 text-2xs text-tertiary group-hover:text-secondary"
	>
		<PanelRight size={12} />
		<span class="hidden sm:inline">Preview</span>
	</span>
</button>
