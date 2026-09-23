<script lang="ts">
	import { page } from '$app/state'
	import { resource } from 'runed'
	import { Code, Eye, FileText, Link2Off } from 'lucide-svelte'
	import { Button, EmptyState, Skeleton } from '$lib/components/common'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import ArtifactBody from '$lib/components/copilot/chat/artifacts/ArtifactBody.svelte'
	import ArtifactExportButton from '$lib/components/copilot/chat/artifacts/ArtifactExportButton.svelte'
	import { AiService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { displayDate } from '$lib/utils'

	const id = $derived(page.params.id ?? '')
	// The link carries its workspace; the store only catches up once the layout applies it.
	const workspace = $derived(page.url.searchParams.get('workspace') ?? $workspaceStore)

	type Loaded =
		| { state: 'found'; artifact: Awaited<ReturnType<typeof AiService.getSharedAiArtifact>> }
		| { state: 'gone' }
		| { state: 'error'; message: string }

	const shared = resource(
		() => ({ workspace, id }),
		async ({ workspace, id }): Promise<Loaded | undefined> => {
			if (!workspace || !id) return undefined
			try {
				return {
					state: 'found',
					artifact: await AiService.getSharedAiArtifact({ workspace, id })
				}
			} catch (err) {
				const status = (err as { status?: number })?.status
				// A malformed id is a 400 from the path extractor, and means the same to the reader.
				if (status === 404 || status === 400) return { state: 'gone' }
				return { state: 'error', message: String((err as { body?: unknown })?.body ?? err) }
			}
		}
	)

	const artifact = $derived(shared.current?.state === 'found' ? shared.current.artifact : undefined)

	let showSource = $state(false)
	const source = $derived(artifact?.kind !== 'md' || showSource)

	let removing = $state(false)
	async function stopSharing() {
		if (!workspace || !artifact) return
		removing = true
		try {
			await AiService.unshareAiArtifact({ workspace, id: artifact.id })
			shared.mutate({ state: 'gone' })
			sendUserToast('Stopped sharing: the link no longer opens')
		} catch (err) {
			sendUserToast(`Could not stop sharing: ${(err as { body?: unknown })?.body ?? err}`, true)
		} finally {
			removing = false
		}
	}
</script>

<div class="flex h-full flex-col px-4 py-6 sm:px-8">
	<div class="mx-auto flex h-full min-h-0 w-full max-w-4xl flex-col gap-3">
		{#if artifact}
			<div class="flex flex-wrap items-start justify-between gap-3">
				<div class="flex min-w-0 flex-col gap-1">
					<div class="flex min-w-0 items-center gap-2">
						<FileText size={16} class="shrink-0 text-secondary" />
						<h1 class="truncate text-lg font-semibold text-emphasis" title={artifact.name}>
							{artifact.name}
						</h1>
					</div>
					<span class="text-xs font-normal text-secondary">
						Shared by {artifact.created_by} · v{artifact.version} · {displayDate(
							artifact.shared_at
						)} · expires {displayDate(artifact.expires_at)}
					</span>
				</div>
				<div class="flex shrink-0 items-center gap-2">
					{#if artifact.can_unshare}
						<Button
							unifiedSize="sm"
							variant="default"
							destructive
							loading={removing}
							onClick={stopSharing}
						>
							Stop sharing
						</Button>
					{/if}
					<ArtifactExportButton
						name={artifact.name}
						kind={artifact.kind}
						content={artifact.content}
					/>
					{#if artifact.kind === 'md'}
						<ToggleButtonGroup
							noWFull
							selected={showSource ? 'source' : 'preview'}
							onSelected={(v) => (showSource = v === 'source')}
						>
							{#snippet children({ item })}
								<ToggleButton
									{item}
									value="preview"
									icon={Eye}
									iconOnly
									tooltip="Preview"
									size="sm"
								/>
								<ToggleButton
									{item}
									value="source"
									icon={Code}
									iconOnly
									tooltip="View source"
									size="sm"
								/>
							{/snippet}
						</ToggleButtonGroup>
					{/if}
				</div>
			</div>
			<div class="min-h-0 flex-1 overflow-auto rounded-md bg-surface-tertiary px-8">
				<ArtifactBody content={artifact.content} {source} />
			</div>
		{:else if shared.current?.state === 'gone'}
			<EmptyState
				icon={Link2Off}
				title="This shared artifact is no longer available"
				description="Its link expired, or its author stopped sharing it."
			/>
		{:else if shared.current?.state === 'error'}
			<EmptyState
				icon={Link2Off}
				title="Could not load this shared artifact"
				description={shared.current.message}
			/>
		{:else}
			<Skeleton layout={[[3], 1, [30]]} />
		{/if}
	</div>
</div>
