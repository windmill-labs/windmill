<script lang="ts">
	import type { CompletedJob } from '$lib/gen'

	import DisplayResult from './DisplayResult.svelte'
	import Tabs from './common/tabs/Tabs.svelte'
	import Tab from './common/tabs/Tab.svelte'
	import TabContent from './common/tabs/TabContent.svelte'
	import LogViewer from './LogViewer.svelte'

	export let job: CompletedJob | undefined
</script>

{#if job}
	<Tabs selected="results">
		<Tab value="results">Results</Tab>
		<Tab value="logs">Logs</Tab>
		<svelte:fragment slot="content">
			<TabContent value="results" class="border p-2 h-36 overflow-auto">
				<DisplayResult result={job.result} />
			</TabContent>
			<TabContent value="logs" class="border h-36 overflow-auto">
				<LogViewer content={job.logs ?? ''} isLoading={false} />
			</TabContent>
		</svelte:fragment>
	</Tabs>
{/if}
