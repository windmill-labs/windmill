<script lang="ts">
	import { page } from '$app/state'
	import { resource } from 'runed'
	import { Code, Eye, Link2Off } from 'lucide-svelte'
	import { Button, EmptyState, Skeleton } from '$lib/components/common'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import PageHeaderContent from '$lib/components/PageHeaderContent.svelte'
	import { pageHeader } from '$lib/components/pageHeaderRegistry.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import ArtifactBody from '$lib/components/copilot/chat/artifacts/ArtifactBody.svelte'
	import ArtifactExportButton from '$lib/components/copilot/chat/artifacts/ArtifactExportButton.svelte'
	import MermaidCodeBlock from '$lib/components/copilot/chat/artifacts/MermaidCodeBlock.svelte'
	import { sharedArtifactPath } from '$lib/components/copilot/chat/artifacts/artifactSharing'
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

	// The header picker's siblings. A backend without the list endpoint leaves the picker with
	// the open artifact alone rather than failing the page.
	const siblings = resource(
		() => workspace,
		async (workspace) => {
			if (!workspace) return []
			try {
				return await AiService.listSharedAiArtifacts({ workspace })
			} catch {
				return []
			}
		}
	)
	const navItems = $derived.by(() => {
		const items = (siblings.current ?? []).map((a) => ({ id: a.id, name: a.name }))
		if (artifact && !items.some((a) => a.id === artifact.id)) {
			items.unshift({ id: artifact.id, name: artifact.name })
		}
		return items
	})

	// The bar's one fold (`actionFolds` below), once the name has given all it can: Stop sharing
	// and Copy keep their icons and drop their labels.
	const folded = $derived(pageHeader.actionFold >= 1)

	let showSource = $state(false)
	const source = $derived(artifact?.kind !== 'md' || showSource)

	let removing = $state(false)
	async function stopSharing() {
		if (!workspace || !artifact) return
		removing = true
		try {
			await AiService.unshareAiArtifact({ workspace, id: artifact.id })
			shared.mutate({ state: 'gone' })
			siblings.refetch()
			sendUserToast('Stopped sharing: the link no longer opens')
		} catch (err) {
			sendUserToast(`Could not stop sharing: ${(err as { body?: unknown })?.body ?? err}`, true)
		} finally {
			removing = false
		}
	}
</script>

<PageHeaderContent
	section={{
		label: artifact?.name ?? 'Shared artifact',
		navArtifacts: workspace
			? {
					items: navItems,
					currentId: artifact?.id,
					href: (id) => sharedArtifactPath(workspace, id)
				}
			: undefined
	}}
	afterName={artifact ? shareDetails : undefined}
	actions={artifact ? artifactActions : undefined}
	separator="always"
	actionFolds={1}
/>

{#snippet shareDetails()}
	{#if artifact}
		<Tooltip>
			Shared by {artifact.created_by} · v{artifact.version} · {displayDate(artifact.shared_at)} · expires
			{displayDate(artifact.expires_at)}
		</Tooltip>
	{/if}
{/snippet}

{#snippet artifactActions()}
	{#if artifact}
		<div class="flex items-center gap-2">
			{#if artifact.can_unshare}
				<Button
					unifiedSize="sm"
					variant="default"
					destructive
					loading={removing}
					onClick={stopSharing}
					iconOnly={folded}
					startIcon={folded ? { icon: Link2Off } : undefined}
					title="Stop sharing"
				>
					Stop sharing
				</Button>
			{/if}
			<ArtifactExportButton
				name={artifact.name}
				kind={artifact.kind}
				content={artifact.content}
				iconOnly={folded}
			/>
			{#if artifact.kind === 'md'}
				<ToggleButtonGroup
					noWFull
					selected={showSource ? 'source' : 'preview'}
					onSelected={(v) => (showSource = v === 'source')}
				>
					{#snippet children({ item })}
						<ToggleButton {item} value="preview" icon={Eye} iconOnly tooltip="Preview" size="sm" />
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
	{/if}
{/snippet}

{#if artifact}
	<div class="h-full min-h-0 overflow-auto bg-surface-tertiary px-8">
		<!-- Full height only for the source editor: the rendered document's sticky top fade sticks
		     only while its parent box lasts, so that box must grow with the text. No width cap for
		     the document either: it sets its own column, and a wide table grows past it. -->
		<div class={source ? 'h-full max-w-4xl' : ''}>
			<ArtifactBody content={artifact.content} {source} pre={MermaidCodeBlock} />
		</div>
	</div>
{:else}
	<div class="mx-auto flex h-full w-full max-w-4xl flex-col px-4 py-6 sm:px-8">
		{#if shared.current?.state === 'gone'}
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
{/if}
