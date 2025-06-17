<script lang="ts">
	import { Button } from '../common'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import Tooltip from '../Tooltip.svelte'
	import { AlertCircle, CheckCircle2, Filter, Hourglass, PlayCircle, X } from 'lucide-svelte'
	import JsonEditor from '../JsonEditor.svelte'
	import Toggle from '../Toggle.svelte'
	import Label from '../Label.svelte'
	import Section from '../Section.svelte'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import { createEventDispatcher, untrack } from 'svelte'
	import ToggleButtonMore from '../common/toggleButton-v2/ToggleButtonMore.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import Select from '../select/Select.svelte'

	interface Props {
		// Filters
		path?: string | null
		label?: string | null
		concurrencyKey?: string | null
		worker?: string | null
		tag?: string | null
		success?: 'running' | 'waiting' | 'suspended' | 'queued' | 'success' | 'failure' | undefined
		isSkipped?: boolean | undefined
		argFilter: string
		argError: string
		resultFilter: string
		resultError: string
		jobKindsCat: string
		user?: string | null
		folder?: string | null
		mobile?: boolean
		schedulePath: string | undefined
		allowWildcards?: boolean
		// Autocomplete data
		paths?: string[]
		usernames?: string[]
		folders?: string[]
		allWorkspaces?: boolean
		filterBy?:
			| 'path'
			| 'user'
			| 'folder'
			| 'label'
			| 'concurrencyKey'
			| 'worker'
			| 'tag'
			| 'schedulePath'
	}

	let {
		path = $bindable(null),
		label = $bindable(null),
		concurrencyKey = $bindable(null),
		worker = $bindable(null),
		tag = $bindable(null),
		success = $bindable(undefined),
		isSkipped = $bindable(undefined),
		argFilter = $bindable(),
		argError = $bindable(),
		resultFilter = $bindable(),
		resultError = $bindable(),
		jobKindsCat = $bindable(),
		user = $bindable(null),
		folder = $bindable(null),
		mobile = false,
		schedulePath = $bindable(),
		allowWildcards = $bindable(false),
		paths = [],
		usernames = [],
		folders = [],
		allWorkspaces = $bindable(false),
		filterBy = $bindable('path')
	}: Props = $props()

	let copyArgFilter = $state(argFilter)
	let copyResultFilter = $state(resultFilter)

	const dispatch = createEventDispatcher()

	function autosetFilter() {
		if (path !== null && path !== '' && filterBy !== 'path') {
			filterBy = 'path'
		} else if (user !== null && user !== '' && filterBy !== 'user') {
			filterBy = 'user'
		} else if (folder !== null && folder !== '' && filterBy !== 'folder') {
			filterBy = 'folder'
		} else if (label !== null && label !== '' && filterBy !== 'label') {
			filterBy = 'label'
		} else if (concurrencyKey !== null && concurrencyKey !== '' && filterBy !== 'concurrencyKey') {
			filterBy = 'concurrencyKey'
		} else if (tag !== null && tag !== '' && filterBy !== 'tag') {
			filterBy = 'tag'
		} else if (schedulePath !== undefined && schedulePath !== '' && filterBy !== 'schedulePath') {
			filterBy = 'schedulePath'
		} else if (worker !== null && worker !== '' && filterBy !== 'worker') {
			filterBy = 'worker'
		}
	}

	let labelTimeout: NodeJS.Timeout | undefined = $state(undefined)
	let concurrencyKeyTimeout: NodeJS.Timeout | undefined = $state(undefined)
	let tagTimeout: NodeJS.Timeout | undefined = $state(undefined)
	let workerTimeout: NodeJS.Timeout | undefined = $state(undefined)

	let allWorkspacesValue = $state(allWorkspaces ? 'all' : 'admins')
	let displayedLabel = $derived(label)
	let displayedConcurrencyKey = $derived(concurrencyKey)
	let displayedTag = $derived(tag)
	let displayedSchedule = $derived(schedulePath)
	let displayedWorker = $derived(worker)
	$effect(() => {
		;(path || user || folder || label || worker || concurrencyKey || tag || schedulePath) &&
			untrack(() => autosetFilter())
	})
</script>

<div class="flex gap-4">
	{#if !mobile}
		<div class="flex gap-2">
			{#if $workspaceStore == 'admins'}
				<div class="relative">
					<span class="text-xs absolute -top-4">Workspaces</span>
					<ToggleButtonGroup
						bind:selected={allWorkspacesValue}
						on:selected={({ detail }) => (allWorkspaces = detail === 'all')}
					>
						{#snippet children({ item })}
							<ToggleButton value={'admins'} label="Admins" {item} />
							<ToggleButton value={'all'} label="All" {item} />
						{/snippet}
					</ToggleButtonGroup>
				</div>
			{/if}

			<div class="relative">
				<span class="text-xs absolute -top-4">Filter by</span>
				<ToggleButtonGroup
					bind:selected={filterBy}
					on:selected={(e) => {
						if (e.detail != filterBy) {
							path = null
							user = null
							folder = null
							label = null
							concurrencyKey = null
							tag = null
							schedulePath = undefined
						}
					}}
				>
					{#snippet children({ item })}
						<ToggleButton value="path" label="Path" {item} />
						<ToggleButton value="user" label="User" {item} />
						<ToggleButton value="folder" label="Folder" {item} />
						<ToggleButtonMore
							togglableItems={[
								{ label: 'Schedule path', value: 'schedulePath' },
								{ label: 'Concurrency key', value: 'concurrencyKey' },
								{ label: 'Label', value: 'label' },
								{ label: 'Tag', value: 'tag' },
								{ label: 'Worker', value: 'worker' }
							]}
							{item}
							bind:selected={filterBy}
						/>
					{/snippet}
				</ToggleButtonGroup>
			</div>

			{#if filterBy == 'user'}
				{#key user}
					<div class="relative">
						<span class="text-xs absolute -top-4">User</span>
						<Select
							items={usernames.map((p) => ({ label: p, value: p }))}
							bind:value={() => user ?? undefined, (v) => (user = v ?? null)}
							clearable
							onClear={() => ((user = null), dispatch('reset'))}
							inputClass="!h-[32px]"
							onCreateItem={(item) => (usernames.push(item), (user = item))}
							createText="Press enter to use this value"
						/>
					</div>
				{/key}
			{:else if filterBy == 'folder'}
				{#key folder}
					<div class="relative">
						<span class="text-xs absolute -top-4">Folder</span>

						<Select
							items={folders.map((p) => ({ label: p, value: p }))}
							bind:value={() => folder ?? undefined, (v) => (folder = v ?? null)}
							clearable
							onClear={() => ((folder = null), dispatch('reset'))}
							inputClass="!h-[32px]"
						/>
					</div>
				{/key}
			{:else if filterBy === 'path'}
				{#key path}
					<div class="relative">
						<span class="text-xs absolute -top-4">Path</span>
						<Select
							items={paths.map((p) => ({ label: p, value: p }))}
							bind:value={() => path ?? undefined, (v) => (path = v ?? null)}
							clearable
							onClear={() => ((path = null), dispatch('reset'))}
							inputClass="!h-[32px]"
							onCreateItem={(item) => (paths.push(item), (path = item))}
							createText="Press enter to use this value"
						/>
					</div>
				{/key}
			{:else if filterBy === 'label'}
				{#key label}
					<div class="relative">
						{#if label}
							<button
								class="absolute top-2 right-2 z-50"
								onclick={() => {
									label = null
									dispatch('reset')
								}}
							>
								<X size={14} />
							</button>
						{/if}

						<span class="text-xs absolute -top-4"
							>Label <Tooltip
								><a
									href="https://www.windmill.dev/docs/core_concepts/monitor_past_and_future_runs#jobs-labels"
									target="_blank">Job Labels</a
								> are string values in the array at the result field 'wm_labels' to easily filter them.</Tooltip
							></span
						>

						<!-- svelte-ignore a11y_autofocus -->
						<input
							autofocus
							type="text"
							class="!h-[32px] py-1 !text-xs !w-64"
							bind:value={displayedLabel}
							onkeydown={(e) => {
								if (labelTimeout) {
									clearTimeout(labelTimeout)
								}

								labelTimeout = setTimeout(() => {
									label = displayedLabel
								}, 1000)
							}}
						/>
						<div class="absolute top-10">
							<Toggle
								bind:checked={allowWildcards}
								size="xs"
								options={{ right: 'allow wildcards (*)' }}
							></Toggle>
						</div>
					</div>
				{/key}
			{:else if filterBy === 'concurrencyKey'}
				{#key concurrencyKey}
					<div class="relative">
						{#if concurrencyKey}
							<button
								class="absolute top-2 right-2 z-50"
								onclick={() => {
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

						<!-- svelte-ignore a11y_autofocus -->
						<input
							autofocus
							type="text"
							class="!h-[32px] py-1 !text-xs !w-64"
							bind:value={displayedConcurrencyKey}
							onkeydown={(e) => {
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
			{:else if filterBy === 'tag'}
				{#key tag}
					<div class="relative">
						{#if tag}
							<button
								class="absolute top-2 right-2 z-50"
								onclick={() => {
									tag = null
									dispatch('reset')
								}}
							>
								<X size={14} />
							</button>
						{/if}
						<span class="text-xs absolute -top-4"> Tag </span>

						<!-- svelte-ignore a11y_autofocus -->
						<input
							autofocus
							type="text"
							class="!h-[32px] py-1 !text-xs !w-64"
							bind:value={displayedTag}
							onkeydown={(e) => {
								if (tagTimeout) {
									clearTimeout(tagTimeout)
								}

								tagTimeout = setTimeout(() => {
									tag = displayedTag
								}, 1000)
							}}
						/>
						<div class="absolute top-10">
							<Toggle
								bind:checked={allowWildcards}
								size="xs"
								options={{ right: 'allow wildcards (*)' }}
							></Toggle>
						</div>
					</div>
				{/key}
			{:else if filterBy === 'schedulePath'}
				{#key tag}
					<div class="relative">
						{#if tag}
							<button
								class="absolute top-2 right-2 z-50"
								onclick={() => {
									schedulePath = undefined
									dispatch('reset')
								}}
							>
								<X size={14} />
							</button>
						{/if}
						<span class="text-xs absolute -top-4"> Schedule Path </span>

						<!-- svelte-ignore a11y_autofocus -->
						<input
							autofocus
							type="text"
							class="!h-[32px] py-1 !text-xs !w-64"
							bind:value={displayedSchedule}
							onkeydown={(e) => {
								if (tagTimeout) {
									clearTimeout(tagTimeout)
								}

								tagTimeout = setTimeout(() => {
									schedulePath = displayedSchedule
								}, 1000)
							}}
						/>
					</div>
				{/key}
			{:else if filterBy === 'worker'}
				{#key worker}
					<div class="relative">
						{#if worker}
							<button
								class="absolute top-2 right-2 z-50"
								onclick={() => {
									worker = null
									dispatch('reset')
								}}
							>
								<X size={14} />
							</button>
						{/if}
						<span class="text-xs absolute -top-4"> Worker </span>

						<!-- svelte-ignore a11y_autofocus -->
						<input
							autofocus
							type="text"
							class="!h-[32px] py-1 !text-xs !w-64"
							bind:value={displayedWorker}
							onkeydown={(e) => {
								if (workerTimeout) {
									clearTimeout(workerTimeout)
								}

								workerTimeout = setTimeout(() => {
									worker = displayedWorker
								}, 1000)
							}}
						/>
						<div class="absolute top-10">
							<Toggle
								bind:checked={allowWildcards}
								size="xs"
								options={{ right: 'allow wildcards (*)' }}
							></Toggle>
						</div>
					</div>
				{/key}
			{/if}
		</div>
		<div class="relative">
			<span class="text-xs absolute -top-4">Kind</span>
			<ToggleButtonGroup bind:selected={jobKindsCat}>
				{#snippet children({ item })}
					<ToggleButton value="all" label="All" {item} />
					<ToggleButton
						value="runs"
						label="Runs"
						showTooltipIcon
						tooltip="Runs are jobs that have no parent jobs (flows are jobs that are parent of the jobs they start), they have been triggered through the UI, a schedule or webhook"
						{item}
					/>
					<ToggleButton
						value="previews"
						label="Previews"
						showTooltipIcon
						tooltip="Previews are jobs that have been started in the editor as 'Tests'"
						{item}
					/>
					<ToggleButton
						value="dependencies"
						label="Deps"
						showTooltipIcon
						tooltip="Deploying a script, flow or an app launch a dependency job that create and then attach the lockfile to the deployed item. This mechanism ensure that logic is always executed with the exact same direct and indirect dependencies."
						{item}
					/>
					<ToggleButton
						value="deploymentcallbacks"
						label="Sync"
						showTooltipIcon
						tooltip="Sync jobs that are triggered on every script deployment to sync the workspace with the Git repository configured in the the workspace settings"
						{item}
					/>
				{/snippet}
			</ToggleButtonGroup>
		</div>
		<div class="relative">
			<span class="text-xs absolute -top-4">Status</span>
			<ToggleButtonGroup
				selected={success ?? 'all'}
				on:selected={({ detail }) => {
					success = detail === 'all' ? undefined : detail
					dispatch('successChange', success)
				}}
			>
				{#snippet children({ item })}
					<ToggleButton value={'all'} label="All" {item} />
					<ToggleButton
						value={'running'}
						tooltip="Running"
						class="whitespace-nowrap"
						icon={PlayCircle}
						iconProps={{ color: success === 'running' ? 'blue' : 'gray' }}
						{item}
					/>
					<ToggleButton
						value={'success'}
						tooltip="Success"
						class="whitespace-nowrap"
						icon={CheckCircle2}
						iconProps={{ color: success === 'success' ? 'green' : 'gray' }}
						{item}
					/>
					<ToggleButton
						value={'failure'}
						tooltip="Failure"
						class="whitespace-nowrap"
						icon={AlertCircle}
						iconProps={{ color: success === 'failure' ? 'red' : 'gray' }}
						{item}
					/>
					{#if success == 'waiting'}
						<ToggleButton
							value={'waiting'}
							tooltip="Waiting"
							class="whitespace-nowrap"
							icon={Hourglass}
							iconProps={{ color: 'blue' }}
							{item}
						/>
					{:else if success == 'suspended'}
						<ToggleButton
							value={'suspended'}
							tooltip="Suspended"
							class="whitespace-nowrap"
							icon={Hourglass}
							iconProps={{ color: 'blue' }}
							{item}
						/>
					{/if}
				{/snippet}
			</ToggleButtonGroup>
		</div>
	{/if}

	<Popover
		floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
		contentClasses="p-4"
		closeButton
	>
		{#snippet trigger()}
			<Button color="dark" size="xs" nonCaptureEvent={true} startIcon={{ icon: Filter }}>
				More filters
			</Button>
		{/snippet}

		{#snippet content()}
			<Section label="Filters">
				<div class="w-102 flex flex-col gap-4">
					{#if mobile || true}
						<Label label="Filter by">
							<ToggleButtonGroup
								bind:selected={filterBy}
								on:selected={(e) => {
									if (e.detail != filterBy) {
										path = null
										user = null
										folder = null
										label = null
										concurrencyKey = null
										tag = null
										schedulePath = undefined
									}
								}}
							>
								{#snippet children({ item })}
									<ToggleButton value="path" label="Path" {item} />
									<ToggleButton value="user" label="User" {item} />
									<ToggleButton value="folder" label="Folder" {item} />
									<ToggleButton value="schedulePath" label="Schedule" {item} />
									<ToggleButton value="concurrencyKey" label="Concurrency" {item} />
									<ToggleButton value="tag" label="Tag" {item} />
									<ToggleButton value="label" label="Label" {item} />
									<ToggleButton value="worker" label="Worker" {item} />
								{/snippet}
							</ToggleButtonGroup>
						</Label>

						{#if filterBy == 'user'}
							<Label label="User">
								<Select
									disablePortal
									items={usernames.map((p) => ({ label: p, value: p }))}
									bind:value={() => user ?? undefined, (v) => (user = v ?? null)}
									clearable
									onClear={() => ((user = null), dispatch('reset'))}
									inputClass="!h-[32px]"
								/>
							</Label>
						{:else if filterBy == 'folder'}
							<Label label="Folder">
								<Select
									disablePortal
									items={folders.map((p) => ({ label: p, value: p }))}
									bind:value={() => folder ?? undefined, (v) => (folder = v ?? null)}
									clearable
									onClear={() => ((folder = null), dispatch('reset'))}
									inputClass="!h-[32px]"
								/>
							</Label>
						{:else if filterBy === 'path'}
							<Label label="Path">
								<Select
									disablePortal
									items={paths.map((p) => ({ label: p, value: p }))}
									bind:value={() => path ?? undefined, (v) => (path = v ?? null)}
									clearable
									onClear={() => ((path = null), dispatch('reset'))}
									inputClass="!h-[32px]"
								/>
							</Label>
						{:else if filterBy === 'tag'}
							{#key tag}
								<Label label="Tag">
									<div class="relative w-full">
										{#if tag}
											<button
												class="absolute top-2 right-2 z-50"
												onclick={() => {
													tag = null
												}}
											>
												<X size={14} />
											</button>
										{/if}

										<!-- svelte-ignore a11y_autofocus -->
										<input
											autofocus
											type="text"
											class="!h-[32px] py-1 !text-xs !w-80"
											bind:value={displayedTag}
											onkeydown={(e) => {
												if (tagTimeout) {
													clearTimeout(tagTimeout)
												}

												tagTimeout = setTimeout(() => {
													tag = displayedTag
													console.log(tag)
												}, 1000)
											}}
										/>
									</div></Label
								>
							{/key}
						{:else if filterBy === 'label'}
							{#key label}
								<Label label="Label">
									<div class="relative w-full">
										{#if label}
											<button
												class="absolute top-2 right-2 z-50"
												onclick={() => {
													label = null
												}}
											>
												<X size={14} />
											</button>
										{/if}

										<!-- svelte-ignore a11y_autofocus -->
										<input
											autofocus
											type="text"
											class="!h-[32px] py-1 !text-xs !w-80"
											bind:value={displayedLabel}
											onkeydown={(e) => {
												if (labelTimeout) {
													clearTimeout(labelTimeout)
												}

												labelTimeout = setTimeout(() => {
													label = displayedLabel
												}, 1000)
											}}
										/>
									</div></Label
								>
							{/key}
						{:else if filterBy === 'concurrencyKey'}
							{#key concurrencyKey}
								<Label label="Concurrency key">
									<div class="relative w-full">
										{#if concurrencyKey}
											<button
												class="absolute top-2 right-2 z-50"
												onclick={() => {
													concurrencyKey = null
													// dispatch('reset')
												}}
											>
												<X size={14} />
											</button>
										{/if}

										<!-- svelte-ignore a11y_autofocus -->
										<input
											autofocus
											type="text"
											class="!h-[32px] py-1 !text-xs !w-80"
											bind:value={displayedConcurrencyKey}
											onkeydown={(e) => {
												if (concurrencyKeyTimeout) {
													clearTimeout(concurrencyKeyTimeout)
												}

												concurrencyKeyTimeout = setTimeout(() => {
													concurrencyKey = displayedConcurrencyKey
												}, 1000)
											}}
										/>
									</div>
								</Label>
							{/key}
						{:else if filterBy === 'worker'}
							{#key worker}
								<Label label="worker">
									<div class="relative w-full">
										{#if concurrencyKey}
											<button
												class="absolute top-2 right-2 z-50"
												onclick={() => {
													worker = null
													// dispatch('reset')
												}}
											>
												<X size={14} />
											</button>
										{/if}

										<!-- svelte-ignore a11y_autofocus -->
										<input
											autofocus
											type="text"
											class="!h-[32px] py-1 !text-xs !w-80"
											bind:value={displayedWorker}
											onkeydown={(e) => {
												if (workerTimeout) {
													clearTimeout(workerTimeout)
												}

												workerTimeout = setTimeout(() => {
													worker = displayedWorker
												}, 1000)
											}}
										/>
									</div>
								</Label>
							{/key}
						{/if}

						{#if filterBy === 'tag' || filterBy === 'label' || filterBy === 'worker'}
							<Toggle
								bind:checked={allowWildcards}
								size="xs"
								options={{ right: 'allow wildcards (*)' }}
							></Toggle>
						{/if}
						<Label label="Kind">
							<ToggleButtonGroup bind:selected={jobKindsCat}>
								{#snippet children({ item })}
									<ToggleButton value="all" label="All" {item} />
									<ToggleButton
										value="runs"
										label="Runs"
										showTooltipIcon
										tooltip="Runs are jobs that have no parent jobs (flows are jobs that are parent of the jobs they start), they have been triggered through the UI, a schedule or webhook"
										{item}
									/>
									<ToggleButton
										value="previews"
										label="Previews"
										showTooltipIcon
										tooltip="Previews are jobs that have been started in the editor as 'Tests'"
										{item}
									/>
									<ToggleButton
										value="dependencies"
										label="Deps"
										showTooltipIcon
										tooltip="Deploying a script, flow or an app launch a dependency job that create and then attach the lockfile to the deployed item. This mechanism ensure that logic is always executed with the exact same direct and indirect dependencies."
										{item}
									/>
									<ToggleButton
										value="deploymentcallbacks"
										label="Sync"
										showTooltipIcon
										tooltip="Sync jobs that are triggered on every script deployment to sync the workspace with the Git repository configured in the the workspace settings"
										{item}
									/>
								{/snippet}
							</ToggleButtonGroup>
						</Label>

						<Label label="Status">
							<ToggleButtonGroup
								selected={success ?? 'all'}
								on:selected={({ detail }) => (success = detail === 'all' ? undefined : detail)}
							>
								{#snippet children({ item })}
									<ToggleButton value={'all'} label="All" {item} />
									<ToggleButton
										value={'running'}
										label="Running"
										class="whitespace-nowrap"
										{item}
									/>
									<ToggleButton
										value={'success'}
										label="Success"
										class="whitespace-nowrap"
										{item}
									/>
									<ToggleButton
										value={'failure'}
										label="Failure"
										class="whitespace-nowrap"
										{item}
									/>
								{/snippet}
							</ToggleButtonGroup>
						</Label>
					{/if}

					<Label label="Show skipped flows">
						<div class="flex flex-row gap-1 items-center">
							<Toggle size="xs" bind:checked={isSkipped} />
							<Tooltip>Skipped flows are flows that did an early break</Tooltip>
						</div>
					</Label>

					<span class="text-xs leading-6">
						{`Filter by a json being a subset of the args/result. Try '\{"foo": "bar"\}'`}
					</span>
					<Label label="Filter by args">
						<JsonEditor bind:error={argError} bind:code={copyArgFilter} />
					</Label>
					<Label label="Filter by result">
						<JsonEditor bind:error={resultError} bind:code={copyResultFilter} />
					</Label>

					<div class="flex flex-row gap-2 justify-between">
						<Button
							size="xs"
							color="light"
							on:click={() => {
								argFilter = ''
								resultFilter = ''
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
							}}
						>
							Set args/result filter
						</Button>
					</div>
				</div>
			</Section>
		{/snippet}
	</Popover>
</div>
