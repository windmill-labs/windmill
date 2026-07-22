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
		card.kind === 'raw_app' ? 'app' : card.kind === 'pipeline' ? 'pipeline' : card.kind
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
	title="Open {kindLabel} preview: {card.path}"
	class="group shrink-0 inline-flex items-center gap-1.5 rounded-md border border-light bg-surface pl-1.5 pr-2 py-1 transition-colors hover:bg-surface-hover disabled:opacity-60"
>
	<span class="inline-flex shrink-0">
		<RowIcon kind={iconKind} size={12} />
	</span>
	<span class="inline-flex items-center gap-1 text-2xs text-tertiary group-hover:text-secondary">
		Preview <PanelRight size={11} />
	</span>
</button>
