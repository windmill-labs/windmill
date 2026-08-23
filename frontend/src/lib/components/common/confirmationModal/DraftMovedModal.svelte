<script lang="ts">
	/**
	 * Surfaces the "moved" verdict left by `UserDraftDbSyncer.postSave`: someone
	 * moved this item while the editor was open, so the server refused the
	 * autosave rather than plant a phantom draft-only item at the path the item
	 * has left.
	 *
	 * Continuing pushes the current in-memory draft to the new path (force, since
	 * the draft carried over by the move is older) and follows it there, so edits
	 * made after the move aren't lost to the relocation.
	 */
	import { base } from '$app/paths'
	import { goto } from '$app/navigation'
	import { UserDraftDbSyncer, type UserDraftLastSyncQuery } from '$lib/userDraftDbSyncer.svelte'
	import Modal2 from '$lib/components/common/modal/Modal2.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { FolderInput } from 'lucide-svelte'

	type Props = {
		query: UserDraftLastSyncQuery
		/** Current local draft value, re-pointed at the new path before it is
		 *  pushed there. */
		getLocalDraft: () => unknown
	}

	let { query, getLocalDraft }: Props = $props()

	const moveHandle = $derived(UserDraftDbSyncer.getMove(query))
	let isOpen = $derived(moveHandle.move !== undefined)
	let busy = $state(false)

	const EDITOR_SEGMENT: Partial<Record<string, string>> = {
		script: 'scripts/edit',
		flow: 'flows/edit',
		app: 'apps/edit',
		raw_app: 'apps_raw/edit'
	}

	/** Mirrors the backend's `UserDraftItemKind::typed_path_field`: a script
	 * draft round-trips its own `path`, every other kind writes `draft_path`. */
	function repointed(value: unknown, newPath: string): unknown {
		if (value == undefined || typeof value !== 'object') return value
		const field = query.itemKind === 'script' ? 'path' : 'draft_path'
		return { ...(value as Record<string, unknown>), [field]: newPath }
	}

	async function continueThere() {
		const move = moveHandle.move
		if (!move) return
		busy = true
		try {
			const local = getLocalDraft()
			if (local != undefined) {
				await UserDraftDbSyncer.overwrite({
					workspace: query.workspace,
					itemKind: query.itemKind,
					path: move.movedTo,
					value: repointed(local, move.movedTo)
				})
			}
			UserDraftDbSyncer.clearMove(query)
			const seg = EDITOR_SEGMENT[query.itemKind]
			if (seg) await goto(`${base}/${seg}/${move.movedTo}`)
		} finally {
			busy = false
		}
	}
</script>

<Modal2 bind:isOpen title="This item was moved" fixedWidth="sm" fixedHeight="adaptive">
	<div class="flex flex-col w-full gap-4">
		<div class="flex gap-3 items-start flex-1">
			<FolderInput size={20} class="text-blue-500 shrink-0 mt-0.5" />
			<div class="text-sm text-primary flex flex-col gap-1">
				<p>
					{#if moveHandle.move?.movedBy}
						<span class="font-semibold">{moveHandle.move.movedBy}</span> moved this to
					{:else}
						This was moved to
					{/if}
					<span class="font-mono text-xs">{moveHandle.move?.movedTo}</span>. Your draft moved with
					it, so nothing was saved here.
				</p>
				<p class="text-xs text-secondary">
					Continuing takes your current edits to the new path. Staying here leaves them unsaved.
				</p>
			</div>
		</div>

		<div class="flex justify-end gap-2">
			<Button
				variant="default"
				size="sm"
				disabled={busy}
				on:click={() => UserDraftDbSyncer.clearMove(query)}
			>
				Stay here
			</Button>
			<Button variant="accent" size="sm" loading={busy} on:click={continueThere}>
				Continue at the new path
			</Button>
		</div>
	</div>
</Modal2>
