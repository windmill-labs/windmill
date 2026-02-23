<script lang="ts">
	import MenuLink from './MenuLink.svelte'
	import {
		superadmin,
		usedTriggerKinds,
		userStore,
		workspaceStore,
		isCriticalAlertsUIOpen,
		enterpriseLicense,
		devopsRole,
		tutorialsToDo,
		skippedAll
	} from '$lib/stores'
	import { syncTutorialsTodos } from '$lib/tutorialUtils'
	import { SIDEBAR_SHOW_SCHEDULES } from '$lib/consts'
	import {
		BookOpen,
		ServerCog,
		Boxes,
		Calendar,
		DollarSign,
		Eye,
		Logs,
		FolderCog,
		FolderOpen,
		Github,
		GraduationCap,
		HelpCircle,
		Home,
		LogOut,
		Newspaper,
		Play,
		Route,
		Settings,
		UserCog,
		Plus,
		Unplug,
		AlertCircle,
		Database,
		Pyramid,
		Trash2,
		MailIcon
	} from 'lucide-svelte'
	import UserMenu from './UserMenu.svelte'
	import DiscordIcon from '../icons/brands/Discord.svelte'
	import { WorkspaceService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { clearStores } from '$lib/storeUtils'
	import { goto } from '$lib/navigation'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import { twMerge } from 'tailwind-merge'
	import { onMount } from 'svelte'
	import { base } from '$lib/base'
	import { type Changelog, changelogs } from './changelogs'
	import { page } from '$app/stores'
	import SideBarNotification from './SideBarNotification.svelte'
	import KafkaIcon from '../icons/KafkaIcon.svelte'
	import NatsIcon from '../icons/NatsIcon.svelte'
	import MqttIcon from '../icons/MqttIcon.svelte'
	import AwsIcon from '../icons/AwsIcon.svelte'
	import {
		getAvailableNativeTriggerServices,
		getServiceConfig,
		getServiceIcon
	} from '../triggers/native/utils'
	import type { NativeServiceName } from '$lib/gen/types.gen'
	import {
		Menubar,
		Menu,
		MenuSingleItem,
		MenuItem,
		MeltButton
	} from '$lib/components/meltComponents'
	import MenuButton from './MenuButton.svelte'
	import GoogleCloudIcon from '../icons/GoogleCloudIcon.svelte'

	async function leaveWorkspace() {
		await WorkspaceService.leaveWorkspace({ workspace: $workspaceStore ?? '' })
		sendUserToast('You left the workspace')
		clearStores()
		goto('/user/workspaces')
	}

	async function deleteFork() {
		await WorkspaceService.deleteWorkspace({ workspace: $workspaceStore ?? '' })
		sendUserToast('You deleted the workspace')
		clearStores()
		goto('/user/workspaces')
	}

	let hasNewChangelogs = $state(false)
	let recentChangelogs: Changelog[] = $state([])
	let lastOpened = localStorage.getItem('changelogsLastOpened')
	let availableNativeServices = $state<
		Array<{ service: NativeServiceName; icon: any; config: any }>
	>([])

	async function loadAvailableNativeTriggers() {
		try {
			const services = await getAvailableNativeTriggerServices($workspaceStore!)
			const serviceData = await Promise.all(
				services.map(async (service) => ({
					service,
					icon: await getServiceIcon(service),
					config: getServiceConfig(service)
				}))
			)
			availableNativeServices = serviceData
		} catch (err) {
			console.error('Failed to load available native trigger services:', err)
			availableNativeServices = []
		}
	}

	loadAvailableNativeTriggers()

	onMount(async () => {
		if (lastOpened) {
			// @ts-ignore
			recentChangelogs = changelogs.filter((changelog) => changelog.date > lastOpened)
			hasNewChangelogs =
				recentChangelogs.length > 0 && lastOpened !== new Date().toISOString().split('T')[0]
		} else {
			recentChangelogs = changelogs.slice(0, 3)
		}
		// Sync tutorial progress on mount
		await syncTutorialsTodos()
	})

	function openChangelogs() {
		const today = new Date().toISOString().split('T')[0]
		localStorage.setItem('changelogsLastOpened', today)
		hasNewChangelogs = false
	}

	const thirdMenuLinks = [
		{
			label: 'Help',
			icon: HelpCircle,
			subItems: [
				{
					label: 'Tutorials',
					href: `${base}/tutorials`,
					icon: GraduationCap,
					aiId: 'sidebar-menu-link-tutorials',
					aiDescription: 'Button to navigate to tutorials',
					external: false
				},
				{
					label: 'Docs',
					href: 'https://www.windmill.dev/docs/intro/',
					icon: BookOpen,
					aiId: 'sidebar-menu-link-docs',
					aiDescription: 'Button to navigate to docs',
					external: true
				},
				{
					label: 'Feedbacks',
					href: 'https://discord.gg/V7PM2YHsPB',
					icon: DiscordIcon,
					aiId: 'sidebar-menu-link-feedbacks',
					aiDescription: 'Button to navigate to feedbacks',
					external: true
				},
				{
					label: 'Issues',
					href: 'https://github.com/windmill-labs/windmill/issues/new',
					icon: Github,
					aiId: 'sidebar-menu-link-issues',
					aiDescription: 'Button to navigate to issues',
					external: true
				},
				{
					label: 'Changelog',
					href: 'https://www.windmill.dev/changelog/',
					icon: Newspaper,
					aiId: 'sidebar-menu-link-changelog',
					aiDescription: 'Button to navigate to changelog',
					external: true
				}
			]
		}
	]

	interface Props {
		numUnacknowledgedCriticalAlerts?: number
		isCollapsed?: boolean
	}

	let { numUnacknowledgedCriticalAlerts = 0, isCollapsed = false }: Props = $props()

	let leaveWorkspaceModal = $state(false)
	let deleteWorkspaceForkModal = $state(false)

	function computeAllNotificationsCount(menuItems: any[]) {
		let count = 0
		for (const menuItem of menuItems) {
			count += menuItem?.['notificationCount'] ?? 0
		}
		return count
	}

	const itemClass = twMerge(
		'text-secondary font-normal w-full block px-4 py-2 text-2xs data-[highlighted]:bg-surface-hover data-[highlighted]:text-primary'
	)
	let mainMenuLinks = $derived([
		{
			label: 'Home',
			href: `${base}/`,
			icon: Home,
			aiId: 'sidebar-menu-link-home',
			aiDescription:
				"Button to navigate to home which contains all the user's scripts, flows and apps"
		},
		{
			label: 'Runs',
			href: `${base}/runs`,
			icon: Play,
			aiId: 'sidebar-menu-link-runs',
			aiDescription: 'Button to navigate to runs',
			onclick: () => {
				setTimeout(() => {
					window.dispatchEvent(new Event('popstate'))
				}, 100)
			}
		},
		{
			label: 'Variables',
			href: `${base}/variables`,
			icon: DollarSign,
			disabled: $userStore?.operator,
			aiId: 'sidebar-menu-link-variables',
			aiDescription: 'Button to navigate to variables'
		},
		{
			label: 'Resources',
			href: `${base}/resources`,
			icon: Boxes,
			disabled: $userStore?.operator,
			aiId: 'sidebar-menu-link-resources',
			aiDescription: 'Button to navigate to resources'
		},
		{
			label: 'Assets',
			href: `${base}/assets`,
			icon: Pyramid,
			disabled: $userStore?.operator,
			aiId: 'sidebar-menu-link-assets',
			aiDescription: 'Button to navigate to assets'
		},
		// Add Tutorials to main menu only if not all completed and not skipped
		...($tutorialsToDo.length > 0 && !$skippedAll
			? [
					{
						label: 'Tutorials',
						href: `${base}/tutorials`,
						icon: GraduationCap,
						aiId: 'sidebar-menu-link-tutorials-main',
						aiDescription: 'Button to navigate to tutorials'
					}
				]
			: [])
	])
	let defaultExtraTriggerLinks = $derived([
		{
			label: 'HTTP',
			href: '/routes',
			icon: Route,
			disabled: $userStore?.operator,
			kind: 'http',
			aiId: 'sidebar-menu-link-http',
			aiDescription: 'Button to navigate to HTTP routes'
		},
		{
			label: 'WebSockets',
			href: '/websocket_triggers',
			icon: Unplug,
			disabled: $userStore?.operator,
			kind: 'ws',
			aiId: 'sidebar-menu-link-ws',
			aiDescription: 'Button to navigate to websocket triggers'
		},
		{
			label: 'Postgres',
			href: '/postgres_triggers',
			icon: Database,
			disabled: $userStore?.operator,
			kind: 'postgres',
			aiId: 'sidebar-menu-link-postgres',
			aiDescription: 'Button to navigate to Postgres triggers'
		},
		{
			label: 'Kafka' + ($enterpriseLicense ? '' : ' (EE)'),
			href: '/kafka_triggers',
			icon: KafkaIcon,
			disabled: $userStore?.operator || !$enterpriseLicense,
			kind: 'kafka',
			aiId: 'sidebar-menu-link-kafka',
			aiDescription: 'Button to navigate to Kafka triggers'
		},
		{
			label: 'NATS' + ($enterpriseLicense ? '' : ' (EE)'),
			href: '/nats_triggers',
			icon: NatsIcon,
			disabled: $userStore?.operator || !$enterpriseLicense,
			kind: 'nats',
			aiId: 'sidebar-menu-link-nats',
			aiDescription: 'Button to navigate to NATS triggers'
		},
		{
			label: 'SQS' + ($enterpriseLicense ? '' : ' (EE)'),
			href: '/sqs_triggers',
			icon: AwsIcon,
			disabled: $userStore?.operator || !$enterpriseLicense,
			kind: 'sqs',
			aiId: 'sidebar-menu-link-sqs',
			aiDescription: 'Button to navigate to SQS triggers'
		},
		{
			label: 'GCP Pub/Sub' + ($enterpriseLicense ? '' : ' (EE)'),
			href: '/gcp_triggers',
			icon: GoogleCloudIcon,
			disabled: $userStore?.operator || !$enterpriseLicense,
			kind: 'gcp',
			aiId: 'sidebar-menu-link-gcp',
			aiDescription: 'Button to navigate to GCP Pub/Sub triggers'
		},
		{
			label: 'MQTT',
			href: '/mqtt_triggers',
			icon: MqttIcon,
			disabled: $userStore?.operator,
			kind: 'mqtt',
			aiId: 'sidebar-menu-link-mqtt',
			aiDescription: 'Button to navigate to MQTT triggers'
		},
		{
			label: 'Email',
			href: '/email_triggers',
			icon: MailIcon,
			disabled: $userStore?.operator,
			kind: 'email',
			aiId: 'sidebar-menu-link-email',
			aiDescription: 'Button to navigate to Email triggers'
		}
	])

	let nativeTriggerLinks = $derived(
		availableNativeServices.map(({ service, icon, config }) => ({
			label: config?.serviceDisplayName || service,
			href: `/native_triggers/${service}`,
			icon: icon,
			disabled: $userStore?.operator,
			kind: service,
			aiId: `sidebar-menu-link-${service}`,
			aiDescription: `Button to navigate to ${config?.serviceDisplayName || service} triggers`
		}))
	)

	let allTriggerLinks = $derived([...defaultExtraTriggerLinks, ...nativeTriggerLinks])

	let triggerMenuLinks = $derived([
		{
			label: 'Schedules',
			href: `${base}/schedules`,
			icon: Calendar,
			disabled: !SIDEBAR_SHOW_SCHEDULES || $userStore?.operator,
			aiId: 'sidebar-menu-link-schedules',
			aiDescription: 'Button to navigate to schedules'
		},
		...allTriggerLinks.filter(
			(link) => $usedTriggerKinds.includes(link.kind) || $page.url.pathname.includes(link.href)
		)
	])
	let extraTriggerLinks = $derived(
		allTriggerLinks.filter((link) => {
			return !$page.url.pathname.includes(link.href) && !$usedTriggerKinds.includes(link.kind)
		})
	)
	let secondaryMenuLinks = $derived([
		// {
		// 	label: 'Workspace',
		// 	href: '/workspace_settings',
		// 	icon: Cog,
		// 	disabled: !$userStore?.is_admin && !$userStore?.is_super_admin
		// },
		{
			label: 'Settings',
			icon: Settings,
			aiId: 'sidebar-menu-link-settings',
			aiDescription:
				'Button to navigate to settings, including account, workspace, and instance settings',
			subItems: [
				{
					label: 'Account',
					href: '#user-settings',
					icon: UserCog,
					aiId: 'sidebar-menu-link-account',
					aiDescription: 'Button to navigate to account settings',
					faIcon: undefined
				},
				...($userStore?.is_admin || $superadmin
					? [
							{
								label: 'Workspace',
								href: `${base}/workspace_settings`,
								icon: FolderCog,
								aiId: 'sidebar-menu-link-workspace',
								aiDescription: 'Button to navigate to workspace settings',
								faIcon: undefined
							}
						]
					: []),
				...($superadmin
					? [
							{
								label: 'Instance',
								href: '#superadmin-settings',
								icon: ServerCog,
								aiId: 'sidebar-menu-link-instance',
								aiDescription: 'Button to navigate to instance settings',
								faIcon: undefined
							}
						]
					: []),
				...(!$superadmin && !$userStore?.is_admin
					? [
							{
								label: 'Leave workspace',
								action: () => {
									leaveWorkspaceModal = true
								},
								class: 'text-red-400',
								icon: LogOut,
								faIcon: undefined
							}
						]
					: []),
				...($workspaceStore?.startsWith('wm-fork-')
					? [
							{
								label: 'Delete Forked Workspace',
								action: () => {
									deleteWorkspaceForkModal = true
								},
								icon: Trash2,
								faIcon: undefined
							}
						]
					: [])
			],
			disabled: $userStore?.operator
		},
		{
			label: 'Workers',
			href: `${base}/workers`,
			icon: ServerCog,
			disabled: $userStore?.operator,
			aiId: 'sidebar-menu-link-workers',
			aiDescription: 'Button to navigate to workers'
		},
		{
			label: 'Folders & Groups',
			icon: FolderOpen,
			aiId: 'sidebar-menu-link-folders-groups',
			aiDescription: 'Button to navigate to folders and groups',
			subItems: [
				{
					label: 'Folders',
					href: `${base}/folders`,
					icon: FolderOpen,
					disabled: $userStore?.operator,
					aiId: 'sidebar-menu-link-folders',
					aiDescription: 'Button to navigate to folders',
					faIcon: undefined
				},
				{
					label: 'Groups',
					href: `${base}/groups`,
					icon: UserCog,
					disabled: $userStore?.operator,
					aiId: 'sidebar-menu-link-groups',
					aiDescription: 'Button to navigate to groups',
					faIcon: undefined
				}
			],
			disabled: $userStore?.operator
		},
		$devopsRole || $userStore?.is_admin
			? {
					label: 'Logs',
					icon: Logs,
					aiId: 'sidebar-menu-link-logs',
					aiDescription: 'Button to navigate to logs',
					subItems: [
						{
							label: 'Audit logs',
							href: `${base}/audit_logs`,
							icon: Eye,
							aiId: 'sidebar-menu-link-audit-logs',
							aiDescription: 'Button to navigate to audit logs'
						},
						...($devopsRole
							? [
									{
										label: 'Service logs',
										href: `${base}/service_logs`,
										icon: Logs,
										aiId: 'sidebar-menu-link-service-logs',
										aiDescription: 'Button to navigate to service logs'
									}
								]
							: []),
						...($enterpriseLicense
							? [
									{
										label: 'Critical alerts',
										action: () => {
											isCriticalAlertsUIOpen.set(true)
										},
										icon: AlertCircle,
										notificationCount: numUnacknowledgedCriticalAlerts,
										aiId: 'sidebar-menu-link-critical-alerts',
										aiDescription: 'Button to navigate to critical alerts'
									}
								]
							: [])
					]
				}
			: {
					label: 'Audit logs',
					href: `${base}/audit_logs`,
					icon: Eye,
					disabled: $userStore?.operator,
					aiId: 'sidebar-menu-link-audit-logs',
					aiDescription: 'Button to navigate to audit logs'
				}
	])
</script>

<nav
	class={twMerge(
		'grow flex flex-col overflow-x-hidden scrollbar-hidden px-2 md:pb-2 justify-between gap-2'
	)}
>
	<div class={twMerge('pt-4 mb-6 md:mb-10')}>
		<div class="space-y-1">
			{#each mainMenuLinks as menuLink (menuLink.href ?? menuLink.label)}
				<MenuLink class="!text-xs" {...menuLink} {isCollapsed} />
			{/each}
		</div>
		<div class="pt-4">
			<div
				class="text-secondary text-[0.5rem] uppercase transition-opacity"
				class:opacity-0={isCollapsed}>Triggers</div
			>
			<Menubar class="flex flex-col gap-1">
				{#snippet children({ createMenu })}
					{#each triggerMenuLinks as menuLink (menuLink.href ?? menuLink.label)}
						<MenuLink class="!text-xs" {...menuLink} {isCollapsed} />
					{/each}
					{#if extraTriggerLinks.length > 0 && !$userStore?.operator}
						<Menu {createMenu} usePointerDownOutside>
							{#snippet triggr({ trigger })}
								<MeltButton
									aiId="sidebar-menu-link-add-trigger"
									aiDescription="Button to add a new trigger. Can be HTTP, WebSocket, Postgres, Kafka, NATS, SQS, GCP Pub/Sub, or MQTT"
									class={twMerge(
										'w-full text-secondary text-2xs flex flex-row gap-1 py-1 items-center px-2 hover:bg-surface-hover rounded',
										'data-[highlighted]:bg-surface-hover'
									)}
									meltElement={trigger}
								>
									<Plus size={14} />
								</MeltButton>
							{/snippet}
							{#snippet children({ item })}
								{#each extraTriggerLinks as subItem (subItem.href ?? subItem.label)}
									<MenuItem
										aiId={subItem.aiId}
										aiDescription={subItem.aiDescription}
										href={subItem.disabled ? '' : subItem.href}
										class={twMerge(
											itemClass,
											subItem.disabled ? 'pointer-events-none opacity-50' : ''
										)}
										{item}
										disabled={subItem.disabled}
									>
										<div class="flex flex-row items-center gap-2">
											{#if subItem.icon}
												<subItem.icon size={16} />
											{/if}
											{subItem.label}
										</div>
									</MenuItem>
								{/each}
							{/snippet}
						</Menu>
					{/if}
				{/snippet}
			</Menubar>
		</div>
	</div>
	<div class="flex flex-col h-full justify-end">
		<Menubar class="flex flex-col gap-1 mb-6 md:mb-10">
			{#snippet children({ createMenu })}
				<UserMenu {isCollapsed} {createMenu} />

				{#each secondaryMenuLinks as menuLink (menuLink.href ?? menuLink.label)}
					{#if menuLink.subItems}
						{@const notificationsCount = computeAllNotificationsCount(menuLink.subItems)}
						<Menu {createMenu} usePointerDownOutside>
							{#snippet triggr({ trigger })}
								<MenuButton
									class="!text-2xs"
									{...menuLink}
									{isCollapsed}
									{notificationsCount}
									{trigger}
								/>
							{/snippet}

							{#snippet children({ item })}
								{#each menuLink.subItems as subItem (subItem.href ?? subItem.label)}
									<MenuItem
										class={itemClass}
										href={subItem.href}
										{item}
										onClick={() => {
											subItem?.['action']?.()
										}}
										aiId={subItem.aiId}
										aiDescription={subItem.aiDescription}
									>
										<div class="flex flex-row items-center gap-2">
											{#if subItem.icon}
												<subItem.icon size={16} />
											{/if}
											{subItem.label}
											{#if subItem?.['notificationCount']}
												<div class="ml-auto">
													<SideBarNotification notificationCount={subItem['notificationCount']} />
												</div>
											{/if}
										</div>
									</MenuItem>
								{/each}
							{/snippet}
						</Menu>
					{:else}
						<MenuSingleItem>
							{#snippet children({})}
								<MenuLink class="!text-2xs" {...menuLink} {isCollapsed} />
							{/snippet}
						</MenuSingleItem>
					{/if}
				{/each}
			{/snippet}
		</Menubar>

		<Menubar class="flex flex-col gap-1">
			{#snippet children({ createMenu })}
				{#each thirdMenuLinks as menuLink (menuLink)}
					{#if menuLink.subItems}
						<Menu {createMenu} usePointerDownOutside>
							{#snippet triggr({ trigger })}
								<button
									class="relative w-full"
									onclick={() => {
										if (menuLink.label === 'Help') {
											openChangelogs()
										}
									}}
								>
									<MenuButton class="!text-2xs" {...menuLink} {isCollapsed} {trigger} />
									{#if menuLink.label === 'Help' && hasNewChangelogs}
										<span
											class={twMerge(
												'flex h-2 w-2 absolute',
												isCollapsed ? 'top-1 right-1' : 'right-2 top-1/2 -translate-y-1/2'
											)}
										>
											<span
												class="animate-ping absolute inline-flex h-full w-full rounded-full bg-frost-400 opacity-75"
											></span>
											<span class="relative inline-flex rounded-full h-2 w-2 bg-frost-500"></span>
										</span>
									{/if}
								</button>
							{/snippet}
							{#snippet children({ item })}
								{#each menuLink.subItems as subItem (subItem.href ?? subItem.label)}
									<MenuItem
										href={subItem.href}
										class={itemClass}
										target={subItem.external !== false ? '_blank' : undefined}
										{item}
									>
										<div class="flex flex-row items-center gap-2">
											{#if subItem.icon}
												<subItem.icon size={16} />
											{/if}

											{subItem.label}
										</div>
									</MenuItem>
								{/each}
								{#if recentChangelogs.length > 0}
									<div class="w-full h-1 border-t"></div>
									<span class="text-xs px-4 font-bold"> Latest changelogs </span>
									{#each recentChangelogs as changelog}
										<MenuItem href={changelog.href} class={itemClass} target="_blank" {item}>
											<div class="flex flex-row items-center gap-2">
												{changelog.label}
											</div>
										</MenuItem>
									{/each}
								{/if}
							{/snippet}
						</Menu>
					{/if}
				{/each}
			{/snippet}
		</Menubar>
	</div>
</nav>

<ConfirmationModal
	open={leaveWorkspaceModal}
	title="Leave workspace"
	confirmationText="Leave workspace"
	on:canceled={() => {
		leaveWorkspaceModal = false
	}}
	on:confirmed={() => {
		leaveWorkspace()
	}}
>
	<div class="flex flex-col w-full space-y-4">
		<span>Are you sure you want to leave this workspace?</span>
	</div>
</ConfirmationModal>

{#if $workspaceStore?.startsWith('wm-fork-')}
	<ConfirmationModal
		open={deleteWorkspaceForkModal}
		title="Delete forked workspace"
		confirmationText="Remove"
		on:canceled={() => {
			deleteWorkspaceForkModal = false
		}}
		on:confirmed={() => {
			deleteFork()
		}}
	>
		<div class="flex flex-col w-full space-y-4">
			<span>Are you sure you want to delete this workspace fork? (deleting {$workspaceStore})</span>
		</div>
	</ConfirmationModal>
{/if}
