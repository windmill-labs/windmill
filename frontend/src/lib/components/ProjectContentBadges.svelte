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

	/** The kinds, in the order a project is read. Shared by the badges and the sentence
	 *  below so the same project can never be counted two ways. */
	function kinds(counts: ProjectContentCounts) {
		return [
			{ label: 'app', count: counts.apps },
			{ label: 'flow', count: counts.flows },
			{ label: 'script', count: counts.scripts },
			{ label: 'resource', count: counts.resources },
			{ label: 'trigger', count: counts.triggers ?? 0 },
			{ label: 'data table migration', count: counts.migrations ?? 0 }
		].filter((c) => c.count > 0)
	}

	/**
	 * The same counts as one line of text, for callers with a row to sit on rather than
	 * a space for chips — the import step names them beside the task that imports them.
	 * Empty when a project has nothing in it, so a caller can drop the whole phrase.
	 */
	export function contentSummary(counts: ProjectContentCounts): string {
		return kinds(counts)
			.map((c) => `${c.count} ${c.label}${c.count === 1 ? '' : 's'}`)
			.join(', ')
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
	const ICONS: Record<string, any> = {
		app: LayoutDashboard,
		flow: BarsStaggered,
		script: Code2,
		resource: Database,
		trigger: Zap,
		'data table migration': Table2
	}
	const shown = $derived(kinds(counts).map((c) => ({ ...c, icon: ICONS[c.label] })))
</script>

<div class="flex flex-wrap items-center gap-1.5">
	{#each shown as c (c.label)}
		<Badge color="transparent" small icon={{ icon: c.icon, position: 'left' }}>
			{c.count}
			{c.label}{c.count === 1 ? '' : 's'}
		</Badge>
	{/each}
</div>
