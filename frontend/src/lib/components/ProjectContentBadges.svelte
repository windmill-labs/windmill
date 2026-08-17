<script lang="ts" module>
	export interface ProjectContentCounts {
		apps: number
		flows: number
		scripts: number
		resources: number
		/** Only the import step counts these. */
		triggers?: number
		migrations?: number
	}
</script>

<script lang="ts">
	import { Code2, Database, LayoutDashboard, Table2, Zap } from 'lucide-svelte'
	import BarsStaggered from '$lib/components/icons/BarsStaggered.svelte'
	import Badge from '$lib/components/common/badge/Badge.svelte'

	// What a project contains, as one row of badges. Shared by the wizard's project
	// card and its import step so the same project never gets counted two ways.
	interface Props {
		counts: ProjectContentCounts
	}

	let { counts }: Props = $props()

	// Transparent badges throughout: six kinds in six colours turned a summary into
	// a paint chart. The icon carries the kind, the colour carries nothing.
	//
	// Zero counts are dropped rather than shown: a project with no apps should read
	// as "no apps", not as a "0 apps" chip the eye has to discount.
	const shown = $derived(
		[
			{ label: 'app', count: counts.apps, icon: LayoutDashboard },
			{ label: 'flow', count: counts.flows, icon: BarsStaggered },
			{ label: 'script', count: counts.scripts, icon: Code2 },
			{ label: 'resource', count: counts.resources, icon: Database },
			{ label: 'trigger', count: counts.triggers ?? 0, icon: Zap },
			{ label: 'data table migration', count: counts.migrations ?? 0, icon: Table2 }
		].filter((c) => c.count > 0)
	)
</script>

<div class="flex flex-wrap items-center gap-1.5">
	{#each shown as c (c.label)}
		<Badge color="transparent" small icon={{ icon: c.icon, position: 'left' }}>
			{c.count}
			{c.label}{c.count === 1 ? '' : 's'}
		</Badge>
	{/each}
</div>
