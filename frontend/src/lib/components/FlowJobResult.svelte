<script lang="ts">
	import type { CompletedJob } from '$lib/gen'

	import DisplayResult from './DisplayResult.svelte'
	import Tabs from './tabs/Tabs.svelte'
	import Tab from './tabs/Tab.svelte'
	import TabPanel from './tabs/TabPanel.svelte'

	let value = 0
	export let job: CompletedJob | undefined
</script>

{#if job}
	<div>
		<Tabs>
			<Tab bind:value index={0}>Results</Tab>
			<Tab bind:value index={1}>Logs</Tab>
		</Tabs>
		<TabPanel bind:value index={0} class="border p-2 h-36 overflow-y-scroll">
			<DisplayResult result={job.result} />
		</TabPanel>
		<TabPanel bind:value index={1} class="border p-2 h-36 overflow-y-scroll">
			<div class="text-xs p-4 bg-gray-50 overflow-auto max-h-80 border">
				<pre class="w-full">{job.logs}</pre>
			</div>
		</TabPanel>
	</div>
{/if}
