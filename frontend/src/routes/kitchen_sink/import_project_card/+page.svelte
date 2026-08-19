<script lang="ts">
	import DarkModeToggle from '$lib/components/sidebar/DarkModeToggle.svelte'
	import ImportProjectCard, {
		type ImportProjectSummary
	} from '$lib/components/ImportProjectCard.svelte'

	// Bench fixtures. The card itself takes real data from the hub now; these only
	// exist so the layout can be judged without a running hub.
	const MOCK_PROJECT: ImportProjectSummary = {
		slug: 'github-release-dashboard',
		name: 'Github release dashboard',
		summary:
			"Browse a repo's latest releases, format them into a readable digest and post it to your team on a schedule.",
		author: 'Tristan795',
		apps: ['github', 'slack', 'postgresql'],
		iconApps: ['github', 'slack', 'postgresql'],
		hub: 'https://hub.windmill.dev',
		counts: { apps: 1, flows: 10, scripts: 4, resources: 2 }
	}

	// Design bench for the hub import card. No auth, no API — everything on this
	// page is local state, so the card can be tweaked and re-rendered instantly.
	// The card ships inside CenteredModal on /projects/import, whose content box is
	// `max-w-[640px]` with `sm:px-10`: 560px of usable width, which is the frame the
	// card is previewed in below.
	const MODAL_CONTENT_WIDTH = 560

	const PRESETS: { label: string; project: ImportProjectSummary }[] = [
		{
			label: 'Github (real)',
			project: { ...MOCK_PROJECT, counts: { apps: 1, flows: 1, scripts: 1, resources: 1 } }
		},
		{ label: 'Github (padded)', project: MOCK_PROJECT },
		{
			label: 'Big',
			project: {
				slug: 'stripe-billing-suite',
				name: 'Stripe billing suite',
				summary:
					'Sync subscriptions, dunning and invoices between Stripe and your data warehouse, with alerting on failed payments.',
				author: 'windmill-labs',
				apps: ['stripe', 'postgresql', 'slack'],
				iconApps: ['stripe', 'postgresql', 'slack'],
				hub: 'https://hub.windmill.dev',
				counts: { apps: 3, flows: 12, scripts: 34, resources: 6 }
			}
		},
		{
			label: 'Minimal',
			project: {
				slug: 'hello-world',
				name: 'Hello world',
				summary: 'One script, nothing else.',
				author: 'ruben',
				apps: [],
				iconApps: [],
				hub: 'https://hub.windmill.dev',
				counts: { apps: 0, flows: 0, scripts: 1, resources: 0 }
			}
		},
		{
			label: 'Overflowing',
			project: {
				slug: 'enterprise-data-platform-migration-toolkit-v2',
				name: 'Enterprise data platform migration toolkit (v2, incremental)',
				summary:
					'A deliberately long summary used to check where the text clamps: it should wrap to two lines and then truncate with an ellipsis rather than push the chips out of the card or grow it past the modal width.',
				author: 'a-very-long-hub-username',
				apps: ['github', 'slack', 'postgresql'],
				iconApps: ['github', 'slack', 'postgresql'],
				hub: 'https://hub.windmill.dev',
				counts: { apps: 2, flows: 128, scripts: 256, resources: 41 }
			}
		}
	]

	let project = $state<ImportProjectSummary>({ ...PRESETS[0].project })
	let hubHost = $state('hub.windmill.dev')

	function apply(p: ImportProjectSummary) {
		project = { ...p, counts: { ...p.counts }, apps: [...p.apps] }
	}

	const ALL_APPS = ['github', 'slack', 'postgresql', 'stripe']

	function toggleApp(a: string) {
		project.apps = project.apps.includes(a)
			? project.apps.filter((x) => x !== a)
			: [...project.apps, a]
	}

	const field =
		'w-full rounded-md border border-border-light bg-surface px-2 py-1 text-xs text-primary'
	const label = 'block text-[11px] font-medium uppercase tracking-wide text-tertiary mb-1'
</script>

<div class="min-h-screen bg-surface-secondary p-6">
	<div class="mx-auto flex max-w-6xl flex-col gap-6">
		<div class="flex items-center justify-between">
			<div>
				<h1 class="text-lg font-semibold text-emphasis">Import project card</h1>
				<p class="text-xs text-secondary">
					Design bench for the card shown on <span class="font-mono">/user/workspaces</span> when arriving
					from the hub.
				</p>
			</div>
			<DarkModeToggle />
		</div>

		<div class="flex flex-wrap gap-2">
			{#each PRESETS as p (p.label)}
				<button
					type="button"
					class="rounded-md border border-border-light bg-surface px-2.5 py-1 text-xs text-primary hover:bg-surface-hover"
					onclick={() => apply(p.project)}
				>
					{p.label}
				</button>
			{/each}
		</div>

		<div class="grid grid-cols-1 gap-6 lg:grid-cols-[280px_1fr]">
			<!-- Controls -->
			<div class="flex flex-col gap-3 rounded-lg border border-border-light bg-surface p-4">
				<div>
					<label class={label} for="name">Name</label>
					<input id="name" class={field} bind:value={project.name} />
				</div>
				<div>
					<label class={label} for="summary">Summary</label>
					<textarea id="summary" rows="3" class={field} bind:value={project.summary}></textarea>
				</div>
				<div class="grid grid-cols-2 gap-2">
					<div>
						<label class={label} for="author">Author</label>
						<input id="author" class={field} bind:value={project.author} />
					</div>
					<div>
						<label class={label} for="slug">Slug</label>
						<input id="slug" class={field} bind:value={project.slug} />
					</div>
				</div>
				<div>
					<label class={label} for="host">Hub host</label>
					<input id="host" class={field} bind:value={hubHost} />
				</div>

				<div>
					<span class={label}>Contents</span>
					<div class="grid grid-cols-2 gap-2">
						<label class="text-xs text-secondary">
							apps
							<input type="number" min="0" class={field} bind:value={project.counts.apps} />
						</label>
						<label class="text-xs text-secondary">
							flows
							<input type="number" min="0" class={field} bind:value={project.counts.flows} />
						</label>
						<label class="text-xs text-secondary">
							scripts
							<input type="number" min="0" class={field} bind:value={project.counts.scripts} />
						</label>
						<label class="text-xs text-secondary">
							resources
							<input type="number" min="0" class={field} bind:value={project.counts.resources} />
						</label>
					</div>
				</div>

				<div>
					<span class={label}>Integrations</span>
					<div class="flex flex-wrap gap-2">
						{#each ALL_APPS as a (a)}
							<button
								type="button"
								class="rounded-md border px-2 py-1 text-xs {project.apps.includes(a)
									? 'border-blue-400 bg-blue-50 text-blue-700 dark:bg-blue-900/40 dark:text-blue-200'
									: 'border-border-light bg-surface text-secondary'}"
								onclick={() => toggleApp(a)}
							>
								{a}
							</button>
						{/each}
					</div>
				</div>
			</div>

			<!-- Previews -->
			<div class="flex flex-col gap-6">
				<div>
					<p class="mb-2 text-xs text-tertiary">
						In context — CenteredModal body ({MODAL_CONTENT_WIDTH}px,
						<span class="font-mono">bg-surface</span>)
					</p>
					<div
						class="rounded-md bg-surface p-4 sm:px-10 sm:py-8"
						style="width: {MODAL_CONTENT_WIDTH + 80}px; max-width: 100%"
					>
						<ImportProjectCard {project} {hubHost} />
						<div class="text-sm font-semibold text-emphasis">Workspaces</div>
						<div class="mt-2 rounded-md border border-border-light p-3 text-xs text-secondary">
							Admins <span class="font-mono">admins</span>
						</div>
					</div>
				</div>

				<div>
					<p class="mb-2 text-xs text-tertiary">Standalone, full width</p>
					<ImportProjectCard {project} {hubHost} />
				</div>
			</div>
		</div>
	</div>
</div>
