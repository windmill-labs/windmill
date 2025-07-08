<script lang="ts">
	import { classNames } from '$lib/utils'
	import { Pane, Splitpanes } from 'svelte-splitpanes'
	import AiChat from './AIChat.svelte'
	import { zIndexes } from '$lib/zIndexes'
	import { loadCopilot, userStore, workspaceStore } from '$lib/stores'
	import { chatState } from './sharedChatState.svelte'

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
</script>

{#if !disableAi}
	<Splitpanes horizontal={false} class="flex-1 min-h-0">
		<Pane size={99.8 - chatState.size} minSize={50} class="flex flex-col min-h-0">
			<div
				id="content"
				class={classNames(
					'w-full flex-1 flex flex-col overflow-y-auto',
					noBorder || $userStore?.operator ? '!pl-0' : isCollapsed ? 'md:pl-12' : 'md:pl-40',
					'transition-all ease-in-out duration-200'
				)}
			>
				<main class="flex-1 flex flex-col">
					<div class="relative w-full flex-1 flex flex-col">
						<div
							class={classNames(
								'pt-2 px-4 sm:px-4 flex flex-row justify-between items-center shadow-sm max-w-7xl md:hidden',
								noBorder || $userStore?.operator ? 'hidden' : ''
							)}
						>
							<button
								aria-label="Menu"
								type="button"
								onclick={() => {
									onMenuOpen?.()
								}}
								class="h-8 w-8 inline-flex items-center justify-center rounded-md text-tertiary hover:text-primary focus:outline-none focus:ring-2 focus:ring-inset focus:ring-indigo-500"
							>
								<svg
									class="h-6 w-6"
									xmlns="http://www.w3.org/2000/svg"
									fill="none"
									viewBox="0 0 24 24"
									stroke-width="2"
									stroke="currentColor"
									aria-hidden="true"
								>
									<path
										stroke-linecap="round"
										stroke-linejoin="round"
										d="M4 6h16M4 12h16M4 18h16"
									/>
								</svg>
							</button>
						</div>
						<div class="flex-1">
							{@render children?.()}
						</div>
					</div>
				</main>
			</div>
		</Pane>
		<Pane
			bind:size={chatState.size}
			minSize={15}
			class={`flex flex-col min-h-0 z-[${zIndexes.aiChat}]`}
		>
			<AiChat />
		</Pane>
	</Splitpanes>
{:else}
	{@render children?.()}
{/if}
