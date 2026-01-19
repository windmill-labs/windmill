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

	interface Props {
		noPadding?: boolean
		isCollapsed?: boolean
		children: any
		onMenuOpen?: () => void
		disableAi?: boolean
	}
	let {
		noPadding: noBorder = false,
		isCollapsed = false,
		children,
		onMenuOpen,
		disableAi
	}: Props = $props()

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

{#if !disableAi}
	<Splitpanes horizontal={false} class="flex-1 min-h-0">
		<Pane size={100 - chatState.size} minSize={50} class="flex flex-col grow min-h-0 ">
			<div
				id="content"
				class={classNames(
					'w-full flex-1 flex flex-col overflow-y-auto min-h-0',
					noBorder || $userStore?.operator ? '!pl-0' : isCollapsed ? 'md:pl-12' : 'md:pl-40',
					'transition-all ease-in-out duration-200'
				)}
			>
				<main class="flex-1 flex flex-col min-h-0">
					<div class="relative w-full flex-1 flex flex-col min-h-0">
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
							></Button>
						</div>
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
	{@render children?.()}
{/if}
