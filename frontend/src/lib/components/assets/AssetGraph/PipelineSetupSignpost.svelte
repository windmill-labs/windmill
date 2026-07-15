<script lang="ts">
	import { resource } from 'runed'
	import { WorkspaceService } from '$lib/gen'
	import { base } from '$lib/base'
	import { CircleCheck, CircleAlert, Database, HardDrive, ArrowRight } from 'lucide-svelte'

	interface Props {
		workspace: string
	}
	let { workspace }: Props = $props()

	// A pipeline can't materialize anything until the workspace has (a) object
	// storage for the parquet files and (b) at least one DuckLake catalog. A
	// brand-new user has no signal these are prerequisites — this first-run
	// checklist surfaces them and links straight to the settings that fix them.
	//
	// Both probes fail SAFE: an errored settings read (e.g. non-admin) resolves
	// to `undefined`, and an undefined state never renders a red "missing" —
	// the signpost only asserts "not configured" when it positively knows so.
	let storage = resource([() => workspace], async ([ws]) => {
		if (!ws) return undefined
		try {
			const s = await WorkspaceService.getSettings({ workspace: ws })
			return !!s.large_file_storage
		} catch {
			return undefined
		}
	})
	let ducklakes = resource([() => workspace], async ([ws]) => {
		if (!ws) return undefined
		try {
			return (await WorkspaceService.listDucklakes({ workspace: ws })).length > 0
		} catch {
			return undefined
		}
	})

	// Only surface the signpost once we have a definite "not configured" for at
	// least one prerequisite — never while loading, and never on an errored
	// probe (which would nag a user who can't act on it anyway).
	let storageMissing = $derived(storage.current === false)
	let ducklakeMissing = $derived(ducklakes.current === false)
	let show = $derived(storageMissing || ducklakeMissing)

	type Step = {
		done: boolean | undefined
		icon: typeof Database
		title: string
		description: string
		href: string
		cta: string
	}
	let steps = $derived<Step[]>([
		{
			done: storage.current,
			icon: HardDrive,
			title: 'Workspace object storage',
			description:
				'Materialized partitions and DuckLake data files are written to S3 / object storage.',
			href: `${base}/workspace_settings?tab=windmill_lfs`,
			cta: 'Configure object storage'
		},
		{
			done: ducklakes.current,
			icon: Database,
			title: 'A DuckLake catalog',
			description:
				'DuckLake is the table format pipelines materialize into — add at least one catalog.',
			href: `${base}/workspace_settings?tab=ducklake`,
			cta: 'Configure DuckLake'
		}
	])
</script>

{#if show}
	<div
		class="flex flex-col gap-3 rounded-lg border border-amber-300 dark:border-amber-900/60 bg-amber-50 dark:bg-amber-950/30 p-4"
	>
		<div class="flex items-start gap-2">
			<CircleAlert size={18} class="text-amber-600 dark:text-amber-400 mt-0.5 shrink-0" />
			<div class="flex flex-col gap-0.5">
				<h3 class="text-sm font-semibold text-emphasis">Finish setting up pipelines</h3>
				<p class="text-xs text-tertiary">
					Pipelines materialize data into DuckLake tables backed by object storage. Configure the
					following before your first pipeline can run.
				</p>
			</div>
		</div>

		<ul class="flex flex-col gap-2">
			{#each steps as step (step.title)}
				{@const Icon = step.icon}
				<li class="flex items-center gap-3 rounded-md border bg-surface px-3 py-2">
					{#if step.done === true}
						<CircleCheck size={16} class="text-green-600 dark:text-green-400 shrink-0" />
					{:else if step.done === false}
						<CircleAlert size={16} class="text-amber-600 dark:text-amber-400 shrink-0" />
					{:else}
						<Icon size={16} class="text-tertiary shrink-0" />
					{/if}
					<div class="flex flex-col min-w-0 flex-1">
						<span class="text-xs font-semibold text-primary">{step.title}</span>
						<span class="text-2xs text-tertiary">{step.description}</span>
					</div>
					{#if step.done !== true}
						<a
							href={step.href}
							class="shrink-0 inline-flex items-center gap-1 text-xs text-blue-500 hover:underline whitespace-nowrap"
						>
							{step.cta}
							<ArrowRight size={12} />
						</a>
					{:else}
						<span class="shrink-0 text-2xs text-green-700 dark:text-green-400">Configured</span>
					{/if}
				</li>
			{/each}
		</ul>
	</div>
{/if}
