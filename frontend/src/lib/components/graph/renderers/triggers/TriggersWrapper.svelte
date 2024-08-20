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
	import TriggerCount from './TriggerCount.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { createEventDispatcher } from 'svelte'

	let schedules: Schedule[] | undefined = undefined

	export let path: string
	export let isEditor: boolean

	const dispatch = createEventDispatcher()

	async function loadSchedules() {
		if (!path) return
		try {
			schedules = await ScheduleService.listSchedules({
				workspace: $workspaceStore ?? '',
				path: path,
				isFlow: true
			})
		} catch (e) {
			console.error('impossible to load schedules')
		}
	}

	loadSchedules()

	let drawer: Drawer | undefined = undefined
	let selectedTab: 'webhooks' | 'mail' | 'schedules' = 'webhooks'
</script>

{#if isEditor}
	<Drawer bind:this={drawer} size="600px">
		<DrawerContent title="Triggers" noPadding on:close={drawer.closeDrawer}>
			<Tabs bind:selected={selectedTab}>
				<Tab value="webhooks">Webhooks</Tab>
				<Tab value="mail">Mail</Tab>
				<svelte:fragment slot="content">
					{#if selectedTab === 'webhooks'}
						<WebhooksPanel scopes={[`run:flow/${path}`]} {path} isFlow={true} args={{}} token="" />
					{/if}

					{#if selectedTab === 'mail'}
						<EmailTriggerPanel token="" scopes={[`run:flow/${path}`]} {path} isFlow={true} />
					{/if}
				</svelte:fragment>
			</Tabs>
		</DrawerContent>
	</Drawer>
{/if}

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
						if (isEditor) {
							selectedTab = 'webhooks'
							drawer?.openDrawer()
						} else {
							dispatch('triggerDetail', 'webhooks')
						}
					}}
				>
					<Webhook size={12} />
				</TriggerButton>
			</Popover>
			<Popover>
				<svelte:fragment slot="text">See default mail trigger</svelte:fragment>
				<TriggerButton
					on:click={() => {
						if (isEditor) {
							selectedTab = 'mail'
							drawer?.openDrawer()
						} else {
							dispatch('triggerDetail', 'mail')
						}
					}}
				>
					<Mail size={12} />
				</TriggerButton>
			</Popover>
			<Popover>
				<svelte:fragment slot="text">See all schedules</svelte:fragment>
				<TriggerButton
					on:click={() => {
						if (isEditor) {
							dispatch('openSchedules')
						} else {
							dispatch('triggerDetail', 'schedule')
						}
					}}
				>
					<TriggerCount count={schedules?.length} />
					<Calendar size={12} />
				</TriggerButton>
			</Popover>
		</div>
	</div>
</div>
