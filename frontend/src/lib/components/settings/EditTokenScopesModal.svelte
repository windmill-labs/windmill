<script lang="ts">
	import { untrack } from 'svelte'
	import { UserService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import Button from '../common/button/Button.svelte'
	import Modal from '../common/modal/Modal.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import ScopesPicker from './ScopesPicker.svelte'
	import McpAccessPicker, {
		MCP_SESSION,
		RUNNABLES_AS_TOOLS,
		mcpAccessOf
	} from '../mcp/McpAccessPicker.svelte'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'
	import { enterpriseLicense, usersWorkspaceStore } from '$lib/stores'
	import Select from '../select/Select.svelte'
	import { displayDate } from '$lib/utils'

	const operatingWorkspace = useOperatingWorkspace()

	interface Props {
		open: boolean
		tokenPrefix?: string
		initialLabel?: string
		/** When false, the label, expiration and allowed IPs are read-only (system tokens — see
		 * `isUserToken`). */
		labelEditable?: boolean
		initialScopes?: string[]
		tokenWorkspaceId?: string
		initialExpiration?: string
		initialAllowedIps?: string[]
		onSaved?: () => void
	}

	let {
		open = $bindable(),
		tokenPrefix,
		initialLabel,
		labelEditable = true,
		initialScopes,
		tokenWorkspaceId,
		initialExpiration,
		initialAllowedIps,
		onSaved
	}: Props = $props()

	const DAY_SECS = 24 * 60 * 60
	type ExpirationChoice = 'keep' | 'never' | number
	const expirationItems: { label: string; value: ExpirationChoice }[] = [
		{ label: 'Keep current', value: 'keep' },
		{ label: '1 hour from now', value: 60 * 60 },
		{ label: '1 day from now', value: DAY_SECS },
		{ label: '7 days from now', value: 7 * DAY_SECS },
		{ label: '30 days from now', value: 30 * DAY_SECS },
		{ label: '90 days from now', value: 90 * DAY_SECS },
		{ label: '180 days from now', value: 180 * DAY_SECS },
		{ label: '365 days from now', value: 365 * DAY_SECS },
		{ label: 'Never', value: 'never' }
	]
	let expirationChoice = $state<ExpirationChoice>('keep')
	let allowedIpsText = $state('')
	const parseIps = (text: string) =>
		text
			.split(/[\s,]+/)
			.map((s) => s.trim())
			.filter((s) => s.length > 0)

	// Treat as MCP only when *all* existing scopes are mcp:* — mixed-scope or
	// null-scope tokens fall back to the standard picker so non-MCP scopes are
	// never silently dropped.
	const isMcp = $derived(
		(initialScopes ?? []).length > 0 && (initialScopes ?? []).every((s) => s.startsWith('mcp:'))
	)
	const mcpWorkspaceId = $derived(tokenWorkspaceId ?? $operatingWorkspace ?? '')
	// The workspaces the token reaches: its own, its list, or every one its owner is in.
	const mcpReached = $derived.by(() => {
		if (tokenWorkspaceId) return [tokenWorkspaceId]
		const list = (initialScopes ?? []).find((s) => s.startsWith('mcp:workspaces:'))
		return list
			? list.slice('mcp:workspaces:'.length).split(',')
			: ($usersWorkspaceStore?.workspaces ?? []).map((w) => w.id)
	})

	let labelValue = $state('')
	let pickedScopes = $state<string[] | null>(null)
	let mcpScopes = $state<string[]>([])
	let saving = $state(false)

	// The access picker re-derives a token's scopes from what it can show, which drops what it
	// does not model (an old token's endpoint list, say). So they are rewritten only once the
	// user has edited a selection or picked another access; until then the token keeps exactly
	// the scopes it had, and a plain `mcp:all` keeps the endpoint tools.
	let mcpTouched = $state(false)
	const mcpEdited = $derived.by(() => {
		const access = mcpAccessOf(mcpScopes)
		return mcpTouched && (access === 'selected' || access !== mcpAccessOf(initialScopes ?? []))
	})
	// An unedited older token (a plain `mcp:all` or an endpoint selection) keeps the endpoint tools
	// on the per-workspace URLs, which the picker's levels do not describe, so it gets no warning.
	const describedByPicker = (s: string) =>
		[MCP_SESSION, RUNNABLES_AS_TOOLS, 'mcp:read_only', 'mcp:read_run'].includes(s)
	const warnsFor = $derived(
		mcpEdited || (initialScopes ?? []).some(describedByPicker) ? mcpReached : []
	)
	// Carried over as is, the picker knowing nothing of them: dropping the workspace list
	// would widen the token to every workspace, dropping `mcp:session` would move a sign-in
	// back to the endpoint tools once it is given `mcp:all` again.
	const isWorkspaceList = (s: string) => s.startsWith('mcp:workspaces:')
	const keptAside = (s: string) =>
		isWorkspaceList(s) ||
		s === MCP_SESSION ||
		(mcpAccessOf(mcpScopes) !== 'selected' && s.startsWith('mcp:hub:'))
	const scopesToSave = $derived(
		isMcp
			? mcpEdited
				? [...new Set([...mcpScopes, ...(initialScopes ?? []).filter(keptAside)])]
				: (initialScopes ?? [])
			: pickedScopes
	)

	// Seed the label field from the token whenever the modal opens or the token
	// identity changes (untracked read so typing doesn't reset it). ScopesPicker
	// seeds itself via the `{#key tokenPrefix}` remount below.
	$effect(() => {
		tokenPrefix
		if (open) {
			untrack(() => {
				labelValue = initialLabel ?? ''
				mcpTouched = false
				expirationChoice = 'keep'
				allowedIpsText = (initialAllowedIps ?? []).join(', ')
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
			const limits: { expiration?: string | null; allowed_ips?: string[] | null } = {}
			if (labelEditable && expirationChoice !== 'keep') {
				limits.expiration =
					expirationChoice === 'never'
						? null
						: new Date(Date.now() + expirationChoice * 1000).toISOString()
			}
			const ips = parseIps(allowedIpsText)
			if (labelEditable && $enterpriseLicense && !scopesEqual(ips, initialAllowedIps ?? [])) {
				limits.allowed_ips = ips.length > 0 ? ips : null
			}
			const limitsChanged = Object.keys(limits).length > 0
			if (limitsChanged) {
				await UserService.updateTokenLimits({ tokenPrefix, requestBody: limits })
			}
			if (labelChanged || scopesChanged || limitsChanged) {
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

		{#if labelEditable}
			<div class="flex flex-col gap-1">
				<span class="text-xs font-semibold text-emphasis">Expiration</span>
				<span class="text-2xs text-secondary">
					Currently {initialExpiration ? displayDate(initialExpiration) : 'never expires'}
				</span>
				<Select bind:value={expirationChoice} inputClass="w-full" items={expirationItems} />
				<p class="text-2xs text-hint">
					A later expiration is shortened to this instance's maximum token lifetime, if it has one.
				</p>
			</div>

			{#if $enterpriseLicense}
				<div class="flex flex-col gap-1">
					<span class="text-xs font-semibold text-emphasis">Allowed IPs</span>
					<TextInput
						inputProps={{ type: 'text', placeholder: '203.0.113.4, 10.0.0.0/8' }}
						bind:value={allowedIpsText}
						class="w-full"
					/>
					<p class="text-2xs text-hint">
						IP addresses or CIDR ranges, separated by commas. Leave empty to allow any.
					</p>
				</div>
			{/if}
		{/if}

		<div class="flex flex-col gap-1">
			{#if !isMcp}
				<span class="text-xs font-semibold text-emphasis">Scopes</span>
			{/if}
			{#key tokenPrefix}
				{#if isMcp}
					<div
						role="group"
						onpointerdown={() => (mcpTouched = true)}
						onkeydown={() => (mcpTouched = true)}
					>
						<McpAccessPicker
							workspaceId={mcpWorkspaceId}
							workspaces={warnsFor}
							initialScopes={(initialScopes ?? []).filter((s) => !isWorkspaceList(s))}
							bind:scopes={mcpScopes}
						/>
					</div>
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
