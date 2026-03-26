<script lang="ts">
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import { copyToClipboard } from '$lib/utils'
	import { Highlight } from 'svelte-highlight'
	import json from 'svelte-highlight/languages/json'
	import { yaml } from 'svelte-highlight/languages'
	import YAML from 'yaml'
	import { Button } from '$lib/components/common'
	import { Copy, Download } from 'lucide-svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import HighlightTheme from '$lib/components/HighlightTheme.svelte'
	import type { NewScript } from '$lib/gen'

	let drawer: Drawer | undefined = $state()
	let scriptData: Record<string, any> | undefined = $state(undefined)
	let rawType: 'json' | 'yaml' = $state('yaml')

	export function open(script: NewScript & { modules?: Record<string, any> | null }) {
		const exportData: Record<string, any> = {}
		// Export all meaningful fields
		if (script.summary) exportData.summary = script.summary
		if (script.description) exportData.description = script.description
		exportData.content = script.content
		exportData.language = script.language
		if (script.schema) exportData.schema = script.schema
		if (script.kind && script.kind !== 'script') exportData.kind = script.kind
		if (script.is_template) exportData.is_template = script.is_template
		if (script.tag) exportData.tag = script.tag
		if (script.envs?.length) exportData.envs = script.envs
		if (script.concurrent_limit != null) exportData.concurrent_limit = script.concurrent_limit
		if (script.concurrency_time_window_s != null)
			exportData.concurrency_time_window_s = script.concurrency_time_window_s
		if (script.cache_ttl != null) exportData.cache_ttl = script.cache_ttl
		if (script.dedicated_worker) exportData.dedicated_worker = script.dedicated_worker
		if (script.ws_error_handler_muted) exportData.ws_error_handler_muted = script.ws_error_handler_muted
		if (script.priority != null) exportData.priority = script.priority
		if (script.restart_unless_cancelled) exportData.restart_unless_cancelled = script.restart_unless_cancelled
		if (script.timeout != null) exportData.timeout = script.timeout
		if (script.concurrency_key) exportData.concurrency_key = script.concurrency_key
		if (script.debounce_key) exportData.debounce_key = script.debounce_key
		if (script.debounce_delay_s != null) exportData.debounce_delay_s = script.debounce_delay_s
		if (script.debounce_args_to_accumulate?.length)
			exportData.debounce_args_to_accumulate = script.debounce_args_to_accumulate
		if (script.visible_to_runner_only) exportData.visible_to_runner_only = script.visible_to_runner_only
		if (script.auto_kind) exportData.auto_kind = script.auto_kind
		if (script.has_preprocessor) exportData.has_preprocessor = script.has_preprocessor
		if (script.modules && Object.keys(script.modules).length > 0) exportData.modules = script.modules

		scriptData = exportData
		drawer?.toggleDrawer()
	}

	function getContent(): string {
		if (rawType === 'yaml') {
			return YAML.stringify(scriptData ?? {})
		}
		return JSON.stringify(scriptData ?? {}, null, 4)
	}

	function downloadFile() {
		const content = getContent()
		const ext = rawType === 'yaml' ? 'yaml' : 'json'
		const blob = new Blob([content], { type: 'text/plain' })
		const url = URL.createObjectURL(blob)
		const a = document.createElement('a')
		a.href = url
		a.download = `wac-script.${ext}`
		a.click()
		URL.revokeObjectURL(url)
	}
</script>

<HighlightTheme />

<Drawer bind:this={drawer} size="800px">
	<DrawerContent title="Export Workflow-as-Code" on:close={() => drawer?.toggleDrawer()}>
		<div>
			<Tabs bind:selected={rawType}>
				<Tab value="yaml" label="YAML" />
				<Tab value="json" label="JSON" />
				{#snippet content()}
					<div class="relative pt-2">
						<div class="absolute top-2 right-2 z-20 flex gap-1">
							<Button
								on:click={() => copyToClipboard(getContent())}
								variant="accent"
								size="sm"
								startIcon={{ icon: Copy }}
								iconOnly
							/>
							<Button
								on:click={downloadFile}
								variant="accent"
								size="sm"
								startIcon={{ icon: Download }}
								iconOnly
							/>
						</div>
						{#key rawType}
							<Highlight
								class="overflow-auto px-1 flex-1"
								language={rawType === 'yaml' ? yaml : json}
								code={getContent()}
							/>
						{/key}
					</div>
				{/snippet}
			</Tabs>
		</div>
	</DrawerContent>
</Drawer>
