<script lang="ts">
	import { WorkspaceIntegrationService } from '$lib/gen'
	import type { NativeServiceName, NativeTriggerConnection } from '$lib/gen/types.gen'
	import { Button } from '$lib/components/common'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { createAsyncConfirmationModal } from '$lib/components/common/confirmationModal/asyncConfirmationModal.svelte'
	import { sendUserToast } from '$lib/toast'
	import { Plus, Unplug } from 'lucide-svelte'
	import { userStore } from '$lib/stores'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'
	import { connectNativeAccount, getServiceConfig } from './utils'

	interface Props {
		service: NativeServiceName
		value: string | undefined
		/** An existing trigger keeps the connection its webhook was registered under. */
		locked?: boolean
		disabled?: boolean
		/** Only offer the caller's own connections. */
		ownOnly?: boolean
		/** Select the first connection when none is chosen yet. */
		autoSelect?: boolean
		clearable?: boolean
		placeholder?: string
		/** Called instead of setting `value` directly when the user picks or clears a connection. */
		onSelect?: (path: string | undefined) => void
		/** Called after an account is connected or disconnected from the picker. */
		onConnectionsChange?: () => void
	}

	let {
		service,
		value = $bindable(),
		locked = false,
		disabled = false,
		ownOnly = false,
		autoSelect = true,
		clearable = false,
		placeholder,
		onSelect,
		onConnectionsChange
	}: Props = $props()

	const operatingWorkspace = useOperatingWorkspace()
	const displayName = $derived(getServiceConfig(service)?.serviceDisplayName ?? service)
	const confirmationModal = createAsyncConfirmationModal()

	let connections = $state<NativeTriggerConnection[] | undefined>(undefined)
	let connecting = $state(false)

	async function load() {
		try {
			connections = await WorkspaceIntegrationService.listNativeTriggerConnections({
				workspace: $operatingWorkspace!,
				serviceName: service
			})
		} catch (err: any) {
			connections = []
			sendUserToast(`Failed to load ${displayName} connections: ${err.body ?? err.message}`, true)
		}
		if (autoSelect && !locked && value === undefined && offered.length > 0) {
			select(offered[0].path)
		}
	}

	$effect(() => {
		service
		$operatingWorkspace
		load()
	})

	function select(path: string | undefined) {
		if (onSelect) onSelect(path)
		else value = path
	}

	async function connect() {
		connecting = true
		try {
			const path = await connectNativeAccount($operatingWorkspace!, service)
			await load()
			sendUserToast(`${displayName} account connected`)
			onConnectionsChange?.()
			select(path)
		} catch (err: any) {
			sendUserToast(`Could not connect ${displayName}: ${err.body ?? err.message}`, true)
		} finally {
			connecting = false
		}
	}

	async function disconnect(path: string) {
		const confirmed = await confirmationModal.ask({
			title: `Disconnect ${path}?`,
			confirmationText: 'Disconnect',
			children: `This deletes the ${displayName} triggers created with this account. This action cannot be undone.`
		})
		if (!confirmed) return
		try {
			await WorkspaceIntegrationService.deleteNativeTriggerConnection({
				workspace: $operatingWorkspace!,
				serviceName: service,
				path
			})
			sendUserToast(`${path} disconnected`)
			if (value === path) select(undefined)
			await load()
			onConnectionsChange?.()
		} catch (err: any) {
			sendUserToast(`Failed to disconnect ${path}: ${err.body ?? err.message}`, true)
		}
	}

	// Members cannot write the workspace connection; the server refuses anyone else it does not own.
	function canDisconnect(c: NativeTriggerConnection): boolean {
		if ($userStore?.is_admin) return true
		return !c.workspace && !!c.owner && c.owner === $userStore?.username
	}

	const offered = $derived(
		(connections ?? []).filter((c) => !ownOnly || c.owner === $userStore?.username)
	)
	const items = $derived.by(() => {
		const list = offered.map((c) => ({
			label: c.workspace && !ownOnly ? `${c.path} (workspace connection)` : c.path,
			value: c.path
		}))
		if (value && !list.some((i) => i.value === value)) {
			list.push({ label: value, value })
		}
		return list
	})
	const unusable = $derived(
		!ownOnly && connections !== undefined && !!value && !connections.some((c) => c.path === value)
	)
</script>

<div class="flex flex-col gap-2">
	<Select
		{items}
		bind:value={() => value, (v) => select(v)}
		onClear={() => select(undefined)}
		{clearable}
		disabled={disabled || locked}
		loading={connections === undefined || connecting}
		placeholder={placeholder ?? `Connect a ${displayName} account to create this trigger`}
		noItemsMsg="No {displayName} account connected yet"
	>
		{#snippet endSnippet({ item, close })}
			{@const connection = connections?.find((c) => c.path === item.value)}
			{#if connection && canDisconnect(connection)}
				<Button
					unifiedSize="xs"
					variant="subtle"
					iconOnly
					startIcon={{ icon: Unplug }}
					title="Disconnect"
					onclick={() => (close(), disconnect(connection.path))}
				/>
			{/if}
		{/snippet}
		{#snippet bottomSnippet({ close })}
			<div class="flex bg-surface border-t">
				<Button
					unifiedSize="md"
					variant="subtle"
					wrapperClasses="flex-1"
					btnClasses="rounded-none"
					startIcon={{ icon: Plus }}
					disabled={disabled || connecting}
					onclick={() => (close(), connect())}
				>
					Connect a {displayName} account
				</Button>
			</div>
		{/snippet}
	</Select>
	{#if locked}
		<p class="text-2xs text-secondary">
			The webhook is registered under this account. To use another one, create a new trigger.
		</p>
	{/if}
	{#if unusable}
		<Alert type="warning" title="You cannot use this connection" size="xs">
			Saving acts as the account at {value}. Ask its owner for read access to it, or create a new
			trigger with your own account.
		</Alert>
	{/if}
</div>

<ConfirmationModal {...confirmationModal.props} />
