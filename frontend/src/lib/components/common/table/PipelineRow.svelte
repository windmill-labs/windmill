<script lang="ts">
	import { base } from '$lib/base'
	import Dropdown from '$lib/components/DropdownV2.svelte'
	import ConfirmationModal from '../confirmationModal/ConfirmationModal.svelte'
	import {
		deletePipeline,
		planPipelineDelete,
		type PipelineDeletePlan
	} from '$lib/components/assets/AssetGraph/pipelineDelete'
	import { userStore, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { Loader2, Trash } from 'lucide-svelte'
	import Row from './Row.svelte'

	interface Props {
		folder: string
		depth?: number
		/** Shown instead of `f/<folder>`, e.g. under the folder's own tree node. */
		label?: string
		onDeleted?: () => void
	}

	let { folder, depth = 0, label, onDeleted }: Props = $props()

	let confirmOpen = $state(false)
	let deleting = $state(false)
	let plan = $state<PipelineDeletePlan | undefined>(undefined)
	let planError = $state<string | undefined>(undefined)
	// The backend deletes scripts for admins only; everyone else archives them.
	let hardDelete = $derived(!!($userStore?.is_admin || $userStore?.is_super_admin))

	async function openConfirm() {
		const ws = $workspaceStore
		if (!ws) return
		plan = undefined
		planError = undefined
		confirmOpen = true
		try {
			plan = await planPipelineDelete(ws, folder)
		} catch (e: any) {
			planError = e?.body ?? e?.message ?? String(e)
		}
	}

	async function confirmDelete() {
		const ws = $workspaceStore
		if (!ws || !plan) return
		deleting = true
		try {
			const { removed, failures } = await deletePipeline(ws, folder, plan, hardDelete)
			// Loaded on demand: the home list should not pull in the session runtime.
			const { forgetDeletedPipeline } = await import(
				'$lib/components/sessions/sessionRuntime.svelte'
			)
			forgetDeletedPipeline(ws, folder)
			if (failures.length) {
				sendUserToast(`Pipeline f/${folder} was only partly removed`, true, [], failures.join('\n'))
			} else {
				sendUserToast(`Removed pipeline f/${folder} (${removed} item${removed === 1 ? '' : 's'})`)
			}
			onDeleted?.()
		} catch (e: any) {
			sendUserToast(`Could not remove pipeline f/${folder}: ${e?.body ?? e?.message ?? e}`, true)
		} finally {
			deleting = false
			confirmOpen = false
		}
	}
</script>

<Row
	kind="data_pipeline"
	marked={undefined}
	href="{base}/pipeline/{encodeURIComponent(folder)}"
	workspaceId={$workspaceStore ?? ''}
	path={`f/${folder}`}
	summary={label ?? `Pipeline · ${folder}`}
	canFavorite={false}
	{depth}
>
	{#snippet actions()}
		{#if !$userStore?.operator}
			<Dropdown
				items={[
					{
						displayName: 'Delete',
						icon: Trash,
						type: 'delete',
						action: () => void openConfirm()
					}
				]}
			/>
		{/if}
	{/snippet}
</Row>

<ConfirmationModal
	open={confirmOpen}
	title="Delete pipeline f/{folder}"
	confirmationText={hardDelete ? 'Delete' : 'Archive'}
	loading={deleting}
	confirmDisabled={!plan}
	trashbin={hardDelete}
	on:canceled={() => (confirmOpen = false)}
	on:confirmed={confirmDelete}
>
	<div class="flex flex-col gap-3 text-xs">
		{#if planError}
			<span class="text-red-600 dark:text-red-400">Could not list the pipeline: {planError}</span>
		{:else if !plan}
			<span class="flex items-center gap-2 text-secondary">
				<Loader2 size={14} class="animate-spin" /> Listing what this removes…
			</span>
		{:else}
			{#if plan.scripts.some((s) => !s.draftOnly)}
				{@render group(
					hardDelete ? 'Scripts deleted' : 'Scripts archived',
					plan.scripts.filter((s) => !s.draftOnly).map((s) => s.displayPath)
				)}
			{/if}
			{#if plan.scripts.some((s) => s.draftOnly)}
				{@render group(
					'Draft scripts discarded',
					plan.scripts.filter((s) => s.draftOnly).map((s) => s.displayPath)
				)}
			{/if}
			{#if plan.triggers.length}
				{@render group(
					'Triggers deleted',
					plan.triggers.map((t) => `${t.path} (${t.kind})`)
				)}
			{/if}
			{#if plan.hasDraft}
				{@render group('Unsaved pipeline changes discarded', ['Your pipeline draft'])}
			{/if}
			{#if !plan.scripts.length && !plan.triggers.length && !plan.hasDraft}
				<span class="text-secondary">Nothing left to remove in this pipeline.</span>
			{/if}
			<span class="text-secondary">
				The tables and files the pipeline wrote are kept, and so is the folder f/{folder}.
			</span>
		{/if}
	</div>
</ConfirmationModal>

{#snippet group(title: string, entries: string[])}
	<div class="flex flex-col gap-1">
		<span class="font-semibold text-emphasis">{title} ({entries.length})</span>
		<ul class="max-h-40 overflow-auto rounded-md border bg-surface-secondary px-3 py-1.5">
			{#each entries as entry (entry)}
				<li class="font-mono text-2xs text-primary truncate">{entry}</li>
			{/each}
		</ul>
	</div>
{/snippet}
