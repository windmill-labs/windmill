<script lang="ts">
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Plus, Star, Loader2, RotateCcw } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'
	import { triggerIconMap, type Trigger, type TriggerType } from './utils'
	import AddTriggersButton from './AddTriggersButton.svelte'
	import TriggerLabel from './TriggerLabel.svelte'
	import DeleteTriggerButton from './DeleteTriggerButton.svelte'
	import { triggerableByAI } from '$lib/actions/triggerableByAI.svelte'

	interface Props {
		// Props
		selectedTrigger?: number | undefined
		triggers?: Trigger[]
		isEditor?: boolean
		webhookToken?: number
		emailToken?: number
		onSelect?: (trigger: number) => void
		onAddDraftTrigger?: (type: TriggerType) => void
		onDeleteDraft?: (triggerIndex: number) => void
		onReset?: (triggerIndex: number) => void
	}

	let {
		selectedTrigger = undefined,
		triggers = [],
		isEditor = false,
		webhookToken,
		emailToken,
		onSelect,
		onAddDraftTrigger,
		onDeleteDraft,
		onReset
	}: Props = $props()

	// Component state
	let loading = false
</script>

<div class="flex flex-col space-y-2 w-full">
	<div class="w-full">
		<AddTriggersButton {onAddDraftTrigger} setDropdownWidthToButtonWidth class="w-full" {isEditor}>
			<Button
				aiId="add-trigger"
				aiDescription="Add a new trigger"
				size="xs"
				color="blue"
				startIcon={{ icon: Plus }}
				nonCaptureEvent
				btnClasses="w-full justify-center"
			>
				<span>Add trigger</span>
			</Button>
		</AddTriggersButton>
	</div>
	<DataTable {loading} size="sm" tableFixed={true}>
		<tbody>
			{#each triggers as trigger, index}
				{@const SvelteComponent = triggerIconMap[trigger.type]}
				<tr
					class={twMerge(
						'hover:bg-surface-hover cursor-pointer h-10 group',
						selectedTrigger === index ? 'bg-surface-hover ' : ''
					)}
					onclick={() => onSelect?.(index)}
					use:triggerableByAI={{
						id: `trigger-${trigger.id}`,
						description: `See ${trigger.type} triggers`,
						callback: () => onSelect?.(index)
					}}
				>
					<td class="w-12 text-center py-2 px-2">
						<div class="relative flex justify-center items-center">
							<SvelteComponent
								size={16}
								class={trigger.isDraft ? 'text-frost-400' : 'text-tertiary'}
							/>

							{#if trigger.isPrimary}
								<Star size={10} class="absolute -mt-3 ml-3 text-blue-400" />
							{/if}
						</div>
					</td>
					<td class="py-2 px-2 text-xs">
						<div class="flex items-center justify-between gap-1">
							<div class="flex items-center grow min-w-0">
								<TriggerLabel {trigger} />
								{#if trigger.type === 'webhook' && webhookToken}
									<span
										class="ml-2 px-1.5 text-xs rounded-md bg-tertiary/50 group-hover:bg-primary text-primary-inverse py-0.5"
									>
										{`${webhookToken} token${webhookToken > 1 ? 's' : ''}`}
									</span>
								{:else if trigger.type === 'email' && emailToken}
									<span
										class="ml-2 text-xs rounded-md bg-tertiary/50 group-hover:bg-primary text-primary-inverse px-1.5 py-0.5"
									>
										{`${emailToken} token${emailToken > 1 ? 's' : ''}`}
									</span>
								{/if}
							</div>

							{#if !['email', 'webhook', 'cli'].includes(trigger.type)}
								{#if trigger.isDraft}
									<DeleteTriggerButton {trigger} onDelete={() => onDeleteDraft?.(index)} small />
								{:else if !!trigger.draftConfig && !trigger.isDraft}
									<Button
										size="xs"
										color="light"
										btnClasses="transition-all duration-200 text-transparent hover:bg-surface group-hover:text-primary bg-transparent px-1 py-1"
										startIcon={{ icon: RotateCcw }}
										iconOnly
										title="Reset to deployed version"
										on:click={() => onReset?.(index)}
									/>
								{/if}
							{/if}
						</div>
					</td>
				</tr>
			{/each}

			{#if !loading && triggers.length === 0}
				<tr>
					<td colspan="2" class="text-center py-4 text-tertiary text-sm"> No triggers found </td>
				</tr>
			{/if}
			{#if loading && triggers.length === 0}
				<tr>
					<td colspan="2" class="text-center py-4 text-tertiary text-sm">
						<div class="flex justify-center items-center gap-2">
							<Loader2 class="animate-spin" size={16} />
							<span>Loading triggers...</span>
						</div>
					</td>
				</tr>
			{/if}
		</tbody>
	</DataTable>
</div>
