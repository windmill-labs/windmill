<script lang="ts">
	import type { CustomInstanceDbTag, GetCustomInstanceDbsResponse } from '$lib/gen'
	import type { ResourceReturn } from 'runed'
	import Select from '../select/Select.svelte'
	import { safeSelectItems } from '../select/utils.svelte'
	import { isCustomInstanceDbEnabled } from './utils.svelte'
	import Button from '../common/button/Button.svelte'
	import CustomInstanceDbWizardModal from './CustomInstanceDbWizardModal.svelte'
	import { ArrowRight, TriangleAlert } from 'lucide-svelte'
	import type { ConfirmationModalHandle } from '../common/confirmationModal/asyncConfirmationModal.svelte'
	import DBManagerDrawer from '../DBManagerDrawer.svelte'
	import type { Snippet } from 'svelte'

	type Props = {
		value: string | undefined
		customInstanceDbs: ResourceReturn<GetCustomInstanceDbsResponse>
		confirmationModal: ConfirmationModalHandle
		dbManagerDrawer: DBManagerDrawer | undefined
		wizardBottomHint?: Snippet | undefined
		class?: string
		tag?: CustomInstanceDbTag
	}
	let {
		value = $bindable(),
		customInstanceDbs,
		confirmationModal,
		dbManagerDrawer,
		wizardBottomHint,
		class: className,
		tag
	}: Props = $props()

	let openedDbNameWizard = $state(false)

	let status = $derived(customInstanceDbs.current?.[value ?? ''])
</script>

<div class="flex relative items-center {className}">
	<Select
		class="flex-1"
		inputClass="pr-20"
		bind:value
		onCreateItem={(i) => (value = i)}
		placeholder="PostgreSQL database name"
		items={safeSelectItems(Object.keys(customInstanceDbs.current ?? {}))}
		disabled={!$isCustomInstanceDbEnabled}
	/>

	<Button
		spacingSize="xs2"
		variant="default"
		wrapperClasses={'absolute right-1.5 h-6'}
		onClick={() => (openedDbNameWizard = true)}
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
</div>

<CustomInstanceDbWizardModal
	{customInstanceDbs}
	{confirmationModal}
	{dbManagerDrawer}
	{tag}
	bottomHint={wizardBottomHint}
	bind:opened={
		() =>
			openedDbNameWizard
				? { dbname: value ?? '', status: customInstanceDbs.current?.[value ?? ''] }
				: undefined,
		(v) => !v && (openedDbNameWizard = false)
	}
/>
