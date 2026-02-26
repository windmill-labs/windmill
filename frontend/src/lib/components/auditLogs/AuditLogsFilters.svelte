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
		AppService,
		CancelError
	} from '$lib/gen'

	import { userStore, workspaceStore } from '$lib/stores'
	import { ChevronDown, Download, Loader2, RefreshCcw } from 'lucide-svelte'
	import { onDestroy, untrack } from 'svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import Select from '../select/Select.svelte'
	import { usePromise } from '$lib/svelte5Utils.svelte'
	import { safeSelectItems } from '../select/utils.svelte'
	import { CancelablePromiseUtils } from '$lib/cancelable-promise-utils'
	import { sendUserToast } from '$lib/toast'

	let usernames: string[] | undefined = $state()
	let resources = usePromise(() => loadResources($workspaceStore!), { loadInit: false })

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
		loading?: boolean
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
		scope = $bindable(undefined),
		loading = $bindable(false)
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

	function loadLogs() {
		loading = true

		let username_ = username == 'all' ? undefined : username
		let operation_ = operation == 'all' || operation == '' ? undefined : operation
		let actionKind_ = actionKind == 'all' ? undefined : actionKind
		let resource_ = resource == 'all' || resource == '' ? undefined : resource

		let _promise = AuditService.listAuditLogs({
			workspace: scope === 'instance' ? 'global' : $workspaceStore!,
			page: pageIndex,
			perPage,
			before,
			after,
			username: username_,
			operation: operation_,
			resource: resource_,
			actionKind: actionKind_,
			allWorkspaces: scope === 'all_workspaces'
		})
		let promise = CancelablePromiseUtils.map(_promise, (value) => {
			logs = value
			hasMore = !logs || (logs.length > 0 && logs.length === perPage)
			loading = false
		})
		promise = CancelablePromiseUtils.onTimeout(promise, 4000, () => {
			sendUserToast(
				'Loading audit logs is taking longer than expected...',
				'warning',
				perPage > 25
					? [{ label: 'Reduce to 25 items per page', callback: () => (perPage = 25) }]
					: []
			)
		})
		promise = CancelablePromiseUtils.catchErr(promise, (e) => {
			if (e instanceof CancelError) return CancelablePromiseUtils.pure<void>(undefined)
			return CancelablePromiseUtils.err<void>(e)
		})
		return promise
	}

	async function loadUsers() {
		usernames =
			$userStore?.is_admin || $userStore?.is_super_admin
				? await UserService.listUsernames({ workspace: $workspaceStore! })
				: [$userStore?.username ?? '']
	}

	function updateQueryParams() {
		const queryParams: string[] = []

		function addQueryParam(key: string, value: string | number | undefined | null) {
			if (value !== undefined && value !== null && value !== '' && value !== 'all') {
				queryParams.push(`${key}=${encodeURIComponent(value)}`)
			}
		}

		addQueryParam('username', username)
		addQueryParam('page', pageIndex)
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
		goto(query, { replaceState: true, keepFocus: true })
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
		USERS_SCIM_CREATE: 'users.scim_create',
		USERS_SCIM_DELETE: 'users.scim_delete',
		USERS_SCIM_UPDATE: 'users.scim_update',
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
		INSTANCE_GROUPS_SCIM_CREATE: 'instance_groups.scim_create',
		INSTANCE_GROUPS_SCIM_DELETE: 'instance_groups.scim_delete',
		INSTANCE_GROUPS_SCIM_UPDATE: 'instance_groups.scim_update',
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

	let refresh = $state(0)
	let lastRefresh = $state(-1)

	function downloadAuditLogsAsJson() {
		if (!logs || logs.length === 0) {
			sendUserToast('No audit logs to download', true)
			return
		}

		const jsonContent = JSON.stringify(logs, null, 2)
		const blob = new Blob([jsonContent], { type: 'application/json' })
		const url = URL.createObjectURL(blob)

		const timestamp = new Date().toISOString().replace(/[:.]/g, '-')
		const filename = `audit_logs_${$workspaceStore}_${timestamp}.json`

		const link = document.createElement('a')
		link.href = url
		link.download = filename
		document.body.appendChild(link)
		link.click()
		document.body.removeChild(link)
		URL.revokeObjectURL(url)
	}

	// observe all the variables that should trigger an update
	$effect(() => {
		;[refresh, username, perPage, before, after, operation, resource, actionKind, scope, pageIndex]
		return untrack(() => {
			if (refresh !== lastRefresh) {
				loadUsers()
				resources.refresh()
				lastRefresh = refresh
			}
			updateQueryParams()
			let promise = loadLogs()
			return () => promise?.cancel()
		})
	})
</script>

<div class="flex flex-col items-center gap-2 2xl:flex-row mt-4 xl:mt-0 pr-2">
	{#if $workspaceStore == 'admins'}
		<div class="flex gap-1 relative">
			<span class="text-xs absolute font-semibold text-emphasis -top-4">Scope</span>
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
	<div class="flex relative">
		<span class="text-xs absolute font-semibold text-emphasis -top-4">From</span>
		<input type="text" value={after ?? 'From'} disabled />
		<CalendarPicker
			clearable
			date={after}
			placement="bottom-end"
			label="From"
			on:change={({ detail }) => {
				after = new Date(detail).toISOString()
			}}
			on:clear={() => {
				after = undefined
			}}
		/>
	</div>
	<div class="flex relative">
		<span class="text-xs absolute font-semibold text-emphasis -top-4">To</span>
		<input type="text" value={before ?? 'To'} disabled />
		<CalendarPicker
			clearable
			bind:date={before}
			label="To"
			placement="bottom-end"
			on:change={({ detail }) => {
				before = new Date(detail).toISOString()
			}}
			on:clear={() => {
				before = undefined
			}}
		/>
	</div>

	<div class="flex relative">
		<span class="text-xs absolute font-semibold text-emphasis -top-4">Username</span>
		<Select
			bind:value={username}
			RightIcon={ChevronDown}
			items={usernames
				? [
						...($userStore?.is_admin || $userStore?.is_super_admin
							? [{ value: 'all', label: 'all' }]
							: []),
						...usernames.map((e) => ({
							value: e,
							label: e,
							disabled: e !== username && !$userStore?.is_admin && !$userStore?.is_super_admin
						}))
					]
				: []}
		/>
	</div>
	<div class="flex relative">
		<span class="text-xs absolute font-semibold text-emphasis -top-4">Resource</span>

		<Select
			onCreateItem={(r) => (resources.value?.push(r), (resource = r))}
			createText="Press enter to use this value"
			bind:value={resource}
			items={safeSelectItems(['all', ...(resources.value ?? [])])}
			inputClass="dark:!bg-gray-700"
			RightIcon={ChevronDown}
		/>
	</div>

	<div class="flex relative">
		<span class="text-xs absolute font-semibold text-emphasis -top-4">Operation</span>

		<Select
			bind:value={operation}
			items={['all', ...Object.values(operations)].map((r) => ({ value: r, label: r }))}
			inputClass="dark:!bg-gray-700"
			RightIcon={ChevronDown}
		/>
	</div>

	<div class="flex relative">
		<span class="text-xs absolute font-semibold text-emphasis -top-4">Action</span>

		<Select
			bind:value={actionKind}
			RightIcon={ChevronDown}
			items={[
				{ value: 'all', label: 'all' },
				{ value: 'create', label: 'Create' },
				{ value: 'update', label: 'Update' },
				{ value: 'delete', label: 'Delete' },
				{ value: 'execute', label: 'Execute' }
			]}
		/>
	</div>

	<div class="flex flex-row">
		<Button
			variant="subtle"
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
			unifiedSize="md"
		>
			Clear filters
		</Button>
		<Button
			variant="subtle"
			on:click={downloadAuditLogsAsJson}
			unifiedSize="md"
			title="Downloads currently displayed logs only (up to {perPage} entries)"
		>
			<div class="flex flex-row gap-1 items-center">
				<Download size={14} />
				Download JSON
			</div>
		</Button>
		<Button
			variant="accent"
			on:click={() => {
				refresh++
			}}
			unifiedSize="md"
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
