<script lang="ts">
	import { page } from '$app/state'
	import { resource } from 'runed'
	import { CircleCheck, Link2Off, LogIn } from 'lucide-svelte'
	import { Button, EmptyState, Skeleton } from '$lib/components/common'
	import WindmillIcon from '$lib/components/icons/WindmillIcon.svelte'
	import SecretRequestCard from '$lib/components/copilot/chat/SecretRequestCard.svelte'
	import { SecretCheckFailed, saveSecretRequest } from '$lib/components/copilot/chat/secretRequest'
	import type {
		SecretRequest,
		SecretRequestOutcome,
		SecretSubmission
	} from '$lib/components/copilot/chat/shared'
	import { checkSavedResource } from '$lib/components/resourceConnectionTest'
	import { applyDarkModeVariant } from '$lib/darkModeVariant'
	import { AiService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'

	// The page an MCP client's `request_secret` link opens: the chat's card on a page of its
	// own, outside the app's navigation, drawn from the request the link's signed token carries.
	// It sets up what the logged-in layout otherwise would.
	const darkMode =
		window.localStorage.getItem('dark-mode') ??
		(window.matchMedia('(prefers-color-scheme: dark)').matches ? 'dark' : 'light')
	document.documentElement.classList.toggle('dark', darkMode === 'dark')
	applyDarkModeVariant()

	// The link fixes its workspace: the page offers no other.
	const workspace = page.url.searchParams.get('workspace') ?? ''
	const token = page.url.searchParams.get('token') ?? ''
	workspaceStore.set(workspace)

	type Loaded =
		| { state: 'found'; request: SecretRequest }
		| { state: 'signin' }
		| { state: 'error'; message: string }

	const opened = resource(
		() => ({ workspace, token }),
		async ({ workspace, token }): Promise<Loaded> => {
			if (!workspace || !token) return { state: 'error', message: 'This link is incomplete.' }
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
		const check = request.resource
			? await checkSavedResource({
					workspace: request.workspace,
					path: outcome.path,
					resourceType: request.resource.type,
					value: submission.fields
				})
			: undefined
		const full = check ? { ...outcome, check } : outcome
		if (check && !check.ok) {
			saved = full
			throw new SecretCheckFailed(check.error)
		}
		done = full
	}
</script>

<div class="flex min-h-screen flex-col bg-surface">
	<div class="border-b bg-surface-secondary">
		<div class="mx-auto flex w-full max-w-2xl items-center gap-3 px-4 py-3">
			<WindmillIcon height="20px" width="20px" />
			<span class="text-sm font-semibold text-emphasis">Secure input</span>
			{#if workspace}
				<span class="text-xs text-secondary">· {workspace}</span>
			{/if}
		</div>
	</div>

	<div class="mx-auto w-full max-w-2xl flex-1 px-4 py-8">
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
				<div class="flex flex-col items-center gap-2 py-8 text-center">
					<CircleCheck class="text-green-600 dark:text-green-400" size={28} />
					<p class="text-sm font-medium text-emphasis">
						{request.resource ? 'Connected' : 'Saved'}
						{request.label} as <span class="font-mono">{done.path}</span>
					</p>
					{#if done.check}
						<p class="text-xs text-secondary">
							{done.check.ok
								? 'The connection test passed.'
								: `The connection test failed: ${done.check.error}`}
						</p>
					{/if}
					<p class="text-xs text-secondary">
						You can close this page and tell your AI assistant it is saved{done.path !==
						request.path
							? `, at ${done.path}`
							: ''}.
					</p>
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
</div>
