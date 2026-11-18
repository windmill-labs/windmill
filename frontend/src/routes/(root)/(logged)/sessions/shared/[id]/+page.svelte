<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/state'
	import { EllipsisVertical, Link2Off, X } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import DropdownV2 from '$lib/components/DropdownV2.svelte'
	import PageHeaderContent from '$lib/components/PageHeaderContent.svelte'
	import SharedSessionBody from '$lib/components/sessions/SharedSessionBody.svelte'
	import SharedSessionTitle from '$lib/components/sessions/SharedSessionTitle.svelte'
	import { useSharedSession } from '$lib/components/sessions/sharedSession.svelte'
	import { resource } from 'runed'
	import { UserService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'

	const workspace = $derived(page.url.searchParams.get('workspace') ?? $workspaceStore ?? '')
	const id = $derived(page.params.id ?? '')

	const shared = useSharedSession(() => ({ workspace, id }))
	// This URL is reachable by a signed-in user of another workspace when the share is public:
	// they get the standalone page's read-only presentation, with nothing to remove or open.
	const member = resource(
		() => workspace,
		(workspace) =>
			workspace
				? UserService.whoami({ workspace }).then(
						() => true,
						() => false
					)
				: Promise.resolve(false)
	)
	const isMember = $derived(member.current === true)
	const found = $derived(shared.found)
	const name = $derived(found?.session.name ?? 'Shared session')

	// Staying would show the share still open, and reloading it would list it again.
	async function removeFromList() {
		if (await shared.removeFromList()) await goto('/')
	}
</script>

{#snippet sessionName()}
	<SharedSessionTitle
		{name}
		author={found?.session.created_by}
		sharedAt={found?.session.shared_at}
	/>
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
	{:else if found && isMember}
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

<div class="h-full min-h-0">
	<SharedSessionBody loaded={shared.current} {workspace} member={isMember} />
</div>
