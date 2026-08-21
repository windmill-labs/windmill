<script lang="ts" module>
	export interface ImportProjectSummary {
		slug: string
		name: string
		summary: string
		author: string
		/** Integration slugs, for the chips and the fallback icons. */
		apps: string[]
		/** The project's uploaded logo, when it has one. */
		logoUrl?: string
		/** Integration slugs to draw, most representative first. */
		iconApps: string[]
		counts: { apps: number; flows: number; scripts: number; resources: number }
	}
</script>

<script lang="ts">
	import { ExternalLink, LayoutGrid } from 'lucide-svelte'
	import ProjectContentBadges from '$lib/components/ProjectContentBadges.svelte'
	import { hubAppIcon } from '$lib/hubProject'

	interface Props {
		project: ImportProjectSummary
		/** Where the project is coming from, shown next to the author. */
		hubHost?: string
	}

	let { project, hubHost = 'hub.windmill.dev' }: Props = $props()

	// Protocol-relative on purpose: the same hub is https in production and plain
	// http when it's a local dev instance, and this way the link follows whichever
	// scheme the page itself was served over.
	const hubProjectUrl = $derived(`//${hubHost}/projects/${project.slug}`)

	// Resolved locally rather than fetched: these are Windmill's own bundled icons, so the
	// card draws them synchronously instead of waiting on the hub — and keeps working on a
	// hub that refuses uncredentialed reads.
	const icons = $derived(
		project.iconApps
			.slice(0, 4)
			.map(hubAppIcon)
			.filter((c): c is NonNullable<typeof c> => !!c)
	)

	// The icon row shows the integrations the tile is not already showing: with an
	// uploaded logo the tile shows none of them, so the row shows them all.
	const restIcons = $derived(project.logoUrl ? icons : icons.slice(1))
</script>

<div class="mb-4 w-full">
	<!-- No border: the subject of the page, not one card among the choices below,
	     which are the things with edges because they are selectable. -->
	<div class="w-full">
		<div class="flex items-start gap-3">
			<!-- The project's own logo when it has one, otherwise the icon of the
			     integration it is filed under, otherwise a neutral placeholder. -->
			<div class="flex h-12 w-12 shrink-0 items-center justify-center">
				{#if project.logoUrl}
					<img src={project.logoUrl} alt="" class="max-h-10 max-w-10 object-contain" />
				{:else if icons[0]}
					{@const Icon = icons[0]}
					<span class="inline-flex h-7 w-7 text-primary [&>svg]:h-full [&>svg]:w-full">
						<Icon size={28} />
					</span>
				{:else}
					<LayoutGrid size={22} class="text-secondary" />
				{/if}
			</div>

			<div class="min-w-0 flex-1">
				<!-- The title carries the link back to the hub page, so the card can state
			     where the project comes from without spending a line on the URL. The
			     arrow only appears on hover — at rest the title reads as a title. -->
				<a
					href={hubProjectUrl}
					target="_blank"
					rel="noopener noreferrer"
					class="group inline-flex max-w-full items-center gap-1 text-xs font-semibold text-emphasis hover:underline"
				>
					<span class="truncate">{project.name}</span>
					<ExternalLink
						size={13}
						class="shrink-0 text-tertiary opacity-0 transition group-hover:opacity-100"
					/>
				</a>
				<p class="mt-0.5 line-clamp-2 text-xs text-secondary">{project.summary}</p>
				<p class="mt-1 text-xs text-tertiary">
					by <span class="font-medium text-secondary">{project.author}</span>
					· <span class="font-mono">{project.slug}</span>
				</p>

				<!-- What the import will create, aligned under the title rather than in a
			     band of its own: the counts belong to the project above them. -->
				<div class="mt-3">
					<ProjectContentBadges counts={project.counts} />
				</div>
			</div>

			<!-- The integrations, minus whichever one is already standing in as the logo. -->
			{#if restIcons.length > 0}
				<div class="flex shrink-0 items-center gap-1.5 pt-0.5">
					{#each restIcons as Icon, i (i)}
						<span class="inline-flex h-4 w-4 text-primary opacity-80 [&>svg]:h-full [&>svg]:w-full">
							<Icon size={16} />
						</span>
					{/each}
				</div>
			{/if}
		</div>
	</div>

	<!-- The connector to what follows lives in the page, not here: what comes next is
	     either the new-workspace offer or the workspace list, and only the page knows
	     which. -->
</div>
