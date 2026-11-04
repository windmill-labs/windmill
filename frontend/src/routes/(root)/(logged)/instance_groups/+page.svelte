<script lang="ts">
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'
	import InstanceGroupsTable from '$lib/components/instanceGroups/InstanceGroupsTable.svelte'
	import NewInstanceGroupPopover from '$lib/components/instanceGroups/NewInstanceGroupPopover.svelte'

	let table: InstanceGroupsTable | undefined = $state()
</script>

<CenteredPage>
	<PageHeader title="Instance groups">
		<NewInstanceGroupPopover onCreated={(name) => table?.created(name)} />
	</PageHeader>

	<div class="relative mb-20 pt-8">
		<InstanceGroupsTable bind:this={table} />
	</div>
</CenteredPage>
