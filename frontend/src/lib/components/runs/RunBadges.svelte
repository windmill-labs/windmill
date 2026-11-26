<script lang="ts">
	import Tooltip from '$lib/components/meltComponents/Tooltip.svelte'
	import PreprocessedArgsDisplay from '$lib/components/runs/PreprocessedArgsDisplay.svelte'
	import { getJobKindDisplayLabel, msToCompactDuration, truncateHash } from '$lib/utils'
	import { truncateRev } from '$lib/utils'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { AlertTriangle, ListFilter, Tag } from 'lucide-svelte'
	import type { Job } from '$lib/gen'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'
	import { userWorkspaces } from '$lib/stores'
	import {
		devWorkspaceTagSuffix,
		devWorkspaceSkipNonDevForks,
		isIsolatedWorkspaceId,
		loadDevWorkspaceTagSuffix
	} from '$lib/devWorkspaceTags'

	const operatingWorkspace = useOperatingWorkspace()
	loadDevWorkspaceTagSuffix()

	interface Props {
		job: Job
		displayPersistentScriptDefinition?: boolean
		openPersistentScriptDrawer?: () => void
		concurrencyKey?: string
		/** When the job was first due, if its concurrency limit held it back. */
		concurrencyDeferredFrom?: string
		/** When its concurrency limit let in the job it had held back. */
		concurrencyReleasedAt?: string
		showScriptHash?: boolean
		onFilterByConcurrencyKey?: (key: string) => void
		large?: boolean
	}

	let {
		job,
		displayPersistentScriptDefinition,
		openPersistentScriptDrawer,
		concurrencyKey,
		concurrencyDeferredFrom,
		concurrencyReleasedAt,
		showScriptHash = true,
		onFilterByConcurrencyKey,
		large = false
	}: Props = $props()

	// From when the limit first held the job to when it let it in, counting live while it holds
	// it; the rest until it started is queue time. A job let in before the end of the hold was
	// recorded shows nothing.
	// A job canceled while held completes without ever being let in: it is not held any more.
	let heldNow = $derived(
		job.type === 'QueuedJob' &&
			!!concurrencyDeferredFrom &&
			!concurrencyReleasedAt &&
			!job.started_at
	)
	let now = $state(Date.now())
	$effect(() => {
		if (!heldNow) return
		const id = setInterval(() => (now = Date.now()), 1000)
		return () => clearInterval(id)
	})
	let heldMs = $derived(
		concurrencyDeferredFrom && (concurrencyReleasedAt || heldNow)
			? (concurrencyReleasedAt ? new Date(concurrencyReleasedAt).getTime() : now) -
					new Date(concurrencyDeferredFrom).getTime()
			: undefined
	)
	let queuedAfterMs = $derived(
		concurrencyReleasedAt && job.started_at
			? Math.max(new Date(job.started_at).getTime() - new Date(concurrencyReleasedAt).getTime(), 0)
			: undefined
	)
	// The steps of the wait-time warning on the status badge.
	let heldColor = $derived(
		heldMs == undefined
			? ''
			: heldMs > 300_000
				? 'text-red-500'
				: heldMs > 30_000
					? 'text-orange-500'
					: heldMs > 5_000
						? 'text-yellow-500'
						: 'text-gray-400'
	)
</script>

{#if job.script_hash && showScriptHash && job.job_kind !== 'aiagent'}
	{#if job.job_kind == 'script'}
		<a href="/scripts/get/{job.script_hash}?workspace={$operatingWorkspace}"
			><Badge color="gray" title={`Script hash: ${job.script_hash}`} {large}
				>{truncateHash(job.script_hash)}
			</Badge></a
		>
	{:else}
		<div>
			<Badge color="gray" {large}>{truncateHash(job.script_hash)}</Badge>
		</div>
	{/if}
{/if}
{#if job && 'job_kind' in job}
	<div>
		<Badge color="gray" {large}>Kind: {getJobKindDisplayLabel(job.job_kind, job.script_path)}</Badge
		>
	</div>
{/if}
{#if job && job.flow_status && job.job_kind === 'script'}
	<PreprocessedArgsDisplay preprocessed={job.preprocessed} />
{/if}
{#if displayPersistentScriptDefinition}
	<button onclick={() => openPersistentScriptDrawer?.()}>
		<Badge color="red" {large}>persistent</Badge>
	</button>
{/if}
<!-- 0 is how a job without priority is stored. -->
{#if job && 'priority' in job && job.priority != undefined && job.priority !== 0}
	<div>
		<Badge color="blue" {large}>priority: {job.priority}</Badge>
	</div>
{/if}
{#if job.tag != undefined}
	<!-- for related places search: ADD_NEW_LANG -->
	<div>
		<Badge color="indigo" {large}>
			Tag: {job.tag}
			{#if $devWorkspaceTagSuffix && job.tag.endsWith($devWorkspaceTagSuffix) && isIsolatedWorkspaceId(job.workspace_id, $userWorkspaces, $devWorkspaceSkipNonDevForks)}
				<Tooltip>
					{#snippet text()}
						Suffixed with <code>{$devWorkspaceTagSuffix}</code> because this workspace is a dev workspace
						or a fork: its jobs run only on worker groups serving dev workspaces.
					{/snippet}
				</Tooltip>
			{/if}
		</Badge>
	</div>
{/if}
{#if !job.visible_to_owner}
	<div>
		<Badge color="red" {large}>
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
			<Badge color="blue" {large} title="Label: {label}"
				><Tag size={10} class="inline -mt-px" />{label}</Badge
			>
		</div>
	{/each}
{/if}
{#snippet held()}
	{#if heldMs != undefined && heldMs > 0}
		<span class="ml-1 inline-flex items-center gap-0.5 {heldColor}">
			<AlertTriangle size={10} />{msToCompactDuration(heldMs, 0)}
		</span>
	{/if}
{/snippet}

{#if concurrencyKey}
	<div>
		<Tooltip>
			{#snippet text()}
				{#if heldMs != undefined}
					Held by its concurrency limit for {msToCompactDuration(heldMs, 0)}{queuedAfterMs
						? `, then queued ${msToCompactDuration(queuedAfterMs, 0)}`
						: ''}.
				{/if}
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
					<a href={`/runs/?job_kinds=all&graph=ConcurrencyChart&concurrency_key=${concurrencyKey}`}>
						{concurrencyKey}
					</a>
				{/if}
			{/snippet}
			{#if onFilterByConcurrencyKey}
				<button onclick={() => onFilterByConcurrencyKey?.(concurrencyKey)}>
					<Badge {large}>Concurrency: {truncateRev(concurrencyKey, 20)}{@render held()}</Badge>
				</button>
			{:else}
				<a href={`/runs/?job_kinds=all&graph=ConcurrencyChart&concurrency_key=${concurrencyKey}`}>
					<Badge {large}>Concurrency: {truncateRev(concurrencyKey, 20)}{@render held()}</Badge>
				</a>
			{/if}
		</Tooltip>
	</div>
{/if}
