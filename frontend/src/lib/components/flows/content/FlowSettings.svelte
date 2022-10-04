<script lang="ts">
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import TabContent from '$lib/components/common/tabs/TabContent.svelte'

	import { flowStore } from '$lib/components/flows/flowStore'
	import Path from '$lib/components/Path.svelte'
	import Required from '$lib/components/Required.svelte'
	import FlowCard from '../common/FlowCard.svelte'
	import FlowSchedules from './FlowSchedules.svelte'
	import SvelteMarkdown from 'svelte-markdown'

	export let initialPath: string

	export let defaultTab = 'metadata'
</script>

<FlowCard title="Settings">
	<Tabs selected={defaultTab}>
		<Tab value="metadata">Metadata</Tab>
		<Tab value="schedule">Schedule</Tab>

		<svelte:fragment slot="content">
			<TabContent value="metadata" class="p-4">
				<Path bind:path={$flowStore.path} {initialPath} namePlaceholder="my_flow" kind="flow">
					<div slot="ownerToolkit">
						Flow permissions depend on their path. Select the group <span class="font-mono"
							>all</span
						>
						to share your flow, and <span class="font-mono">user</span> to keep it private.
						<a href="https://docs.windmill.dev/docs/reference/namespaces">docs</a>
					</div>
				</Path>

				<label class="block my-4">
					<span class="text-gray-700">Summary <Required required={false} /></span>
					<textarea
						bind:value={$flowStore.summary}
						class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-300 focus:ring focus:ring-indigo-200 focus:ring-opacity-50"
						placeholder="A very short summary of the flow displayed when the flow is listed"
						rows="1"
						id="flow-summary"
					/>
				</label>

				<label class="block my-4" for="inp">
					<span class="text-gray-700"
						>Description<Required required={false} detail="accept markdown formatting" />
						<textarea
							id="inp"
							bind:value={$flowStore.description}
							class="
					mt-1
					block
					w-full
					rounded-md
					border-gray-300
					shadow-sm
					focus:border-indigo-300 focus:ring focus:ring-indigo-200 focus:ring-opacity-50
					"
							placeholder="A description to help users understand what this script does and how to use it."
							rows="3"
						/>
					</span>
				</label>

				<div>
					<h3 class="text-gray-700 ">Description rendered</h3>
					<div
						class="prose mt-5 text-xs shadow-inner shadow-blue p-4 overflow-auto"
						style="max-height: 200px;"
					>
						<SvelteMarkdown source={$flowStore.description ?? ''} />
					</div>
				</div>
			</TabContent>
			<TabContent value="schedule" class="p-4">
				<FlowSchedules />
			</TabContent>
		</svelte:fragment>
	</Tabs>
</FlowCard>
