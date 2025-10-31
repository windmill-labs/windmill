<script lang="ts">
	import { MailboxService, type ActionToTake } from '$lib/gen'
	import { workspaceStore, superadmin } from '$lib/stores'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'

	interface Props {
		triggerTable: string
		triggerPath: string
		actionToTake: ActionToTake
		canWrite: boolean
		disabled?: boolean
	}

	let {
		triggerTable,
		triggerPath,
		actionToTake = $bindable(),
		canWrite,
		disabled = false
	}: Props = $props()

	let previousActionToTake: ActionToTake = $state(actionToTake)
	let showMailboxWarning = $state(false)
	let mailboxMessageCount = $state(0)

	async function handleActionToTakeChange({ detail }: { detail: ActionToTake }) {
		const newAction = detail

		// Check if changing from send_to_mailbox to run_job
		if (previousActionToTake === 'send_to_mailbox' && newAction === 'run_job' && triggerPath) {
			try {
				// Generate mailbox ID: <trigger_table>::<trigger_path>
				const mailboxId = `${triggerTable}::${triggerPath}`
				
				const messages = await MailboxService.listMailboxMessages({
					workspace: $workspaceStore!,
					mailboxId,
					mailboxType: 'trigger'
				})

				if (messages.length > 0) {
					// Show warning and set message count
					mailboxMessageCount = messages.length
					showMailboxWarning = true
				} else {
					showMailboxWarning = false
				}
			} catch (error) {
				console.error('Failed to check mailbox messages:', error)
				// Continue with the change even if check fails
				showMailboxWarning = false
			}
		} else {
			showMailboxWarning = false
		}

		// Apply the change
		previousActionToTake = actionToTake
		actionToTake = newAction
	}

	// Update previous action when actionToTake changes from parent
	$effect(() => {
		previousActionToTake = actionToTake
	})
</script>

{#if $superadmin}
	<div class="flex flex-col gap-2">
		<p class="text-xs text-tertiary mb-2">
			Choose whether to execute the trigger immediately or send it to the mailbox for manual
			handling.
		</p>
		<ToggleButtonGroup
			selected={actionToTake}
			on:selected={handleActionToTakeChange}
			disabled={!canWrite || disabled}
		>
			{#snippet children({ item, disabled })}
				<ToggleButton label="Run immediately" value="run_job" {item} {disabled} />
				<ToggleButton label="Send to mailbox" value="send_to_mailbox" {item} {disabled} />
			{/snippet}
		</ToggleButtonGroup>

		{#if showMailboxWarning && actionToTake === 'run_job'}
			<div class="mt-3 p-3 bg-amber-50 dark:bg-amber-900/20 border border-amber-200 dark:border-amber-800 rounded-md">
				<div class="flex items-start space-x-2">
					<svg class="w-5 h-5 text-amber-500 mt-0.5 flex-shrink-0" fill="currentColor" viewBox="0 0 20 20">
						<path fill-rule="evenodd" d="M8.257 3.099c.765-1.36 2.722-1.36 3.486 0l5.58 9.92c.75 1.334-.213 2.98-1.742 2.98H4.42c-1.53 0-2.493-1.646-1.743-2.98l5.58-9.92zM11 13a1 1 0 11-2 0 1 1 0 012 0zm-1-8a1 1 0 00-1 1v3a1 1 0 002 0V6a1 1 0 00-1-1z" clip-rule="evenodd" />
					</svg>
					<div class="flex-1">
						<h4 class="text-sm font-medium text-amber-800 dark:text-amber-200">
							{mailboxMessageCount} message{mailboxMessageCount === 1 ? '' : 's'} will be processed
						</h4>
						<p class="text-sm text-amber-700 dark:text-amber-300 mt-1">
							There {mailboxMessageCount === 1 ? 'is' : 'are'} <strong>{mailboxMessageCount}</strong> 
							message{mailboxMessageCount === 1 ? '' : 's'} in the mailbox for this trigger. 
							These will be automatically processed and executed as jobs when you save the trigger.
						</p>
					</div>
				</div>
			</div>
		{/if}
	</div>
{/if}