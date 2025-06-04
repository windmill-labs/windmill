<script lang="ts">
	import { base } from '$lib/base'
	import { copilotInfo } from '$lib/stores'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import DarkPopover from '$lib/components/Popover.svelte'
	import { ExternalLink } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import { getModifierKey } from '$lib/utils'
	import WindmillAiIcon from '$lib/components/icons/WindmillAiIcon.svelte'

	let { openPanel }: { openPanel: () => void } = $props()
</script>

{#snippet button(onClick: () => void)}
	<Button
		color="light"
		variant="border"
		size="xs"
		on:click={onClick}
		startIcon={{ icon: WindmillAiIcon, props: { className: 'm-[1px]' } }}
		iconOnly
		btnClasses="!text-violet-800 dark:!text-violet-400 border border-gray-200 dark:border-gray-600 bg-surface"
	>
		AI Panel
	</Button>
{/snippet}

{#if $copilotInfo.enabled}
	<DarkPopover>
		<svelte:fragment slot="text">
			<div class="flex flex-row gap-1">
				Show the AI Panel.

				<div class="flex flex-row items-center !text-md opacity-60 gap-0 font-normal">
					{getModifierKey()}L
				</div>
			</div>
		</svelte:fragment>
		{@render button(openPanel)}
	</DarkPopover>
{:else}
	<Popover placement="bottom">
		<svelte:fragment slot="trigger">
			{@render button(() => {})}
		</svelte:fragment>
		<svelte:fragment slot="content">
			<div class="block text-primary p-4">
				<p class="text-sm"
					>Enable Windmill AI in the <a
						href="{base}/workspace_settings?tab=ai"
						target="_blank"
						class="inline-flex flex-row items-center gap-1"
						>workspace settings <ExternalLink size={16} /></a
					></p
				>
			</div>
		</svelte:fragment>
	</Popover>
{/if}
