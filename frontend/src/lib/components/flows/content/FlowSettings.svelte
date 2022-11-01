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
	import Toggle from '$lib/components/Toggle.svelte'
	import { Alert } from '$lib/components/common'

	export let initialPath: string

	export let defaultTab = 'metadata'
</script>

<FlowCard title="Settings">
	<Tabs selected={defaultTab}>
		<Tab value="metadata">Metadata</Tab>
		<Tab value="schedule">Schedule</Tab>
		<Tab value="same-worker">Same Worker</Tab>

		<svelte:fragment slot="content">
			<TabContent value="metadata" class="p-4">
				<Path bind:path={$flowStore.path} {initialPath} namePlaceholder="my_flow" kind="flow">
					<div slot="ownerToolkit">
						Flow permissions depend on their path. Select the group
						<span class="font-mono"> all </span>
						to share your flow, and <span class="font-mono">user</span> to keep it private.
						<a href="https://docs.windmill.dev/docs/reference/namespaces">docs</a>
					</div>
				</Path>

				<label class="block my-4">
					<span class="text-gray-700 text-sm">Summary <Required required={false} /></span>
					<textarea
						bind:value={$flowStore.summary}
						class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-300 focus:ring focus:ring-indigo-200 focus:ring-opacity-50"
						placeholder="A very short summary of the flow displayed when the flow is listed"
						rows="1"
						id="flow-summary"
					/>
				</label>

				<label class="block my-4" for="inp">
					<span class="text-gray-700 text-sm">
						Description
						<Required required={false} detail="markdown" />
						<textarea
							id="inp"
							bind:value={$flowStore.description}
							class="mt-1 block w-full rounded-md border-gray-300 shadow-sm focus:border-indigo-300 
							focus:ring focus:ring-indigo-200 focus:ring-opacity-50"
							placeholder="A description to help users understand what this flow does and how to use it. Markdown accepted."
							rows="3"
						/>
					</span>
				</label>

				<div>
					<div class="font-bold pb-1 mt-4">Description preview</div>
					{#if $flowStore.description}
						<div class="prose max-h-48 mt-5 text-xs shadow-inner shadow-blue p-4 overflow-auto">
							<SvelteMarkdown source={$flowStore.description} />
						</div>
					{:else}
						<div class="text-sm text-gray-500"> Enter a description to see the preview </div>
					{/if}
				</div>
			</TabContent>
			<TabContent value="schedule" class="p-4">
				<Alert type="info" title="Primary Schedule">
					Flows can be triggered by any schedules, their webhooks or their UI but they only have
					only one primary schedules with which they share the same path. The primary schedule can
					be set here.
				</Alert>
				<div class="mt-4" />
				<FlowSchedules />
			</TabContent>

			<TabContent value="same-worker" class="p-4 flex flex-col">
				<Alert type="info" title="Toggle Same Worker to have all steps be ran on the same worker">
					Steps will be run one after the other on the same worker, and will share a folder at
					`/shared` in which they can store heavier data and pass them to the next step. <br /><br
					/>Beware that the `/shared` folder is not preserved across suspends and sleeps.
				</Alert>
				<span class="my-2 text-sm font-bold">Same Worker</span>
				<Toggle
					bind:checked={$flowStore.value.same_worker}
					options={{
						right: 'Same Worker'
					}}
				/>
			</TabContent>
		</svelte:fragment>
	</Tabs>
</FlowCard>
