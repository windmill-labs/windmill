<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import type { ActionKind } from '$lib/common'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { Alert } from '$lib/components/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import CalendarPicker from '$lib/components/common/calendarPicker/CalendarPicker.svelte'
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Row from '$lib/components/table/Row.svelte'
	import { AuditLog, AuditService, UserService } from '$lib/gen'
	import { enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import { displayDate } from '$lib/utils'
	import { ArrowRight, Loader2, RefreshCcw } from 'lucide-svelte'
	import { onDestroy } from 'svelte'
	import { Splitpanes, Pane } from 'svelte-splitpanes'

	let logs: AuditLog[]
	let usernames: string[]
	let loading: boolean = false

	let username: string = $page.url.searchParams.get('username') ?? 'all'
	let pageIndex: number | undefined = Number($page.url.searchParams.get('page')) || 0
	let before: string | undefined = $page.url.searchParams.get('before') ?? undefined
	let after: string | undefined = $page.url.searchParams.get('after') ?? undefined
	let perPage: number | undefined = Number($page.url.searchParams.get('perPage')) || 100
	let operation: string = $page.url.searchParams.get('operation') ?? 'all'
	let resource: string | undefined = $page.url.searchParams.get('resource') ?? undefined
	let actionKind: ActionKind | 'all' =
		($page.url.searchParams.get('actionKind') as ActionKind) ?? 'all'

	async function loadLogs(
		username: string | undefined,
		page: number | undefined,
		perPage: number | undefined,
		before: string | undefined,
		after: string | undefined,
		operation: string | undefined,
		resource: string | undefined,
		actionKind: ActionKind | undefined | 'all'
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

		logs = await AuditService.listAuditLogs({
			workspace: $workspaceStore!,
			page,
			perPage,
			before,
			after,
			username,
			operation,
			resource,
			actionKind
		})

		loading = false
	}

	async function loadUsers() {
		usernames =
			$userStore?.is_admin || $userStore?.is_super_admin
				? await UserService.listUsernames({ workspace: $workspaceStore! })
				: [$userStore?.username ?? '']
	}

	function updateQueryParams({
		username,
		pageIndex,
		perPage,
		before,
		after,
		operation,
		resource,
		actionKind
	}: {
		username?: string | undefined
		pageIndex?: number | undefined
		perPage?: number | undefined
		before?: string | undefined
		after?: string | undefined
		operation?: string | undefined
		resource?: string | undefined
		actionKind?: ActionKind | undefined | 'all'
	}) {
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

		const query = '?' + queryParams.join('&')
		goto(query)
	}

	$: updateQueryParams({
		username,
		pageIndex,
		perPage,
		before,
		after,
		operation,
		resource,
		actionKind
	})

	function kindToBadgeColor(kind: string) {
		if (kind == 'Execute') {
			return 'blue'
		} else if (kind == 'Delete') {
			return 'red'
		} else if (kind == 'Update') {
			return 'yellow'
		} else if (kind == 'Create') {
			return 'green'
		}
		return 'gray'
	}

	$: {
		if ($workspaceStore && refresh) {
			loadUsers()
			loadLogs(username, pageIndex, perPage, before, after, operation, resource, actionKind)
		}
	}

	let selectedId: number | undefined = undefined

	window.addEventListener('popstate', handlePopState)

	function handlePopState() {
		const urlSearchParams = new URLSearchParams(window.location.search)
		username = urlSearchParams.get('username') ?? 'all'
		pageIndex = Number(urlSearchParams.get('page')) || 0
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
		JOBS: 'jobs',
		JOBS_CANCEL: 'jobs.cancel',
		JOBS_FORCE_CANCEL: 'jobs.force_cancel',
		JOBS_DISAPPROVAL: 'jobs.disapproval',
		JOBS_DELETE: 'jobs.delete',
		ACCOUNT_DELETE: 'account.delete',
		OPENAI_REQUEST: 'openai.request',
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
		WORKSPACES_EDIT_OPENAI_RESOURCE_PATH: 'workspaces.edit_openai_resource_path',
		WORKSPACES_EDIT_ERROR_HANDLER: 'workspaces.edit_error_handler',
		WORKSPACES_CREATE: 'workspaces.create',
		WORKSPACES_UPDATE: 'workspaces.update',
		WORKSPACES_ARCHIVE: 'workspaces.archive',
		WORKSPACES_UNARCHIVE: 'workspaces.unarchive',
		WORKSPACES_DELETE: 'workspaces.delete'
	}
	let refresh = 1
</script>

<div class="w-full h-screen">
	<div class="h-full">
		<div class="px-2">
			<span class="flex items-center space-x-2 flex-row justify-between">
				<div class="flex flex-row flex-wrap justify-between pt-6 pb-2 my-4 px-4">
					<h1 class="!text-2xl font-semibold leading-6 tracking-tight">{'Audit logs'}</h1>
					<Tooltip
						light
						documentationLink="https://www.windmill.dev/docs/core_concepts/audit_logs"
						scale={0.9}
						wrapperClass="flex items-center"
					>
						You can only see your own audit logs unless you are an admin.
					</Tooltip>
				</div>
				<div class="flex flex-row my-3 items-center gap-2">
					<div class="flex gap-1 relative w-full">
						<span class="text-xs absolute -top-4">Min datetime</span>
						<input type="text" value={after ?? 'After'} disabled />
						<CalendarPicker
							date={after}
							placement="bottom-end"
							label="Min datetimes"
							on:change={async ({ detail }) => {
								console.log(detail)
								after = new Date(detail).toISOString()
							}}
						/>
					</div>
					<div class="flex gap-1 relative w-full">
						<span class="text-xs absolute -top-4">Max datetime</span>
						<input type="text" value={before ?? 'Before'} disabled />
						<CalendarPicker
							bind:date={before}
							label="Max datetimes"
							placement="bottom-end"
							on:change={async ({ detail }) => {
								console.log(detail)

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
						<span class="text-xs absolute -top-4">Operation</span>

						<select bind:value={operation}>
							<option selected value="all">all</option>
							{#each Object.keys(operations) as e}
								<option value={operations[e]}>{e}</option>
							{/each}
						</select>
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

					<Button
						variant="border"
						color="light"
						on:click={() => {
							after = undefined
							before = undefined
							username = 'all'
							operation = 'all'
							actionKind = 'all'
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
			</span>

			{#if !$enterpriseLicense}
				<Alert title="Redacted audit logs" type="warning">
					You need an enterprise license to see unredacted audit logs.
				</Alert>
				<div class="py-2" />
			{/if}
		</div>
		<SplitPanesWrapper>
			<Splitpanes>
				<Pane size={70} minSize={50}>
					<DataTable
						on:next={() => {
							pageIndex = (pageIndex ?? 1) + 1
						}}
						on:previous={() => {
							pageIndex = (pageIndex ?? 1) - 1
						}}
						currentPage={pageIndex}
						paginated
						rounded={false}
						size="sm"
						bind:perPage
					>
						<Head>
							<Cell first head>Timestamp</Cell>
							<Cell head>Username</Cell>
							<Cell head>Operation name</Cell>
							<Cell head>Resource</Cell>
						</Head>
						{#if logs?.length > 0}
							<tbody class="divide-y">
								{#each logs as { id, timestamp, username, operation: op, action_kind, resource }}
									<Row
										hoverable
										selected={id === selectedId}
										on:click={() => {
											selectedId = id
										}}
									>
										<Cell first>
											{displayDate(timestamp)}
										</Cell>

										<Cell>
											<Badge>
												{username}
											</Badge>
										</Cell>
										<Cell>
											<div class="flex flex-col gap-1 items-start">
												<Badge
													on:click={() => {
														// @ts-ignore
														actionKind = action_kind.toLocaleLowerCase()
													}}
													color={kindToBadgeColor(action_kind)}>{action_kind}</Badge
												>
												<Badge
													on:click={() => {
														operation = op
													}}
												>
													{op}
												</Badge>
											</div>
										</Cell>
										<Cell last>{resource}</Cell>
									</Row>
								{/each}
							</tbody>
						{:else}
							<div class="text-sm h-full m-auto w-full"> No items </div>
						{/if}
					</DataTable>
				</Pane>
				<Pane size={30} minSize={15}>
					<div class="p-4 flex flex-col gap-2 border-t items-start">
						{#if selectedId}
							{@const log = logs.find((e) => e.id === selectedId)}
							{#if log}
								<span class="font-semibold text-xs leading-6">ID</span>
								<span class="text-xs">{log.id}</span>
								<span class="font-semibold text-xs leading-6">Parameters</span>
								<div class="text-xs p-2 bg-surface-secondary rounded-sm">
									{JSON.stringify(log.parameters, null, 2)}
								</div>

								{#if log?.parameters?.uuid}
									<Button
										href={`run/${log.parameters.uuid}`}
										color="light"
										variant="border"
										size="xs"
										target="_blank"
									>
										View run
									</Button>
								{/if}

								{#if log.operation === AuditLog.operation.JOBS_RUN_SCRIPT}
									<Button
										href={`scripts/get/${log.resource}`}
										color="dark"
										variant="contained"
										size="xs"
										target="_blank"
									>
										View script
									</Button>
								{/if}

								{#if [AuditLog.operation.JOBS_RUN_FLOW, AuditLog.operation.FLOWS_CREATE, AuditLog.operation.FLOWS_UPDATE].includes(log.operation)}
									<Button
										href={`flows/get/${log.resource}`}
										color="dark"
										variant="contained"
										size="xs"
										target="_blank"
									>
										<div class="flex flex-row gap-1 items-center">
											View flow
											<ArrowRight size={14} />
										</div>
									</Button>
								{/if}
								{#if [AuditLog.operation.APPS_UPDATE, AuditLog.operation.APPS_CREATE].includes(log.operation)}
									<Button
										href={`apps/get/${log.resource}`}
										color="dark"
										variant="contained"
										size="xs"
										target="_blank"
									>
										<div class="flex flex-row gap-1 items-center">
											View app
											<ArrowRight size={14} />
										</div>
									</Button>
								{/if}
							{/if}
						{:else}
							<span class="text-xs">No log selected</span>
						{/if}
					</div>
				</Pane>
			</Splitpanes>
		</SplitPanesWrapper>
	</div>
</div>
