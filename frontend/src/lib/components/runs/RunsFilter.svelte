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

	// Filters
	export let path: string | null = null
	export let success: 'running' | 'success' | 'failure' | undefined = undefined
	export let isSkipped: boolean | undefined = undefined
	export let argFilter: string
	export let argError: string
	export let resultFilter: string
	export let resultError: string
	export let jobKindsCat: string
	export let user: string | null = null
	export let folder: string | null = null
	export let hideSchedules: boolean | undefined = undefined
	export let mobile: boolean = false

	// Autocomplete data
	export let paths: string[] = []
	export let usernames: string[] = []
	export let folders: string[] = []

	let copyArgFilter = argFilter
	let copyResultFilter = resultFilter

	export let filterBy: 'path' | 'user' | 'folder' = 'path'

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
		}
	}
</script>

<div class="flex gap-4">
	{#if !mobile}
		<div class="flex gap-2">
			<div class="relative">
				<span class="text-xs absolute -top-4">Filter by</span>
				<ToggleButtonGroup
					bind:selected={filterBy}
					on:selected={() => {
						if (!manuallySet) {
							path = null
							user = null
							folder = null
						} else {
							manuallySet = false
						}
					}}
				>
					<ToggleButton value="path" label="Path" />
					<ToggleButton value="user" label="User" />
					<ToggleButton value="folder" label="Folder" />
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
								}}
							>
								<X size={14} />
							</button>
						{:else}
							<ChevronDown class="absolute top-2 right-2" size={14} />
						{/if}

						<span class="text-xs absolute -top-4">User</span>
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
				{/key}
			{:else if filterBy == 'folder'}
				{#key folder}
					<div class="relative">
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

						<span class="text-xs absolute -top-4">Folder</span>

						<AutoComplete
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
								}}
							>
								<X size={14} />
							</button>
						{:else}
							<ChevronDown class="absolute top-2 right-2" size={14} />
						{/if}

						<span class="text-xs absolute -top-4">Path</span>

						<AutoComplete
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

				<Label label="Hide future scheduled jobs from schedules">
					<Toggle size="xs" bind:checked={hideSchedules} />
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
						Set
					</Button>
				</div>
			</div>
		</Section>
	</Popup>
</div>
