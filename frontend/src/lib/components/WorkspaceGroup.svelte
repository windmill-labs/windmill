<script lang="ts">
	import { X } from 'lucide-svelte'
	import { Button, Popup } from './common'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import { WorkerService } from '$lib/gen'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import { createEventDispatcher } from 'svelte'
	import { sendUserToast } from '$lib/toast'

	export let name: string
	export let config:
		| undefined
		| {
				dedicated_worker?: string
				worker_tags?: string[]
		  }

	let nconfig: any = config
		? config.worker_tags != undefined || config.dedicated_worker != undefined
			? config
			: {
					worker_tags: []
			  }
		: {
				worker_tags: []
		  }

	const defaultTags = [
		'deno',
		'python3',
		'go',
		'bash',
		'powershell',
		'dependency',
		'flow',
		'hub',
		'other',
		'bun'
	]
	const nativeTags = ['nativets', 'postgresql', 'mysql', 'graphql', 'snowflake']

	let newTag: string = ''
	$: selected = nconfig?.dedicated_worker != undefined ? 'dedicated' : 'normal'

	const dispatch = createEventDispatcher()

	async function deleteWorkerGroup() {
		await WorkerService.deleteWorkerGroup({ name })
		dispatch('reload')
	}
	let dirty = false
	let open = false
</script>

<ConfirmationModal
	{open}
	title="Delete worker group"
	confirmationText="Remove"
	on:canceled={() => {
		open = false
	}}
	on:confirmed={async () => {
		deleteWorkerGroup()
		open = false
	}}
>
	<div class="flex flex-col w-full space-y-4">
		<span>Are you sure you want to remove this worker nconfig?</span>
	</div>
</ConfirmationModal>

<div class="flex gap-2 items-center"
	><h4 class="py-4 truncate w-40">{name}</h4>
	<Popup
		floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
		containerClasses="border rounded-lg shadow-lg p-4 bg-surface"
	>
		<svelte:fragment slot="button">
			<Button color="light" size="xs" nonCaptureEvent={true}>
				<div class="flex flex-row gap-1 items-center"
					>{config == undefined ? 'create' : 'edit'} config</div
				>
			</Button>
		</svelte:fragment>
		<ToggleButtonGroup
			{selected}
			on:selected={(e) => {
				dirty = true
				if (nconfig == undefined) {
					nconfig = {}
				}
				console.log(e.detail)
				if (e.detail == 'dedicated') {
					nconfig.dedicated_worker = ''
					nconfig.worker_tags = undefined
				} else {
					nconfig.dedicated_worker = undefined
					nconfig.worker_tags = []
				}
			}}
			class="mb-4"
		>
			<ToggleButton position="left" value="normal" size="sm" label="Any jobs within worker tags" />
			<ToggleButton
				position="dedicated"
				value="dedicated"
				size="sm"
				label="Dedicated to a script"
			/>
		</ToggleButtonGroup>
		{#if selected == 'normal'}
			{#if nconfig?.worker_tags != undefined}
				<div class="flex flex-col gap-1 pb-2">
					{#each nconfig.worker_tags as tag}
						<div class="flex gap-1 items-center"
							><div>- {tag}</div>
							<button
								class="z-10 rounded-full p-1 duration-200 hover:bg-gray-200"
								aria-label="Remove item"
								on:click|preventDefault|stopPropagation={() => {
									if (nconfig != undefined) {
										nconfig.worker_tags = nconfig?.worker_tags?.filter((t) => t != tag) ?? []
									}
								}}
							>
								<X size={14} />
							</button></div
						>
					{/each}
				</div>
				<input type="text" placeholder="new tag" bind:value={newTag} />
				<div class="mt-1" />
				<Button
					variant="contained"
					color="blue"
					size="xs"
					disabled={newTag == '' || nconfig.worker_tags?.includes(newTag)}
					on:click={() => {
						if (nconfig != undefined) {
							nconfig.worker_tags = [...(nconfig?.worker_tags ?? []), newTag.replaceAll(' ', '_')]
							newTag = ''
							dirty = true
						}
					}}
				>
					Add tag
				</Button>
			{/if}
		{:else if selected == 'dedicated'}
			{#if nconfig?.dedicated_worker != undefined}
				<input
					placeholder="Script path"
					type="text"
					on:change={() => {
						dirty = true
					}}
					bind:value={nconfig.dedicated_worker}
				/>
			{/if}
		{/if}
		<div class="mt-4" />
		<Button
			variant="contained"
			color="dark"
			size="xs"
			on:click={async () => {
				await WorkerService.updateWorkerGroup({ name, requestBody: nconfig })
				sendUserToast(
					'Setting configuration, it can take up to 30s to get propagated to all workers'
				)
				dispatch('reload')
			}}
			disabled={!dirty}
		>
			Set configuration
		</Button>
	</Popup>
	{#if config}
		<Button color="light" size="xs" on:click={() => (open = true)} btnClasses="text-red-400">
			delete config
		</Button>
	{/if}
</div>
