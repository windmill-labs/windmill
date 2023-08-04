<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import type { ActionKind } from '$lib/common'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import { Alert } from '$lib/components/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Row from '$lib/components/table/Row.svelte'
	import { AuditLog, AuditService, UserService } from '$lib/gen'
	import { enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import { displayDate } from '$lib/utils'
	import { faCross, faEdit, faPlay, faPlus, faQuestion } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'

	let logs: AuditLog[]
	let usernames: string[]

	// Get all page params
	let username: string | undefined = $page.url.searchParams.get('username') ?? undefined
	let pageIndex: number | undefined = Number($page.url.searchParams.get('page')) || undefined
	let before: string | undefined = $page.url.searchParams.get('before') ?? undefined
	let after: string | undefined = $page.url.searchParams.get('after') ?? undefined
	let perPage: number | undefined = Number($page.url.searchParams.get('perPage')) || 100
	let operation: string | undefined = $page.url.searchParams.get('operation') ?? undefined
	let resource: string | undefined = $page.url.searchParams.get('resource') ?? undefined
	let actionKind: ActionKind | undefined =
		($page.url.searchParams.get('actionKind') as ActionKind) ?? undefined

	async function loadLogs(username: string | undefined, page: number | undefined): Promise<void> {
		if (username == 'all') {
			username = undefined
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
	}

	async function loadUsers() {
		usernames =
			$userStore?.is_admin || $userStore?.is_super_admin
				? await UserService.listUsernames({ workspace: $workspaceStore! })
				: [$userStore?.username ?? '']
	}

	async function gotoUsername(username: string | undefined): Promise<void> {
		goto(`?username=` + (username ? encodeURIComponent(username) : ''))
	}

	async function gotoPage(index: number): Promise<void> {
		pageIndex = index
		goto(`?page=${index}` + (username ? `&username=${encodeURIComponent(username)}` : ''))
	}

	function kindToIcon(kind: string) {
		if (kind == 'Execute') {
			return faPlay
		} else if (kind == 'Delete') {
			return faCross
		} else if (kind == 'Update') {
			return faEdit
		} else if (kind == 'Create') {
			return faPlus
		}
		return faQuestion
	}

	$: {
		if ($workspaceStore) {
			loadUsers()
			loadLogs(username, pageIndex)
		}
	}

	let selectedId: number | undefined = undefined
</script>

<CenteredPage>
	<PageHeader
		title="Audit logs"
		tooltip="You can only see your own audit logs unless you are an admin."
		documentationLink="https://www.windmill.dev/docs/core_concepts/audit_logs"
	/>

	{#if !$enterpriseLicense}
		<Alert title="Redacted audit logs" type="warning"
			>You need an enterprise license to see unredacted audit logs.</Alert
		>
		<div class="py-2" />
	{/if}
	<!-- Filtering -->
	<div class="flex flex-row my-3">
		<label>
			<select class="px-6" bind:value={username} on:change={() => gotoUsername(username)}>
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
		</label>
	</div>

	<div class="flex flex-row">
		<div class="w-2/3">
			<DataTable
				on:next={() => {
					gotoPage((pageIndex ?? 1) + 1)
				}}
				on:previous={() => {
					gotoPage((pageIndex ?? 1) - 1)
				}}
				currentPage={pageIndex}
				paginated={true}
				rounded={false}
			>
				<Head>
					<Cell first head>Timestamp</Cell>
					<Cell head>Username</Cell>
					<Cell head>Operation name</Cell>
					<Cell head>Resource</Cell>
					<Cell head>Parameters</Cell>
				</Head>
				<tbody class="divide-y">
					{#if logs}
						{#each logs as { id, timestamp, username, operation, action_kind, resource, parameters }}
							<Row
								hoverable
								selected={id === selectedId}
								on:click={() => {
									selectedId = id
								}}
							>
								<Cell first>
									<div class="whitespace-nowrap overflow-x-auto no-scrollbar max-w-xs">
										{displayDate(timestamp)}
									</div>
								</Cell>

								<Cell>
									<div class="whitespace-nowrap overflow-x-auto no-scrollbar w-20">
										{username}
									</div>
								</Cell>
								<Cell>
									<div class="flex flex-row gap-1">
										<Badge>{action_kind}</Badge>
										<Badge>{operation}</Badge>
									</div>
								</Cell>
								<Cell>{resource}</Cell>
								<Cell last>
									{#if parameters}
										<div class="overflow-x-auto no-scrollbar max-w-xs">
											<pre>{JSON.stringify(parameters, null)}</pre>
										</div>
									{/if}
								</Cell>
							</Row>
						{/each}
					{/if}
				</tbody>
			</DataTable>
		</div>
		<div class="w-1/3 border-y border-r">
			<div class="p-2">
				{#if selectedId}
					{@const log = logs.find((e) => e.id === selectedId)}
					{JSON.stringify(log, null, 2)}
				{/if}
			</div>
		</div>
	</div>

	{#if logs?.length == 0}
		<div class="bg-red-100 border-l-4 border-red-600 text-orange-700 p-4 m-4" role="alert">
			<p class="font-bold">No logs</p>
			<p>
				Either there is no audit logs for this person or you do not have access to them for this
				person
			</p>
		</div>
	{/if}
</CenteredPage>

<style lang="postcss">
	/* Hide scrollbar for Chrome, Safari and Opera */
	.no-scrollbar::-webkit-scrollbar {
		display: none;
	}

	/* Hide scrollbar for IE, Edge and Firefox */
	.no-scrollbar {
		-ms-overflow-style: none; /* IE and Edge */
		scrollbar-width: none; /* Firefox */
	}
	td {
		@apply text-xs p-1;
	}
</style>
