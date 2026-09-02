<script lang="ts">
	import { ArrowUpRight } from 'lucide-svelte'
	import { Skeleton } from '$lib/components/common'
	import HubProjectCard from './HubProjectCard.svelte'
	import { fetchPopularHubProjects, HUB_BASE_URL, type HubProject } from './hubProjects'
	import { sendUserToast } from '$lib/toast'

	let projects = $state<HubProject[] | undefined>(undefined)

	$effect(() => {
		fetchPopularHubProjects()
			.then((p) => (projects = p))
			.catch((e) => {
				console.error('Could not load Hub projects', e)
				projects = []
			})
	})

	function importProject(project: HubProject) {
		// The import wizard is the next piece; until it exists, say so rather than
		// leaving the primary action of this screen inert.
		sendUserToast(`Import wizard for “${project.name}” is not wired up yet`)
	}
</script>

<div class="flex flex-col gap-4 py-6">
	<div class="flex flex-row items-end justify-between gap-4 flex-wrap">
		<div>
			<h2 class="text-lg font-semibold text-emphasis">Start from a project</h2>
			<p class="text-xs text-secondary mt-1">
				Import a ready-made project from the Hub — scripts, flows and apps you can run now and edit
				as your own.
			</p>
		</div>
		<a
			href="{HUB_BASE_URL}/projects"
			target="_blank"
			rel="noreferrer"
			class="text-xs text-secondary hover:text-emphasis inline-flex items-center gap-1 whitespace-nowrap"
		>
			Browse all projects<ArrowUpRight size={14} />
		</a>
	</div>

	{#if projects == undefined}
		<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
			{#each new Array(6) as _, i (i)}
				<Skeleton layout={[[6], 0.5, [2]]} />
			{/each}
		</div>
	{:else if projects.length > 0}
		<div class="grid grid-cols-1 sm:grid-cols-2 lg:grid-cols-3 gap-3">
			{#each projects as project (project.slug)}
				<HubProjectCard {project} onImport={importProject} />
			{/each}
		</div>
	{/if}

	<p class="text-xs text-hint">
		Or start from scratch with the <span class="text-secondary">New</span> button above.
	</p>
</div>
