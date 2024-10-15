<script lang="ts">
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import { Tabs } from '$lib/components/common'
	import WebhooksPanel from '$lib/components/details/WebhooksPanel.svelte'
	import EmailTriggerPanel from '$lib/components/details/EmailTriggerPanel.svelte'
	import RoutesPanel from '$lib/components/triggers/RoutesPanel.svelte'
	import RunPageSchedules from '$lib/components/RunPageSchedules.svelte'
	import { canWrite } from '$lib/utils'
	import { userStore } from '$lib/stores'
	import FlowCard from '../common/FlowCard.svelte'
	import { getContext } from 'svelte'
	import type { FlowEditorContext } from '../types'
	import type { TriggerContext } from '$lib/components/triggers'

	export let noEditor: boolean
	export let newFlow = false
	let path = ''

	const { pathStore, flowStore } = getContext<FlowEditorContext>('FlowEditorContext')
	const { selectedTrigger } = getContext<TriggerContext>('TriggerContext')

	$: path = $pathStore
</script>

<FlowCard {noEditor} title="Flow Triggers">
	<div class="pt-4">
		<Tabs bind:selected={$selectedTrigger}>
			<Tab value="webhooks" selectedClass="text-primary font-semibold">Webhooks</Tab>
			<Tab value="emails" selectedClass="text-primary text-sm font-semibold">Email</Tab>
			<Tab value="routes" selectedClass="text-primary text-sm font-semibold">Routes</Tab>
			<Tab value="schedules" selectedClass="text-primary text-sm font-semibold">Schedules</Tab>

			<svelte:fragment slot="content">
				{#if $selectedTrigger === 'webhooks'}
					<div class="p-4">
						<WebhooksPanel
							{newFlow}
							scopes={[`run:flow/${path}`]}
							{path}
							isFlow={true}
							args={{}}
							token=""
						/>
					</div>
				{/if}

				{#if $selectedTrigger === 'emails'}
					<div class="p-4">
						<EmailTriggerPanel
							{newFlow}
							token=""
							scopes={[`run:flow/${path}`]}
							{path}
							isFlow={true}
						/>
					</div>
				{/if}

				{#if $selectedTrigger === 'routes'}
					<div class="p-4">
						<RoutesPanel {newFlow} path={path ?? ''} isFlow={true} />
					</div>
				{/if}

				{#if $selectedTrigger === 'schedules'}
					<div class="p-2">
						<RunPageSchedules
							schema={$flowStore.schema}
							isFlow={true}
							path={path ?? ''}
							newItem={newFlow}
							can_write={canWrite(path, {}, $userStore)}
						/>
					</div>
				{/if}
			</svelte:fragment>
		</Tabs>
	</div>
</FlowCard>
