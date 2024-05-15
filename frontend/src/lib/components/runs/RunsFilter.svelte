<script lang="ts">
	import { Button, Popup } from '../common'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import Tooltip from '../Tooltip.svelte'
	import AutoComplete from 'simple-svelte-autocomplete'
	import { AlertCircle, CheckCircle2, ChevronDown, Filter, PlayCircle, X } from 'lucide-svelte'
	import JsonEditor from '../apps/editor/settingsPanel/inputEditor/JsonEditor.svelte'
	import Toggle from '../Toggle.svelte'
	import Label from '../Label.svelte'
	import Section from '../Section.svelte'
	import CloseButton from '../common/CloseButton.svelte'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import ToggleButtonMore from '../common/toggleButton-v2/ToggleButtonMore.svelte'

	// Filters
	export let path: string | null = null
	export let label: string | null = null
	export let concurrencyKey: string | null = null
	export let success: 'running' | 'success' | 'failure' | undefined = undefined
	export let isSkipped: boolean | undefined = undefined
	export let argFilter: string
	export let argError: string
	export let resultFilter: string
	export let resultError: string
	export let jobKindsCat: string
	export let user: string | null = null
	export let folder: string | null = null
	export let mobile: boolean = false

	// Autocomplete data
	export let paths: string[] = []
	export let usernames: string[] = []
	export let folders: string[] = []
	export let allWorkspaces = false

	$: displayedLabel = label
	$: displayedConcurrencyKey = concurrencyKey

	let copyArgFilter = argFilter
	let copyResultFilter = resultFilter

	export let filterBy: 'path' | 'user' | 'folder' | 'label' | 'concurrencyKey' = 'path'

	const dispatch = createEventDispatcher()

	let manuallySet = false

	$: {
		if (path !== null && path !== '' && filterBy !== 'path') {
			manuallySet = true
			filterBy = 'path'
		} else if (user !== null && user !== '' && filterBy !== 'user') {
			manuallySet = true
			filterBy = 'user'
		} else if (folder !== null && folder !== '' && filterBy !== 'folder') {
			manuallySet = true
			filterBy = 'folder'
		} else if (label !== null && label !== '' && filterBy !== 'label') {
			manuallySet = true
			filterBy = 'label'
		} else if (concurrencyKey !== null && concurrencyKey !== '' && filterBy !== 'concurrencyKey') {
			manuallySet = true
			filterBy = 'concurrencyKey'
		}
	}

	let labelTimeout: NodeJS.Timeout | undefined = undefined
	let concurrencyKeyTimeout: NodeJS.Timeout | undefined = undefined
</script>

<div class="flex gap-4">
	{#if !mobile}
		<div class="flex gap-2">
			{#if $workspaceStore == 'admins'}
				<div class="relative">
					<span class="text-xs absolute -top-4">Workspaces</span>
					<ToggleButtonGroup bind:selected={allWorkspaces}>
						<ToggleButton value={false} label="Admins" />
						<ToggleButton value={true} label="All" />
					</ToggleButtonGroup>
				</div>
			{/if}

			<div class="relative">
				<span class="text-xs absolute -top-4">Filter by</span>
				<ToggleButtonGroup
					bind:selected={filterBy}
					on:selected={() => {
						if (!manuallySet) {
							path = null
							user = null
							folder = null
							label = null
							concurrencyKey = null
						} else {
							manuallySet = false
						}
					}}
				>
					<ToggleButton value="path" label="Path" />
					<ToggleButton value="user" label="User" />
					<ToggleButton value="folder" label="Folder" />
					<ToggleButtonMore
						togglableItems={[
							{ label: 'Concurrency key', value: 'concurrencyKey' },
							{ label: 'Label', value: 'label' }
						]}
					/>
				</ToggleButtonGroup>
			</div>

			{#if filterBy == 'user'}
				{#key user}
					<div class="relative">
						{#if user}
							<button
								class="absolute top-2 right-2 z-50"
								on:click={() => {
									user = null
									dispatch('reset')
								}}
							>
								<X size={14} />
							</button>
						{:else}
							<ChevronDown class="absolute top-2 right-2" size={14} />
						{/if}

						<span class="text-xs absolute -top-4">User</span>
						<AutoComplete
							create
							onCreate={(user) => {
								usernames.push(user)
								return user
							}}
							createText="Press enter to use this value"
							noInputStyles
							items={usernames}
							value={user}
							bind:selectedItem={user}
							inputClassName="!h-[32px] py-1 !text-xs !w-64"
							hideArrow
							className={user ? '!font-bold' : ''}
							dropdownClassName="!font-normal !w-64 !max-w-64"
						/>
					</div>
				{/key}
			{:else if filterBy == 'folder'}
				{#key folder}
					<div class="relative">
						{#if folder}
							<button
								class="absolute top-2 right-2 z-50"
								on:click={() => {
									folder = null
									dispatch('reset')
								}}
							>
								<X size={14} />
							</button>
						{:else}
							<ChevronDown class="absolute top-2 right-2" size={14} />
						{/if}

						<span class="text-xs absolute -top-4">Folder</span>

						<AutoComplete
							noInputStyles
							items={folders}
							value={folder}
							bind:selectedItem={folder}
							inputClassName="!h-[32px] py-1 !text-xs !w-64"
							hideArrow
							className={folder ? '!font-bold' : ''}
							dropdownClassName="!font-normal !w-64 !max-w-64"
						/>
					</div>
				{/key}
			{:else if filterBy === 'path'}
				{#key path}
					<div class="relative">
						{#if path}
							<button
								class="absolute top-2 right-2 z-50"
								on:click={() => {
									path = null
									dispatch('reset')
								}}
							>
								<X size={14} />
							</button>
						{:else}
							<ChevronDown class="absolute top-2 right-2" size={14} />
						{/if}

						<span class="text-xs absolute -top-4">Path</span>

						<AutoComplete
							create
							onCreate={(path) => {
								paths.push(path)
								return path
							}}
							createText="Press enter to use this value"
							noInputStyles
							items={paths}
							value={path}
							bind:selectedItem={path}
							inputClassName="!h-[32px] py-1 !text-xs !w-64"
							hideArrow
							className={path ? '!font-bold' : ''}
							dropdownClassName="!font-normal !w-64 !max-w-64"
						/>
					</div>
				{/key}
			{:else if filterBy === 'label'}
				{#key label}
					<div class="relative">
						{#if label}
							<button
								class="absolute top-2 right-2 z-50"
								on:click={() => {
									label = null
									dispatch('reset')
								}}
							>
								<X size={14} />
							</button>
						{/if}

						<span class="text-xs absolute -top-4"
							>Label <Tooltip
								>Labels are string values in the array at the result field 'wm_labels' to easily
								filter them</Tooltip
							></span
						>

						<input
							autofocus
							type="text"
							class="!h-[32px] py-1 !text-xs !w-64"
							bind:value={displayedLabel}
							on:keydown={(e) => {
								if (labelTimeout) {
									clearTimeout(labelTimeout)
								}

								labelTimeout = setTimeout(() => {
									label = displayedLabel
								}, 1000)
							}}
						/>
					</div>
				{/key}
			{:else if filterBy === 'concurrencyKey'}
				{#key concurrencyKey}
					<div class="relative">
						{#if concurrencyKey}
							<button
								class="absolute top-2 right-2 z-50"
								on:click={() => {
									concurrencyKey = null
									dispatch('reset')
								}}
							>
								<X size={14} />
							</button>
						{/if}
						<span class="text-xs absolute -top-4"
							>Concurrency Key <Tooltip>
								For concurrency limited jobs, the concurrency key defines a group of jobs that share
								the same limits.
								{#if !$enterpriseLicense}
									Concurrency limits are an EE feature.
								{/if}
							</Tooltip></span
						>

						<input
							autofocus
							type="text"
							class="!h-[32px] py-1 !text-xs !w-64"
							bind:value={displayedConcurrencyKey}
							on:keydown={(e) => {
								if (concurrencyKeyTimeout) {
									clearTimeout(concurrencyKeyTimeout)
								}

								concurrencyKeyTimeout = setTimeout(() => {
									concurrencyKey = displayedConcurrencyKey
								}, 1000)
							}}
						/>
					</div>
				{/key}
			{/if}
		</div>
		<div class="relative">
			<span class="text-xs absolute -top-4">Kind</span>
			<ToggleButtonGroup bind:selected={jobKindsCat}>
				<ToggleButton value="all" label="All" />
				<ToggleButton
					value="runs"
					label="Runs"
					showTooltipIcon
					tooltip="Runs are jobs that have no parent jobs (flows are jobs that are parent of the jobs they start), they have been triggered through the UI, a schedule or webhook"
				/>
				<ToggleButton
					value="previews"
					label="Previews"
					showTooltipIcon
					tooltip="Previews are jobs that have been started in the editor as 'Tests'"
				/>
				<ToggleButton
					value="dependencies"
					label="Deps"
					showTooltipIcon
					tooltip="Deploying a script, flow or an app launch a dependency job that create and then attach the lockfile to the deployed item. This mechanism ensure that logic is always executed with the exact same direct and indirect dependencies."
				/>
				<ToggleButton
					value="deploymentcallbacks"
					label="Sync"
					showTooltipIcon
					tooltip="Sync jobs that are triggered on every script deployment to sync the workspace with the Git repository configured in the the workspace settings"
				/>
			</ToggleButtonGroup>
		</div>
		<div class="relative">
			<span class="text-xs absolute -top-4">Status</span>
			<ToggleButtonGroup bind:selected={success}>
				<ToggleButton value={undefined} label="All" />
				<ToggleButton
					value={'running'}
					label="Running"
					class="whitespace-nowrap"
					icon={PlayCircle}
					iconProps={{ color: success === 'running' ? 'blue' : 'gray' }}
				/>
				<ToggleButton
					value={'success'}
					label="Success"
					class="whitespace-nowrap"
					icon={CheckCircle2}
					iconProps={{ color: success === 'success' ? 'green' : 'gray' }}
				/>
				<ToggleButton
					value={'failure'}
					label="Failure"
					class="whitespace-nowrap"
					icon={AlertCircle}
					iconProps={{ color: success === 'failure' ? 'red' : 'gray' }}
				/>
			</ToggleButtonGroup>
		</div>
	{/if}

	<Popup
		floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
		containerClasses="border rounded-lg shadow-lg p-6 bg-surface"
		let:close
	>
		<svelte:fragment slot="button">
			<Button color="dark" size="xs" nonCaptureEvent={true} startIcon={{ icon: Filter }}>
				More filters
			</Button>
		</svelte:fragment>

		<Section label="Filters">
			<svelte:fragment slot="action">
				<CloseButton on:close={() => close(null)} />
			</svelte:fragment>

			<div class="w-80 flex flex-col gap-4">
				{#if mobile}
					<Label label="Filter by">
						<ToggleButtonGroup
							bind:selected={filterBy}
							on:selected={() => {
								if (!manuallySet) {
									path = null
									user = null
									folder = null
									label = null
								} else {
									manuallySet = false
								}
							}}
						>
							<ToggleButton value="path" label="Path" />
							<ToggleButton value="user" label="User" />
							<ToggleButton value="folder" label="Folder" />
						</ToggleButtonGroup>
					</Label>

					{#if filterBy == 'user'}
						{#key user}
							<Label label="User">
								<div class="relative w-full">
									{#if user}
										<button
											class="absolute top-2 right-2 z-50"
											on:click={() => {
												user = null
											}}
										>
											<X size={14} />
										</button>
									{:else}
										<ChevronDown class="absolute top-2 right-2" size={14} />
									{/if}

									<AutoComplete
										items={usernames}
										value={user}
										bind:selectedItem={user}
										inputClassName="!h-[32px] py-1 !text-xs !w-64"
										hideArrow
										className={user ? '!font-bold' : ''}
										dropdownClassName="!font-normal !w-64 !max-w-64"
									/>
								</div>
							</Label>
						{/key}
					{:else if filterBy == 'folder'}
						{#key folder}
							<Label label="Folder">
								<div class="relative w-full">
									{#if folder}
										<button
											class="absolute top-2 right-2 z-50"
											on:click={() => {
												folder = null
											}}
										>
											<X size={14} />
										</button>
									{:else}
										<ChevronDown class="absolute top-2 right-2" size={14} />
									{/if}

									<AutoComplete
										noInputStyles
										items={folders}
										value={folder}
										bind:selectedItem={folder}
										inputClassName="!h-[32px] py-1 !text-xs !w-64"
										hideArrow
										className={folder ? '!font-bold' : ''}
										dropdownClassName="!font-normal !w-64 !max-w-64"
									/>
								</div>
							</Label>
						{/key}
					{:else if filterBy === 'path'}
						{#key path}
							<Label label="Path">
								<div class="relative w-full">
									{#if path}
										<button
											class="absolute top-2 right-2 z-50"
											on:click={() => {
												path = null
											}}
										>
											<X size={14} />
										</button>
									{:else}
										<ChevronDown class="absolute top-2 right-2" size={14} />
									{/if}

									<AutoComplete
										noInputStyles
										items={paths}
										value={path}
										bind:selectedItem={path}
										inputClassName="!h-[32px] py-1 !text-xs !w-80"
										hideArrow
										className={path ? '!font-bold' : ''}
										dropdownClassName="!font-normal !w-80 !max-w-80"
									/>
								</div>
							</Label>
						{/key}
					{/if}

					<Label label="Kind">
						<ToggleButtonGroup bind:selected={jobKindsCat}>
							<ToggleButton value="all" label="All" />
							<ToggleButton
								value="runs"
								label="Runs"
								showTooltipIcon
								tooltip="Runs are jobs that have no parent jobs (flows are jobs that are parent of the jobs they start), they have been triggered through the UI, a schedule or webhook"
							/>
							<ToggleButton
								value="previews"
								label="Previews"
								showTooltipIcon
								tooltip="Previews are jobs that have been started in the editor as 'Tests'"
							/>
							<ToggleButton
								value="dependencies"
								label="Deps"
								showTooltipIcon
								tooltip="Deploying a script, flow or an app launch a dependency job that create and then attach the lockfile to the deployed item. This mechanism ensure that logic is always executed with the exact same direct and indirect dependencies."
							/>
							<ToggleButton
								value="deploymentcallbacks"
								label="Sync"
								showTooltipIcon
								tooltip="Sync jobs that are triggered on every script deployment to sync the workspace with the Git repository configured in the the workspace settings"
							/>
						</ToggleButtonGroup>
					</Label>

					<Label label="Status">
						<ToggleButtonGroup bind:selected={success}>
							<ToggleButton value={undefined} label="All" />
							<ToggleButton value={'running'} label="Running" class="whitespace-nowrap" />
							<ToggleButton value={'success'} label="Success" class="whitespace-nowrap" />
							<ToggleButton value={'failure'} label="Failure" class="whitespace-nowrap" />
						</ToggleButtonGroup>
					</Label>
				{/if}

				<Label label="Show Skipped Flows">
					<div class="flex flex-row gap-1 items-center">
						<Toggle size="xs" bind:checked={isSkipped} />
						<Tooltip>Skipped flows are flows that did an early break</Tooltip>
					</div>
				</Label>

				<span class="text-xs leading-6">
					{`Filter by a json being a subset of the args/result. Try '\{"foo": "bar"\}'`}
				</span>
				<Label label="Filter by args">
					<JsonEditor on:change bind:error={argError} bind:code={copyArgFilter} />
				</Label>
				<Label label="Filter by result">
					<JsonEditor on:change bind:error={resultError} bind:code={copyResultFilter} />
				</Label>

				<div class="flex flex-row gap-2 justify-between">
					<Button
						size="xs"
						color="light"
						on:click={() => {
							argFilter = ''
							resultFilter = ''
							close(null)
						}}
					>
						Clear
					</Button>

					<Button
						size="xs"
						color="dark"
						on:click={() => {
							argFilter = copyArgFilter
							resultFilter = copyResultFilter
							close(null)
						}}
					>
						Set args/result filter
					</Button>
				</div>
			</div>
		</Section>
	</Popup>
</div>
