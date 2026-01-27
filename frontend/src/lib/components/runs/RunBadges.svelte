<script lang="ts">
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import PreprocessedArgsDisplay from '$lib/components/runs/PreprocessedArgsDisplay.svelte'
	import { truncateHash } from '$lib/utils'
	import { base } from '$app/paths'
	import { truncateRev } from '$lib/utils'
	import WorkerHostname from '$lib/components/WorkerHostname.svelte'
	import { workspaceStore } from '$lib/stores'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { ListFilter } from 'lucide-svelte'
	import type { Job } from '$lib/gen'

	interface Props {
		job: Job
		displayPersistentScriptDefinition?: boolean
		openPersistentScriptDrawer?: () => void
		concurrencyKey?: string
		showScriptHash?: boolean
		verySmall?: boolean
		onFilterByConcurrencyKey?: (key: string) => void
		onFilterByWorker?: (worker: string) => void
	}

	let {
		job,
		displayPersistentScriptDefinition,
		openPersistentScriptDrawer,
		concurrencyKey,
		showScriptHash = true,
		verySmall = false,
		onFilterByConcurrencyKey,
		onFilterByWorker
	}: Props = $props()
</script>

{#if job.script_hash && showScriptHash && job.job_kind !== 'aiagent'}
	{#if job.job_kind == 'script'}
		<a href="{base}/scripts/get/{job.script_hash}?workspace={$workspaceStore}"
			><Badge color="gray" {verySmall}>{truncateHash(job.script_hash)}</Badge></a
		>
	{:else}
		<div>
			<Badge color="gray" {verySmall}>{truncateHash(job.script_hash)}</Badge>
		</div>
	{/if}
{/if}
{#if job && 'job_kind' in job}
	<div>
		<Badge color="blue" {verySmall}>{job.job_kind}</Badge>
	</div>
{/if}
{#if job && job.flow_status && job.job_kind === 'script'}
	<PreprocessedArgsDisplay preprocessed={job.preprocessed} />
{/if}
{#if displayPersistentScriptDefinition}
	<button onclick={() => openPersistentScriptDrawer?.()}>
		<Badge color="red">persistent</Badge>
	</button>
{/if}
{#if job && 'priority' in job}
	<div>
		<Badge color="blue" {verySmall}>priority: {job.priority}</Badge>
	</div>
{/if}
{#if job.tag != undefined}
	<!-- for related places search: ADD_NEW_LANG -->
	<div>
		<Badge color="indigo" {verySmall}>Tag: {job.tag}</Badge>
	</div>
{/if}
{#if !job.visible_to_owner}
	<div>
		<Badge color="red" {verySmall}>
			only visible to you
			<Tooltip>
				{#snippet text()}
					The option to hide this run from the owner of this script or flow was activated
				{/snippet}
			</Tooltip>
		</Badge>
	</div>
{/if}
{#if job?.['labels'] && Array.isArray(job?.['labels']) && job?.['labels'].length > 0}
	{#each job?.['labels'] as label}
		<div>
			<Badge {verySmall}>Label: {label}</Badge>
		</div>
	{/each}
{/if}
{#if concurrencyKey}
	<div>
		<Tooltip notClickable>
			{#snippet text()}
				This job has concurrency limits enabled with the key:
				{#if onFilterByConcurrencyKey}
					<Button
						class="inline-text"
						size="xs2"
						color="light"
						onclick={() => onFilterByConcurrencyKey?.(concurrencyKey)}
					>
						{concurrencyKey}
						<ListFilter class="inline-block" size={10} />
					</Button>
				{:else}
					<a
						href={`${base}/runs/?job_kinds=all&graph=ConcurrencyChart&concurrency_key=${concurrencyKey}`}
					>
						{concurrencyKey}
					</a>
				{/if}
			{/snippet}
			{#if onFilterByConcurrencyKey}
				<button onclick={() => onFilterByConcurrencyKey?.(concurrencyKey)}>
					<Badge {verySmall}>Concurrency: {truncateRev(concurrencyKey, 20)}</Badge>
				</button>
			{:else}
				<a
					href={`${base}/runs/?job_kinds=all&graph=ConcurrencyChart&concurrency_key=${concurrencyKey}`}
				>
					<Badge {verySmall}>Concurrency: {truncateRev(concurrencyKey, 20)}</Badge>
				</a>
			{/if}
		</Tooltip>
	</div>
{/if}
{#if job?.worker}
	<div>
		<Tooltip notClickable>
			{#snippet text()}
				This job was run on worker:
				{#if onFilterByWorker}
					<Button
						class="inline-text"
						size="xs2"
						color="light"
						onclick={() => job?.worker && onFilterByWorker?.(job.worker)}
					>
						{job?.worker}
						<ListFilter class="inline-block" size={10} />
					</Button>
				{:else}
					<a href={`${base}/runs/?job_kinds=all&worker=${job?.worker}`}>
						{job?.worker}
					</a>
				{/if}
				<br />
				<WorkerHostname worker={job?.worker!} minTs={job?.['created_at']} />
			{/snippet}
			{#if onFilterByWorker}
				<button onclick={() => job?.worker && onFilterByWorker?.(job.worker)}>
					<Badge {verySmall}>Worker: {truncateRev(job?.worker, 20)}</Badge>
				</button>
			{:else}
				<a href={`${base}/runs/?job_kinds=all&worker=${job?.worker}`}>
					<Badge {verySmall}>Worker: {truncateRev(job?.worker, 20)}</Badge>
				</a>
			{/if}
		</Tooltip>
	</div>
{/if}
