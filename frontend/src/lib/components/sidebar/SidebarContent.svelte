<script lang="ts">
	import MenuLink from './MenuLink.svelte'
	import {
		superadmin,
		usedTriggerKinds,
		userStore,
		userWorkspaces,
		workspaceStore,
		isCriticalAlertsUIOpen,
		enterpriseLicense,
		devopsRole,
		tutorialsToDo,
		skippedAll
	} from '$lib/stores'
	import { findWorkspaceDescendants, workspaceIsFork } from '$lib/utils/workspaceHierarchy'
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
		Users,
		Plus,
		Unplug,
		AlertCircle,
		Database,
		Pyramid,
		Trash2,
		MailIcon,
		ChevronDown,
		ChevronRight
	} from 'lucide-svelte'
	import { useLocalStorageValue } from '$lib/svelte5Utils.svelte'
	import { slide } from 'svelte/transition'
	import UserMenu from './UserMenu.svelte'
	import DiscordIcon from '../icons/brands/Discord.svelte'
	import { WorkspaceService } from '$lib/gen'
	import { sendUserToast } from '$lib/toast'
	import { clearStores, switchWorkspace } from '$lib/storeUtils'
	import Toggle from '$lib/components/Toggle.svelte'
	import { goto } from '$lib/navigation'
	import ConfirmationModal from '../common/confirmationModal/ConfirmationModal.svelte'
	import { twMerge } from 'tailwind-merge'
	import { onMount } from 'svelte'
	import { base } from '$lib/base'
	import { page } from '$app/state'
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
	import AzureIcon from '../icons/AzureIcon.svelte'
	import {
		deleteSessionsForWorkspace,
		reconcileAfterWorkspaceChange
	} from '$lib/components/sessions/sessionState.svelte'
	import { leaveCurrentWorkspace } from './leaveWorkspace'
	import { markChangelogsOpened, readRecentChangelogs } from './changelogs'

	type ForkedDatatable = {
		name: string
		resourceType: string
		resourcePath: string
		dropOnDelete: boolean
	}
	let forkedDatatables: ForkedDatatable[] = $state([])

	async function loadForkedDatatables() {
		if (!$workspaceStore) return
		try {
			const settings = await WorkspaceService.getPublicSettings({ workspace: $workspaceStore })
			const datatables = settings.datatable?.datatables ?? {}
			forkedDatatables = Object.entries(datatables)
				.filter(([_, dt]) => dt.forked_from != null)
				.map(([name, dt]) => ({
					name,
					resourceType: dt.database.resource_type ?? 'instance',
					resourcePath: dt.database.resource_path ?? '',
					dropOnDelete: true
				}))
		} catch {
			forkedDatatables = []
		}
	}

	async function deleteFork() {
		const workspace = $workspaceStore ?? ''
		// Capture the parent before delete so we can land the user there
		// instead of dropping them back on the workspace-picker menu.
		// Only valid if the parent is still in the user's workspace list.
		const parentId = $userWorkspaces.find((w) => w.id === workspace)?.parent_workspace_id
		const parentStillAccessible = !!(parentId && $userWorkspaces.find((w) => w.id === parentId))
		const dbsToDrop = forkedDatatables.filter((dt) => dt.dropOnDelete).map((dt) => dt.name)

		if (dbsToDrop.length > 0) {
			const errors = await WorkspaceService.dropForkedDatatableDatabases({
				workspace,
				requestBody: { datatable_names: dbsToDrop }
			})
			for (const err of errors) {
				sendUserToast(err, true)
			}
		}

		// Fork-scoped ducklake namespaces (metadata schemas + data files) — driven by the
		// backend registry, so no per-lake selection is needed. Best-effort: a failure is
		// surfaced but doesn't block the workspace delete (the registry rows survive a
		// failed cleanup and re-creating the same fork id retries it — the toast tells the
		// user something was left behind).
		try {
			const dlErrors = await WorkspaceService.dropForkedDucklakeNamespaces({ workspace })
			for (const err of dlErrors) {
				sendUserToast(err, true)
			}
		} catch (err) {
			sendUserToast(`Failed to drop fork ducklake namespaces: ${err}`, true)
		}

		if (deleteForkedChildren && forkedDescendants.length > 0) {
			for (const child of forkedDescendants) {
				try {
					await WorkspaceService.dropForkedDucklakeNamespaces({ workspace: child.id }).then(
						(errs) => errs.forEach((e) => sendUserToast(e, true)),
						(err) =>
							sendUserToast(`Failed to drop fork ducklake namespaces of ${child.id}: ${err}`, true)
					)
					await WorkspaceService.deleteWorkspace({ workspace: child.id })
				} catch (err) {
					sendUserToast(`Failed to delete forked child ${child.id}: ${err}`, true)
					return
				}
				// Backend delete is authoritative; session cleanup is best-effort so a
				// local failure can't abort the remaining (parent) deletes.
				await deleteSessionsForWorkspace(child.id).catch((e) =>
					console.error(`Session cleanup for ${child.id} failed`, e)
				)
			}
		}

		await WorkspaceService.deleteWorkspace({ workspace })
		await deleteSessionsForWorkspace(workspace).catch((e) =>
			console.error('Session cleanup after workspace delete failed', e)
		)
		sendUserToast('You deleted the workspace')
		if (parentStillAccessible && parentId) {
			// Refresh the workspace list AND reconcile session lifecycle before
			// landing on the parent. The refresh keeps the sidebar's
			// `visibleSessions` filter from rendering every committed session as
			// "Fork — no longer available" (it reads `$userWorkspaces`, which
			// `clearStores()` would null). The reconcile re-roots surviving child
			// forks: deleting this fork without "delete children" re-parents them
			// via the backend's ON DELETE SET NULL, so their sessions' stored
			// `workspace_root_id` must be recomputed off the now-deleted ancestor.
			// A reconcile failure must NOT strand the user on the just-deleted
			// workspace — always fall through to the parent switch + navigation.
			try {
				await reconcileAfterWorkspaceChange()
			} catch (e) {
				console.error('Failed to reconcile sessions after workspace delete', e)
			}
			switchWorkspace(parentId)
			await goto('/')
		} else {
			clearStores()
			await goto('/user/workspaces')
		}
	}

	let deleteForkedChildren = $state(false)
	const forkedDescendants = $derived(
		$workspaceStore ? findWorkspaceDescendants($workspaceStore, $userWorkspaces ?? []) : []
	)
	// Fork/dev workspaces are detected by their parent link, not the `wm-fork-` id prefix.
	const currentWsIsFork = $derived(workspaceIsFork($workspaceStore, $userWorkspaces ?? []))

	const { recent: recentChangelogs, hasNew } = readRecentChangelogs()
	let hasNewChangelogs = $state(hasNew)
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

	const triggersCollapsed = useLocalStorageValue(
		'windmill_triggers_section_collapsed',
		false,
		'boolean'
	)

	onMount(async () => {
		// Sync tutorial progress on mount
		await syncTutorialsTodos()
	})

	function openChangelogs() {
		markChangelogsOpened()
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
		// Render the workspace-content nav (Home/Runs/Variables/… + triggers).
		showMain?: boolean
		// Render the bottom account group (User, Settings, Workers, Folders, Logs, Help).
		// Splitting these lets a host show content nav and account nav in separate rails.
		showSecondary?: boolean
		// Main-menu labels to omit here because the host renders them elsewhere
		// (e.g. the session-mode rail lifts Home/Runs up next to Favorites/Search).
		excludeMainLabels?: string[]
	}

	let {
		numUnacknowledgedCriticalAlerts = 0,
		isCollapsed = false,
		showMain = true,
		showSecondary = true,
		excludeMainLabels = []
	}: Props = $props()

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
	let mainMenuLinks = $derived(
		[
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
				aiId: 'sidebar-menu-link-assets',
				aiDescription: 'Button to navigate to assets'
			},
			{
				label: 'Folders',
				href: `${base}/folders`,
				icon: FolderOpen,
				disabled: $userStore?.operator,
				aiId: 'sidebar-menu-link-folders',
				aiDescription: 'Button to navigate to folders'
			},
			{
				label: 'Groups',
				href: `${base}/groups`,
				icon: Users,
				disabled: $userStore?.operator,
				aiId: 'sidebar-menu-link-groups',
				aiDescription: 'Button to navigate to groups'
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
		].filter((l) => !excludeMainLabels.includes(l.label))
	)
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
			label: 'Azure Event Grid' + ($enterpriseLicense ? '' : ' (EE)'),
			href: '/azure_triggers',
			icon: AzureIcon,
			disabled: $userStore?.operator || !$enterpriseLicense,
			kind: 'azure',
			aiId: 'sidebar-menu-link-azure',
			aiDescription: 'Button to navigate to Azure Event Grid triggers'
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
			(link) => $usedTriggerKinds.includes(link.kind) || page.url.pathname.includes(link.href)
		)
	])
	let extraTriggerLinks = $derived(
		allTriggerLinks.filter((link) => {
			return !page.url.pathname.includes(link.href) && !$usedTriggerKinds.includes(link.kind)
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
				...(currentWsIsFork
					? [
							{
								label: 'Delete Forked Workspace',
								action: async () => {
									await loadForkedDatatables()
									deleteForkedChildren = false
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
	<div class={twMerge('pt-4 flex flex-col grow')}>
		{#if showMain}
			<div class="space-y-1">
				{#each mainMenuLinks as menuLink (menuLink.href ?? menuLink.label)}
					<MenuLink class="!text-xs" {...menuLink} {isCollapsed} />
				{/each}
			</div>
			<div class="pt-4">
				{#if isCollapsed}
					<div class="text-secondary text-[0.5rem] uppercase transition-opacity opacity-0">
						Triggers
					</div>
				{:else}
					<button
						type="button"
						onclick={() => (triggersCollapsed.val = !triggersCollapsed.val)}
						class="text-secondary text-[0.5rem] uppercase flex flex-row items-center gap-1 rounded px-1 -mx-1 py-0.5 hover:bg-surface-hover focus:outline-none"
						aria-expanded={!triggersCollapsed.val}
					>
						Triggers
						{#if triggersCollapsed.val}
							<ChevronRight size={10} />
						{:else}
							<ChevronDown size={10} />
						{/if}
					</button>
				{/if}
				{#if isCollapsed || !triggersCollapsed.val}
					<div transition:slide={{ duration: 180 }}>
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
				{/if}
			</div>
		{/if}
		{#if showSecondary}
			<div class="flex flex-col gap-2 mt-auto pt-4">
				<!-- Single Menubar so melt-ui's hover-to-switch spans the whole bottom
			     group (Settings/Workers/Folders/Logs AND Help). With Help in its own
			     Menubar the menus stack instead of switching (WIN-1993). Each group
			     keeps its own flex container for spacing. -->
				<Menubar class="flex flex-col gap-2">
					{#snippet children({ createMenu })}
						<div class="flex flex-col gap-1">
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
												showChevron
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
																<SideBarNotification
																	notificationCount={subItem['notificationCount']}
																/>
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
						</div>

						<div class="flex flex-col gap-1">
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
												<MenuButton
													class="!text-2xs"
													{...menuLink}
													{isCollapsed}
													showChevron
													{trigger}
												/>
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
														<span class="relative inline-flex rounded-full h-2 w-2 bg-frost-500"
														></span>
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
						</div>
					{/snippet}
				</Menubar>
			</div>
		{/if}
	</div></nav
>

<ConfirmationModal
	open={leaveWorkspaceModal}
	title="Leave workspace"
	confirmationText="Leave workspace"
	on:canceled={() => {
		leaveWorkspaceModal = false
	}}
	on:confirmed={() => {
		void leaveCurrentWorkspace()
	}}
>
	<div class="flex flex-col w-full space-y-4">
		<span>Are you sure you want to leave this workspace?</span>
	</div>
</ConfirmationModal>

{#if currentWsIsFork}
	<ConfirmationModal
		open={deleteWorkspaceForkModal}
		title="Delete forked workspace"
		confirmationText="Remove"
		on:canceled={() => {
			deleteWorkspaceForkModal = false
		}}
		on:confirmed={() => {
			deleteWorkspaceForkModal = false
			deleteFork()
		}}
	>
		<div class="flex flex-col w-full space-y-4">
			<span>Are you sure you want to delete this workspace fork? (deleting {$workspaceStore})</span>
			{#if forkedDescendants.length > 0}
				<div class="border rounded-md divide-y">
					<div class="px-4 py-2 flex items-center justify-between gap-2">
						<div class="flex flex-col min-w-0">
							<span class="text-xs font-semibold text-secondary">Forked children</span>
							<span class="text-3xs text-hint">
								This fork has {forkedDescendants.length} forked
								{forkedDescendants.length === 1 ? 'child' : 'children'} (transitively).
							</span>
						</div>
						<Toggle
							class="shrink-0"
							size="xs"
							bind:checked={deleteForkedChildren}
							options={{ right: 'Also delete children' }}
						/>
					</div>
					<ul class="px-4 py-2 text-3xs text-hint max-h-32 overflow-y-auto">
						{#each forkedDescendants as child}
							<li class="font-mono truncate" title={child.id}>{child.id}</li>
						{/each}
					</ul>
				</div>
			{/if}
			{#if forkedDatatables.length > 0}
				<div class="border rounded-md divide-y">
					<div class="px-4 py-2 text-xs font-semibold text-secondary"> Forked databases </div>
					{#each forkedDatatables as dt}
						<div class="flex items-center justify-between px-4 py-2">
							<div class="flex flex-col">
								<span class="text-xs font-medium text-secondary">{dt.name}</span>
								<span class="text-3xs text-hint">
									{dt.resourceType === 'instance'
										? dt.resourcePath
										: `${$workspaceStore?.replace(/-/g, '_')}__${dt.name}`}
								</span>
							</div>
							<Toggle
								class="shrink-0"
								size="xs"
								bind:checked={dt.dropOnDelete}
								options={{ right: 'Drop database' }}
							/>
						</div>
					{/each}
				</div>
			{/if}
		</div>
	</ConfirmationModal>
{/if}
