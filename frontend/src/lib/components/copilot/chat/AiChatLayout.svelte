<script lang="ts">
	// The docked chat beside the embedded script and flow editors (ScriptWrapper, FlowWrapper,
	// exported by the components package). The app itself has no docked chat: AI work there
	// happens in sessions.
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import { zIndexes } from '$lib/zIndexes'
	import { workspaceStore } from '$lib/stores'
	import { chatState } from './sharedChatState.svelte'
	import { loadCopilot } from '$lib/components/copilot/loadCopilot'
	import { copilotInfo } from '$lib/aiStore'
	import { aiChatManager } from './AIChatManager.svelte'
	import { onDestroy } from 'svelte'
	import CreatedResourceActionDrawers from './CreatedResourceActionDrawers.svelte'

	interface Props {
		children: any
		disableAi?: boolean
	}
	let { children, disableAi }: Props = $props()

	$effect(() => {
		chatState.dockedChatAvailable = !disableAi
		if (disableAi) {
			chatState.size = 0
		}
		return () => {
			chatState.dockedChatAvailable = false
		}
	})

	// The workspace AI config gates far more than the pane (code completion, metadata generation).
	$effect(() => {
		if ($workspaceStore && !disableAi) {
			loadCopilot($workspaceStore)
		}
	})

	// The pane restores its last open state from localStorage before the config can say
	// the workspace hid the assistant; close it as soon as that is known.
	$effect(() => {
		if ($copilotInfo.workspaceDisabled && chatState.size > 0) {
			aiChatManager.closeChat()
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
	<CreatedResourceActionDrawers />
	<Splitpanes horizontal={false} class="flex-1 min-h-0">
		<Pane size={100 - chatState.size} minSize={50} class="flex flex-col grow min-h-0 ">
			<div id="content" class="w-full flex-1 flex flex-col overflow-y-auto min-h-0">
				<main class="flex-1 flex flex-col min-h-0">
					<div class="relative w-full flex-1 flex flex-col min-h-0">
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
				<div class="flex-1 min-h-0">
					<!-- Dynamic: the chat reaches ~100 modules, loaded only once the pane opens. -->
					{#await import('./AIChat.svelte') then AiChat}
						<AiChat.default />
					{/await}
				</div>
			</Pane>
		{/if}
	</Splitpanes>
{:else}
	<div id="content" class="flex-1 min-h-0 flex flex-col overflow-y-auto">
		<div class="flex-1 min-h-0 flex flex-col">
			{@render children?.()}
		</div>
	</div>
{/if}
