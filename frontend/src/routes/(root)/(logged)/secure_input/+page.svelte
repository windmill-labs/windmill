<script lang="ts">
	import { page } from '$app/state'
	import { resource } from 'runed'
	import { CircleCheck, Link2Off } from 'lucide-svelte'
	import { EmptyState, Skeleton } from '$lib/components/common'
	import SecretRequestCard from '$lib/components/copilot/chat/SecretRequestCard.svelte'
	import {
		SecretCheckFailed,
		resourcePageUrl,
		saveSecretRequest,
		variablePageUrl
	} from '$lib/components/copilot/chat/secretRequest'
	import type {
		SecretRequest,
		SecretRequestOutcome,
		SecretSubmission
	} from '$lib/components/copilot/chat/shared'
	import { checkSavedResource } from '$lib/components/resourceConnectionTest'
	import { AiService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'

	// The page an MCP client's `request_secret` link opens: the chat's card, drawn from the
	// request the link's signed token carries.
	const token = $derived(page.url.searchParams.get('token') ?? '')
	// The link carries its workspace; the store only catches up once the layout applies it.
	const workspace = $derived(page.url.searchParams.get('workspace') ?? $workspaceStore)

	type Loaded = { state: 'found'; request: SecretRequest } | { state: 'error'; message: string }

	const opened = resource(
		() => ({ workspace, token }),
		async ({ workspace, token }): Promise<Loaded | undefined> => {
			if (!workspace) return undefined
			if (!token) return { state: 'error', message: 'This link has no request in it.' }
			try {
				const request = await AiService.openSecretRequest({ workspace, requestBody: { token } })
				return { state: 'found', request: request as unknown as SecretRequest }
			} catch (err) {
				const body = (err as { body?: unknown })?.body
				return { state: 'error', message: typeof body === 'string' ? body : String(err) }
			}
		}
	)

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

<div class="mx-auto flex w-full max-w-2xl flex-col gap-4 px-4 py-10">
	{#if opened.current?.state === 'error'}
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
				description="Tell your AI assistant you dismissed the request."
			/>
		{:else if done}
			{@const href = request.resource
				? resourcePageUrl(done.path, request.workspace)
				: variablePageUrl(done.path, request.workspace)}
			<div class="flex flex-col items-center gap-2 py-8 text-center">
				<CircleCheck class="text-green-600 dark:text-green-400" size={28} />
				<p class="text-sm font-medium text-emphasis">
					{request.resource ? 'Connected' : 'Saved'}
					{request.label} as
					<a {href} class="font-mono">{done.path}</a>
				</p>
				{#if done.check}
					<p class="text-xs text-secondary">
						{done.check.ok
							? 'The connection test passed.'
							: `The connection test failed: ${done.check.error}`}
					</p>
				{/if}
				<p class="text-xs text-secondary">
					You can close this page and tell your AI assistant it is saved{done.path !== request.path
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
