<script lang="ts">
	import type {
		CustomInstanceDb,
		CustomInstanceDbTag,
		ListCustomInstanceDbsResponse
	} from '$lib/gen'
	import type { ResourceReturn } from 'runed'
	import Select from '../select/Select.svelte'
	import { safeSelectItems } from '../select/utils.svelte'
	import { isCustomInstanceDbEnabled } from './utils.svelte'
	import Button from '../common/button/Button.svelte'
	import CustomInstanceDbWizardModal from './CustomInstanceDbWizardModal.svelte'
	import { ArrowRight, TriangleAlert } from 'lucide-svelte'
	import type { ConfirmationModalHandle } from '../common/confirmationModal/asyncConfirmationModal.svelte'
	import type { Snippet } from 'svelte'

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

	let openedDbNameWizard:
		| {
				status: CustomInstanceDb | undefined
				dbname: string
		  }
		| undefined = $state(undefined)

	let status = $derived(customInstanceDbs.current?.[value ?? ''])
	let onlySelectedTags = $derived(
		safeSelectItems(
			Object.entries(customInstanceDbs.current ?? {})
				.filter(([_, db]) => !tag || db.tag === tag)
				.map(([name, _]) => name)
		)
	)
	let open = $state(false)
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
			{@render customInstanceDbWizardButton({
				dbname: item.value,
				status: customInstanceDbs.current?.[item.value]
			})}
		{/snippet}
	</Select>
	{#if value}
		{@render customInstanceDbWizardButton({ dbname: value, status }, 'absolute right-1.5')}
	{/if}
</div>

<CustomInstanceDbWizardModal
	{customInstanceDbs}
	{confirmationModal}
	{tag}
	bottomHint={wizardBottomHint}
	bind:opened={() => openedDbNameWizard, (v) => !v && (openedDbNameWizard = undefined)}
/>

{#snippet customInstanceDbWizardButton(db: typeof openedDbNameWizard, clazz: string = '')}
	<Button
		spacingSize="xs2"
		variant="default"
		wrapperClasses="bg-surface-input h-6 -my-2 {clazz}"
		onClick={() => ((openedDbNameWizard = db), (open = false))}
	>
		{#if !db?.status}
			<span class="text-yellow-600 dark:text-yellow-400">
				Setup <ArrowRight class="inline" size={14} />
			</span>
		{:else if !db.status.success}
			<span class="text-red-400 flex gap-1">
				Error <TriangleAlert class="inline" size={16} />
			</span>
		{:else}
			<div class="w-1.5 h-1.5 rounded-full bg-green-400"></div>
		{/if}
	</Button>
{/snippet}
