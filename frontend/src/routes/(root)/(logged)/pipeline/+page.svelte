<script lang="ts">
	import { userStore } from '$lib/stores'
	import Button from '$lib/components/common/button/Button.svelte'
	import PipelinePickerModal from '$lib/components/assets/AssetGraph/PipelinePickerModal.svelte'
	import { ArrowRight, NetworkIcon } from 'lucide-svelte'

	// Modal is open by default on landing; the editor shell stays empty
	// behind it until the user picks or creates a folder.
	let pickerOpen = $state(true)
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

	<PipelinePickerModal bind:open={pickerOpen} />
{/if}
