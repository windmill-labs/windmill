<script lang="ts">
	import { goto } from '$app/navigation'
	import Dropdown from '$lib/components/Dropdown.svelte'
	import SharedBadge from '$lib/components/SharedBadge.svelte'
	import type ShareModal from '$lib/components/ShareModal.svelte'
	import Star from '$lib/components/Star.svelte'

	import type { ListableApp } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { faEdit, faEye } from '@fortawesome/free-solid-svg-icons'
	import { createEventDispatcher } from 'svelte'
	import { LayoutDashboard } from 'svelte-lucide'

	import Button from '../button/Button.svelte'

	export let app: ListableApp & { canWrite: boolean }
	export let marked: string | undefined
	export let starred: boolean
	export let shareModal: ShareModal

	let { summary, path, extra_perms, canWrite, workspace_id } = app

	const dispatch = createEventDispatcher()
	//  on:click={() => goto(`/apps/get/${path}`)}
</script>

<tr class="hover:bg-gray-50 cursor-pointer">
	<td class="pl-4 py-4 pr-1 w-8">
		<div
			class="bg-[#fff7ed] rounded-md p-1 flex justify-center items-center border-orange-200 border"
		>
			<LayoutDashboard size="18px" color="#fb923c" />
		</div>
	</td>

	<td class="px-2 py-4">
		<div class="text-gray-900 max-w-md flex-wrap text-md font-semibold mb-1">
			{#if marked}
				{@html marked}
			{:else}
				{!summary || summary.length == 0 ? path : summary}
			{/if}
		</div>
		<div class="text-gray-600 text-xs ">
			{path}
		</div>
	</td>
	<td class="px-2 py-4 w-64">
		<div class="flex flex-row max-w-xs gap-1 items-start flex-wrap">
			<SharedBadge {canWrite} extraPerms={extra_perms} />
		</div>
	</td>
	<td class="py-4 text-left text-sm font-semibold text-gray-900 px-2 w-0">
		{#if path}
			<Star
				kind="app"
				{path}
				{starred}
				workspace_id={workspace_id ?? $workspaceStore ?? ''}
				on:starred={() => {
					dispatch('change')
				}}
			/>
		{/if}
	</td>

	<td class="py-4 pl-2 pr-6 w-0">
		<div class="w-full flex gap-1 items-center justify-end">
			<Dropdown
				dropdownItems={[
					{
						displayName: 'View app',
						icon: faEye,
						href: `/apps/get/${path}`
					}
				]}
			/>

			{#if canWrite}
				<div>
					<Button
						color="light"
						size="xs"
						variant="border"
						startIcon={{ icon: faEdit }}
						href="/apps/edit/{path}"
					>
						Edit
					</Button>
				</div>
			{/if}

			<Button
				href="/apps/get/{path}"
				color="dark"
				size="xs"
				spacingSize="md"
				startIcon={{ icon: faEye }}
			>
				View
			</Button>
		</div>
	</td>
</tr>
