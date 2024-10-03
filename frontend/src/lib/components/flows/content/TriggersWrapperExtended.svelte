<script lang="ts">
	import { Calendar, Mail, Webhook, Clipboard, PlusIcon } from 'lucide-svelte'
	import TriggerButton from '../../graph/renderers/triggers/TriggerButton.svelte'
	import {
		HttpTriggerService,
		ScheduleService,
		SettingService,
		type HttpTrigger,
		type Schedule
	} from '$lib/gen'
	import { userStore, workspaceStore } from '$lib/stores'
	import Popover from '$lib/components/Popover.svelte'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import { DrawerContent, Tabs, Button } from '$lib/components/common'
	import WebhooksPanel from '$lib/components/details/WebhooksPanel.svelte'
	import WebhooksQuickConfig from '$lib/components/details/webhookQuickConfig.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import EmailTriggerPanel from '$lib/components/details/EmailTriggerPanel.svelte'
	import { createEventDispatcher, onMount } from 'svelte'
	import { Route } from 'lucide-svelte'
	import { canWrite, copyToClipboard, generateRandomString } from '$lib/utils'
	import RoutesPanel from '$lib/components/triggers/RoutesPanel.svelte'
	import RunPageSchedules from '$lib/components/RunPageSchedules.svelte'
	import RouteEditor from '$lib/components/triggers/RouteEditor.svelte'
	import PrimarySchedule from '$lib/components/PrimarySchedule.svelte'
	import ScheduleEditor from '$lib/components/ScheduleEditor.svelte'
	import { page } from '$app/stores'
	import { isCloudHosted } from '$lib/cloud'
	import { base } from '$lib/base'
	import { sendUserToast } from '$lib/toast'
	import UserSettings from '$lib/components/UserSettings.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { base32 } from 'rfc4648'

	let schedules: Schedule[] | undefined = undefined
	let schedule: Schedule | undefined = undefined
	let scheduleEditor: ScheduleEditor
	let triggers: (HttpTrigger & { canWrite: boolean })[] | undefined = undefined
	let emailToken: string = ''
	let routeEditor: RouteEditor
	let userSettings: UserSettings
	let emailDomain: string | null = null
	export let path: string
	export let isEditor: boolean
	export let newFlow: boolean
	export let hash: string | undefined = undefined
	export let isFlow: boolean = true

	let primaryScheduleExists: boolean = false

	const dispatch = createEventDispatcher()

	export async function loadSchedule() {
		try {
			let exists = await ScheduleService.existsSchedule({
				workspace: $workspaceStore ?? '',
				path
			})
			if (exists) {
				schedule = await ScheduleService.getSchedule({
					workspace: $workspaceStore ?? '',
					path
				})
			} else {
				schedule = undefined
			}
		} catch (e) {
			console.log('no primary schedule')
		}
	}

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
					isFlow: isFlow
				})
			).filter((s) => s.path != path)
		} catch (e) {
			console.error('impossible to load schedules')
		}
	}

	async function setScheduleEnabled(path: string, enabled: boolean): Promise<void> {
		try {
			await ScheduleService.setScheduleEnabled({
				path,
				workspace: $workspaceStore!,
				requestBody: { enabled }
			})
			loadSchedule()

			sendUserToast(`Schedule ${enabled ? 'enabled' : 'disabled'}`)
		} catch (err) {
			sendUserToast(`Cannot ` + (enabled ? 'disable' : 'enable') + ` schedule: ${err}`, true)
			loadSchedule()
		}
	}

	async function loadTriggers() {
		try {
			triggers = (
				await HttpTriggerService.listHttpTriggers({
					workspace: $workspaceStore ?? '',
					path,
					isFlow: isFlow
				})
			).map((x) => {
				return { canWrite: canWrite(x.path, x.extra_perms!, $userStore), ...x }
			})
		} catch (e) {
			console.error('impossible to load http routes')
		}
	}

	function computeFullRoute(path: string, route_path: string): string {
		return `${$page.url.origin}${base}/api/r/${
			isCloudHosted() ? $workspaceStore + '/' : ''
		}${route_path}`
	}

	onMount(() => {
		if (!newFlow) {
			loadSchedule()
			loadSchedules()
			loadTriggers()
		}
	})

	let drawer: Drawer | undefined = undefined
	let selectedTab: 'webhooks' | 'mail' | 'routes' | 'schedules' = 'webhooks'

	$: emailTokenEmpty = emailToken.length === 0

	let requestType: 'hash' | 'path' = 'path'

	async function getEmailDomain() {
		emailDomain =
			((await SettingService.getGlobal({
				key: 'email_domain'
			})) as any) ?? null
	}
	getEmailDomain()

	function emailAddress() {
		const pathOrHash = requestType === 'hash' ? hash : path.replaceAll('/', '.')
		const plainPrefix = `${$workspaceStore}+${
			(requestType === 'hash' ? 'hash.' : isFlow ? 'flow.' : '') + pathOrHash
		}+${emailToken}`
		const encodedPrefix = base32
			.stringify(new TextEncoder().encode(plainPrefix), {
				pad: false
			})
			.toLowerCase()
		return `${pathOrHash}+${encodedPrefix}@${emailDomain}`
	}

	export let email: string = ''

	$: email = emailAddress()
</script>

<RouteEditor
	on:update={() => {
		loadTriggers()
	}}
	bind:this={routeEditor}
/>

<ScheduleEditor
	on:update={() => {
		loadSchedule()
		loadSchedules()
	}}
	bind:this={scheduleEditor}
/>

<UserSettings
	bind:this={userSettings}
	on:tokenCreated={(e) => {
		emailToken = e.detail
	}}
	newTokenLabel={`${$userStore?.username ?? 'superadmin'}-${generateRandomString(4)}`}
	scopes={[`run:flow/${path}`]}
/>

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
							/>
						{/if}

						{#if selectedTab === 'mail'}
							<EmailTriggerPanel token="" scopes={[`run:flow/${path}`]} {path} {isFlow} />
						{/if}

						{#if !newFlow && selectedTab === 'routes'}
							<RoutesPanel path={path ?? ''} {isFlow} bind:triggers />
						{/if}

						{#if !newFlow && selectedTab === 'schedules'}
							<RunPageSchedules
								{isFlow}
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
	<div class="flex flex-row gap-2 items-start">
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
				<Calendar size={12} />
			</TriggerButton>
		</Popover>
		<div class="flex flex-col w-full gap-2">
			{#if primaryScheduleExists && schedule}
				<PrimarySchedule
					{schedule}
					isFlow={true}
					path={path ?? ''}
					can_write={canWrite(path, {}, $userStore)}
					{scheduleEditor}
					{setScheduleEnabled}
					light={true}
				/>
			{/if}
			{#if schedules?.length == 0 || !schedules}
				<div class="text-xs text-secondary px-2"> No other schedules </div>
			{:else}
				<div class="flex flex-col w-full divide-y px-2 gap-2">
					{#each schedules as schedule (schedule.path)}
						<div class="flex flex-row w-full flex-nowrap text-xs text-primary items-center">
							<div class="truncate flex-initial">{schedule.path}</div>
							<div class="flex flex-row grow gap-2 shrink-0 items-center justify-center">
								<div>{schedule.schedule}</div>
								<div>{schedule.enabled ? 'on' : 'off'}</div>
							</div>
							<button
								class="flex-shrink-0"
								on:click={() => scheduleEditor?.openEdit(schedule.path, true)}>Edit</button
							>
						</div>
					{/each}
				</div>
			{/if}
		</div>
	</div>
	<div class="w-full flex flex-col gap-2.5 mt-1">
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
			<WebhooksQuickConfig scopes={[`run:flow/${path}`]} {path} isFlow={true} args={{}} token="" />
		</div>

		<div class="flex flex-row gap-2 w-full items-center">
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
			<input bind:value={emailToken} placeholder="paste your token here" class="!text-xs" />
			<Button
				spacingSize="xs2"
				size="xs"
				color="light"
				variant="border"
				on:click={userSettings.openDrawer}
			>
				<PlusIcon size={12} />
				<Tooltip light>
					The token will have a scope such that it can only be used to trigger this script. It is
					safe to share as it cannot be used to impersonate you.
				</Tooltip>
			</Button>
			<Button
				size="xs"
				color="light"
				variant="border"
				on:click={() => {
					copyToClipboard(email)
				}}
				disabled={emailTokenEmpty}
			>
				Get email address
			</Button>
		</div>
		<div class="flex flex-row items-start gap-2 w-full">
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
					<Route size={12} />
				</TriggerButton>
			</Popover>
			{#if triggers?.length == 0 || !triggers}
				<div class="text-xs text-secondary px-2"> No http routes </div>
			{:else}
				<div class="flex flex-col divide-y w-full px-2 gap-2">
					{#each triggers as trigger (trigger.path)}
						<div class="flex flex-row w-full flex-nowrap text-xs text-primary items-center">
							<div class="truncate flex-initial">{trigger.path}</div>
							<div class="flex flex-row grow gap-2 shrink-0 items-center justify-center">
								<div class="truncate mx-2 text-xs text-tertiary font-mono">
									{trigger.http_method.toUpperCase()} /{trigger.route_path}
								</div>
								<button
									class="flex-shrink-0"
									on:click={(e) => {
										e.preventDefault()
										copyToClipboard(computeFullRoute(trigger.path, trigger.route_path))
									}}
									title="Copy full endpoint path"
								>
									<Clipboard size={12} />
								</button>
							</div>
							<button
								class="flex-shrink-0"
								on:click={() => routeEditor?.openEdit(trigger.path, true)}
							>
								{#if trigger.canWrite}
									Edit
								{:else}
									View
								{/if}
							</button>
						</div>
					{/each}
				</div>
			{/if}
		</div>
	</div>
{/if}
