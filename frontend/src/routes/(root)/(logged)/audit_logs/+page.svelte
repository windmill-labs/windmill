<script lang="ts">
	import { AuditService, AuditLog, UserService } from '$lib/gen'
	import type { ActionKind } from '$lib/common'
	import { page } from '$app/stores'
	import { displayDate } from '$lib/utils'
	import { goto } from '$app/navigation'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import { superadmin, userStore, workspaceStore } from '$lib/stores'
	import TableCustom from '$lib/components/TableCustom.svelte'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import Icon from 'svelte-awesome'
	import { faCross, faEdit, faPlay, faPlus, faQuestion } from '@fortawesome/free-solid-svg-icons'

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
</script>

<CenteredPage>
	<PageHeader
		title="Audit logs"
		tooltip="You can only see your own audit logs unless you are an admin."
	/>

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

	<TableCustom
		on:next={() => {
			gotoPage((pageIndex ?? 1) + 1)
		}}
		on:previous={() => {
			gotoPage((pageIndex ?? 1) - 1)
		}}
		currentPage={pageIndex}
		paginated={true}
	>
		<tr slot="header-row">
			<th>id</th>
			<th>timestamp</th>

			<th class="px-1">op kind</th>
			<th>username</th>
			<th>operation name</th>
			<th>resource</th>
			<th>parameters</th>
		</tr>
		<tbody slot="body">
			{#if logs}
				{#each logs as { id, timestamp, username, operation, action_kind, resource, parameters }}
					<tr>
						<td>{id}</td>
						<td class="">
							<div class="whitespace-nowrap overflow-x-auto no-scrollbar max-w-xs">
								{displayDate(timestamp)}
							</div>
						</td>
						<td class="">
							<Icon class="inline m-1" data={kindToIcon(action_kind)} scale={1} />
						</td>
						<td class="">
							<div class="whitespace-nowrap overflow-x-auto no-scrollbar w-20">
								{username}
							</div>
						</td>
						<td class=""><pre>{operation}</pre></td>
						<td class="">{resource}</td>
						<td class="">
							{#if parameters}
								<div class="overflow-x-auto no-scrollbar max-w-xs">
									<pre>{JSON.stringify(parameters, null)}</pre>
								</div>
							{/if}
						</td>
					</tr>
				{/each}
			{/if}
		</tbody>
	</TableCustom>

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

<style>
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
