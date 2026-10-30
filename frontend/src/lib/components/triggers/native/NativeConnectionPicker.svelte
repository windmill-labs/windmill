<script lang="ts">
	import { WorkspaceIntegrationService } from '$lib/gen'
	import type { NativeServiceName, NativeTriggerConnection } from '$lib/gen/types.gen'
	import { Button } from '$lib/components/common'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import AppConnectDrawer from '$lib/components/AppConnectDrawer.svelte'
	import ConfirmationModal from '$lib/components/common/confirmationModal/ConfirmationModal.svelte'
	import { createAsyncConfirmationModal } from '$lib/components/common/confirmationModal/asyncConfirmationModal.svelte'
	import { sendUserToast } from '$lib/toast'
	import { Plus, RefreshCw, Unplug } from 'lucide-svelte'
	import {
		useOperatingUser,
		useOperatingWorkspace
	} from '$lib/components/operatingWorkspace.svelte'
	import { connectNativeAccount, getServiceConfig } from './utils'

	interface Props {
		service: NativeServiceName
		value: string | undefined
		/** An existing trigger keeps the connection its webhook was registered under. */
		locked?: boolean
		disabled?: boolean
		/** Select the first connection when none is chosen yet. */
		autoSelect?: boolean
		clearable?: boolean
		/** Suffix the workspace connection's label, where it sits among personal ones. */
		labelWorkspace?: boolean
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
		autoSelect = true,
		clearable = false,
		labelWorkspace = true,
		placeholder,
		onSelect,
		onConnectionsChange
	}: Props = $props()

	const operatingWorkspace = useOperatingWorkspace()
	const operatingUser = useOperatingUser()
	const displayName = $derived(getServiceConfig(service)?.serviceDisplayName ?? service)
	const confirmationModal = createAsyncConfirmationModal()

	let connections = $state<NativeTriggerConnection[] | undefined>(undefined)
	let connecting = $state(false)
	let connectDrawer: AppConnectDrawer | undefined = $state(undefined)

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
		if (autoSelect && !locked && value === undefined && connections.length > 0) {
			select(connections[0].path)
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

	async function connect(path: string) {
		connecting = true
		try {
			path = await connectNativeAccount($operatingWorkspace!, service, path)
			await load()
			sendUserToast(`${displayName} account connected`)
			onConnectionsChange?.()
			select(path)
		} catch (err: any) {
			sendUserToast(`Could not connect ${displayName}: ${err.body ?? err.message}`, true)
			throw err
		} finally {
			connecting = false
		}
	}

	async function reconnect(c: NativeTriggerConnection) {
		if (c.owner && c.owner !== operatingUser.current?.username) {
			const confirmed = await confirmationModal.ask({
				title: `Reconnect ${c.path} with your account?`,
				confirmationText: 'Reconnect',
				children: `${c.path} will act as the ${displayName} account you sign in with, instead of ${c.owner}'s. Every trigger using it will act as you.`
			})
			if (!confirmed) return
		}
		await connect(c.path).catch(() => {})
	}

	async function disconnect(path: string) {
		const confirmed = await confirmationModal.ask({
			title: `Disconnect ${path}?`,
			confirmationText: 'Disconnect',
			children: `This deletes every ${displayName} trigger that uses this account, including ones other members created. This action cannot be undone.`
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
		if (operatingUser.current?.is_admin) return true
		return !c.workspace && !!c.owner && c.owner === operatingUser.current?.username
	}

	// The server lets only the connection's owner or an admin replace it.
	function canReconnect(c: NativeTriggerConnection): boolean {
		return (
			!!operatingUser.current?.is_admin ||
			(!!c.owner && c.owner === operatingUser.current?.username)
		)
	}

	const items = $derived.by(() => {
		const list = (connections ?? []).map((c) => ({
			label: c.workspace && labelWorkspace ? `${c.path} (shared account)` : c.path,
			value: c.path
		}))
		if (value && !list.some((i) => i.value === value)) {
			list.push({ label: value, value })
		}
		return list
	})
	const unusable = $derived(
		connections !== undefined && !!value && !connections.some((c) => c.path === value)
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
			{#if connection && canReconnect(connection)}
				<Button
					unifiedSize="xs"
					variant="subtle"
					startIcon={{ icon: RefreshCw }}
					disabled={connecting}
					onclick={() => (close(), reconnect(connection))}
				>
					Reconnect
				</Button>
			{/if}
			{#if connection && canDisconnect(connection)}
				<Button
					unifiedSize="xs"
					variant="subtle"
					destructive
					startIcon={{ icon: Unplug }}
					onclick={() => (close(), disconnect(connection.path))}
				>
					Disconnect
				</Button>
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
					onclick={() => (close(), connectDrawer?.open(getServiceConfig(service)?.resourceType))}
				>
					Connect a {displayName} account
				</Button>
			</div>
		{/snippet}
	</Select>
	{#if locked}
		{@const current = connections?.find((c) => c.path === value)}
		<div class="flex items-center justify-between gap-2">
			<p class="text-2xs text-secondary">
				The webhook is registered under this account. To use another one, create a new trigger.
			</p>
			{#if current && canReconnect(current)}
				<Button
					unifiedSize="xs"
					variant="subtle"
					startIcon={{ icon: RefreshCw }}
					disabled={connecting}
					onclick={() => reconnect(current)}
				>
					Reconnect
				</Button>
			{/if}
		</div>
	{/if}
	{#if unusable}
		<Alert type="warning" title="You cannot use this connection" size="xs">
			Saving acts as the account at {value}. Ask its owner for read access to it, or create a new
			trigger with your own account.
		</Alert>
	{/if}
</div>

<AppConnectDrawer
	bind:this={connectDrawer}
	native={{
		title: `Connect a ${displayName} account`,
		defaultName: `native_${service}`,
		connect
	}}
/>

<ConfirmationModal {...confirmationModal.props} />
