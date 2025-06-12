<script lang="ts">
	import { base } from '$lib/base'
	import { Button } from '../common'

	import { SUPPORTED_LANGUAGES } from './lib'
	import type { SupportedLanguage } from '$lib/common'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import { autoPlacement } from '@floating-ui/core'
	import { WandSparkles } from 'lucide-svelte'
	import { aiChatManager } from './chat/AIChatManager.svelte'
	import { copilotInfo } from '$lib/stores'

	let {
		lang
	}: {
		lang: SupportedLanguage
	} = $props()
</script>

{#if SUPPORTED_LANGUAGES.has(lang)}
	<Popover
		floatingConfig={{
			middleware: [
				autoPlacement({
					allowedPlacements: ['bottom-end', 'top-end']
				})
			]
		}}
		displayArrow={true}
	>
		<svelte:fragment slot="trigger">
			<div class="flex flex-row">
				<Button
					title="Fix code"
					size="xs"
					color="light"
					spacingSize="xs2"
					startIcon={{ icon: WandSparkles }}
					on:click={async () => {
						if ($copilotInfo.enabled) {
							await aiChatManager.fix()
						}
					}}
					btnClasses="text-violet-800 dark:text-violet-400 bg-violet-100 dark:bg-gray-700 min-w-[84px]"
					propagateEvent={!$copilotInfo.enabled}
				>
					AI Fix
				</Button>
			</div>
		</svelte:fragment>
		<svelte:fragment slot="content">
			<div class="p-4">
				<div class="w-80">
					<p class="text-sm"
						>Enable Windmill AI in the <a
							class="inline-flex flex-row items-center gap-1"
							href="{base}/workspace_settings?tab=ai"
							target="_blank">workspace settings</a
						></p
					></div
				>
			</div>
		</svelte:fragment>
	</Popover>
{/if}
