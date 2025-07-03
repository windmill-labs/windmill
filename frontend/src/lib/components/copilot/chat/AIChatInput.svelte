<script lang="ts">
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import AvailableContextList from './AvailableContextList.svelte'
	import ContextElementBadge from './ContextElementBadge.svelte'
	import ContextTextarea from './ContextTextarea.svelte'
	import autosize from '$lib/autosize'
	import type { ContextElement } from './context'
	import { aiChatManager } from './AIChatManager.svelte'

	function clickOutside(node: HTMLElement) {
		function handleClick(event: MouseEvent) {
			if (node && !node.contains(event.target as Node) && editingMessageIndex !== null) {
				editingMessageIndex = null
			}
		}

		document.addEventListener('click', handleClick, true)
		return {
			destroy() {
				document.removeEventListener('click', handleClick, true)
			}
		}
	}

	interface Props {
		availableContext: ContextElement[]
		selectedContext: ContextElement[]
		isFirstMessage?: boolean
		disabled?: boolean
		placeholder?: string
		initialInstructions?: string
		editingMessageIndex?: number | null
	}

	let {
		availableContext,
		selectedContext = $bindable([]),
		disabled = false,
		isFirstMessage = false,
		placeholder = 'Ask anything',
		initialInstructions = '',
		editingMessageIndex = $bindable(null)
	}: Props = $props()

	let contextTextareaComponent: ContextTextarea | undefined = $state()
	let instructionsTextareaComponent: HTMLTextAreaElement | undefined = $state()
	let instructions = $state(initialInstructions)

	export function focusInput() {
		if (aiChatManager.mode === 'script') {
			contextTextareaComponent?.focus()
		} else {
			instructionsTextareaComponent?.focus()
		}
	}

	function addContextToSelection(contextElement: ContextElement) {
		if (
			selectedContext &&
			availableContext &&
			!selectedContext.find(
				(c) => c.type === contextElement.type && c.title === contextElement.title
			) &&
			availableContext.find(
				(c) => c.type === contextElement.type && c.title === contextElement.title
			)
		) {
			selectedContext = [...selectedContext, contextElement]
		}
	}

	function sendRequest() {
		if (aiChatManager.loading) {
			return
		}
		if (editingMessageIndex !== null) {
			aiChatManager.restartLastGeneration(editingMessageIndex, instructions)
			editingMessageIndex = null
		} else {
			aiChatManager.sendRequest({ instructions })
			instructions = ''
		}
	}
</script>

<div use:clickOutside>
	{#if aiChatManager.mode === 'script'}
		<div class="flex flex-row gap-1 mb-1 overflow-scroll pt-2 px-2 no-scrollbar">
			<Popover>
				<svelte:fragment slot="trigger">
					<div
						class="border rounded-md px-1 py-0.5 font-normal text-tertiary text-xs hover:bg-surface-hover"
						>@</div
					>
				</svelte:fragment>
				<svelte:fragment slot="content" let:close>
					<AvailableContextList
						{availableContext}
						{selectedContext}
						onSelect={(element) => {
							addContextToSelection(element)
							close()
						}}
					/>
				</svelte:fragment>
			</Popover>
			{#each selectedContext as element}
				<ContextElementBadge
					contextElement={element}
					deletable
					on:delete={() => {
						selectedContext = selectedContext?.filter(
							(c) => c.type !== element.type || c.title !== element.title
						)
					}}
				/>
			{/each}
		</div>
		<ContextTextarea
			bind:this={contextTextareaComponent}
			bind:value={instructions}
			{availableContext}
			{selectedContext}
			{isFirstMessage}
			{placeholder}
			onAddContext={(contextElement) => addContextToSelection(contextElement)}
			onSendRequest={() => {
				sendRequest()
			}}
			{disabled}
			onEscape={() => {
				editingMessageIndex = null
			}}
		/>
	{:else}
		<div class="relative w-full px-2 scroll-pb-2 pt-2" use:clickOutside>
			<textarea
				bind:this={instructionsTextareaComponent}
				bind:value={instructions}
				use:autosize
				onkeydown={(e) => {
					if (e.key === 'Enter' && !e.shiftKey) {
						e.preventDefault()
						sendRequest()
					} else if (e.key === 'Escape') {
						editingMessageIndex = null
					}
				}}
				rows={3}
				{placeholder}
				class="resize-none"
				{disabled}
			></textarea>
		</div>
	{/if}
</div>
