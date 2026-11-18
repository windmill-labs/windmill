<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/state'
	import { resource } from 'runed'
	import { LogIn } from 'lucide-svelte'
	import { Button } from '$lib/components/common'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import WindmillIcon from '$lib/components/icons/WindmillIcon.svelte'
	import PageHeaderBar from '$lib/components/PageHeaderBar.svelte'
	import PageHeaderContent from '$lib/components/PageHeaderContent.svelte'
	import { pageHeader } from '$lib/components/pageHeaderRegistry.svelte'
	import SharedSessionBody from '$lib/components/sessions/SharedSessionBody.svelte'
	import SharedSessionTitle from '$lib/components/sessions/SharedSessionTitle.svelte'
	import { useSharedSession } from '$lib/components/sessions/sharedSession.svelte'
	import { sharedSessionAppHref } from '$lib/components/sessions/sessionSharing'
	import { applyDarkModeVariant } from '$lib/darkModeVariant'
	import { UserService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'

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
	// does not download it twice.
	const member = resource(
		() => workspace,
		async (workspace) => {
			if (!workspace) return undefined
			const isMember = await UserService.whoami({ workspace }).then(
				() => true,
				() => false
			)
			if (isMember) await goto(sharedSessionAppHref(workspace, id), { replaceState: true })
			return isMember
		}
	)

	const shared = useSharedSession(() => (member.current === false ? { workspace, id } : undefined))
	const found = $derived(shared.found)
	const name = $derived(found?.session.name ?? 'Shared AI session')

	// Only a public share reaches a reader with no session. Signing in as a member brings them
	// back here, and on to the in-app page where the drafts open.
	const signedOut = resource(
		() => member.current === false,
		(nonMember) =>
			nonMember
				? UserService.getCurrentEmail().then(
						() => false,
						() => true
					)
				: Promise.resolve(false)
	)
	const rightInset = $derived(pageHeader.content?.barRightInset)
	const signInHref = $derived(
		signedOut.current
			? `/user/login?rd=${encodeURIComponent(page.url.pathname + page.url.search)}`
			: undefined
	)
</script>

{#if member.current !== false}
	<CenteredModal title="Opening the shared session" loading />
{:else}
	{#snippet sessionName()}
		<SharedSessionTitle
			{name}
			author={found?.session.created_by}
			sharedAt={found?.session.shared_at}
			isPublic={found?.session.is_public}
		/>
	{/snippet}

	<!-- In the header while the preview is closed, and on the preview's tab strip while it is open,
	     where the header no longer reaches. -->
	{#snippet headerActions()}
		{#if signInHref && rightInset === undefined}
			<div class="flex items-center gap-2">
				<span class="text-2xs text-hint">Signed out</span>
				<Button unifiedSize="sm" variant="default" startIcon={{ icon: LogIn }} href={signInHref}>
					Sign in
				</Button>
			</div>
		{/if}
	{/snippet}
	{#snippet previewActions()}
		{#if signInHref}
			<span class="text-2xs text-hint">Signed out</span>
			<Button unifiedSize="xs" variant="default" startIcon={{ icon: LogIn }} href={signInHref}>
				Sign in
			</Button>
		{/if}
	{/snippet}

	<PageHeaderContent
		section={{ label: name, parent: workspace, content: sessionName, noNavPicker: true }}
		actions={headerActions}
	/>

	<div class="relative flex h-screen flex-col bg-surface">
		<!-- `navHidden`: the reader may not be a member, so the bar offers no workspace to navigate.
		     With the preview open the row floats and stops where the panel starts, as the app
		     layout's header does beside a live session's panel. -->
		<div
			class="flex items-center bg-surface pl-4 {rightInset === undefined
				? 'shrink-0'
				: 'absolute left-0 top-0 z-30'}"
			style:right={rightInset === undefined ? undefined : `${rightInset}px`}
		>
			<a
				href="https://www.windmill.dev"
				target="_blank"
				rel="noopener"
				title="windmill.dev"
				aria-label="Windmill website"
				class="shrink-0 rounded focus-visible:outline focus-visible:outline-2 focus-visible:outline-offset-2"
			>
				<WindmillIcon height="20px" width="20px" />
			</a>
			<!-- `navHidden` pads the bar for a name that starts the row; here the icon does, so the
			     name sits as far from it as the sidebar's "Windmill / Home" does from its icon. -->
			<div class="-ml-2 min-w-0 flex-1">
				<PageHeaderBar navHidden />
			</div>
		</div>
		<!-- The bar draws its bottom edge only while something inside `#content` is scrolled
		     under it. -->
		<div id="content" class="min-h-0 flex-1">
			<SharedSessionBody
				loaded={shared.current}
				{workspace}
				{signInHref}
				previewActions={signInHref ? previewActions : undefined}
			/>
		</div>
	</div>
{/if}
