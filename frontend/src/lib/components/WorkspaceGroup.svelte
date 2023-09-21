<script lang="ts">
	import { X } from 'lucide-svelte'
	import { Button, Popup } from './common'
	import ToggleButton from './common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from './common/toggleButton-v2/ToggleButtonGroup.svelte'
	import { ConfigService } from '$lib/gen'
	import ConfirmationModal from './common/confirmationModal/ConfirmationModal.svelte'
	import { createEventDispatcher } from 'svelte'
	import { sendUserToast } from '$lib/toast'
	import { enterpriseLicense, superadmin } from '$lib/stores'
	import Tooltip from './Tooltip.svelte'

	export let name: string
	export let config:
		| undefined
		| {
				dedicated_worker?: string
				worker_tags?: string[]
		  }
	export let top: boolean

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
		await ConfigService.deleteConfig({ name: 'worker__' + name })
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
	{#if $superadmin}
		<Popup
			floatingConfig={{ strategy: 'absolute', placement: top ? 'top-start' : 'bottom-start' }}
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
				<ToggleButton
					position="left"
					value="normal"
					size="sm"
					label="Any jobs within worker tags"
				/>
				<ToggleButton
					position="dedicated"
					value="dedicated"
					size="sm"
					label="Dedicated to a script"
				/>
			</ToggleButtonGroup>
			{#if selected == 'normal'}
				{#if nconfig?.worker_tags != undefined}
					<div class="flex gap-3 gap-y-2 flex-wrap pb-2">
						{#each nconfig.worker_tags as tag}
							<div class="flex gap-0.5 items-center"
								><div class="text-2xs p-1 rounded border text-primary">{tag}</div>
								<button
									class="z-10 rounded-full p-1 duration-200 hover:bg-gray-200"
									aria-label="Remove item"
									on:click|preventDefault|stopPropagation={() => {
										if (nconfig != undefined) {
											nconfig.worker_tags = nconfig?.worker_tags?.filter((t) => t != tag) ?? []
										}
									}}
								>
									<X size={12} />
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
					<div class="flex flex-wrap mt-2 items-center gap-1">
						<Button
							variant="contained"
							color="light"
							size="xs"
							on:click={() => {
								if (nconfig != undefined) {
									nconfig.worker_tags = defaultTags.concat(nativeTags)
									dirty = true
								}
							}}
						>
							Reset to all tags <Tooltip>{defaultTags.concat(nativeTags).join(', ')}</Tooltip>
						</Button>
						<Button
							variant="contained"
							color="light"
							size="xs"
							on:click={() => {
								if (nconfig != undefined) {
									nconfig.worker_tags = defaultTags
									dirty = true
								}
							}}
						>
							Reset to all tags minus native ones <Tooltip>{defaultTags.join(', ')}</Tooltip>
						</Button>
						<Button
							variant="contained"
							color="light"
							size="xs"
							on:click={() => {
								if (nconfig != undefined) {
									nconfig.worker_tags = nativeTags
									dirty = true
								}
							}}
						>
							Reset to native tags <Tooltip>{nativeTags.join(', ')}</Tooltip>
						</Button>
					</div>
				{/if}
			{:else if selected == 'dedicated'}
				{#if nconfig?.dedicated_worker != undefined}
					<input
						placeholder="<workspace>:<script path>"
						type="text"
						on:change={() => {
							dirty = true
						}}
						bind:value={nconfig.dedicated_worker}
					/>
					<p class="text-2xs text-tertiary max-w-md mt-2"
						>Workers will get killed upon detecting this setting change. It is assumed they are in
						an environment where the supervisor will restart them. Upon restart, they will pick the
						new dedicated worker config.</p
					>
				{/if}
			{/if}
			<div class="mt-4" />
			<div class="flex gap-1 items-center">
				<Button
					variant="contained"
					color="dark"
					size="xs"
					on:click={async () => {
						await ConfigService.updateConfig({ name: 'worker__' + name, requestBody: nconfig })
						sendUserToast('Configuration set')
						dispatch('reload')
					}}
					disabled={!dirty || !$enterpriseLicense}
				>
					Apply changes {#if !$enterpriseLicense}(ee only){/if}
				</Button>
				{#if !$enterpriseLicense}<Tooltip
						>{selected == 'dedicated'
							? 'Dedicated workers are an enterprise only feature'
							: 'The Worker Group Manager UI is an enterprise only feature. However, workers can still have their WORKER_TAGS passed as env'}</Tooltip
					>{/if}
			</div>
		</Popup>
		{#if config}
			<Button color="light" size="xs" on:click={() => (open = true)} btnClasses="text-red-400">
				delete config
			</Button>
		{/if}
	{/if}
</div>
