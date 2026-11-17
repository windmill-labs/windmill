<script lang="ts">
	import { page } from '$app/state'
	import { resource } from 'runed'
	import { Globe, Users } from 'lucide-svelte'
	import { Badge, Button } from '$lib/components/common'
	import WindmillIcon from '$lib/components/icons/WindmillIcon.svelte'
	import SharedSessionBody from '$lib/components/sessions/SharedSessionBody.svelte'
	import { useSharedSession } from '$lib/components/sessions/sharedSession.svelte'
	import { sharedSessionAppHref } from '$lib/components/sessions/sessionSharing'
	import { applyDarkModeVariant } from '$lib/darkModeVariant'
	import { UserService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { displayDate } from '$lib/utils'

	// Reached without signing in when the share is public, so this page sets up what the
	// logged-in layout otherwise would.
	const darkMode =
		window.localStorage.getItem('dark-mode') ??
		(window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')
	document.documentElement.classList.toggle('dark', darkMode === 'dark')
	applyDarkModeVariant()

	const workspace = $derived(page.params.workspace ?? '')
	const id = $derived(page.params.id ?? '')

	$effect(() => {
		$workspaceStore = workspace
	})

	// A member reads the share inside the app. Asked before the snapshot is fetched, so a member
	// does not download it twice; a full load, so this page's image policy stays behind.
	const member = resource(
		() => workspace,
		async (workspace) => {
			if (!workspace) return undefined
			const isMember = await UserService.whoami({ workspace }).then(
				() => true,
				() => false
			)
			if (isMember) window.location.replace(sharedSessionAppHref(workspace, id))
			return isMember
		}
	)

	const shared = useSharedSession(() => (member.current === false ? { workspace, id } : undefined))
	const found = $derived(shared.found)
</script>

<!-- The transcript is its author's text, rendered to whoever holds the link: an image in it
     that points elsewhere would report every reader to that host. A policy stays with the
     document once set, so every link out of this page is a full load (`data-sveltekit-reload`). -->
<svelte:head>
	<meta http-equiv="content-security-policy" content="img-src 'self' data: blob:" />
</svelte:head>

<div class="flex min-h-screen flex-col bg-surface" data-sveltekit-reload>
	<div class="border-b bg-surface-secondary">
		<div class="mx-auto flex w-full max-w-3xl flex-wrap items-center gap-3 px-4 py-3">
			<WindmillIcon height="20px" width="20px" />
			{#if found}
				<div class="flex min-w-0 flex-1 flex-col">
					<div class="flex min-w-0 items-center gap-2">
						<h1 class="truncate text-sm font-semibold text-emphasis" title={found.session.name}>
							{found.session.name}
						</h1>
						{#if found.session.is_public}
							<Badge color="blue" icon={{ icon: Globe, position: 'left' }}>Public</Badge>
						{:else}
							<Badge color="gray" icon={{ icon: Users, position: 'left' }}>{workspace}</Badge>
						{/if}
					</div>
					<span class="text-xs font-normal text-secondary">
						Read-only AI session shared by {found.session.created_by} · {displayDate(
							found.session.shared_at
						)}
					</span>
				</div>
				{#if found.session.can_unshare}
					<Button
						unifiedSize="sm"
						variant="default"
						destructive
						loading={shared.removing}
						onClick={shared.stopSharing}
					>
						Stop sharing
					</Button>
				{/if}
			{:else}
				<span class="text-sm font-semibold text-emphasis">Shared AI session</span>
			{/if}
		</div>
	</div>

	<div class="mx-auto flex w-full max-w-3xl flex-1 flex-col px-4 pt-6">
		<SharedSessionBody loaded={shared.current} {workspace} />
	</div>
</div>
