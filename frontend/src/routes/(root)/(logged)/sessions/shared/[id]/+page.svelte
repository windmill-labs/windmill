<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/state'
	import { EllipsisVertical, Link2Off, Share2, X } from 'lucide-svelte'
	import { Badge, Button } from '$lib/components/common'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import PageHeaderContent from '$lib/components/PageHeaderContent.svelte'
	import SharedSessionBody from '$lib/components/sessions/SharedSessionBody.svelte'
	import { useSharedSession } from '$lib/components/sessions/sharedSession.svelte'
	import { userColorClass, userInitials } from '$lib/components/userInitials'
	import { workspaceStore } from '$lib/stores'
	import { displayDate } from '$lib/utils'

	const workspace = $derived(page.url.searchParams.get('workspace') ?? $workspaceStore ?? '')
	const id = $derived(page.params.id ?? '')

	const shared = useSharedSession(() => ({ workspace, id }))
	const found = $derived(shared.found)
	const name = $derived(found?.session.name ?? 'Shared session')

	// Staying would show the share still open, and reloading it would list it again.
	async function removeFromList() {
		if (await shared.removeFromList()) await goto('/')
	}
</script>

{#snippet sessionName()}
	<span class="truncate text-xs font-normal text-emphasis" title={name}>{name}</span>
	{#if found}
		{@const author = found.session.created_by}
		<span class="inline-flex shrink-0 items-center gap-1" title="Shared by {author}">
			<Badge color="blue" icon={{ icon: Share2, position: 'left' }}>Shared</Badge>
			<span
				class="inline-flex h-4 w-4 items-center justify-center rounded-full text-[8px] font-semibold text-white {userColorClass(
					author
				)}"
			>
				{userInitials(author)}
			</span>
		</span>
	{/if}
{/snippet}

{#snippet headerActions()}
	{#if found?.session.mine}
		<Button
			unifiedSize="sm"
			variant="default"
			destructive
			loading={shared.removing}
			onClick={shared.stopSharing}
		>
			Stop sharing
		</Button>
	{:else if found}
		<div class="flex items-center gap-1">
			<Button
				unifiedSize="sm"
				variant="default"
				startIcon={{ icon: X }}
				loading={shared.removing}
				title="Remove it from Shared with you in the sidebar. The link keeps working."
				onClick={removeFromList}
			>
				Remove from my list
			</Button>
			<!-- An admin may take down a share made by someone else, which is moderation rather than
			     the reader's own action. -->
			{#if found.session.can_unshare}
				<DropdownV2
					fixedHeight={false}
					placement="bottom-end"
					enableFlyTransition
					items={[
						{
							displayName: 'Stop sharing',
							description: 'Takes the link down for everyone. Allowed because you are an admin.',
							icon: Link2Off,
							type: 'delete',
							action: () => shared.stopSharing()
						}
					]}
				>
					{#snippet buttonReplacement()}
						<span
							class="inline-flex h-7 w-7 items-center justify-center rounded text-tertiary hover:bg-surface-hover hover:text-primary"
							title="More"
						>
							<EllipsisVertical size={14} />
						</span>
					{/snippet}
				</DropdownV2>
			{/if}
		</div>
	{/if}
{/snippet}

<PageHeaderContent
	section={{ label: name, content: sessionName, noNavPicker: true }}
	actions={headerActions}
	actingWorkspaceId={workspace}
/>

<div class="h-full min-h-0 flex flex-col">
	<div class="flex-1 min-h-0 overflow-y-auto">
		<div class="mx-auto flex min-h-full w-full max-w-3xl flex-col px-7 pt-4">
			<SharedSessionBody loaded={shared.current} {workspace} member />
		</div>
	</div>
	{#if found}
		<div class="mx-auto w-full max-w-3xl px-7 pb-3 pt-2 text-center text-xs text-secondary">
			Read-only {found.session.is_public ? 'public ' : ''}copy shared by {found.session.created_by}
			· {displayDate(found.session.shared_at)}
		</div>
	{/if}
</div>
