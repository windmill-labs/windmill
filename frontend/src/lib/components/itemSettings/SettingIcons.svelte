<script lang="ts">
	import Popover from '$lib/components/Popover.svelte'
	import type { ScriptSettingsBadge } from '$lib/components/scriptSettings'

	interface Props {
		badges: ScriptSettingsBadge[]
	}

	let { badges }: Props = $props()
</script>

<!-- The configured settings of a tab, drawn on the tab the way the step "Run settings" tab
     draws its own. -->
{#if badges.length > 0}
	<div class="flex flex-row items-center gap-1 text-secondary">
		{#each badges as badge (badge.key)}
			{@const Icon = badge.icon}
			<Popover notClickable>
				<Icon size={12} aria-hidden="true" />
				<span class="sr-only">{badge.label}: {badge.detail}</span>
				{#snippet text()}
					<span class="font-semibold">{badge.label}</span> · {badge.detail}
				{/snippet}
			</Popover>
		{/each}
	</div>
{/if}
