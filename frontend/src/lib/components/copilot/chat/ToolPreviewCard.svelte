<script lang="ts">
	import { PanelRight } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import RowIcon from '$lib/components/common/table/RowIcon.svelte'
	import { runToolDisplayAction } from './createdResourceActions.svelte'
	import { openItemPreviewAction, type PreviewCardKind } from './shared'

	interface Props {
		card: { kind: PreviewCardKind; path: string }
	}

	let { card }: Props = $props()

	const kindLabel = $derived(card.kind === 'raw_app' ? 'app' : card.kind)

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

<Button
	variant="default"
	unifiedSize="2xs"
	disabled={opening}
	title="Open {kindLabel} preview: {card.path}"
	onClick={open}
	startIcon={{ icon: RowIcon, props: { kind: card.kind, size: 12 } }}
	endIcon={{ icon: PanelRight }}
	wrapperClasses="shrink-0"
>
	Preview
</Button>
