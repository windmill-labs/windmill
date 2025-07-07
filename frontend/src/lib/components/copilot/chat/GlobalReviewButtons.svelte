<script lang="ts">
	import Button from '$lib/components/common/button/Button.svelte'

	interface Props {
		onAcceptAll: () => void
		onRejectAll: () => void
	}

	const { onAcceptAll, onRejectAll }: Props = $props()
</script>

<div class="absolute flex flex-row bottom-10 left-1/2 z-10 -translate-x-1/2 bg-surface rounded-md">
	<Button
		class="px-2 py-1 bg-[#a0e6a0] rounded-l text-sm font-semibold text-black flex flex-row items-center gap-2"
		onclick={onAcceptAll}
		shortCut={{ key: '↓' }}>Accept all</Button
	>
	<Button
		class="px-2 py-1 bg-[#e6a0a0] rounded-r text-sm font-semibold text-black flex flex-row items-center gap-2"
		onclick={onRejectAll}
		shortCut={{ key: 'esc', withoutModifier: true }}>Reject all</Button
	>
</div>
