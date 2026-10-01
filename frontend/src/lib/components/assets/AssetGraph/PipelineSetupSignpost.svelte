<script lang="ts">
	import { resource } from 'runed'
	import { WorkspaceService } from '$lib/gen'
	import { base } from '$lib/base'
	import { CircleCheck, CircleAlert, Database, HardDrive, ArrowRight } from 'lucide-svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import { Button } from '$lib/components/common'

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
	<Alert type="warning" title="Finish setting up pipelines" hideIcon>
		<div class="flex flex-col gap-3">
			<p>
				Pipelines materialize data into DuckLake tables backed by object storage. Configure the
				following before your first pipeline can run.
			</p>
			<ul class="flex flex-col gap-2">
				{#each steps as step (step.title)}
					{@const Icon = step.icon}
					<li class="flex items-center gap-3 rounded-md border bg-surface-tertiary px-3 py-2">
						{#if step.done === true}
							<CircleCheck size={16} class="text-green-600 dark:text-green-400 shrink-0" />
						{:else if step.done === false}
							<CircleAlert size={16} class="text-yellow-600 dark:text-yellow-400 shrink-0" />
						{:else}
							<Icon size={16} class="text-secondary shrink-0" />
						{/if}
						<div class="flex flex-col min-w-0 flex-1">
							<span class="text-xs font-semibold text-emphasis">{step.title}</span>
							<span class="text-2xs text-secondary">{step.description}</span>
						</div>
						{#if step.done !== true}
							<Button
								variant="default"
								unifiedSize="sm"
								href={step.href}
								endIcon={{ icon: ArrowRight }}
								wrapperClasses="shrink-0"
							>
								{step.cta}
							</Button>
						{:else}
							<span class="shrink-0 text-2xs text-green-700 dark:text-green-400">Configured</span>
						{/if}
					</li>
				{/each}
			</ul>
		</div>
	</Alert>
{/if}
