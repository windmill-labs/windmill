<script context="module">
	export function load() {
		return {
			stuff: { title: 'New Schedule' }
		}
	}
</script>

<script lang="ts">
	import { page } from '$app/stores'
	import { sendUserToast, formatCron } from '$lib/utils'
	import { ScriptService, Script, ScheduleService, type Flow, FlowService } from '$lib/gen'
	import Toggle from '$lib/components/Toggle.svelte'

	import Path from '$lib/components/Path.svelte'
	import { Button } from '$lib/components/common'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import { goto } from '$app/navigation'
	import { workspaceStore } from '$lib/stores'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import Required from '$lib/components/Required.svelte'
	import CronInput, { OFFSET } from '$lib/components/CronInput.svelte'
	import PageHeader from '$lib/components/PageHeader.svelte'

	let initialPath = $page.url.searchParams.get('edit') || ''
	let edit = initialPath === '' ? false : true
	let schedule: string = '0 0 12 * *'

	let script_path = $page.url.searchParams.get('path') || ''
	let initialScriptPath = script_path
	let is_flow = $page.url.searchParams.get('isFlow') == 'true'
	let itemKind: 'flow' | 'script' = is_flow ? 'flow' : 'script'

	$: is_flow = itemKind == 'flow'

	let runnable: Script | Flow | undefined
	let args: Record<string, any> = {}

	let isValid = true

	let path: string = ''
	let enabled: boolean = false
	let pathError = ''

	let validCRON = true
	$: allowSchedule = isValid && validCRON && script_path != ''

	$: script_path && loadScript(script_path)

	async function loadScript(p: string | undefined): Promise<void> {
		if (p) {
			if (is_flow) {
				runnable = await FlowService.getFlowByPath({ workspace: $workspaceStore!, path: p })
			} else {
				runnable = await ScriptService.getScriptByPath({ workspace: $workspaceStore!, path: p })
			}
		} else {
			runnable = undefined
		}
	}

	async function loadSchedule(): Promise<void> {
		try {
			const s = await ScheduleService.getSchedule({
				workspace: $workspaceStore!,
				path: initialPath
			})
			enabled = s.enabled
			schedule = s.schedule
			script_path = s.script_path ?? ''
			initialScriptPath = script_path
			args = s.args ?? {}
		} catch (err) {
			sendUserToast(`Could not load schedule: ${err}`, true)
		}
	}

	async function scheduleScript(): Promise<void> {
		if (edit) {
			await ScheduleService.updateSchedule({
				workspace: $workspaceStore!,
				path: initialPath,
				requestBody: {
					schedule: formatCron(schedule),
					script_path: script_path,
					is_flow: is_flow,
					args
				}
			})
			goto('/schedules')
		} else {
			await ScheduleService.createSchedule({
				workspace: $workspaceStore!,
				requestBody: {
					path,
					schedule: formatCron(schedule),
					offset: OFFSET,
					script_path,
					is_flow,
					args,
					enabled: true
				}
			})
			goto('/schedules')
		}
	}

	$: {
		if ($workspaceStore) {
			if (edit) {
				loadSchedule()
			}
		}
	}
</script>

<CenteredPage>
	<PageHeader title={edit ? 'Edit schedule ' + initialPath : 'New schedule'} />

	<div>
		{#if !edit}
			<h2 class="border-b pb-1 mt-8 mb-2">Save schedule as</h2>

			<Path
				bind:error={pathError}
				bind:path
				{initialPath}
				namePlaceholder={'my_schedule'}
				kind="schedule"
			/>
		{/if}

		<h2 class="border-b pb-1 mt-8 mb-2">Script</h2>
		<p class="text-xs mb-1 text-gray-600">
			Pick a script or flow to be triggered by the schedule<Required required={true} />
		</p>
		<ScriptPicker
			initialPath={initialScriptPath}
			kind={Script.kind.SCRIPT}
			allowFlow={true}
			bind:itemKind
			bind:scriptPath={script_path}
		/>
		<div class={edit ? '' : 'mt-2 md:mt-6'}>
			<h2 class="border-b pb-1 mt-8 mb-2">Arguments</h2>
			{#if runnable}
				{#if runnable?.schema && runnable.schema.properties && Object.keys(runnable.schema.properties).length > 0}
					<SchemaForm schema={runnable.schema} bind:isValid bind:args />
				{:else}
					<div class="text-xs texg-gray-700">
						This {is_flow ? 'flow' : 'script'} takes no argument
					</div>
				{/if}
			{:else}
				<div class="text-xs texg-gray-700 my-2">
					Pick a {is_flow ? 'flow' : 'script'} and fill its argument here
				</div>
			{/if}
		</div>
		<h2 class="border-b pb-1 mt-8 mb-2">
			<span class="mr-1">Schedule</span>
			<Tooltip>Schedules use CRON syntax. Seconds are mandatory.</Tooltip>
		</h2>
		<CronInput bind:schedule bind:validCRON />
		{#if edit}
			<h2 class="border-b pb-1 mt-8 mb-2">Enable</h2>
			<Toggle
				checked={enabled}
				on:change={async (e) => {
					await ScheduleService.setScheduleEnabled({
						path: initialPath,
						workspace: $workspaceStore ?? '',
						requestBody: { enabled: e.detail }
					})
					sendUserToast(`${e.detail ? 'enabled' : 'disabled'} schedule ${initialPath}`)
				}}
			/>
		{/if}
		<div class="flex flex-row-reverse mt-2 ">
			<div>
				<Button disabled={!allowSchedule || pathError != ''} on:click={scheduleScript}>
					{edit ? 'Save' : 'Schedule'}
				</Button>
			</div>
		</div>
	</div>
</CenteredPage>
