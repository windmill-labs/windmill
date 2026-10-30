<script module lang="ts">
	/** Scrolls to a setting's row and flashes it, once its tab has rendered. */
	export function revealSetting(root: HTMLElement | undefined, setting: string) {
		const row = root?.querySelector<HTMLElement>(`[data-setting="${setting}"]`)
		if (!row) return
		row.scrollIntoView({ block: 'center', behavior: 'smooth' })
		row.classList.add('setting-highlight')
		setTimeout(() => row.classList.remove('setting-highlight'), 2000)
	}
</script>

<script lang="ts">
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import type { ScriptSettingsBadge } from '$lib/components/scriptSettings'

	// The settings turned on in the Runtime and Permissions tabs, in the settings header so they
	// show whichever tab is open.

	interface Props {
		groups: { tab: string; badges: ScriptSettingsBadge[] }[]
		/** Opens `tab` on the row whose `data-setting` is `setting`. */
		onopen: (tab: string, setting: string) => void
		/** Chips shown before the rest collapse into a "+n" popover. */
		max?: number
	}

	let { groups, onopen, max = 4 }: Props = $props()

	let chips = $derived(groups.flatMap((g) => g.badges.map((badge) => ({ tab: g.tab, badge }))))
	let shown = $derived(chips.slice(0, max))
	let rest = $derived(chips.slice(max))
</script>

{#snippet chip({ tab, badge }: { tab: string; badge: ScriptSettingsBadge })}
	<Badge
		small
		clickable
		class="whitespace-nowrap"
		icon={{ icon: badge.icon, position: 'left' }}
		title="{badge.label}: {badge.detail}"
		onclick={() => onopen(tab, badge.setting)}
	>
		{badge.short}
	</Badge>
{/snippet}

{#if chips.length > 0}
	<div class="flex flex-row flex-nowrap items-center gap-1">
		{#each shown as c (c.badge.key)}
			{@render chip(c)}
		{/each}
		{#if rest.length > 0}
			<Popover
				floatingConfig={{ strategy: 'absolute', placement: 'bottom-start' }}
				contentClasses="border border-light rounded-lg shadow-lg p-2 surface-tertiary max-w-xs"
				openOnHover
				debounceDelay={150}
			>
				{#snippet trigger()}
					<Badge small clickable>+{rest.length}</Badge>
				{/snippet}
				{#snippet content()}
					<div class="flex flex-wrap gap-1">
						{#each rest as c (c.badge.key)}
							{@render chip(c)}
						{/each}
					</div>
				{/snippet}
			</Popover>
		{/if}
	</div>
{/if}
