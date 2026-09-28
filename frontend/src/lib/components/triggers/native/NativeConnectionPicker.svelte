<script lang="ts">
	import { WorkspaceIntegrationService } from '$lib/gen'
	import type { NativeServiceName, NativeTriggerConnection } from '$lib/gen/types.gen'
	import { Button } from '$lib/components/common'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import { sendUserToast } from '$lib/toast'
	import { Plug } from 'lucide-svelte'
	import { useOperatingWorkspace } from '$lib/components/operatingWorkspace.svelte'
	import { connectNativeAccount, getServiceConfig } from './utils'

	interface Props {
		service: NativeServiceName
		value: string | undefined
		/** An existing trigger keeps the connection its webhook was registered under. */
		locked?: boolean
		disabled?: boolean
	}

	let { service, value = $bindable(), locked = false, disabled = false }: Props = $props()

	const operatingWorkspace = useOperatingWorkspace()
	const displayName = $derived(getServiceConfig(service)?.serviceDisplayName ?? service)

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
		if (!locked && value === undefined && connections.length > 0) {
			value = connections[0].path
		}
	}

	$effect(() => {
		service
		$operatingWorkspace
		load()
	})

	async function connect() {
		connecting = true
		try {
			const path = await connectNativeAccount($operatingWorkspace!, service)
			await load()
			value = path
			sendUserToast(`${displayName} account connected`)
		} catch (err: any) {
			sendUserToast(`Could not connect ${displayName}: ${err.body ?? err.message}`, true)
		} finally {
			connecting = false
		}
	}

	const items = $derived.by(() => {
		const list = (connections ?? []).map((c) => ({ label: c.path, value: c.path }))
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
	<div class="flex flex-row gap-2 items-center">
		<Select
			class="grow"
			{items}
			bind:value
			disabled={disabled || locked}
			loading={connections === undefined}
			placeholder="Connect an account to create this trigger"
			noItemsMsg="No {displayName} account connected yet"
		/>
		{#if !locked}
			<Button
				unifiedSize="md"
				variant="default"
				startIcon={{ icon: Plug }}
				onclick={connect}
				disabled={disabled || connecting}
				loading={connecting}
			>
				Connect account
			</Button>
		{/if}
	</div>
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
