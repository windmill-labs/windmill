<script lang="ts">
	import { UserService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { workspaceStore } from '$lib/stores'
	import Button from '../common/button/Button.svelte'
	import Modal from '../common/modal/Modal.svelte'
	import ScopesPicker from './ScopesPicker.svelte'

	interface Props {
		open: boolean
		tokenPrefix?: string
		initialScopes?: string[]
		tokenWorkspaceId?: string
		onSaved?: () => void
	}

	let {
		open = $bindable(),
		tokenPrefix,
		initialScopes,
		tokenWorkspaceId,
		onSaved
	}: Props = $props()

	// Treat as MCP only when *all* existing scopes are mcp:* — mixed-scope or
	// null-scope tokens fall back to the standard picker so non-MCP scopes are
	// never silently dropped.
	const isMcp = $derived(
		(initialScopes ?? []).length > 0 && (initialScopes ?? []).every((s) => s.startsWith('mcp:'))
	)
	const mcpWorkspaceId = $derived(tokenWorkspaceId ?? $workspaceStore ?? '')

	let pickedScopes = $state<string[] | null>(null)
	let saving = $state(false)

	async function save() {
		if (!tokenPrefix) return
		saving = true
		try {
			await UserService.updateTokenScopes({
				tokenPrefix,
				requestBody: { scopes: pickedScopes }
			})
			sendUserToast('Token scopes updated')
			onSaved?.()
			open = false
		} catch (err) {
			sendUserToast(`Failed to update scopes: ${err.body ?? err.message}`, true)
		} finally {
			saving = false
		}
	}

	const saveDisabled = $derived(saving || !tokenPrefix || (isMcp && !pickedScopes))
</script>

<Modal bind:open title="Edit token scopes" class="!max-w-3xl">
	<div class="flex flex-col gap-3">
		<div class="text-xs text-secondary">
			Token <span class="font-mono">{tokenPrefix}****</span>
		</div>

		{#key tokenPrefix}
			<ScopesPicker
				mode={isMcp ? 'mcp' : 'standard'}
				workspaceId={mcpWorkspaceId}
				{initialScopes}
				bind:value={pickedScopes}
			/>
		{/key}
	</div>

	{#snippet actions()}
		<Button size="sm" variant="accent" disabled={saveDisabled} on:click={save}>Save</Button>
	{/snippet}
</Modal>
