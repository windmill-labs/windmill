<script module lang="ts">
	export const runsFiltersSchema = z.object({
		path: z.string().nullable().default(null),
		worker: z.string().nullable().default(null),
		user: z.string().nullable().default(null),
		folder: z.string().nullable().default(null),
		label: z.string().nullable().default(null),
		concurrency_key: z.string().nullable().default(null),
		tag: z.string().nullable().default(null),
		allow_wildcards: z.boolean().default(false),
		show_future_jobs: z.boolean().default(true),
		success: z
			.enum(['running', 'suspended', 'waiting', 'success', 'failure'])
			.nullable()
			.default(null),
		show_skipped: z.boolean().default(false),
		show_schedules: z.boolean().default(true),
		min_ts: z.string().nullable().default(null),
		max_ts: z.string().nullable().default(null),
		schedule_path: z.string().nullable().default(null),
		job_kinds: z.string().default('runs'),
		all_workspaces: z.boolean().default(false),
		arg: z.string().default(''),
		result: z.string().default(''),
		job_trigger_kind: z
			.string()
			.transform((s) => s as JobTriggerKind)
			.nullable()
			.default(null),
		per_page: z.number().default(1000)
	})
	export type RunsFilters = z.infer<typeof runsFiltersSchema>
</script>

<script lang="ts">
	import { Button } from '../common'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import { CircleAlert, CircleCheck, Hourglass, ListFilterPlus, CirclePlay, X } from 'lucide-svelte'
	import JsonEditor from '../JsonEditor.svelte'
	import Toggle from '../Toggle.svelte'
	import Label from '../Label.svelte'
	import Section from '../Section.svelte'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import { createEventDispatcher } from 'svelte'
	import ToggleButtonMore from '../common/toggleButton-v2/ToggleButtonMore.svelte'
	import Popover from '$lib/components/meltComponents/Popover.svelte'
	import Select from '../select/Select.svelte'
	import { safeSelectItems } from '../select/utils.svelte'
	import RunOption from './RunOption.svelte'
	import DropdownSelect from '../DropdownSelect.svelte'
	import TooltipV2 from '$lib/components/meltComponents/Tooltip.svelte'
	import TextInput from '../text_input/TextInput.svelte'
	import { jobTriggerKinds, triggerDisplayNamesMap } from '../triggers/utils'
	import type { JobTriggerKind } from '$lib/gen'
	import { watch } from 'runed'
	import z from 'zod'

	interface Props {
		// Filters
		path?: string | null
		label?: string | null
		concurrencyKey?: string | null
		worker?: string | null
		tag?: string | null
		success?: 'running' | 'waiting' | 'suspended' | 'queued' | 'success' | 'failure' | null
		showSkipped?: boolean | undefined
		argFilter: string
		argError: string
		resultFilter: string
		jobTriggerKind: JobTriggerKind | null
		resultError: string
		jobKindsCat: string
		user?: string | null
		folder?: string | null
		mobile?: boolean
		schedulePath: string | null
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
		small?: boolean
		calendarSmall?: boolean
	}

	let {
		path = $bindable(),
		label = $bindable(),
		concurrencyKey = $bindable(),
		worker = $bindable(),
		tag = $bindable(),
		success = $bindable(),
		showSkipped = $bindable(),
		argFilter = $bindable(),
		argError = $bindable(),
		resultFilter = $bindable(),
		jobTriggerKind = $bindable(),
		resultError = $bindable(),
		jobKindsCat = $bindable(),
		user = $bindable(),
		folder = $bindable(),
		mobile = false,
		schedulePath = $bindable(),
		allowWildcards = $bindable(),
		paths = [],
		usernames = [],
		folders = [],
		allWorkspaces = $bindable(),
		filterBy = $bindable(),
		small = false,
		calendarSmall = false
	}: Props = $props()

	let copyArgFilter = $state(argFilter)
	let copyResultFilter = $state(resultFilter)

	const dispatch = createEventDispatcher()

	function autosetFilter() {
		if (path && filterBy !== 'path') {
			filterBy = 'path'
		} else if (user && filterBy !== 'user') {
			filterBy = 'user'
		} else if (folder && filterBy !== 'folder') {
			filterBy = 'folder'
		} else if (label && filterBy !== 'label') {
			filterBy = 'label'
		} else if (concurrencyKey && filterBy !== 'concurrencyKey') {
			filterBy = 'concurrencyKey'
		} else if (tag && filterBy !== 'tag') {
			filterBy = 'tag'
		} else if (schedulePath && filterBy !== 'schedulePath') {
			filterBy = 'schedulePath'
		} else if (worker && filterBy !== 'worker') {
			filterBy = 'worker'
		}
	}

	let labelTimeout: ReturnType<typeof setInterval> | undefined = $state()
	let concurrencyKeyTimeout: ReturnType<typeof setInterval> | undefined = $state(undefined)
	let tagTimeout: ReturnType<typeof setInterval> | undefined = $state(undefined)
	let workerTimeout: ReturnType<typeof setInterval> | undefined = $state(undefined)

	let allWorkspacesValue = $state(allWorkspaces ? 'all' : 'admins')
	let displayedLabel = $derived(label)
	let displayedConcurrencyKey = $derived(concurrencyKey)
	let displayedTag = $derived(tag)
	let displayedSchedule = $derived(schedulePath ?? undefined)
	let displayedWorker = $derived(worker)

	watch([() => [path, user, folder, label, worker, concurrencyKey, tag, schedulePath]], () => {
		autosetFilter()
	})

	watch(
		() => copyArgFilter,
		() => {
			if (!copyArgFilter) argFilter = ''
			else if (copyArgFilter !== argFilter && !argError) argFilter = copyArgFilter
		}
	)
	watch(
		() => copyResultFilter,
		() => {
			if (!copyResultFilter) resultFilter = ''
			else if (copyResultFilter !== resultFilter && !resultError) resultFilter = copyResultFilter
		}
	)

	function resetFilter() {
		path = null
		user = null
		folder = null
		label = null
		concurrencyKey = null
		tag = null
		schedulePath = null
		worker = null
	}
</script>

{#snippet runsTooltip()}
	<TooltipV2 placement="right">
		{#snippet text()}
			'Runs are jobs that have no parent jobs (flows are jobs that are parent of the jobs they
			start), they have been triggered through the UI, a schedule or webhook'
		{/snippet}
	</TooltipV2>
{/snippet}
{#snippet previewsTooltip()}
	<TooltipV2 placement="right">
		{#snippet text()}
			'Previews are jobs that have been started in the editor as "Tests"'
		{/snippet}
	</TooltipV2>
{/snippet}
{#snippet dependenciesTooltip()}
	<TooltipV2 placement="right">
		{#snippet text()}
			'Deploying a script, flow or an app launch a dependency job that creates and then attaches the
			lockfile to the deployed item. This mechanism ensures that logic is always executed with the
			exact same direct and indirect dependencies.'
		{/snippet}
	</TooltipV2>
{/snippet}
{#snippet syncTooltip()}
	<TooltipV2 placement="right">
		{#snippet text()}
			'Sync jobs that are triggered on every script deployment to sync the workspace with the Git
			repository configured in the workspace settings'
		{/snippet}
	</TooltipV2>
{/snippet}

{#if !mobile}
	{#if $workspaceStore == 'admins'}
		<RunOption label="Workspaces" for="workspaces">
			<ToggleButtonGroup
				bind:selected={allWorkspacesValue}
				on:selected={({ detail }) => (allWorkspaces = detail === 'all')}
			>
				{#snippet children({ item })}
					<ToggleButton value={'admins'} label="Admins" {item} />
					<ToggleButton value={'all'} label="All" {item} />
				{/snippet}
			</ToggleButtonGroup>
		</RunOption>
	{/if}
	<!-- Filter by -->
	<div class="flex flex-row gap-2">
		<RunOption label="Filter by" for="filter-by">
			<ToggleButtonGroup
				bind:selected={filterBy}
				on:selected={(e) => {
					if (e.detail != filterBy) {
						resetFilter()
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
						bind:selected={
							() => filterBy,
							(v) => {
								resetFilter()
								filterBy = v
							}
						}
					/>
				{/snippet}
			</ToggleButtonGroup>
		</RunOption>

		{#if filterBy == 'user'}
			{#key user}
				<RunOption label="User" for="user">
					<Select
						items={safeSelectItems(usernames)}
						bind:value={() => user ?? undefined, (v) => (user = v ?? null)}
						clearable
						onClear={() => ((user = null), dispatch('reset'))}
						onCreateItem={(item) => (usernames.push(item), (user = item))}
						createText="Press enter to use this value"
						id="user"
					/>
				</RunOption>
			{/key}
		{:else if filterBy == 'folder'}
			<RunOption label="Folder" for="folder">
				{#key folder}
					<Select
						items={safeSelectItems(folders)}
						bind:value={() => folder ?? undefined, (v) => (folder = v ?? null)}
						clearable
						onClear={() => ((folder = null), dispatch('reset'))}
						id="folder"
					/>
				{/key}
			</RunOption>
		{:else if filterBy === 'path'}
			<RunOption label="Path" for="path">
				{#key path}
					<Select
						items={safeSelectItems(paths)}
						bind:value={() => path ?? undefined, (v) => (path = v ?? null)}
						clearable
						onClear={() => ((path = null), dispatch('reset'))}
						onCreateItem={(item) => (paths.push(item), (path = item))}
						createText="Press enter to use this value"
						id="path"
					/>
				{/key}
			</RunOption>
		{:else if filterBy === 'label'}
			<RunOption label="Label" for="label">
				{#snippet tooltip()}
					<a
						href="https://www.windmill.dev/docs/core_concepts/monitor_past_and_future_runs#jobs-labels"
						target="_blank">Job Labels</a
					> are string values in the array at the result field 'wm_labels' to easily filter them.
				{/snippet}
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

						<!-- svelte-ignore a11y_autofocus -->
						<TextInput
							inputProps={{
								autofocus: true,
								type: 'text',
								onkeydown: (e) => {
									if (labelTimeout) {
										clearTimeout(labelTimeout)
									}

									labelTimeout = setTimeout(() => {
										label = displayedLabel
									}, 1000)
								}
							}}
							bind:value={() => displayedLabel ?? undefined, (v) => (displayedLabel = v ?? null)}
						/>
						<div class="absolute -top-4 right-0">
							<Toggle
								bind:checked={allowWildcards}
								size="2xs"
								options={{ right: '(*)', title: 'allow wildcards (*)' }}
							></Toggle>
						</div>
					</div>
				{/key}
			</RunOption>
		{:else if filterBy === 'concurrencyKey'}
			<RunOption label="Concurrency Key" for="concurrencyKey">
				{#snippet tooltip()}
					For concurrency limited jobs, the concurrency key defines a group of jobs that share the
					same limits.
					{#if !$enterpriseLicense}
						Concurrency limits are an EE feature.
					{/if}
				{/snippet}
				{#key concurrencyKey}
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

					<!-- svelte-ignore a11y_autofocus -->
					<TextInput
						inputProps={{
							autofocus: true,
							type: 'text',
							onkeydown: (e) => {
								if (concurrencyKeyTimeout) {
									clearTimeout(concurrencyKeyTimeout)
								}

								concurrencyKeyTimeout = setTimeout(() => {
									concurrencyKey = displayedConcurrencyKey
								}, 1000)
							}
						}}
						bind:value={
							() => displayedConcurrencyKey ?? undefined,
							(v) => (displayedConcurrencyKey = v ?? null)
						}
					/>
				{/key}
			</RunOption>
		{:else if filterBy === 'tag'}
			<RunOption label="Tag" for="tag">
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

						<!-- svelte-ignore a11y_autofocus -->
						<TextInput
							inputProps={{
								autofocus: true,
								type: 'text',
								id: 'tag',
								onkeydown: (e) => {
									if (tagTimeout) {
										clearTimeout(tagTimeout)
									}

									tagTimeout = setTimeout(() => {
										tag = displayedTag
									}, 1000)
								}
							}}
							bind:value={() => displayedTag ?? undefined, (v) => (displayedTag = v ?? null)}
						/>
						<div class="absolute -top-4 right-0">
							<Toggle
								bind:checked={allowWildcards}
								size="2xs"
								options={{ right: 'wildcards (*)', title: 'allow wildcards (*)' }}
							></Toggle>
						</div>
					</div>
				{/key}
			</RunOption>
		{:else if filterBy === 'schedulePath'}
			<RunOption label="Schedule Path" for="schedulePath">
				{#key tag}
					<div class="relative">
						{#if tag}
							<button
								class="absolute top-2 right-2 z-50"
								onclick={() => {
									schedulePath = null
									dispatch('reset')
								}}
							>
								<X size={14} />
							</button>
						{/if}

						<!-- svelte-ignore a11y_autofocus -->
						<TextInput
							inputProps={{
								autofocus: true,
								type: 'text',
								onkeydown: (e) => {
									if (tagTimeout) {
										clearTimeout(tagTimeout)
									}

									tagTimeout = setTimeout(() => {
										schedulePath = displayedSchedule ?? null
									}, 1000)
								},
								id: 'schedulePath'
							}}
							bind:value={displayedSchedule}
						/>
					</div>
				{/key}
			</RunOption>
		{:else if filterBy === 'worker'}
			<RunOption label="Worker" for="worker">
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

						<!-- svelte-ignore a11y_autofocus -->
						<TextInput
							inputProps={{
								autofocus: true,
								onkeydown: (e) => {
									if (workerTimeout) {
										clearTimeout(workerTimeout)
									}

									workerTimeout = setTimeout(() => {
										worker = displayedWorker
									}, 1000)
								},
								id: 'worker'
							}}
							bind:value={() => displayedWorker ?? undefined, (v) => (displayedWorker = v ?? null)}
						/>
						<div class="absolute -top-4 right-0">
							<Toggle
								bind:checked={allowWildcards}
								size="2xs"
								options={{ right: 'wildcards (*)', title: 'allow wildcards (*)' }}
							></Toggle>
						</div>
					</div>
				{/key}
			</RunOption>
		{/if}
	</div>

	<!-- Kind -->
	<RunOption label="Kind" for="kind">
		{#if small && !calendarSmall}
			<DropdownSelect
				btnClasses="min-w-24 h-9 bg-surface-secondary font-normal"
				items={[
					{
						displayName: 'All',
						action: () => {
							jobKindsCat = 'all'
						},
						id: 'all'
					},
					{
						displayName: 'Runs',
						action: () => {
							jobKindsCat = 'runs'
						},
						id: 'runs',
						extra: runsTooltip
					},
					{
						displayName: 'Previews',
						action: () => {
							jobKindsCat = 'previews'
						},
						id: 'previews',
						extra: previewsTooltip
					},
					{
						displayName: 'Deps',
						action: () => {
							jobKindsCat = 'dependencies'
						},
						id: 'dependencies',
						extra: dependenciesTooltip
					},
					{
						displayName: 'Sync',
						action: () => {
							jobKindsCat = 'deploymentcallbacks'
						},
						id: 'deploymentcallbacks',
						extra: syncTooltip
					}
				]}
				selected={jobKindsCat}
			/>
		{:else}
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
						value="dependencies"
						label="Deps"
						showTooltipIcon
						tooltip="Deploying a script, flow or an app launch a dependency job that create and then attach the lockfile to the deployed item. This mechanism ensure that logic is always executed with the exact same direct and indirect dependencies."
						{item}
					/>
					<ToggleButtonMore
						togglableItems={[
							{
								label: 'Previews',
								value: 'previews',
								tooltip: "Previews are jobs that have been started in the editor as 'Tests'"
							},
							{
								label: 'Sync',
								value: 'deploymentcallbacks',
								tooltip:
									'Sync jobs that are triggered on every script deployment to sync the workspace with the Git repository configured in the the workspace settings'
							}
						]}
						{item}
						bind:selected={
							() => jobKindsCat,
							(v) => {
								resetFilter()
								jobKindsCat = v
							}
						}
					/>
				{/snippet}
			</ToggleButtonGroup>
		{/if}
	</RunOption>
	<!-- Status -->
	<RunOption label="Status" for="status">
		<ToggleButtonGroup
			selected={success ?? 'all'}
			on:selected={({ detail }) => {
				success = detail === 'all' ? undefined : detail
				dispatch('successChange', success)
			}}
			id="status"
		>
			{#snippet children({ item })}
				<ToggleButton value={'all'} label="All" {item} />
				<ToggleButton
					value={'running'}
					tooltip="Running"
					class="whitespace-nowrap"
					icon={CirclePlay}
					iconProps={{
						class:
							'group-data-[state=on]:text-yellow-600 dark:group-data-[state=on]:text-yellow-400'
					}}
					{item}
				/>
				<ToggleButton
					value={'success'}
					tooltip="Success"
					class="whitespace-nowrap"
					icon={CircleCheck}
					iconProps={{
						class: 'group-data-[state=on]:text-green-500 dark:group-data-[state=on]:text-green-300'
					}}
					{item}
				/>
				<ToggleButton
					value={'failure'}
					tooltip="Failure"
					class="whitespace-nowrap"
					icon={CircleAlert}
					iconProps={{
						class: 'group-data-[state=on]:text-red-500 dark:group-data-[state=on]:text-red-300'
					}}
					{item}
				/>
				{#if success == 'waiting'}
					<ToggleButton
						value={'waiting'}
						tooltip="Waiting"
						class="whitespace-nowrap"
						icon={Hourglass}
						selectedColor="blue"
						{item}
					/>
				{:else if success == 'suspended'}
					<ToggleButton
						value={'suspended'}
						tooltip="Suspended"
						class="whitespace-nowrap"
						icon={Hourglass}
						selectedColor="purple"
						{item}
					/>
				{/if}
			{/snippet}
		</ToggleButtonGroup>
	</RunOption>
{/if}

<RunOption label="_" for="more-filters" noLabel>
	<Popover
		floatingConfig={{ strategy: 'absolute', placement: 'bottom-end' }}
		contentClasses="p-4"
		closeButton
		usePointerDownOutside
	>
		{#snippet trigger()}
			<Button
				variant="default"
				unifiedSize="md"
				nonCaptureEvent={true}
				startIcon={{ icon: ListFilterPlus }}
				iconOnly
			></Button>
		{/snippet}

		{#snippet content()}
			<Section label="Filters">
				<div class="w-102 flex flex-col gap-6">
					{#if mobile}
						{#if $workspaceStore == 'admins'}
							<Label label="Workspaces">
								<ToggleButtonGroup
									bind:selected={allWorkspacesValue}
									on:selected={({ detail }) => (allWorkspaces = detail === 'all')}
								>
									{#snippet children({ item })}
										<ToggleButton value={'admins'} label="Admins" {item} />
										<ToggleButton value={'all'} label="All" {item} />
									{/snippet}
								</ToggleButtonGroup>
							</Label>
						{/if}
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
										schedulePath = null
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
									items={safeSelectItems(usernames)}
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
									items={safeSelectItems(folders)}
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
									items={safeSelectItems(paths)}
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
								size="2xs"
								options={{ right: 'wildcards (*)', title: 'allow wildcards (*)' }}
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
						<span class="text-2xs text-secondary">
							Skipped flows are flows that did an early break
						</span>
						<div class="flex flex-row gap-1 items-center">
							<Toggle size="sm" bind:checked={showSkipped} />
						</div>
					</Label>

					<Label label="Filter by trigger kind">
						<span class="text-2xs text-secondary">
							{`Filter by what kind of trigger started the run.`}
						</span>
						<Select
							items={jobTriggerKinds.map((value) => ({
								label: triggerDisplayNamesMap[value],
								value
							}))}
							bind:value={() => jobTriggerKind ?? undefined, (v) => (jobTriggerKind = v ?? null)}
							clearable
						/>
					</Label>

					<Label label="Filter by args">
						<span class="text-2xs text-secondary">
							{`Filter by a json being a subset of the args/result. Try '\{"foo": "bar"\}'`}
						</span>
						<JsonEditor bind:error={argError} bind:code={copyArgFilter} />
					</Label>
					<Label label="Filter by result">
						<span class="text-2xs text-secondary">
							{`Filter by a json being a subset of the args/result. Try '\{"foo": "bar"\}'`}
						</span>
						<JsonEditor bind:error={resultError} bind:code={copyResultFilter} />
					</Label>
				</div>
			</Section>
		{/snippet}
	</Popover>
</RunOption>
