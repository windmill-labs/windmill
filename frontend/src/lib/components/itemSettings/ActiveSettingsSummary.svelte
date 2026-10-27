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
	import { ArrowRight } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import type { ScriptSettingsBadge } from '$lib/components/scriptSettings'

	// The settings turned on in the other tabs, at the top of the Metadata tab, where the tab icons
	// alone are easy to miss.

	interface Props {
		groups: { tab: string; label: string; badges: ScriptSettingsBadge[] }[]
		/** Opens `tab`, on the row whose `data-setting` is `setting` when one is given. */
		onopen: (tab: string, setting?: string) => void
	}

	let { groups, onopen }: Props = $props()

	let active = $derived(groups.filter((g) => g.badges.length > 0))
</script>

{#if active.length > 0}
	<div
		class="grid grid-cols-[auto_1fr] items-center gap-x-2 gap-y-1 rounded-md border border-light px-2 py-1.5"
	>
		{#each active as group (group.tab)}
			<Button
				variant="subtle"
				unifiedSize="xs"
				endIcon={{ icon: ArrowRight }}
				wrapperClasses="justify-self-start"
				onclick={() => onopen(group.tab)}
			>
				{group.label}
			</Button>
			<div class="flex flex-row flex-wrap items-center gap-1">
				{#each group.badges as badge (badge.key)}
					<Badge
						small
						clickable
						icon={{ icon: badge.icon, position: 'left' }}
						title="{badge.label}: {badge.detail}"
						onclick={() => onopen(group.tab, badge.setting)}
					>
						{badge.short}
					</Badge>
				{/each}
			</div>
		{/each}
	</div>
{/if}
