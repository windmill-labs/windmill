<script lang="ts">
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import TabContent from '$lib/components/common/tabs/TabContent.svelte'

	import { flowStore } from '$lib/components/flows/flowStore'
	import Path from '$lib/components/Path.svelte'
	import Required from '$lib/components/Required.svelte'
	import { getContext } from 'svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import type { FlowEditorContext } from '../types'
	import FlowSchedules from './FlowSchedules.svelte'
	import FlowRetries from './FlowRetries.svelte'

	const { path } = getContext<FlowEditorContext>('FlowEditorContext')

	export let defaultTab = 'configuration'
</script>

<FlowCard title="Settings">
	<Tabs selected={defaultTab}>
		<Tab value="configuration">Configuration</Tab>
		<Tab value="schedule">Schedule</Tab>
		<Tab value="retries">Retries</Tab>

		<svelte:fragment slot="content">
			<TabContent value="configuration" class="p-4">
				<Path bind:path={$flowStore.path} initialPath={path} namePlaceholder="my_flow" kind="flow">
					<div slot="ownerToolkit">
						Flow permissions depend on their path. Select the group <span class="font-mono"
							>all</span
						>
						to share your flow, and <span class="font-mono">user</span> to keep it private.
						<a href="https://docs.windmill.dev/docs/reference/namespaces">docs</a>
					</div>
				</Path>

				<label class="block mt-4">
					<span class="text-gray-700">Summary <Required required={false} /></span>
					<textarea
						bind:value={$flowStore.summary}
						class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-300 focus:ring focus:ring-indigo-200 focus:ring-opacity-50"
						placeholder="A very short summary of the flow displayed when the flow is listed"
						rows="1"
					/>
				</label>
			</TabContent>
			<TabContent value="schedule" class="p-4">
				<FlowSchedules />
			</TabContent>
			<TabContent value="retries" class="px-4">
				<FlowRetries />
			</TabContent>
		</svelte:fragment>
	</Tabs>
</FlowCard>
