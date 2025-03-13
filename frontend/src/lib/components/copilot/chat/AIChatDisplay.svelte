<script lang="ts">
	import autosize from '$lib/autosize'
	import { twMerge } from 'tailwind-merge'
	import Markdown from 'svelte-exmarkdown'
	import { gfmPlugin } from 'svelte-exmarkdown/gfm'
	import CodeDisplay from './CodeDisplay.svelte'
	import { createEventDispatcher, getContext } from 'svelte'
	import { ChevronDown, Code, HistoryIcon, Loader2, Plus, TriangleAlert, X } from 'lucide-svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import type { AIChatContext, ContextConfig, DisplayMessage } from './core'
	import { copilotInfo, copilotSessionModel } from '$lib/stores'
	import ContextElement from './ContextElement.svelte'

	export let pastChats: { id: string; title: string }[]
	export let messages: DisplayMessage[]
	export let instructions: string
	export let contextConfig: ContextConfig
	export let error: string | undefined

	const dispatch = createEventDispatcher<{
		sendRequest: null
		saveAndClear: null
		deletePastChat: { id: string }
		loadPastChat: { id: string }
	}>()

	const { loading, currentReply } = getContext<AIChatContext>('AIChatContext')

	let scrollEl: HTMLDivElement
	function scrollDown() {
		scrollEl?.scrollTo({
			top: scrollEl.scrollHeight,
			behavior: 'smooth'
		})
	}

	$: model = $copilotInfo.ai_models.length > 1 ? $copilotSessionModel : $copilotInfo.ai_models[0]

	$: (messages.length || $currentReply || $loading) && scrollDown()
</script>

<div class="flex flex-col h-full">
	<div
		class="flex flex-row items-center justify-between gap-2 p-2 border-b border-gray-200 dark:border-gray-600"
	>
		<slot name="header-left" />
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
					<div class="p-1 overflow-y-auto">
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
		<div class="h-full flex flex-col overflow-y-scroll pt-2 px-2" bind:this={scrollEl}>
			{#each messages as message}
				{#if message.role === 'user' && message.contextConfig}
					<div class="flex flex-row gap-1 mb-1">
						{#each Object.entries(message.contextConfig) as [ctxKey, enabled]}
							{#if enabled}
								<ContextElement kind={ctxKey} />
							{/if}
						{/each}
					</div>
				{/if}
				<div
					class={twMerge(
						'text-sm py-1',
						message.role === 'user' &&
							'px-2 border border-gray-300 dark:border-gray-600 bg-gray-50 dark:bg-gray-900 rounded-lg mb-2',
						message.role === 'assistant' && 'px-[1px] mb-6'
					)}
				>
					{#if message.role === 'assistant'}
						<div
							class="prose prose-sm dark:prose-invert w-full max-w-full leading-snug space-y-2 prose-ul:!pl-6"
						>
							<Markdown
								md={message.content}
								plugins={[
									gfmPlugin(),
									{
										renderer: {
											pre: CodeDisplay
										}
									}
								]}
							/>
						</div>
					{:else}
						{message.content}
					{/if}
				</div>
			{/each}
			{#if $loading && !$currentReply}
				<div class="mb-6 py-1">
					<Loader2 class="animate-spin" />
				</div>
			{/if}
		</div>
	{/if}

	<div class="p-2" class:border-t={messages.length > 0}>
		<div class="flex flex-row gap-1 mb-1">
			{#if !contextConfig.code || (!contextConfig.error && error !== undefined)}
				<Popover>
					<svelte:fragment slot="trigger">
						<div
							class="border rounded-md px-1 py-0.5 font-normal text-tertiary text-xs hover:bg-surface-hover"
							>@</div
						>
					</svelte:fragment>
					<svelte:fragment slot="content">
						<div class="flex flex-col gap-1 text-tertiary text-xs p-1 min-w-24">
							{#if !contextConfig.code}
								<button
									class="hover:bg-surface-hover rounded-md p-1 text-left flex flex-row gap-1 items-center font-normal"
									on:click={() => {
										contextConfig.code = true
									}}
								>
									<Code size={16} />
									Code
								</button>
							{/if}
							{#if !contextConfig.error && error !== undefined}
								<button
									class="hover:bg-surface-hover rounded-md p-1 text-left flex flex-row gap-1 items-center font-normal"
									on:click={() => {
										contextConfig.error = true
									}}
								>
									<TriangleAlert size={16} />
									Error
								</button>
							{/if}
							<!-- {#if context.db.enabled}
								<button
									class="hover:bg-surface-hover rounded-md p-1 text-left flex flex-row gap-1 items-center font-normal"
									on:click={() => {
										context.db.enabled = true
									}}	
								>
									<Database size={16} />
									DB
								</button>
							{/if} -->
						</div>
					</svelte:fragment>
				</Popover>
			{/if}
			{#each Object.entries(contextConfig) as [ctxKey, enabled]}
				{#if enabled}
					<ContextElement
						kind={ctxKey}
						error={ctxKey === 'error' && error !== undefined ? error : undefined}
						deletable
						on:click={() => {
							contextConfig[ctxKey] = false
						}}
					/>
				{/if}
			{/each}
		</div>
		<textarea
			on:keypress={(e) => {
				if (e.key === 'Enter' && !e.shiftKey) {
					e.preventDefault()
					dispatch('sendRequest')
				}
			}}
			bind:value={instructions}
			use:autosize
			rows={3}
			placeholder={messages.length > 0 ? 'Ask followup' : 'Ask anything'}
			class="resize-none"
		/>

		<div class="flex flex-row justify-end items-center gap-2 px-0.5">
			<div class="min-w-0">
				<Popover disablePopup={$copilotInfo.ai_models.length <= 1}>
					<svelte:fragment slot="trigger">
						<div class="text-tertiary text-xs flex flex-row items-center gap-0.5 font-normal">
							{model}
							{#if $copilotInfo.ai_models.length > 1}
								<ChevronDown size={16} />
							{/if}
						</div>
					</svelte:fragment>
					<svelte:fragment slot="content" let:close>
						<div class="flex flex-col gap-1 p-1 min-w-24">
							{#each $copilotInfo.ai_models.filter((m) => m !== model) as model}
								<button
									class="text-left text-xs hover:bg-surface-hover rounded-md p-1 font-normal"
									on:click={() => {
										$copilotSessionModel = model
										close()
									}}
								>
									{model}
								</button>
							{/each}
						</div>
					</svelte:fragment>
				</Popover>
			</div>
		</div>
	</div>
</div>
