<script lang="ts">
	import { page } from '$app/state'
	import { resource } from 'runed'
	import { CircleCheck, Link2Off, LogIn } from 'lucide-svelte'
	import { Button, CopyButton, EmptyState, Skeleton } from '$lib/components/common'
	import CenteredModal from '$lib/components/CenteredModal.svelte'
	import SecretRequestCard from '$lib/components/copilot/chat/SecretRequestCard.svelte'
	import {
		SecretCheckFailed,
		checkSubmission,
		saveSecretRequest
	} from '$lib/components/copilot/chat/secretRequest'
	import type {
		SecretRequest,
		SecretRequestOutcome,
		SecretSubmission
	} from '$lib/components/copilot/chat/shared'
	import { applyDarkModeVariant } from '$lib/darkModeVariant'
	import { AiService } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import { getUserExt } from '$lib/user'

	// The page an MCP client's `request_secret` link opens: the chat's card on a page of its
	// own, outside the app's navigation, drawn from the request the link's signed token carries.
	// It sets up what the logged-in layout otherwise would.
	const darkMode =
		window.localStorage.getItem('dark-mode') ??
		(window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')
	document.documentElement.classList.toggle('dark', darkMode === 'dark')
	applyDarkModeVariant()

	// The link fixes its workspace: the page offers no other. The token names it too, read
	// unverified here only to know where to ask when the parameter is lost; the server checks it.
	const token = page.url.searchParams.get('token') ?? ''
	const workspace = page.url.searchParams.get('workspace') || tokenWorkspace(token)

	function tokenWorkspace(token: string): string {
		try {
			const payload = token.split('.')[1].replace(/-/g, '+').replace(/_/g, '/')
			const claims = JSON.parse(atob(payload))
			return typeof claims.w === 'string' ? claims.w : ''
		} catch {
			return ''
		}
	}
	workspaceStore.set(workspace)
	// The card's path picker reads the user of the navigation workspace, which only the app's
	// layout loads otherwise.
	if (workspace) void getUserExt(workspace).then((user) => userStore.set(user))

	type Loaded =
		| { state: 'found'; request: SecretRequest }
		| { state: 'signin' }
		| { state: 'error'; message: string }

	const opened = resource(
		() => ({ workspace, token }),
		async ({ workspace, token }): Promise<Loaded> => {
			if (!workspace)
				return {
					state: 'error',
					message: 'This link is incomplete: open it again from your AI assistant, in full.'
				}
			try {
				const request = await AiService.openSecretRequest({ workspace, requestBody: { token } })
				return { state: 'found', request: request as unknown as SecretRequest }
			} catch (err) {
				if ((err as { status?: number })?.status === 401) return { state: 'signin' }
				const body = (err as { body?: unknown })?.body
				return { state: 'error', message: typeof body === 'string' ? body : String(err) }
			}
		}
	)

	const loginHref = `/user/login?rd=${encodeURIComponent(page.url.pathname + page.url.search)}`

	const own = new Set<string>()
	// Saved, but its connection test failed: kept if the user dismisses the card.
	let saved: SecretRequestOutcome | undefined = undefined
	let done = $state<SecretRequestOutcome | 'dismissed'>()

	async function save(request: SecretRequest, submission: SecretSubmission) {
		const outcome = await saveSecretRequest(request, submission, own)
		const check = await checkSubmission(request, submission)
		const full = check ? { ...outcome, check } : outcome
		if (check && !check.ok) {
			saved = full
			throw new SecretCheckFailed(check.error)
		}
		done = full
	}
</script>

<CenteredModal
	title="Secure input"
	subtitle={workspace ? `Workspace ${workspace}` : undefined}
	centerVertically={false}
	showVersion={false}
	box={false}
>
	<div class="w-full">
		{#if opened.current?.state === 'signin'}
			<EmptyState
				icon={LogIn}
				title="Sign in to enter the secret"
				description="The link opens for the Windmill user who asked for it."
			>
				<Button variant="accent" unifiedSize="md" href={loginHref}>Sign in</Button>
			</EmptyState>
		{:else if opened.current?.state === 'error'}
			<EmptyState
				icon={Link2Off}
				title="This link does not open"
				description={opened.current.message}
			/>
		{:else if opened.current?.state === 'found'}
			{@const request = opened.current.request}
			{#if done === 'dismissed'}
				<EmptyState
					icon={Link2Off}
					title="Nothing was saved"
					description="You can close this page and tell your AI assistant you dismissed the request."
				/>
			{:else if done}
				{@const reply = `It is saved${done.path !== request.path ? `, at ${done.path}` : ''}.`}
				<div class="flex flex-col items-center gap-2 py-8 text-center">
					<CircleCheck class="text-green-600 dark:text-green-400" size={28} />
					<p class="text-sm font-medium text-emphasis">
						Saved {request.label} as a draft of the {request.resource
							? 'resource'
							: 'secret variable'} <span class="font-mono">{done.path}</span>
					</p>
					{#if request.resource}
						<p class="text-xs text-secondary">
							Its secrets are in drafts of the encrypted variables <span class="font-mono"
								>{done.variables.join(', ')}</span
							>.
						</p>
					{/if}
					<p class="text-xs text-secondary"> Scripts and flows can use it once it is deployed. </p>
					{#if done.check}
						<p class="text-xs text-secondary">
							{done.check.ok
								? 'The connection test passed.'
								: `The connection test failed: ${done.check.error}`}
						</p>
					{/if}
					<p class="text-xs text-secondary">
						You can close this page and tell your AI assistant:
					</p>
					<div
						class="flex items-center gap-1 rounded-md border bg-surface-secondary py-1 pl-3 pr-1 text-xs text-primary"
					>
						<span>{reply}</span>
						<CopyButton value={reply} title="Copy the message" />
					</div>
				</div>
			{:else}
				<SecretRequestCard
					{request}
					save={(submission) => save(request, submission)}
					onDismiss={() => (done = saved ?? 'dismissed')}
				/>
			{/if}
		{:else}
			<Skeleton layout={[[16], 1, [4], 1, [4]]} />
		{/if}
	</div>
</CenteredModal>
