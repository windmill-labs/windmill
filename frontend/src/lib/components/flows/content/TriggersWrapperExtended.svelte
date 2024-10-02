<script lang="ts">
	import { Calendar, Mail, Webhook } from 'lucide-svelte'
	import TriggerButton from '../../graph/renderers/triggers/TriggerButton.svelte'
	import { HttpTriggerService, ScheduleService, type HttpTrigger, type Schedule } from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import Popover from '$lib/components/Popover.svelte'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import { DrawerContent, Tabs } from '$lib/components/common'
	import WebhooksPanel from '$lib/components/details/WebhooksPanel.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import EmailTriggerPanel from '$lib/components/details/EmailTriggerPanel.svelte'
	import TriggerCount from '../../graph/renderers/triggers/TriggerCount.svelte'
	import { createEventDispatcher, onMount } from 'svelte'
	import { Route } from 'lucide-svelte'
	import { canWrite } from '$lib/utils'
	import RoutesPanel from '$lib/components/triggers/RoutesPanel.svelte'
	import RunPageSchedules from '$lib/components/RunPageSchedules.svelte'
	import ClipboardPanel from '$lib/components/details/ClipboardPanel.svelte'

	let schedules: Schedule[] | undefined = undefined
	let triggers: (HttpTrigger & { canWrite: boolean })[] | undefined = undefined
	let url: string = ''
	let email: string = ''

	export let path: string
	export let isEditor: boolean
	export let newFlow: boolean

	let primaryScheduleExists: boolean = false

	const dispatch = createEventDispatcher()

	async function loadSchedules() {
		if (!path) return
		try {
			primaryScheduleExists = await ScheduleService.existsSchedule({
				workspace: $workspaceStore ?? '',
				path
			})

			schedules = (
				await ScheduleService.listSchedules({
					workspace: $workspaceStore ?? '',
					path: path,
					isFlow: true
				})
			).filter((s) => s.path != path)
		} catch (e) {
			console.error('impossible to load schedules')
		}
	}

	async function loadTriggers() {
		try {
			triggers = (
				await HttpTriggerService.listHttpTriggers({
					workspace: $workspaceStore ?? '',
					path,
					isFlow: true
				})
			).map((x) => {
				return { canWrite: canWrite(x.path, x.extra_perms!, $userStore), ...x }
			})
		} catch (e) {
			console.error('impossible to load http routes')
		}
	}

	onMount(() => {
		if (!newFlow) {
			loadSchedules()
			loadTriggers()
		}
	})

	let drawer: Drawer | undefined = undefined
	let selectedTab: 'webhooks' | 'mail' | 'routes' | 'schedules' = 'webhooks'
</script>

{#if isEditor}
	{#if !newFlow}
		<Drawer bind:this={drawer} size="600px">
			<DrawerContent title="Triggers" noPadding on:close={drawer.closeDrawer}>
				<Tabs bind:selected={selectedTab}>
					<Tab value="webhooks" disabled={newFlow}>Webhooks</Tab>
					<Tab value="mail" disabled={newFlow}>Mail</Tab>
					<Tab value="routes" disabled={newFlow}>Routes</Tab>
					<Tab value="schedules" disabled={newFlow}>Schedules</Tab>

					<svelte:fragment slot="content">
						{#if selectedTab === 'webhooks'}
							<WebhooksPanel
								scopes={[`run:flow/${path}`]}
								{path}
								isFlow={true}
								args={{}}
								token=""
								bind:url
							/>
						{/if}

						{#if selectedTab === 'mail'}
							<EmailTriggerPanel
								token=""
								scopes={[`run:flow/${path}`]}
								{path}
								isFlow={true}
								bind:email
							/>
						{/if}

						{#if !newFlow && selectedTab === 'routes'}
							<RoutesPanel path={path ?? ''} isFlow={true} bind:triggers />
						{/if}

						{#if !newFlow && selectedTab === 'schedules'}
							<RunPageSchedules
								isFlow={true}
								path={path ?? ''}
								can_write={canWrite(path, {}, $userStore)}
								bind:schedules
							/>
						{/if}
					</svelte:fragment>
				</Tabs>
			</DrawerContent>
		</Drawer>
	{/if}
	<div class="w-full flex flex-col gap-2 mt-1">
		<div class="flex flex-row grow gap-2">
			<Popover>
				<svelte:fragment slot="text">
					{#if newFlow}
						Deploy the flow to see webhooks triggers
					{:else}
						See default webhooks triggers
					{/if}
				</svelte:fragment>
				<TriggerButton
					on:click={() => {
						if (isEditor) {
							selectedTab = 'webhooks'
							drawer?.openDrawer()
						} else {
							dispatch('triggerDetail', 'webhooks')
						}
					}}
					disabled={newFlow}
				>
					<Webhook size={12} />
				</TriggerButton>
			</Popover>
			<ClipboardPanel content={url} />
		</div>

		<div class="flex flex-row gap-2 w-full">
			<Popover>
				<svelte:fragment slot="text">
					{#if newFlow}
						Deploy the flow to see email triggers
					{:else}
						See all email triggers
					{/if}
				</svelte:fragment>
				<TriggerButton
					on:click={() => {
						if (isEditor) {
							selectedTab = 'mail'
							drawer?.openDrawer()
						} else {
							dispatch('triggerDetail', 'mail')
						}
					}}
					disabled={newFlow}
				>
					<Mail size={12} />
				</TriggerButton>
			</Popover>
			<ClipboardPanel content={email} />
		</div>
		<div class="flex flex-row gap-2">
			<Popover>
				<svelte:fragment slot="text">
					{#if newFlow}
						Deploy the flow to add routes triggers
					{:else}
						See all routes triggers
					{/if}
				</svelte:fragment>
				<TriggerButton
					on:click={() => {
						if (isEditor) {
							selectedTab = 'routes'
							drawer?.openDrawer()
						} else {
							dispatch('triggerDetail', 'routes')
						}
					}}
					disabled={newFlow}
				>
					<TriggerCount count={triggers?.length} />
					<Route size={12} />
				</TriggerButton>
			</Popover>
			Routes
		</div>
		<div class="flex flex-row gap-2">
			<Popover>
				<svelte:fragment slot="text">
					{#if newFlow}
						Deploy the flow to add schedules triggers
					{:else}
						See all schedules triggers
					{/if}
				</svelte:fragment>
				<TriggerButton
					on:click={() => {
						if (isEditor) {
							selectedTab = 'schedules'
							drawer?.openDrawer()
						} else {
							dispatch('triggerDetail', 'schedule')
						}
					}}
					disabled={newFlow}
				>
					<TriggerCount count={(schedules?.length ?? 0) + (primaryScheduleExists ? 1 : 0)} />
					<Calendar size={12} />
				</TriggerButton>
			</Popover>
			Schedules
		</div>
	</div>
{/if}
