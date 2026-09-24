<script lang="ts">
	import type { CustomInstanceDbTag, ListExternalInstancePgDatabasesResponse } from '$lib/gen'
	import { SettingService } from '$lib/gen'
	import type { ResourceReturn } from 'runed'
	import Select from '../select/Select.svelte'
	import { safeSelectItems } from '../select/utils.svelte'
	import Button from '../common/button/Button.svelte'
	import { CirclePlus, TriangleAlert } from 'lucide-svelte'
	import Tooltip from '../meltComponents/Tooltip.svelte'
	import { workspaceStore } from '$lib/stores'
	import { sendUserToast } from '$lib/toast'

	type Props = {
		value: string | undefined
		externalInstanceDbs: ResourceReturn<ListExternalInstancePgDatabasesResponse>
		tag: CustomInstanceDbTag
		class?: string
	}
	let { value = $bindable(), externalInstanceDbs, tag, class: className }: Props = $props()

	let creating = $state(false)
	let open = $state(false)

	let items = $derived(
		safeSelectItems(
			Object.entries(externalInstanceDbs.current ?? {})
				.filter(([_, db]) => db.tag === tag)
				.map(([name, _]) => name)
		)
	)

	// A name typed into the select is not a database yet: the cluster only lends databases
	// Windmill created there, so it has to be created before a save can name it.
	async function createDatabase(name: string) {
		creating = true
		try {
			await SettingService.createExternalInstancePgDatabase({ name, requestBody: { tag } })
			await externalInstanceDbs.refetch()
			value = name
			sendUserToast(`Database ${name} created on the external cluster`)
		} catch (e) {
			sendUserToast(e?.body ?? e?.message ?? String(e), true)
		} finally {
			creating = false
		}
	}

	function otherWorkspaces(dbname: string): string[] {
		const all = externalInstanceDbs.current?.[dbname]?.used_by_workspaces ?? []
		return all.filter((w) => w !== $workspaceStore)
	}

	let unknownName = $derived(
		value !== undefined && value !== '' && externalInstanceDbs.current?.[value] === undefined
	)
</script>

<div class="flex relative items-center {className}">
	<Select
		class="flex-1"
		bind:open
		bind:value
		onCreateItem={(i) => (value = i)}
		placeholder="Search or create..."
		showPlaceholderOnOpen
		{items}
		id="external-instance-db-select"
	>
		{#snippet endSnippet({ item })}
			{@render sharedWorkspacesWarning(item.value)}
		{/snippet}
	</Select>
	{#if value}
		<div class="absolute right-1.5 flex items-center gap-1">
			{#if unknownName}
				<Button
					unifiedSize="2xs"
					variant="default"
					wrapperClasses="bg-surface-input h-6 -my-2"
					startIcon={{ icon: CirclePlus }}
					disabled={creating}
					onClick={() => ((open = false), createDatabase(value!))}
				>
					Create
				</Button>
			{:else}
				{@render sharedWorkspacesWarning(value)}
				<div class="w-1.5 h-1.5 rounded-full bg-green-400"></div>
			{/if}
		</div>
	{/if}
</div>

{#snippet sharedWorkspacesWarning(dbname: string)}
	{@const others = otherWorkspaces(dbname)}
	{#if others.length > 0}
		<Tooltip placement="top">
			<TriangleAlert
				class="text-orange-500 dark:text-orange-400"
				size={16}
				aria-label="Database is shared with other workspaces"
			/>
			{#snippet text()}
				This database is also used by workspace{others.length > 1 ? 's' : ''}
				<span class="font-semibold">{others.join(', ')}</span>. Any data written here will be shared
				with {others.length > 1 ? 'them' : 'it'}.
			{/snippet}
		</Tooltip>
	{/if}
{/snippet}
