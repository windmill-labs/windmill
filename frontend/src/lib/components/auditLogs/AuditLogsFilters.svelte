<script lang="ts" module>
	async function loadResources(workspace: string): Promise<string[]> {
		const r = await ResourceService.listResource({ workspace })
		const sPaths = await ScriptService.listScriptPaths({ workspace })
		const fPaths = await FlowService.listFlowPaths({ workspace })
		const a = await AppService.listApps({ workspace })
		return r
			.map((r) => r.path)
			.concat(sPaths)
			.concat(fPaths)
			.concat(a.map((a) => a.path))
			.sort()
	}
</script>

<script lang="ts">
	import { goto } from '$lib/navigation'
	import type { ActionKind } from '$lib/common'

	import Button from '$lib/components/common/button/Button.svelte'
	import CalendarPicker from '$lib/components/common/calendarPicker/CalendarPicker.svelte'
	import {
		type AuditLog,
		AuditService,
		ResourceService,
		UserService,
		ScriptService,
		FlowService,
		AppService
	} from '$lib/gen'

	import { userStore, workspaceStore } from '$lib/stores'
	import { ChevronDown, Loader2, RefreshCcw } from 'lucide-svelte'
	import { onDestroy, tick, untrack } from 'svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import Select from '../Select.svelte'
	import { usePromise } from '$lib/svelte5Utils.svelte'

	let usernames: string[] | undefined = $state()
	let resources = usePromise(() => loadResources($workspaceStore!), { loadInit: false })
	let loading: boolean = $state(false)
	let page: number | undefined = undefined

	interface Props {
		logs?: AuditLog[]
		username?: string
		pageIndex?: number | undefined
		hasMore?: boolean
		before?: string | undefined
		after?: string | undefined
		perPage?: number | undefined
		operation?: string
		resource?: string | undefined
		actionKind?: ActionKind | 'all'
		scope?: undefined | 'all_workspaces' | 'instance'
	}

	let {
		logs = $bindable(undefined),
		username = $bindable('all'),
		pageIndex = $bindable(1),
		hasMore = $bindable(false),
		before = $bindable(undefined),
		after = $bindable(undefined),
		perPage = $bindable(100),
		operation = $bindable(),
		resource = $bindable() as string | undefined,
		actionKind = $bindable(undefined),
		scope = $bindable(undefined)
	}: Props = $props()

	$effect.pre(() => {
		if (logs == undefined) {
			logs = []
		}
		if (operation == undefined) {
			operation = 'all'
		}
		if (resource == undefined) {
			resource = 'all'
		}
		if (actionKind == undefined) {
			actionKind = 'all'
		}
	})

	async function loadLogs(
		username: string | undefined,
		page: number | undefined,
		perPage: number | undefined,
		before: string | undefined,
		after: string | undefined,
		operation: string | undefined,
		resource: string | undefined,
		actionKind: ActionKind | undefined | 'all',
		scope: undefined | 'all_workspaces' | 'instance'
	): Promise<void> {
		loading = true

		if (username == 'all') {
			username = undefined
		}
		if (operation == 'all' || operation == '') {
			operation = undefined
		}

		// @ts-ignore
		if (actionKind == 'all' || actionKind == '') {
			actionKind = undefined
		}

		if (resource == 'all' || resource == '') {
			resource = undefined
		}

		logs = await AuditService.listAuditLogs({
			workspace: scope === 'instance' ? 'global' : $workspaceStore!,
			page,
			perPage,
			before,
			after,
			username,
			operation,
			resource,
			actionKind,
			allWorkspaces: scope === 'all_workspaces'
		})
		hasMore = logs.length > 0 && logs.length === perPage

		loading = false
	}

	async function loadUsers() {
		usernames =
			$userStore?.is_admin || $userStore?.is_super_admin
				? await UserService.listUsernames({ workspace: $workspaceStore! })
				: [$userStore?.username ?? '']
	}

	let initialLoad = true
	function refreshLogs() {
		loadUsers()
		resources.refresh()
		loadLogs(username, page, perPage, before, after, operation, resource, actionKind, scope)
		tick().then(() => {
			initialLoad = false
		})
	}

	function updateLogs() {
		const queryParams: string[] = []

		function addQueryParam(key: string, value: string | number | undefined | null) {
			if (value !== undefined && value !== null && value !== '' && value !== 'all') {
				queryParams.push(`${key}=${encodeURIComponent(value)}`)
			}
		}

		addQueryParam('username', username)
		addQueryParam('page', page)
		addQueryParam('perPage', perPage)
		addQueryParam('before', before)
		addQueryParam('after', after)
		addQueryParam('operation', operation)
		addQueryParam('resource', resource)
		addQueryParam('actionKind', actionKind)
		if (scope && $workspaceStore == 'admins') {
			addQueryParam('scope', scope)
			addQueryParam('workspace', 'admins')
		}
		const query = '?' + queryParams.join('&')
		goto(query)

		loadLogs(username, page, perPage, before, after, operation, resource, actionKind, scope)
	}

	function updateQueryParams() {
		if (initialLoad) {
			return
		}
		page = 1
		pageIndex = 1
		updateLogs()
	}

	function updatePageQueryParams(pageIndex?: number | undefined) {
		if (initialLoad) {
			return
		}
		page = pageIndex
		updateLogs()
	}

	window.addEventListener('popstate', handlePopState)

	function handlePopState() {
		const urlSearchParams = new URLSearchParams(window.location.search)
		username = urlSearchParams.get('username') ?? 'all'
		pageIndex = Number(urlSearchParams.get('page')) || 1
		before = urlSearchParams.get('before') ?? undefined
		after = urlSearchParams.get('after') ?? undefined
		perPage = Number(urlSearchParams.get('perPage')) || 100
		operation = urlSearchParams.get('operation') ?? 'all'
		resource = urlSearchParams.get('resource') ?? undefined

		actionKind = (urlSearchParams.get('actionKind') as ActionKind) ?? 'all'
	}

	onDestroy(() => {
		window.removeEventListener('popstate', handlePopState)
	})

	const operations = {
		JOBS_RUN: 'jobs.run',
		JOBS_RUN_SCRIPT: 'jobs.run.script',
		JOBS_RUN_PREVIEW: 'jobs.run.preview',
		JOBS_RUN_FLOW: 'jobs.run.flow',
		JOBS_RUN_FLOW_PREVIEW: 'jobs.run.flow_preview',
		JOBS_RUN_SCRIPT_HUB: 'jobs.run.script_hub',
		JOBS_RUN_DEPENDENCIES: 'jobs.run.dependencies',
		JOBS_RUN_IDENTITY: 'jobs.run.identity',
		JOBS_RUN_NOOP: 'jobs.run.noop',
		JOBS_FLOW_DEPENDENCIES: 'jobs.flow_dependencies',
		JOBS_APP_DEPENDENCIES: 'jobs.app_dependencies',
		JOBS: 'jobs',
		JOBS_CANCEL: 'jobs.cancel',
		JOBS_FORCE_CANCEL: 'jobs.force_cancel',
		JOBS_DISAPPROVAL: 'jobs.disapproval',
		JOBS_DELETE: 'jobs.delete',
		ACCOUNT_DELETE: 'account.delete',
		AI_REQUEST: 'ai.request',
		RESOURCES_CREATE: 'resources.create',
		RESOURCES_UPDATE: 'resources.update',
		RESOURCES_DELETE: 'resources.delete',
		RESOURCE_TYPES_CREATE: 'resource_types.create',
		RESOURCE_TYPES_UPDATE: 'resource_types.update',
		RESOURCE_TYPES_DELETE: 'resource_types.delete',
		SCHEDULE_CREATE: 'schedule.create',
		SCHEDULE_SETENABLED: 'schedule.setenabled',
		SCHEDULE_EDIT: 'schedule.edit',
		SCHEDULE_DELETE: 'schedule.delete',
		SCRIPTS_CREATE: 'scripts.create',
		SCRIPTS_UPDATE: 'scripts.update',
		SCRIPTS_ARCHIVE: 'scripts.archive',
		SCRIPTS_DELETE: 'scripts.delete',
		USERS_CREATE: 'users.create',
		USERS_DELETE: 'users.delete',
		USERS_SETPASSWORD: 'users.setpassword',
		USERS_UPDATE: 'users.update',
		USERS_LOGIN: 'users.login',
		USERS_LOGIN_FAILURE: 'users.login_failure',
		USERS_LOGOUT: 'users.logout',
		USERS_ACCEPT_INVITE: 'users.accept_invite',
		USERS_DECLINE_INVITE: 'users.decline_invite',
		USERS_TOKEN_CREATE: 'users.token.create',
		USERS_TOKEN_DELETE: 'users.token.delete',
		USERS_ADD_TO_WORKSPACE: 'users.add_to_workspace',
		USERS_ADD_GLOBAL: 'users.add_global',
		USERS_IMPERSONATE: 'users.impersonate',
		USERS_LEAVE_WORKSPACE: 'users.leave_workspace',
		OAUTH_LOGIN: 'oauth.login',
		OAUTH_LOGIN_FAILURE: 'oauth.login_failure',
		OAUTH_SIGNUP: 'oauth.signup',
		VARIABLES_CREATE: 'variables.create',
		VARIABLES_DELETE: 'variables.delete',
		VARIABLES_UPDATE: 'variables.update',
		FLOWS_CREATE: 'flows.create',
		FLOWS_UPDATE: 'flows.update',
		FLOWS_DELETE: 'flows.delete',
		FLOWS_ARCHIVE: 'flows.archive',
		APPS_CREATE: 'apps.create',
		APPS_UPDATE: 'apps.update',
		APPS_DELETE: 'apps.delete',
		FOLDER_CREATE: 'folder.create',
		FOLDER_UPDATE: 'folder.update',
		FOLDER_DELETE: 'folder.delete',
		FOLDER_ADD_OWNER: 'folder.add_owner',
		FOLDER_REMOVE_OWNER: 'folder.remove_owner',
		GROUP_CREATE: 'group.create',
		GROUP_DELETE: 'group.delete',
		GROUP_EDIT: 'group.edit',
		GROUP_ADDUSER: 'group.adduser',
		GROUP_REMOVEUSER: 'group.removeuser',
		IGROUP_CREATE: 'igroup.create',
		IGROUP_DELETE: 'igroup.delete',
		IGROUP_ADDUSER: 'igroup.adduser',
		IGROUP_REMOVEUSER: 'igroup.removeuser',
		VARIABLES_DECRYPT_SECRET: 'variables.decrypt_secret',
		WORKSPACES_EDIT_COMMAND_SCRIPT: 'workspaces.edit_command_script',
		WORKSPACES_EDIT_DEPLOY_TO: 'workspaces.edit_deploy_to',
		WORKSPACES_EDIT_AUTO_INVITE_DOMAIN: 'workspaces.edit_auto_invite_domain',
		WORKSPACES_EDIT_WEBHOOK: 'workspaces.edit_webhook',
		WORKSPACES_EDIT_COPILOT_CONFIG: 'workspaces.edit_copilot_config',
		WORKSPACES_EDIT_ERROR_HANDLER: 'workspaces.edit_error_handler',
		WORKSPACES_CREATE: 'workspaces.create',
		WORKSPACES_UPDATE: 'workspaces.update',
		WORKSPACES_ARCHIVE: 'workspaces.archive',
		WORKSPACES_UNARCHIVE: 'workspaces.unarchive',
		WORKSPACES_DELETE: 'workspaces.delete'
	}

	let refresh = $state(1)
	$effect(() => {
		$workspaceStore && refresh && untrack(() => refreshLogs())
	})
	// observe all the variables that should trigger an update
	$effect(() => {
		;[username, perPage, before, after, operation, resource, actionKind, scope]
		updateQueryParams()
	})
	// observe the pageIndex variable that should trigger an update
	$effect(() => {
		updatePageQueryParams(pageIndex)
	})
</script>

<div class="flex flex-col items-center gap-6 2xl:gap-1 2xl:flex-row mt-4 xl:mt-0">
	{#if $workspaceStore == 'admins'}
		<div class="flex gap-1 relative w-full">
			<span class="text-xs absolute -top-4">Scope</span>
			<ToggleButtonGroup
				selected={scope ?? 'admins'}
				on:selected={({ detail }) => {
					scope = detail === 'admins' ? undefined : detail
				}}
			>
				{#snippet children({ item })}
					<ToggleButton
						value={'admins'}
						label="Admins"
						tooltip="Displays events from the admins workspace only."
						{item}
					/>
					<ToggleButton
						value="all_workspaces"
						label="All"
						tooltip="Displays events from all workspaces."
						{item}
					/>
					<ToggleButton
						value="instance"
						label="Instance"
						tooltip="Displays instance-scope events, such as user logins and registrations, instance user and group management, and worker configuration changes."
						{item}
					/>
				{/snippet}
			</ToggleButtonGroup>
		</div>
	{/if}
	<div class="flex gap-1 relative w-full">
		<span class="text-xs absolute -top-4">After</span>
		<input type="text" value={after ?? 'After'} disabled />
		<CalendarPicker
			date={after}
			placement="bottom-end"
			label="After"
			on:change={async ({ detail }) => {
				after = new Date(detail).toISOString()
			}}
		/>
	</div>
	<div class="flex gap-1 relative w-full">
		<span class="text-xs absolute -top-4">Before</span>
		<input type="text" value={before ?? 'Before'} disabled />
		<CalendarPicker
			bind:date={before}
			label="Before"
			placement="bottom-end"
			on:change={async ({ detail }) => {
				before = new Date(detail).toISOString()
			}}
		/>
	</div>

	<div class="flex gap-1 relative w-full">
		<span class="text-xs absolute -top-4">Username</span>
		<select bind:value={username}>
			{#if usernames}
				{#if $userStore?.is_admin || $userStore?.is_super_admin}
					<option selected>all</option>
				{/if}
				{#each usernames as e}
					{#if e == username || $userStore?.is_admin || $userStore?.is_super_admin}
						<option>{e}</option>
					{:else}
						<option disabled>{e}</option>
					{/if}
				{/each}
			{/if}
		</select>
	</div>
	<div class="flex gap-1 relative w-full">
		<span class="text-xs absolute -top-4">Resource</span>

		<Select
			onCreateItem={(r) => (resources.value?.push(r), (resource = r))}
			createText="Press enter to use this value"
			bind:value={resource}
			items={['all', ...(resources.value ?? [])].map((r) => ({ value: r, label: r }))}
			inputClass="dark:!bg-gray-700"
			RightIcon={ChevronDown}
		/>
	</div>

	<div class="flex gap-1 relative w-full">
		<span class="text-xs absolute -top-4">Operation</span>

		<Select
			bind:value={operation}
			items={['all', ...Object.keys(operations)].map((r) => ({ value: r, label: r }))}
			inputClass="dark:!bg-gray-700"
			RightIcon={ChevronDown}
		/>
	</div>

	<div class="flex gap-1 relative w-full">
		<span class="text-xs absolute -top-4">Action</span>

		<select class="!truncate" bind:value={actionKind}>
			<option selected value="all">all</option>
			{#each ['Create', 'Update', 'Delete', 'Execute'] as e}
				<option value={e.toLocaleLowerCase()}>{e}</option>
			{/each}
		</select>
	</div>

	<div class="flex flex-row gap-1">
		<Button
			variant="contained"
			color="light"
			on:click={() => {
				after = undefined
				before = undefined
				username = 'all'
				operation = 'all'
				actionKind = 'all'
				pageIndex = 1
				perPage = 100
				resource = 'all'
				scope = undefined
			}}
			size="xs"
		>
			Clear filters
		</Button>
		<Button
			variant="contained"
			color="dark"
			on:click={() => {
				refresh++
			}}
			size="xs"
		>
			<div class="flex flex-row gap-1 items-center">
				{#if loading}
					<Loader2 size={14} class="animate-spin" />
				{:else}
					<RefreshCcw size={14} />
				{/if}
				Refresh
			</div>
		</Button>
	</div>
</div>
