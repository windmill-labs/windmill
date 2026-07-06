<script lang="ts">
	import SettingsMenu from '$lib/components/sidebar/SettingsMenu.svelte'
	import DarkModeToggle from '$lib/components/sidebar/DarkModeToggle.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { SIDEBAR_BG, SIDEBAR_BG_DARK } from '$lib/components/sidebar/sidebarChrome'
	import { markChangelogsOpened } from '$lib/components/sidebar/changelogs'
	import {
		userStore,
		usersWorkspaceStore,
		workspaceStore,
		superadmin,
		devopsRole,
		enterpriseLicense,
		isPremiumStore,
		usageStore,
		type UserExt
	} from '$lib/stores'

	// The SettingsMenu variants are driven by GLOBAL stores, so only one variant
	// can render at a time — this page is a switcher, not a gallery. It mutates
	// app-wide stores: use it standalone, then reload before returning to the app.

	let isAdmin = $state(true)
	let isOperator = $state(false)
	let isSuperadmin = $state(true)
	let nonMember = $state(false)
	let hasDevopsRole = $state(true)
	let hasEnterpriseLicense = $state(true)
	let cloudHosted = $state(false)
	let isPremium = $state(false)
	let usage = $state(123)
	let criticalAlerts = $state(9)
	let sessionMode = $state(false)
	let collapsed = $state(false)

	const PRESETS: Record<string, () => void> = {
		'Self-host EE superadmin (dev default)': () => {
			isAdmin = true
			isOperator = false
			isSuperadmin = true
			nonMember = false
			hasDevopsRole = true
			hasEnterpriseLicense = true
			cloudHosted = false
			isPremium = false
			sessionMode = false
		},
		'Self-host CE admin': () => {
			isAdmin = true
			isOperator = false
			isSuperadmin = false
			nonMember = false
			hasDevopsRole = false
			hasEnterpriseLicense = false
			cloudHosted = false
			isPremium = false
			sessionMode = false
		},
		'Non-admin member (sees Leave workspace)': () => {
			isAdmin = false
			isOperator = false
			isSuperadmin = false
			nonMember = false
			hasDevopsRole = false
			hasEnterpriseLicense = true
			cloudHosted = false
			isPremium = false
			sessionMode = false
		},
		'Superadmin, not a member': () => {
			isAdmin = true
			isOperator = false
			isSuperadmin = true
			nonMember = true
			hasDevopsRole = false
			hasEnterpriseLicense = true
			cloudHosted = false
			isPremium = false
			sessionMode = false
		},
		'Cloud free non-admin (user execs quota)': () => {
			isAdmin = false
			isOperator = false
			isSuperadmin = false
			nonMember = false
			hasDevopsRole = false
			hasEnterpriseLicense = true
			cloudHosted = true
			isPremium = false
			sessionMode = false
		},
		'Cloud premium admin': () => {
			isAdmin = true
			isOperator = false
			isSuperadmin = false
			nonMember = false
			hasDevopsRole = false
			hasEnterpriseLicense = true
			cloudHosted = true
			isPremium = true
			sessionMode = false
		},
		'Session mode (no workspace settings/leave)': () => {
			sessionMode = true
		}
	}

	// remountKey forces a full SettingsMenu re-init: the new-changelogs state is
	// read from localStorage once at component init.
	let remountKey = $state(0)
	function simulateUnseenChangelogs() {
		localStorage.setItem('changelogsLastOpened', '2024-01-01')
		remountKey++
	}
	function markChangelogsSeen() {
		markChangelogsOpened()
		remountKey++
	}

	// Push the toggles into the global stores the menu reads.
	$effect(() => {
		$userStore = {
			email: 'jane.doe@windmill.dev',
			username: 'jane',
			is_admin: isAdmin,
			is_super_admin: isSuperadmin,
			operator: isOperator,
			non_member: nonMember,
			created_at: '',
			groups: [],
			pgroups: [],
			folders: [],
			folders_read: [],
			folders_owners: []
		} satisfies UserExt
		$superadmin = isSuperadmin ? 'jane.doe@windmill.dev' : false
		$devopsRole = hasDevopsRole ? 'jane.doe@windmill.dev' : false
		$enterpriseLicense = hasEnterpriseLicense ? 'dev-license' : undefined
		$isPremiumStore = isPremium
		$usageStore = Number(usage)
		$workspaceStore = 'kitchen-sink'
		$usersWorkspaceStore = {
			email: 'jane.doe@windmill.dev',
			workspaces: [
				{
					id: 'kitchen-sink',
					name: 'Kitchen Sink',
					username: 'jane',
					color: '',
					is_dev_workspace: false,
					disabled: false
				}
			]
		}
	})

	let darkMode = $state(false)
</script>

<svelte:head>
	<title>SettingsMenu kitchen sink</title>
</svelte:head>

<div class="min-h-screen p-6 flex flex-col gap-4">
	<div class="flex items-center gap-4">
		<h1 class="text-lg font-semibold text-emphasis">Sidebar SettingsMenu variants</h1>
		<DarkModeToggle bind:darkMode forcedDarkMode={false} />
	</div>
	<p class="text-xs text-secondary max-w-2xl">
		The menu reads global stores, so variants render one at a time. This page mutates app-wide
		stores — reload before navigating back into the real app.
	</p>

	<div class="flex flex-wrap gap-1.5">
		{#each Object.entries(PRESETS) as [label, apply] (label)}
			<Button variant="default" unifiedSize="xs" on:click={apply}>{label}</Button>
		{/each}
	</div>

	<div class="flex flex-row gap-8 items-start">
		<div class="flex flex-col gap-2 w-72 shrink-0 border rounded-md p-4">
			<div class="text-xs font-semibold text-emphasis pb-1">Toggles</div>
			<Toggle bind:checked={isAdmin} options={{ right: 'workspace admin' }} size="xs" />
			<Toggle bind:checked={isOperator} options={{ right: 'operator' }} size="xs" />
			<Toggle bind:checked={isSuperadmin} options={{ right: 'superadmin' }} size="xs" />
			<Toggle
				bind:checked={nonMember}
				options={{ right: 'non-member (superadmin visiting)' }}
				size="xs"
			/>
			<Toggle
				bind:checked={hasDevopsRole}
				options={{ right: 'devops role (service logs)' }}
				size="xs"
			/>
			<Toggle
				bind:checked={hasEnterpriseLicense}
				options={{ right: 'enterprise license (critical alerts)' }}
				size="xs"
			/>
			<Toggle bind:checked={cloudHosted} options={{ right: 'cloud hosted' }} size="xs" />
			<Toggle bind:checked={isPremium} options={{ right: 'premium (cloud)' }} size="xs" />
			<Toggle
				bind:checked={sessionMode}
				options={{ right: 'session mode (hide workspace settings)' }}
				size="xs"
			/>
			<Toggle bind:checked={collapsed} options={{ right: 'collapsed rail' }} size="xs" />
			<label class="text-xs text-secondary flex items-center gap-2 pt-1">
				critical alerts
				<TextInput
					bind:value={criticalAlerts}
					size="xs"
					class="w-20"
					inputProps={{ type: 'number', min: 0 }}
				/>
			</label>
			<label class="text-xs text-secondary flex items-center gap-2">
				user execs (cloud)
				<TextInput
					bind:value={usage}
					size="xs"
					class="w-20"
					inputProps={{ type: 'number', min: 0 }}
				/>
			</label>
			<div class="flex gap-1.5 pt-2">
				<Button variant="default" unifiedSize="xs" on:click={simulateUnseenChangelogs}>
					Simulate new changelogs
				</Button>
				<Button variant="default" unifiedSize="xs" on:click={markChangelogsSeen}>Mark seen</Button>
			</div>
		</div>

		<!-- Sidebar-shaped shell: menus anchor to the bottom and open upward, like
		     the real rail. min-h keeps room for the upward dropdowns. -->
		<div class="flex flex-row gap-6">
			<div
				class="flex flex-col justify-end rounded-md border min-h-[26rem] {collapsed
					? 'w-12'
					: 'w-52'}"
				style:background-color={darkMode ? SIDEBAR_BG_DARK : SIDEBAR_BG}
			>
				{#key remountKey}
					<div class="px-2 pb-2">
						<SettingsMenu
							isCollapsed={collapsed}
							hideWorkspaceSettings={sessionMode}
							numUnacknowledgedCriticalAlerts={Number(criticalAlerts)}
							{cloudHosted}
						/>
					</div>
				{/key}
			</div>
		</div>
	</div>
</div>
