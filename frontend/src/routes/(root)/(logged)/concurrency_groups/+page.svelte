<script lang="ts">
	import type { ConcurrencyGroup } from '$lib/gen'
	import { ConcurrencyGroupsService } from '$lib/gen'

	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import { Button, Drawer, DrawerContent } from '$lib/components/common'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import TableCustom from '$lib/components/TableCustom.svelte'
	import { RefreshCw, Trash } from 'lucide-svelte'
	import { sleep } from '$lib/utils'
	import { onDestroy, onMount } from 'svelte'

	let concurrencyGroups: ConcurrencyGroup[] | undefined = undefined

	let selectedGroup: ConcurrencyGroup | undefined = undefined
	let groupDrawer: Drawer

	let doLoadConcurrencyGroups = false
	let concurrencyGroupsLoading = false

	onMount(() => {
		doLoadConcurrencyGroups = true
		continuouslyLoadConcurrencyGroups()
	})

	onDestroy(() => {
		doLoadConcurrencyGroups = false
	})

	async function continuouslyLoadConcurrencyGroups() {
		while (doLoadConcurrencyGroups) {
			loadConcurrencyGroupsOnce()
			await sleep(2 * 1000)
		}
	}

	async function loadConcurrencyGroupsOnce() {
		if (concurrencyGroupsLoading) {
			return
		}
		const timeStart = new Date().getTime()
		concurrencyGroupsLoading = true

		try {
			concurrencyGroups = await ConcurrencyGroupsService.listConcurrencyGroups()
		} catch (e) {
			concurrencyGroups = undefined
		}

		const endStart = new Date().getTime()
		// toggle concurrencyGroupsLoading to false in 1 secs to let some time for the animation to play
		setTimeout(() => {
			concurrencyGroupsLoading = false
		}, 1000 - (endStart - timeStart))
	}

	async function deleteConcurrencyGroup(concurrencyGroupId: string) {
		await ConcurrencyGroupsService.deleteConcurrencyGroup({
			concurrencyId: concurrencyGroupId
		})
		loadConcurrencyGroupsOnce()
	}
</script>

<Drawer bind:this={groupDrawer}>
	<DrawerContent
		title="Instance Group {selectedGroup?.concurrency_id}"
		on:close={groupDrawer.closeDrawer}
	>
		{#if selectedGroup?.job_uuids && selectedGroup?.job_uuids.length > 0}
			<h3 class="mb-2">Jobs running for this group</h3>

			<ul>
				{#each selectedGroup?.job_uuids as jobUuid}
					<li>
						{jobUuid}
					</li>
				{/each}
			</ul>
		{:else}
			<h3>No job running for this group</h3>
		{/if}
	</DrawerContent>
</Drawer>

<CenteredPage>
	<PageHeader title="Concurrency Groups">
		<Button
			color="light"
			size="md"
			btnClasses="w-full h-8"
			variant="border"
			on:click={loadConcurrencyGroupsOnce}
		>
			<RefreshCw class={concurrencyGroupsLoading ? 'animate-spin' : ''} size="xs" />Refresh
		</Button>
	</PageHeader>

	{#if concurrencyGroups && concurrencyGroups.length > 0}
		<div class="relative mb-20 pt-8">
			<TableCustom>
				<tr slot="header-row">
					<th>Concurrency ID</th>
					<th>Jobs running</th>
					<th />
				</tr>
				<tbody slot="body">
					{#each concurrencyGroups as { concurrency_id, job_uuids }}
						<tr>
							<td>
								<a
									href="#{concurrency_id}"
									on:click={() => {
										selectedGroup = { concurrency_id, job_uuids }
										groupDrawer.openDrawer()
									}}
									>{concurrency_id}
								</a>
							</td>
							<td>
								{job_uuids.length}
							</td>
							<td>
								<div class="flex justify-center">
									<Button
										size="md"
										color="light"
										btnClasses="justify-center w-12"
										startIcon={{ icon: Trash, classes: 'text-red-500' }}
										on:click={() => {
											deleteConcurrencyGroup(concurrency_id)
										}}
										iconOnly={true}
									/>
								</div>
							</td>
						</tr>
					{/each}
				</tbody>
			</TableCustom>
		</div>
	{/if}
</CenteredPage>
