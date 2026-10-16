<script lang="ts">
	import { untrack } from 'svelte'
	import { UserService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import Button from '../common/button/Button.svelte'
	import Modal from '../common/modal/Modal.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import ScopesPicker from './ScopesPicker.svelte'
	import McpAccessPicker from '../mcp/McpAccessPicker.svelte'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'

	const operatingWorkspace = useOperatingWorkspace()

	interface Props {
		open: boolean
		tokenPrefix?: string
		initialLabel?: string
		/** When false, the label is shown read-only (system tokens — see `isUserToken`). */
		labelEditable?: boolean
		initialScopes?: string[]
		tokenWorkspaceId?: string
		onSaved?: () => void
	}

	let {
		open = $bindable(),
		tokenPrefix,
		initialLabel,
		labelEditable = true,
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
	const mcpWorkspaceId = $derived(tokenWorkspaceId ?? $operatingWorkspace ?? '')

	let labelValue = $state('')
	let pickedScopes = $state<string[] | null>(null)
	let mcpScopes = $state<string[]>([])
	let saving = $state(false)

	// The access picker knows nothing of the workspace list, so it is carried over as is:
	// dropping it would widen the token to every workspace.
	const isWorkspaceList = (s: string) => s.startsWith('mcp:workspaces:')
	const scopesToSave = $derived(
		isMcp ? [...mcpScopes, ...(initialScopes ?? []).filter(isWorkspaceList)] : pickedScopes
	)

	// Seed the label field from the token whenever the modal opens or the token
	// identity changes (untracked read so typing doesn't reset it). ScopesPicker
	// seeds itself via the `{#key tokenPrefix}` remount below.
	$effect(() => {
		tokenPrefix
		if (open) {
			untrack(() => {
				labelValue = initialLabel ?? ''
			})
		}
	})

	function scopesEqual(a: string[] | null, b: string[] | null): boolean {
		if (a === null || b === null) return a === b
		if (a.length !== b.length) return false
		const sa = [...a].sort()
		const sb = [...b].sort()
		return sa.every((s, i) => s === sb[i])
	}

	async function save() {
		if (!tokenPrefix) return
		saving = true
		try {
			const trimmed = labelValue.trim()
			const labelChanged = labelEditable && trimmed !== (initialLabel ?? '')
			const scopesChanged = !scopesEqual(scopesToSave, initialScopes ?? null)

			if (labelChanged) {
				await UserService.updateTokenLabel({
					tokenPrefix,
					requestBody: { label: trimmed === '' ? null : trimmed }
				})
			}
			if (scopesChanged) {
				await UserService.updateTokenScopes({
					tokenPrefix,
					requestBody: { scopes: scopesToSave }
				})
			}
			if (labelChanged || scopesChanged) {
				sendUserToast('Token updated')
			}
			onSaved?.()
			open = false
		} catch (err) {
			sendUserToast(`Failed to update token: ${err.body ?? err.message}`, true)
		} finally {
			saving = false
		}
	}

	const saveDisabled = $derived(saving || !tokenPrefix || (isMcp && mcpScopes.length === 0))
</script>

<Modal bind:open title="Edit token" class="!max-w-3xl">
	<div class="flex flex-col gap-3">
		<div class="text-xs text-secondary">
			Token <span class="font-mono">{tokenPrefix}****</span>
		</div>

		<div class="flex flex-col gap-1">
			<span class="text-xs font-semibold text-emphasis">Label</span>
			<TextInput
				inputProps={{ type: 'text', disabled: !labelEditable, placeholder: 'Add a label...' }}
				bind:value={labelValue}
				class="w-full"
			/>
			{#if !labelEditable}
				<span class="text-2xs text-tertiary">System token labels can't be changed.</span>
			{/if}
		</div>

		<div class="flex flex-col gap-1">
			{#if !isMcp}
				<span class="text-xs font-semibold text-emphasis">Scopes</span>
			{/if}
			{#key tokenPrefix}
				{#if isMcp}
					<McpAccessPicker
						workspaceId={mcpWorkspaceId}
						initialScopes={(initialScopes ?? []).filter((s) => !isWorkspaceList(s))}
						bind:scopes={mcpScopes}
					/>
				{:else}
					<ScopesPicker
						mode="standard"
						workspaceId={mcpWorkspaceId}
						{initialScopes}
						bind:value={pickedScopes}
					/>
				{/if}
			{/key}
		</div>
	</div>

	{#snippet actions()}
		<Button size="sm" variant="accent" disabled={saveDisabled} on:click={save}>Save</Button>
	{/snippet}
</Modal>
