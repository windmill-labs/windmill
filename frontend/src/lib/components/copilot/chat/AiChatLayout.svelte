<script lang="ts">
	import { classNames } from '$lib/utils'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import AiChat from './AIChat.svelte'
	import { zIndexes } from '$lib/zIndexes'
	import { userStore, workspaceStore } from '$lib/stores'
	import { chatState } from './sharedChatState.svelte'
	import { loadCopilot } from '$lib/aiStore'
	import { aiChatManager } from './AIChatManager.svelte'
	import { onDestroy } from 'svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import { Menu } from 'lucide-svelte'
	import CreatedResourceActionDrawers from './CreatedResourceActionDrawers.svelte'

	interface Props {
		noPadding?: boolean
		// Rail width in rem (scales with the root font-size, like the rail itself).
		sidebarWidth?: number
		transitionClass?: string
		isMobile?: boolean
		children: any
		onMenuOpen?: () => void
		disableAi?: boolean
	}
	let {
		noPadding: noBorder = false,
		sidebarWidth = 13,
		transitionClass = 'transition-all ease-in-out duration-200',
		isMobile = false,
		children,
		onMenuOpen,
		disableAi
	}: Props = $props()

	// The desktop rail is fixed-positioned, so the content is offset by a matching
	// left padding (in rem, matching the rail). Mobile/operator/borderless: no rail.
	let contentPadLeft = $derived(noBorder || $userStore?.operator || isMobile ? 0 : sidebarWidth)

	$effect(() => {
		if (disableAi) {
			chatState.size = 0
		}
	})

	$effect(() => {
		if ($workspaceStore && !disableAi) {
			loadCopilot($workspaceStore)
		}
	})

	const historyManager = aiChatManager.historyManager
	historyManager.init()

	onDestroy(() => {
		aiChatManager.cancel('aiChatLayout destroyed')
		historyManager.close()
	})
</script>

{#snippet burgerRow()}
	<div
		class={classNames(
			'py-0.5 px-4 sm:px-4 shadow-sm max-w-7xl md:hidden justify-start flex',
			noBorder || $userStore?.operator ? 'hidden' : ''
		)}
	>
		<Button
			variant="subtle"
			unifiedSize="lg"
			onClick={() => onMenuOpen?.()}
			startIcon={{ icon: Menu }}
			iconOnly
		/>
	</div>
{/snippet}

{#if !disableAi}
	<CreatedResourceActionDrawers />
	<Splitpanes horizontal={false} class="flex-1 min-h-0">
		<Pane size={100 - chatState.size} minSize={50} class="flex flex-col grow min-h-0 ">
			<div
				id="content"
				class={classNames('w-full flex-1 flex flex-col overflow-y-auto min-h-0', transitionClass)}
				style:padding-left="{contentPadLeft}rem"
			>
				<main class="flex-1 flex flex-col min-h-0">
					<div class="relative w-full flex-1 flex flex-col min-h-0">
						{@render burgerRow()}
						<div class="flex-1 min-h-0">
							{@render children?.()}
						</div>
					</div>
				</main>
			</div>
		</Pane>
		{#if chatState.size > 1}
			<Pane
				bind:size={chatState.size}
				minSize={15}
				class={`flex flex-col min-h-0 z-[${zIndexes.aiChat}]`}
			>
				<AiChat />
			</Pane>
		{/if}
	</Splitpanes>
{:else}
	<div
		id="content"
		class={classNames('flex-1 min-h-0 flex flex-col', transitionClass)}
		style:padding-left="{contentPadLeft}rem"
	>
		{@render burgerRow()}
		<div class="flex-1 min-h-0 flex flex-col">
			{@render children?.()}
		</div>
	</div>
{/if}
