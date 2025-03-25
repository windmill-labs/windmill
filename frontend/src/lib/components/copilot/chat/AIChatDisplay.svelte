<script lang="ts">
	import { twMerge } from 'tailwind-merge'
	import AssistantMessage from './AssistantMessage.svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import { ChevronDown, HistoryIcon, Loader2, Plus, X } from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import {
		type AIChatContext,
		type DisplayMessage,
		type ContextElement,
		type SelectedContext
	} from './core'
	import {
		COPILOT_SESSION_MODEL_SETTING_NAME,
		COPILOT_SESSION_PROVIDER_SETTING_NAME,
		copilotInfo,
		copilotSessionModel
	} from '$lib/stores'
	import ContextElementBadge from './ContextElementBadge.svelte'
	import { storeLocalSetting } from '$lib/utils'
	import ContextTextarea from './ContextTextarea.svelte'
	import AvailableContextList from './AvailableContextList.svelte'

	export let pastChats: { id: string; title: string }[]
	export let messages: DisplayMessage[]
	export let instructions: string
	export let selectedContext: SelectedContext[]
	export let availableContext: ContextElement[]

	const dispatch = createEventDispatcher<{
		sendRequest: null
		saveAndClear: null
		deletePastChat: { id: string }
		loadPastChat: { id: string }
	}>()

	const { loading, currentReply } = getContext<AIChatContext>('AIChatContext')

	export function enableAutomaticScroll() {
		automaticScroll = true
	}

	let automaticScroll = true
	let scrollEl: HTMLDivElement
	async function scrollDown() {
		scrollEl?.scrollTo({
			top: scrollEl.scrollHeight,
			behavior: 'smooth'
		})
	}

	let height = 0
	$: automaticScroll && height && scrollDown()

	$: providerModel = $copilotSessionModel ??
		$copilotInfo.defaultModel ??
		$copilotInfo.aiModels[0] ?? {
			model: 'No model',
			provider: 'No provider'
		}

	$: console.log($copilotSessionModel, $copilotInfo.defaultModel, $copilotInfo.aiModels[0])

	function addContextToSelection(contextElement: ContextElement) {
		if (!selectedContext.find((c) => c.type === contextElement.type && c.title === contextElement.title)) {
			selectedContext = [
				...selectedContext,
				{
					type: contextElement.type,
					title: contextElement.title
				}
			]
		}
	}

	$: console.log('selectedContext', selectedContext)

</script>

<div class="flex flex-col h-full">
	<div
		class="flex flex-row items-center justify-between gap-2 p-2 border-b border-gray-200 dark:border-gray-600"
	>
		<div class="flex flex-row items-center gap-2">
			<slot name="header-left" />
			<p class="text-sm font-semibold">Chat</p>
		</div>
		<div class="flex flex-row items-center gap-2">
			<Popover>
				<svelte:fragment slot="trigger">
					<Button
						on:click={() => {}}
						title="History"
						size="md"
						btnClasses="!p-1"
						startIcon={{ icon: HistoryIcon }}
						iconOnly
						variant="border"
						color="light"
						propagateEvent
					/>
				</svelte:fragment>
				<svelte:fragment slot="content" let:close>
					<div class="p-1 overflow-y-auto max-h-[300px]">
						{#if pastChats.length === 0}
							<div class="text-center text-tertiary text-xs">No history</div>
						{:else}
							<div class="flex flex-col">
								{#each pastChats as chat}
									<button
										class="text-left flex flex-row items-center gap-2 justify-between hover:bg-gray-100 dark:hover:bg-gray-700 rounded-md p-1"
										on:click={() => {
											dispatch('loadPastChat', { id: chat.id })
											close()
										}}
									>
										<div
											class="text-xs font-medium w-48 text-ellipsis overflow-hidden whitespace-nowrap flex-1"
											title={chat.title}
										>
											{chat.title}
										</div>
										<Button
											iconOnly
											size="xs2"
											btnClasses="!p-1"
											color="light"
											variant="border"
											startIcon={{ icon: X }}
											on:click={() => {
												dispatch('deletePastChat', { id: chat.id })
											}}
										/>
									</button>
								{/each}
							</div>
						{/if}
					</div>
				</svelte:fragment>
			</Popover>
			<Button
				title="New chat"
				on:click={() => {
					dispatch('saveAndClear')
				}}
				size="md"
				btnClasses="!p-1"
				startIcon={{ icon: Plus }}
				iconOnly
				variant="border"
				color="light"
			/>
			<slot name="header-right" />
		</div>
	</div>
	{#if messages.length > 0}
		<div
			class="h-full overflow-y-scroll pt-2"
			bind:this={scrollEl}
			on:wheel={(e) => {
				automaticScroll = false
			}}
		>
			<div class="flex flex-col" bind:clientHeight={height}>
				{#each messages as message}
					{#if message.role === 'user' && message.contextElements}
						<div class="flex flex-row gap-1 mb-1 overflow-scroll no-scrollbar px-2">
							{#each message.contextElements as element}
								<ContextElementBadge contextElement={element} />
							{/each}
						</div>
					{/if}
					<div
						class={twMerge(
							'text-sm py-1 mx-2',
							message.role === 'user' &&
								'px-2 border border-gray-300 dark:border-gray-600 bg-gray-50 dark:bg-gray-900 rounded-lg mb-2',
							message.role === 'assistant' && 'px-[1px] mb-6'
						)}
					>
						{#if message.role === 'assistant'}
							<AssistantMessage {message} />
						{:else}
							{message.content}
						{/if}
					</div>
				{/each}
				{#if $loading && !$currentReply}
					<div class="mb-6 py-1 px-2">
						<Loader2 class="animate-spin" />
					</div>
				{/if}
			</div>
		</div>
	{/if}

	<div class:border-t={messages.length > 0}>
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
				{@const contextElement = availableContext.find((c) => c.type === element.type)}
				{#if contextElement}
					<ContextElementBadge
						{contextElement}
						deletable
						on:delete={() => {
							selectedContext = selectedContext.filter((c) => c.type !== element.type || c.title !== element.title)
						}}
					/>
				{/if}
			{/each}
		</div>
		<ContextTextarea
			{instructions}
			{availableContext}
			{selectedContext}
			placeholder={messages.length > 0 ? 'Ask followup' : 'Ask anything'}
			on:addContext={(e) => addContextToSelection(e.detail.contextElement)}
			on:sendRequest={() => dispatch('sendRequest')}
			on:updateInstructions={(e) => instructions = e.detail.value}
		/>

		<div class="flex flex-row justify-end items-center gap-2 px-0.5">
			<div class="min-w-0">
				<Popover disablePopup={$copilotInfo.aiModels.length <= 1}>
					<svelte:fragment slot="trigger">
						<div class="text-tertiary text-xs flex flex-row items-center gap-0.5 font-normal">
							{providerModel.model}
							{#if $copilotInfo.aiModels.length > 1}
								<ChevronDown size={16} />
							{/if}
						</div>
					</svelte:fragment>
					<svelte:fragment slot="content" let:close>
						<div class="flex flex-col gap-1 p-1 min-w-24">
							{#each $copilotInfo.aiModels.filter((m) => m.model !== providerModel.model) as providerModel}
								<button
									class="text-left text-xs hover:bg-surface-hover rounded-md p-1 font-normal"
									on:click={() => {
										$copilotSessionModel = providerModel
										storeLocalSetting(COPILOT_SESSION_MODEL_SETTING_NAME, providerModel.model)
										storeLocalSetting(COPILOT_SESSION_PROVIDER_SETTING_NAME, providerModel.provider)
										close()
									}}
								>
									{providerModel.model}
								</button>
							{/each}
						</div>
					</svelte:fragment>
				</Popover>
			</div>
		</div>
	</div>
</div>
