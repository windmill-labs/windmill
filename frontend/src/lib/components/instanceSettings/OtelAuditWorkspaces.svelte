<script lang="ts">
	import { WorkspaceService } from '$lib/gen'
	import MultiSelect from '$lib/components/select/MultiSelect.svelte'
	import { safeSelectItems } from '$lib/components/select/utils.svelte'
	import { resource } from 'runed'

	interface Props {
		/** Empty exports every workspace's audit events. Cleared to `[]`, not unset: unset
		 *  falls back to the OTEL_AUDIT_LOGS_WORKSPACES environment variable. */
		value: string[] | undefined
		disabled?: boolean
	}

	let { value = $bindable(), disabled = false }: Props = $props()

	const workspaces = resource(
		() => undefined,
		async () => {
			const all: string[] = []
			for (let page = 1; ; page++) {
				const batch = await WorkspaceService.listWorkspacesAsSuperAdmin({ page, perPage: 100 })
				all.push(...batch.map((w) => w.id))
				if (batch.length < 100) break
			}
			return all
		}
	)

	// Workspaces already listed stay selectable even when they no longer exist.
	let items = $derived(
		safeSelectItems([...new Set([...(workspaces.current ?? []), ...(value ?? [])])])
	)
</script>

<MultiSelect
	{items}
	{disabled}
	placeholder="All workspaces"
	bind:value={() => value ?? [], (v) => (value = v)}
/>
