<script lang="ts">
	import { Drawer, DrawerContent } from '$lib/components/common'
	import CreateToken from '$lib/components/settings/CreateToken.svelte'
	import { accountSettingsHref } from '$lib/components/sidebar/settings'

	let { open = $bindable() }: { open?: boolean } = $props()
</script>

<Drawer bind:open size="800px">
	<DrawerContent
		title="Connect an AI client"
		documentationLink="https://www.windmill.dev/docs/core_concepts/mcp"
		on:close={() => (open = false)}
	>
		<div class="flex flex-col gap-2 pb-8">
			<div class="text-xs text-secondary mb-2">
				Connect Claude, ChatGPT or Cursor to Windmill over MCP: build scripts, flows and apps, run
				your automations and check their runs, from your chat.
			</div>
			<CreateToken mcpOnly onTokenCreated={() => {}} />
			<div class="text-xs text-secondary">
				Tokens you create here are listed, and can be revoked, in
				<a href={accountSettingsHref('tokens')}>Tokens</a>.
			</div>
			<div class="text-xs text-secondary">
				Windmill's own AI assistant is configured in
				<a href={accountSettingsHref('ai')}>AI settings</a>.
			</div>
		</div>
	</DrawerContent>
</Drawer>
