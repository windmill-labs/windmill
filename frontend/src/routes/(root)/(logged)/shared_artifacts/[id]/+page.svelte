<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/state'
	import { resource } from 'runed'
	import { Link2Off } from 'lucide-svelte'
	import { EmptyState, Skeleton } from '$lib/components/common'
	import { artifactPath } from '$lib/components/copilot/chat/artifacts/artifactSharing'
	import { AiService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { apiErrorMessage } from '$lib/utils'

	const id = $derived(page.params.id ?? '')
	// The link carries its workspace; the store only catches up once the layout applies it.
	const workspace = $derived(page.url.searchParams.get('workspace') ?? $workspaceStore)

	// A link from before artifacts were items names its share, which now leads to the artifact.
	const resolved = resource(
		() => ({ workspace, id }),
		async ({ workspace, id }): Promise<{ failed: string } | undefined> => {
			if (!workspace || !id) return undefined
			try {
				const { artifact_id } = await AiService.resolveLegacyAiArtifactLink({ workspace, id })
				await goto(artifactPath(workspace, artifact_id), { replaceState: true })
				return undefined
			} catch (err) {
				const status = (err as { status?: number })?.status
				// A malformed id is a 400 from the path extractor, and means the same to the reader.
				if (status === 404 || status === 400) {
					return { failed: "This artifact isn't shared with you, or no longer exists" }
				}
				return { failed: apiErrorMessage(err) }
			}
		}
	)
</script>

<div class="mx-auto w-full max-w-4xl px-8 py-6">
	{#if resolved.current}
		<EmptyState icon={Link2Off} title={resolved.current.failed} />
	{:else}
		<Skeleton layout={[[3], 1, [30]]} />
	{/if}
</div>
