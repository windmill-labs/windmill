<script lang="ts">
	import { Link, Share2 } from 'lucide-svelte'
	import { resource } from 'runed'
	import { Button } from '$lib/components/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import ClipboardPanel from '$lib/components/details/ClipboardPanel.svelte'
	import { AiService, type SharedAiArtifactInfo } from '$lib/gen'
	import { userWorkspaces, workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { copyToClipboard, displayDate } from '$lib/utils'
	import type { ArtifactKind } from './artifactsDB'
	import { formatRetention, sharedArtifactUrl, shareWorkspaceId } from './artifactSharing'

	interface Props {
		artifactId: string
		name: string
		kind: ArtifactKind
		content: string
		/** The version on screen, which is the one a share copies. */
		version: number
		disabled?: boolean
	}

	let { artifactId, name, kind, content, version, disabled = false }: Props = $props()

	const workspace = $derived(
		$workspaceStore ? shareWorkspaceId($workspaceStore, $userWorkspaces) : undefined
	)

	const status = resource(
		() => ({ workspace, artifactId }),
		async ({ workspace, artifactId }) => {
			if (!workspace) return undefined
			try {
				return await AiService.getAiArtifactShareStatus({ workspace, artifactId })
			} catch (err) {
				// The button still works without it: sharing reports its own failure.
				console.error('Could not read the artifact share status', err)
				return undefined
			}
		}
	)

	const share = $derived(status.current?.share)
	const url = $derived(share && workspace ? sharedArtifactUrl(workspace, share.id) : undefined)
	// A rename earns no version, so the name is compared too.
	const outdated = $derived(
		share !== undefined && (share.version !== version || share.name !== name.trim())
	)

	let saving = $state(false)

	function errorMessage(err: unknown): string {
		const body = (err as { body?: unknown })?.body
		return typeof body === 'string' && body ? body : String(err)
	}

	function reflect(next: SharedAiArtifactInfo | undefined) {
		if (!status.current) return
		status.mutate({ ...status.current, share: next })
	}

	async function shareVersion() {
		if (!workspace) return
		const updating = share !== undefined
		saving = true
		try {
			const shared = await AiService.shareAiArtifact({
				workspace,
				requestBody: { artifact_id: artifactId, name, kind, version, content }
			})
			if (status.current) reflect(shared)
			else await status.refetch()
			if (updating) {
				sendUserToast(`The link now shows v${shared.version}`)
			} else if (await copyToClipboard(sharedArtifactUrl(workspace, shared.id), false)) {
				sendUserToast('Link copied to clipboard')
			}
		} catch (err) {
			sendUserToast(`Could not share the artifact: ${errorMessage(err)}`, true)
		} finally {
			saving = false
		}
	}

	async function stopSharing() {
		if (!workspace || !share) return
		saving = true
		try {
			await AiService.unshareAiArtifact({ workspace, id: share.id })
			reflect(undefined)
			sendUserToast('Stopped sharing: the link no longer opens')
		} catch (err) {
			sendUserToast(`Could not stop sharing: ${errorMessage(err)}`, true)
		} finally {
			saving = false
		}
	}
</script>

<Popover
	placement="bottom-end"
	contentClasses="!bg-surface"
	{disabled}
	triggerAttrs={{ 'aria-label': 'Share artifact', 'aria-haspopup': 'dialog' }}
>
	{#snippet trigger()}
		<Button
			unifiedSize="sm"
			variant="default"
			nonCaptureEvent
			disabled={disabled || !workspace}
			startIcon={{ icon: share ? Link : Share2 }}
			title={share ? 'Shared with the workspace' : 'Share with the workspace'}
		>
			{share ? 'Shared' : 'Share'}
		</Button>
	{/snippet}
	{#snippet content()}
		<div class="flex w-80 flex-col gap-3 p-3 text-xs">
			<div class="flex flex-col gap-1">
				<span class="font-semibold text-emphasis">Share with workspace</span>
				<span class="font-normal text-secondary">
					{#if share}
						Members of {workspace} can open a read-only copy of v{share.version} with this link.
					{:else}
						Members of {workspace} will be able to open a read-only copy of v{version} with a link.
					{/if}
					{#if status.current}
						The copy is deleted {formatRetention(status.current.retention_secs)} after it is shared.
					{/if}
				</span>
			</div>

			{#if share && url}
				<div class="flex flex-col gap-1">
					<ClipboardPanel content={url} size="sm" />
					<span class="text-2xs font-normal text-hint">
						Expires {displayDate(share.expires_at)}
					</span>
				</div>
				{#if outdated}
					<div class="flex items-center justify-between gap-2">
						<span class="font-normal text-secondary">
							The link shows v{share.version}{share.name !== name.trim() ? ` (${share.name})` : ''}.
						</span>
						<Button unifiedSize="sm" variant="accent" loading={saving} onClick={shareVersion}>
							Update to v{version}
						</Button>
					</div>
				{/if}
				<div class="flex justify-end">
					<Button
						unifiedSize="sm"
						variant="default"
						destructive
						disabled={saving}
						onClick={stopSharing}
					>
						Stop sharing
					</Button>
				</div>
			{:else}
				<div class="flex justify-end">
					<Button
						unifiedSize="sm"
						variant="accent"
						startIcon={{ icon: Link }}
						loading={saving}
						disabled={status.loading}
						onClick={shareVersion}
					>
						Create link
					</Button>
				</div>
			{/if}
		</div>
	{/snippet}
</Popover>
