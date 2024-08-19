<script lang="ts">
	import { Calendar, Mail, Webhook } from 'lucide-svelte'
	import TriggerButton from './TriggerButton.svelte'
	import { NODE } from '../../util'
	import { ScheduleService, type Schedule } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import Popover from '$lib/components/Popover.svelte'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import { DrawerContent, Tabs } from '$lib/components/common'
	import WebhooksPanel from '$lib/components/details/WebhooksPanel.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import EmailTriggerPanel from '$lib/components/details/EmailTriggerPanel.svelte'
	import type { FlowEditorContext } from '$lib/components/flows/types'
	import { getContext } from 'svelte'
	import TriggerCount from './TriggerCount.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	let schedules: Schedule[] | undefined = undefined
	const { selectedId, pathStore } = getContext<FlowEditorContext>('FlowEditorContext')
	async function loadSchedules() {
		if (!$pathStore) return
		try {
			schedules = await ScheduleService.listSchedules({
				workspace: $workspaceStore ?? '',
				path: $pathStore,
				isFlow: true
			})
		} catch (e) {
			console.error('impossible to load schedules')
		}
	}
	loadSchedules()
	let drawer: Drawer | undefined = undefined
	let selectedTab: 'webhooks' | 'mail' = 'webhooks'
</script>

<Drawer bind:this={drawer} size="600px">
	<DrawerContent
		title="Triggers"
		noPadding
		on:close={drawer.closeDrawer}
		tooltip="Resources represent connections to third party systems. Learn more on how to integrate external APIs."
		documentationLink="https://www.windmill.dev/docs/integrations/integrations_on_windmill"
	>
		<Tabs bind:selected={selectedTab}>
			<Tab value="webhooks">Webhooks</Tab>
			<Tab value="mail">Mail</Tab>
			<svelte:fragment slot="content">
				{#if selectedTab === 'webhooks'}
					<WebhooksPanel
						scopes={[`run:flow/${$pathStore}`]}
						path={$pathStore}
						isFlow={true}
						args={{}}
						token=""
					/>
				{/if}

				{#if selectedTab === 'mail'}
					<EmailTriggerPanel
						token=""
						scopes={[`run:flow/${$pathStore}`]}
						path={$pathStore}
						isFlow={true}
					/>
				{/if}
			</svelte:fragment>
		</Tabs>
	</DrawerContent>
</Drawer>

<div style={`width: ${NODE.width}px;`}>
	<div class="flex flex-col mx-auto w-min">
		<div
			class="flex flex-row gap-2 px-2 border p-1 rounded-md border-surface-selected bg-surface shadow-md items-center"
		>
			<div class="flex flex-col">
				<div class="flex flex-row items-center text-2xs">
					Triggers

					<Tooltip small wrapperClass="center-center">
						The flow can be triggered by webhooks, emails, or schedules. Click on the icons to see
						the triggers.
					</Tooltip>
				</div>
			</div>
			<Popover>
				<svelte:fragment slot="text">See default webhooks triggers</svelte:fragment>
				<TriggerButton
					on:click={() => {
						selectedTab = 'webhooks'
						drawer?.openDrawer()
					}}
				>
					<Webhook size={12} />
				</TriggerButton>
			</Popover>
			<Popover>
				<svelte:fragment slot="text">See default mail trigger</svelte:fragment>
				<TriggerButton
					on:click={() => {
						selectedTab = 'mail'
						drawer?.openDrawer()
					}}
				>
					<Mail size={12} />
				</TriggerButton>
			</Popover>
			<Popover>
				<svelte:fragment slot="text">See all schedules</svelte:fragment>
				<TriggerButton
					on:click={() => {
						$selectedId = 'settings-schedule'
					}}
				>
					<TriggerCount count={schedules?.length} />
					<Calendar size={12} />
				</TriggerButton>
			</Popover>
		</div>
	</div>
</div>
