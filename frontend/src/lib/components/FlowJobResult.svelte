<script lang="ts">
	import type { CompletedJob } from '$lib/gen'

	import DisplayResult from './DisplayResult.svelte'
	import Tabs from './common/tabs/Tabs.svelte'
	import Tab from './common/tabs/Tab.svelte'
	import TabContent from './common/tabs/TabContent.svelte'

	export let job: CompletedJob | undefined
</script>

{#if job}
	<Tabs selected="results">
		<Tab value="results">Results</Tab>
		<Tab value="logs">Logs</Tab>
		<svelte:fragment slot="content">
			<TabContent value="results" class="border p-2 h-36 overflow-y-scroll">
				<DisplayResult result={job.result} />
			</TabContent>
			<TabContent value="logs" class="border p-2 h-36 overflow-y-scroll">
				<div class="text-xs p-4 bg-gray-50 overflow-auto max-h-80 border">
					<pre class="w-full">{job.logs}</pre>
				</div>
			</TabContent>
		</svelte:fragment>
	</Tabs>
{/if}
