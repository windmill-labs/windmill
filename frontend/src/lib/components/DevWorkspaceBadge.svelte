<!--
@component
The badge naming a dev workspace's environment, wherever one is shown.

A fork that has a colour wears it: the badge is filled with that hue at the lightest shade white
still reads on (`forkAccentStyle`), so a yellow workspace says "dev" in dark yellow rather than in
a blue that belongs to no workspace in particular. A fork with no colour keeps whatever its host
gave it — `fallbackColor` and `fallbackClass`, which is what every dev workspace looked like
before a colour could be picked.
-->
<script lang="ts">
	import { Badge } from '$lib/components/common'
	import type { BadgeColor } from '$lib/components/common/badge/model'
	import { devBadgeText } from '$lib/utils/devWorkspaceLabel'
	import { forkAccentStyle } from '$lib/utils/forkColor'
	import { twMerge } from 'tailwind-merge'

	interface Props {
		/** The workspace's `dev_workspace_label`; spelled the way `devBadgeText` spells it. Nullable
		 *  because that is what the workspace rows carry. */
		label?: string | null
		/** The fork's own colour. Without one — or with an unparsable one — the badge falls back. */
		color?: string | null
		/** The badge's colour when the fork has none of its own. */
		fallbackColor?: BadgeColor
		/** Extra classes for that same case, for a host that tuned the uncoloured badge. */
		fallbackClass?: string
		/** Size and spacing, which the row around the badge decides either way. */
		class?: string
	}

	let {
		label,
		color,
		fallbackColor = 'dark-blue',
		fallbackClass,
		class: className
	}: Props = $props()

	// Set on the badge itself rather than read from an ancestor: a host that paints a coloured chip
	// around it already defines these, and defining them again costs nothing and makes the badge
	// work the same in a row that has no chip.
	const accent = $derived(forkAccentStyle(color))
</script>

<Badge
	color={fallbackColor}
	small
	style={accent}
	class={twMerge(
		accent
			? 'bg-[color:var(--fork-accent-badge)] dark:bg-[color:var(--fork-accent-badge)] text-white dark:text-white'
			: fallbackClass,
		className
	)}>{devBadgeText(label)}</Badge
>
