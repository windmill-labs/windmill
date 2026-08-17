<script lang="ts">
	import type { CustomInstanceDbTag, ListCustomInstanceDbsResponse } from '$lib/gen'
	import type { ResourceReturn } from 'runed'
	import Select from '../select/Select.svelte'
	import { safeSelectItems } from '../select/utils.svelte'
	import { isCustomInstanceDbEnabled } from './utils.svelte'
	import Button from '../common/button/Button.svelte'
	import CustomInstanceDbWizardModal from './CustomInstanceDbWizardModal.svelte'
	import { ArrowRight, TriangleAlert } from 'lucide-svelte'
	import type { ConfirmationModalHandle } from '../common/confirmationModal/asyncConfirmationModal.svelte'
	import type { Snippet } from 'svelte'
	import Tooltip from '../meltComponents/Tooltip.svelte'
	import { workspaceStore } from '$lib/stores'

	type Props = {
		value: string | undefined
		customInstanceDbs: ResourceReturn<ListCustomInstanceDbsResponse>
		confirmationModal: ConfirmationModalHandle
		wizardBottomHint?: Snippet | undefined
		class?: string
		tag?: CustomInstanceDbTag
	}
	let {
		value = $bindable(),
		customInstanceDbs,
		confirmationModal,
		wizardBottomHint,
		class: className,
		tag
	}: Props = $props()

	let openedDbNameWizard: string | undefined = $state(undefined)

	let onlySelectedTags = $derived(
		safeSelectItems(
			Object.entries(customInstanceDbs.current ?? {})
				.filter(([_, db]) => !tag || db.tag === tag)
				.map(([name, _]) => name)
		)
	)
	let open = $state(false)

	function otherWorkspaces(dbname: string): string[] {
		const all = customInstanceDbs.current?.[dbname]?.used_by_workspaces ?? []
		return all.filter((w) => w !== $workspaceStore)
	}
</script>

<div class="flex relative items-center {className}">
	<Select
		class="flex-1"
		bind:open
		bind:value
		onCreateItem={(i) => (value = i)}
		placeholder="Search or create..."
		showPlaceholderOnOpen
		items={onlySelectedTags}
		id="custom-instance-db-select"
		disabled={!$isCustomInstanceDbEnabled}
	>
		{#snippet endSnippet({ item })}
			<div class="flex items-center gap-1">
				{@render sharedWorkspacesWarning(item.value)}
				{@render customInstanceDbWizardButton(item.value)}
			</div>
		{/snippet}
	</Select>
	{#if value}
		<div class="absolute right-1.5 flex items-center gap-1">
			{@render sharedWorkspacesWarning(value)}
			{@render customInstanceDbWizardButton(value)}
		</div>
	{/if}
</div>

<CustomInstanceDbWizardModal
	{customInstanceDbs}
	{confirmationModal}
	{tag}
	bottomHint={wizardBottomHint}
	bind:opened={
		() =>
			openedDbNameWizard
				? { dbname: openedDbNameWizard, status: customInstanceDbs.current?.[openedDbNameWizard] }
				: undefined,
		(v) => !v && (openedDbNameWizard = undefined)
	}
/>

{#snippet customInstanceDbWizardButton(dbname: string)}
	{@const status = customInstanceDbs.current?.[dbname]}
	<Button
		spacingSize="xs2"
		variant="default"
		wrapperClasses="bg-surface-input h-6 -my-2"
		onClick={() => ((openedDbNameWizard = dbname), (open = false))}
	>
		{#if !status}
			<span class="text-yellow-600 dark:text-yellow-400">
				Setup <ArrowRight class="inline" size={14} />
			</span>
		{:else if !status.success}
			<span class="text-red-400 flex gap-1">
				Error <TriangleAlert class="inline" size={16} />
			</span>
		{:else}
			<div class="w-1.5 h-1.5 rounded-full bg-green-400"></div>
		{/if}
	</Button>
{/snippet}

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
