<script lang="ts">
	import { userStore } from '$lib/stores'
	import Button from '$lib/components/common/button/Button.svelte'
	import PipelinePickerModal from '$lib/components/assets/AssetGraph/PipelinePickerModal.svelte'
	import PipelineAlphaAckModal from '$lib/components/assets/AssetGraph/PipelineAlphaAckModal.svelte'
	import { ArrowRight, NetworkIcon } from 'lucide-svelte'
	import { onMount } from 'svelte'

	const ACK_STORAGE_KEY = 'pipeline-alpha-ack'

	// Gate the picker behind a one-time alpha acknowledgement. We read
	// localStorage in onMount (not at module scope) so SSR doesn't blow up.
	let ackOpen = $state(false)
	let pickerOpen = $state(false)

	onMount(() => {
		const acked =
			typeof localStorage !== 'undefined' && localStorage.getItem(ACK_STORAGE_KEY) === 'true'
		if (acked) {
			pickerOpen = true
		} else {
			ackOpen = true
		}
	})

	function handleAck() {
		try {
			localStorage.setItem(ACK_STORAGE_KEY, 'true')
		} catch {
			// Storage may be unavailable (private mode, quota); the ack still
			// flows through for this visit, the user just sees the modal again next time.
		}
		ackOpen = false
		pickerOpen = true
	}
</script>

<svelte:head>
	<title>Pipeline editor — Windmill</title>
</svelte:head>

{#if $userStore?.operator}
	<div class="p-8 text-tertiary">Page not available for operators.</div>
{:else}
	<div class="flex flex-col h-full">
		<div
			class="border-b flex flex-row justify-between gap-2 px-2 py-1 items-center min-h-10 shrink-0 whitespace-nowrap"
		>
			<div class="flex flex-row items-center gap-2">
				<NetworkIcon size={16} class="text-tertiary shrink-0" />
				<h1 class="text-sm font-semibold">Pipeline editor</h1>
				<span class="text-xs text-tertiary">· no folder selected</span>
			</div>
		</div>

		<div class="flex-1 min-h-0 relative bg-surface-secondary">
			<div class="absolute inset-0 flex flex-col items-center justify-center gap-3 text-tertiary">
				<NetworkIcon size={32} class="opacity-50" />
				<span class="text-sm">Pick a folder to open its pipeline.</span>
				<Button
					variant="accent"
					unifiedSize="sm"
					onclick={() => (pickerOpen = true)}
					startIcon={{ icon: ArrowRight }}
				>
					Choose a folder
				</Button>
			</div>
		</div>
	</div>

	<PipelineAlphaAckModal bind:open={ackOpen} onAck={handleAck} />
	<PipelinePickerModal bind:open={pickerOpen} />
{/if}
