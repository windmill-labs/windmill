<script lang="ts">
	import Modal2 from '../common/modal/Modal2.svelte'
	import Button from '../common/button/Button.svelte'
	import { AlertTriangle } from 'lucide-svelte'

	interface Props {
		isOpen: boolean
		triggerPath: string
		messageCount: number
		onClose: () => void
		onConfirm: () => void
	}

	let { isOpen = $bindable(), triggerPath, messageCount, onClose, onConfirm }: Props = $props()
</script>

<Modal2 bind:isOpen title="Mailbox Messages Found" target="#content" fixedSize="md" css={{ popup: { class: 'z-[10000]' } }}>
	<div class="flex flex-col space-y-4">
		<div class="flex items-start space-x-3">
			<AlertTriangle class="text-amber-500 mt-1" size="20" />
			<div class="flex-1">
				<h3 class="font-medium text-primary mb-2">
					{messageCount} message{messageCount === 1 ? '' : 's'} found in mailbox
				</h3>
				<p class="text-sm text-secondary mb-4">
					There {messageCount === 1 ? 'is' : 'are'} 
					<strong>{messageCount}</strong> 
					message{messageCount === 1 ? '' : 's'} in the mailbox for trigger 
					<code class="font-mono text-xs bg-surface-secondary px-1 py-0.5 rounded">{triggerPath}</code>.
				</p>
				<p class="text-sm text-secondary mb-4">
					If you change the action to "Run immediately", these messages will be 
					<strong>automatically processed</strong> and executed as jobs when the trigger is updated.
				</p>
			</div>
		</div>

		<div class="flex flex-col space-y-2 pt-4 border-t border-gray-200 dark:border-gray-700">
			<Button
				size="sm"
				color="blue"
				on:click={onConfirm}
			>
				Continue and process messages
			</Button>
			<p class="text-xs text-tertiary ml-1">
				Change to "Run immediately" and automatically process all existing messages
			</p>

			<Button
				size="sm"
				variant="border"
				on:click={onClose}
			>
				Cancel
			</Button>
			<p class="text-xs text-tertiary ml-1">
				Keep the action as "Send to mailbox"
			</p>
		</div>
	</div>
</Modal2>