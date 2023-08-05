<script lang="ts">
	import { goto } from '$app/navigation'
	import { page } from '$app/stores'
	import type { ActionKind } from '$lib/common'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { Alert } from '$lib/components/common'
	import Badge from '$lib/components/common/badge/Badge.svelte'
	import SplitPanesWrapper from '$lib/components/splitPanes/SplitPanesWrapper.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Head from '$lib/components/table/Head.svelte'
	import Row from '$lib/components/table/Row.svelte'
	import { AuditLog, AuditService, UserService } from '$lib/gen'
	import { enterpriseLicense, userStore, workspaceStore } from '$lib/stores'
	import { displayDate } from '$lib/utils'
	import { Splitpanes, Pane } from 'svelte-splitpanes'

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

	$: {
		if ($workspaceStore) {
			loadUsers()
			loadLogs(username, pageIndex)
		}
	}

	let selectedId: number | undefined = undefined
</script>

<div class="w-full h-screen">
	<div class="h-full">
		<div class="px-2">
			<span class="flex items-center space-x-2">
				<div class="flex flex-row flex-wrap justify-between pt-6 pb-2 my-4">
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
			</span>

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
		</div>
		<SplitPanesWrapper>
			<Splitpanes>
				<Pane size={65} minSize={50}>
					<DataTable
						on:next={() => {
							gotoPage((pageIndex ?? 1) + 1)
						}}
						on:previous={() => {
							gotoPage((pageIndex ?? 1) - 1)
						}}
						currentPage={pageIndex}
						paginated
						rounded={false}
						size="sm"
					>
						<Head>
							<Cell first head>Timestamp</Cell>
							<Cell head>Username</Cell>
							<Cell head>Operation name</Cell>
							<Cell head>Resource</Cell>
						</Head>
						<tbody class="divide-y">
							{#if logs}
								{#each logs as { id, timestamp, username, operation, action_kind, resource }}
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
											<div class="flex flex-col gap-1 items-start">
												<Badge color="blue">{action_kind}</Badge>
												<Badge>{operation}</Badge>
											</div>
										</Cell>
										<Cell last>{resource}</Cell>
									</Row>
								{/each}
							{/if}
						</tbody>
					</DataTable>
				</Pane>
				<Pane size={35} minSize={15}>
					<div class="p-4 flex flex-col gap-2 border-t">
						{#if selectedId}
							{@const log = logs.find((e) => e.id === selectedId)}
							{#if log}
								<span class="font-semibold text-xs leading-6">ID</span>
								<span class="text-xs">{log.id}</span>
								<span class="font-semibold text-xs leading-6">Parameters</span>
								<div class="text-xs p-2 bg-surface-secondary rounded-sm">
									{JSON.stringify(log.parameters, null, 2)}
								</div>
								<span class="font-semibold text-xs leading-6">Username</span>
								<span class="text-xs">{log.username}</span>
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
</style>
