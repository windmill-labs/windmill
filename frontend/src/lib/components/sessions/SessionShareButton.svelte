<script lang="ts">
	import { Globe, Link, Share2 } from 'lucide-svelte'
	import { resource } from 'runed'
	import { Button } from '$lib/components/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import ClipboardPanel from '$lib/components/details/ClipboardPanel.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { AiService, type SharedAiSessionInfo } from '$lib/gen'
	import { userWorkspaces } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'
	import { copyToClipboard, displayDate } from '$lib/utils'
	import { shareWorkspaceId } from '$lib/components/copilot/chat/artifacts/artifactSharing'
	import type { AIChatManager } from '$lib/components/copilot/chat/AIChatManager.svelte'
	import { buildSessionSnapshot, sharedSessionUrl } from './sessionSharing'

	interface Props {
		sessionId: string
		name: string
		/** The workspace the session acts on; undefined until its first message is sent. */
		workspaceId: string | undefined
		manager: AIChatManager
	}

	let { sessionId, name, workspaceId, manager }: Props = $props()

	const workspace = $derived(
		workspaceId ? shareWorkspaceId(workspaceId, $userWorkspaces) : undefined
	)
	const messageCount = $derived(manager.displayMessages.length)
	const shareName = $derived(name.trim() || 'Untitled session')

	const status = resource(
		() => ({ workspace, sessionId }),
		async ({ workspace, sessionId }) => {
			if (!workspace) return undefined
			try {
				return await AiService.getAiSessionShareStatus({ workspace, sessionId })
			} catch (err) {
				// The button still works without it: sharing reports its own failure.
				console.error('Could not read the session share status', err)
				return undefined
			}
		}
	)

	const share = $derived(status.current?.share)
	const url = $derived(share && workspace ? sharedSessionUrl(workspace, share.id) : undefined)
	// A hint only: an edited and regenerated turn keeps both, so updating stays offered either way.
	const outdated = $derived(
		share !== undefined && (share.message_count !== messageCount || share.name !== shareName)
	)

	let saving = $state(false)

	function errorMessage(err: unknown): string {
		const body = (err as { body?: unknown })?.body
		return typeof body === 'string' && body ? body : String((err as Error)?.message ?? err)
	}

	function reflect(next: SharedAiSessionInfo | undefined) {
		if (status.current) status.mutate({ ...status.current, share: next })
	}

	async function shareSnapshot() {
		if (!workspace) return
		const updating = share !== undefined
		saving = true
		try {
			const snapshot = buildSessionSnapshot(
				$state.snapshot(manager.displayMessages),
				$state.snapshot(manager.artifacts.artifacts)
			)
			const shared = await AiService.shareAiSession({
				workspace,
				requestBody: {
					session_id: sessionId,
					name: shareName,
					message_count: messageCount,
					snapshot
				}
			})
			if (status.current) reflect(shared)
			else await status.refetch()
			if (updating) {
				sendUserToast('The link now shows the latest messages')
			} else if (await copyToClipboard(sharedSessionUrl(workspace, shared.id), false)) {
				sendUserToast('Link copied to clipboard')
			}
		} catch (err) {
			sendUserToast(`Could not share the session: ${errorMessage(err)}`, true)
		} finally {
			saving = false
		}
	}

	async function setPublic(isPublic: boolean) {
		if (!workspace || !share) return
		saving = true
		try {
			reflect(
				await AiService.setAiSessionSharePublic({
					workspace,
					id: share.id,
					requestBody: { is_public: isPublic }
				})
			)
		} catch (err) {
			sendUserToast(`Could not change who can open the link: ${errorMessage(err)}`, true)
		} finally {
			saving = false
		}
	}

	async function stopSharing() {
		if (!workspace || !share) return
		saving = true
		try {
			await AiService.unshareAiSession({ workspace, id: share.id })
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
	disabled={!workspace || messageCount === 0}
	triggerAttrs={{ 'aria-label': 'Share session', 'aria-haspopup': 'dialog' }}
>
	{#snippet trigger()}
		<Button
			unifiedSize="sm"
			variant="subtle"
			nonCaptureEvent
			disabled={!workspace || messageCount === 0}
			startIcon={{ icon: share ? (share.is_public ? Globe : Link) : Share2 }}
			title={!workspace || messageCount === 0
				? 'Send a first message to share this session'
				: share
					? share.is_public
						? 'Shared publicly'
						: 'Shared with the workspace'
					: 'Share session'}
		>
			{share ? 'Shared' : 'Share'}
		</Button>
	{/snippet}
	{#snippet content()}
		<div class="flex w-80 flex-col gap-3 p-3 text-xs">
			<div class="flex flex-col gap-1">
				<span class="font-semibold text-emphasis">Share session</span>
				<span class="font-normal text-secondary">
					{#if share?.is_public}
						Anyone with the link can read a copy of this conversation. Nobody can continue it.
					{:else}
						Members of {workspace} can read a copy of this conversation with the link. Nobody can continue
						it.
					{/if}
				</span>
			</div>

			{#if share && url}
				<Toggle
					size="xs"
					checked={share.is_public}
					disabled={saving}
					options={{
						right: 'Public',
						rightTooltip: 'Anyone with the link can open it, without signing in'
					}}
					on:change={(e) => setPublic(e.detail)}
				/>
				<div class="flex flex-col gap-1">
					<ClipboardPanel content={url} size="sm" />
					<span class="text-2xs font-normal text-hint">
						Shows {share.message_count} message{share.message_count === 1 ? '' : 's'} as of {displayDate(
							share.shared_at
						)}
					</span>
				</div>
				<div class="flex items-center justify-between gap-2">
					<span class="font-normal text-secondary">
						{outdated
							? 'The session changed since it was shared.'
							: 'The link shows the session as it was when shared.'}
					</span>
					<Button
						unifiedSize="sm"
						variant={outdated ? 'accent' : 'default'}
						loading={saving}
						disabled={manager.loading}
						onClick={shareSnapshot}
					>
						Update link
					</Button>
				</div>
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
						disabled={status.loading || manager.loading}
						title={manager.loading ? 'Wait for the current turn to finish' : undefined}
						onClick={shareSnapshot}
					>
						Create link
					</Button>
				</div>
			{/if}
		</div>
	{/snippet}
</Popover>
