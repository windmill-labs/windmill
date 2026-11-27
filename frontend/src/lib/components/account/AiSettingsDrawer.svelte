<script lang="ts">
	import { Drawer, DrawerContent } from '$lib/components/common'
	import AIUserSettings from '$lib/components/settings/AIUserSettings.svelte'
	import AiUsagePanel from '$lib/components/workspaceSettings/AiUsagePanel.svelte'
	import { copilotInfo, copilotWorkspace } from '$lib/aiStore'
	import { accountSettingsHref } from '$lib/components/sidebar/settings'

	let { open = $bindable() }: { open?: boolean } = $props()
</script>

<Drawer bind:open size="800px">
	<DrawerContent title="AI settings" on:close={() => (open = false)}>
		<div class="flex flex-col gap-6 pb-8">
			<div class="text-xs text-secondary">
				To use Windmill from Claude, ChatGPT or Cursor, see
				<a href={accountSettingsHref('mcp')}>Connect an AI client</a>.
			</div>
			<AIUserSettings />
			<!-- Keyed on the workspace `copilotInfo` reflects, not on `$workspaceStore`:
			     a session acting on another workspace loads that workspace's AI config
			     while navigation stays put, and pricing usage from one workspace with
			     another's rates would silently misstate it. -->
			{#if $copilotWorkspace}
				<AiUsagePanel
					workspace={$copilotWorkspace}
					modelPricing={$copilotInfo.modelPricing ?? {}}
					scope="self"
				/>
			{/if}
		</div>
	</DrawerContent>
</Drawer>
