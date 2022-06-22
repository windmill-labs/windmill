<script lang="ts">
	import { page } from '$app/stores'
	import { sendUserToast, displayDate } from '../../utils'
	import { ScriptService, type Script, ScheduleService, type Flow, FlowService } from '../../gen'

	import PageHeader from '$lib/components/PageHeader.svelte'
	import Path from '$lib/components/Path.svelte'

	import Tooltip from '$lib/components/Tooltip.svelte'
	import { goto } from '$app/navigation'
	import { workspaceStore } from '../../stores'
	import CenteredPage from '$lib/components/CenteredPage.svelte'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import Required from '$lib/components/Required.svelte'

	let initialPath = $page.url.searchParams.get('edit') || ''
	let edit = initialPath === '' ? false : true
	let scheduleInput: string = '0 0 12 * *'
	let cronError = ''

	let script_path = $page.url.searchParams.get('path') || ''
	let is_flow = $page.url.searchParams.get('isFlow') == 'true'
	let itemKind: 'flow' | 'script' = is_flow ? 'flow' : 'script'

	$: is_flow = itemKind == 'flow'

	let runnable: Script | Flow | undefined
	let args: Record<string, any> = {}

	let validCRON = true
	let allowSchedule: boolean
	let isValid = true
	let preview: string[] = []

	let path: string = ''

	const offset = new Date().getTimezoneOffset()

	$: allowSchedule = isValid && validCRON && script_path != ''

	$: handleScheduleInput(scheduleInput)

	$: script_path && loadScript(script_path)

	async function loadScript(p: string | undefined): Promise<void> {
		if (p) {
			try {
				if (is_flow) {
					runnable = await FlowService.getFlowByPath({ workspace: $workspaceStore!, path: p })
				} else {
					runnable = await ScriptService.getScriptByPath({ workspace: $workspaceStore!, path: p })
				}
			} catch (err) {
				sendUserToast(`Could not load script: ${err}`, true)
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
			scheduleInput = s.schedule
			script_path = s.script_path ?? ''
			args = s.args ?? {}
		} catch (err) {
			sendUserToast(`Could not load schedule: ${err}`, true)
		}
	}

	async function scheduleScript(): Promise<void> {
		try {
			if (edit) {
				await ScheduleService.updateSchedule({
					workspace: $workspaceStore!,
					path: initialPath,
					requestBody: {
						schedule: formatInput(scheduleInput),
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
						schedule: formatInput(scheduleInput),
						offset,
						script_path,
						is_flow,
						args
					}
				})
				goto('/schedules')
			}
		} catch (err) {
			sendUserToast(`Could not schedule script: ${err}`, true)
		}
	}

	function formatInput(input: string): string {
		// Allow for cron expressions inputted by the user to omit month and year
		let splitted = input.split(' ')
		splitted = splitted.filter(String) //remove empty string elements
		if (6 - splitted.length > 0) {
			return splitted.concat(Array(6 - splitted.length).fill('*')).join(' ')
		} else {
			return input
		}
	}

	async function handleScheduleInput(input: string): Promise<void> {
		try {
			preview = await ScheduleService.previewSchedule({
				requestBody: { schedule: formatInput(input), offset }
			})
			cronError = ''
			validCRON = true
		} catch (err) {
			if (err.status == 400 && err.body.includes('cron')) {
				cronError = `Invalid cron expression`
				validCRON = false
			} else {
				sendUserToast(`Cannot preview: ${err}`, true)
				validCRON = false
			}
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
	<PageHeader title={edit ? 'Edit schedule ' + path : 'New schedule'} />

	<div>
		{#if !edit}
			<h2>Save schedule as</h2>

			<Path bind:path {initialPath} namePlaceholder={'my/schedule'}>
				<div slot="ownerToolkit" class="text-gray-700 text-2xs">
					Schedule permissions depend on their path. Select the group <span class="font-mono"
						>all</span
					>
					to share it, and <span class="font-mono">user</span> to keep it private.
					<a href="https://docs.windmill.dev/docs/reference/namespaces">docs</a>
				</div></Path
			>
		{/if}

		<h2 class="my-2 md:mt-6">Script</h2>
		<p class="text-xs text-gray-600">
			Pick a script or flow to be triggered by the schedule<Required required={true} />
		</p>
		<ScriptPicker allowFlow={true} bind:itemKind bind:scriptPath={script_path} />
		<div class="max-w-5xl {edit ? '' : 'mt-2 md:mt-6'}">
			<h2>Arguments</h2>
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
		<h2 class="mt-2 md:mt-6">
			Schedule<Tooltip class="mx-2">Schedules use CRON syntax. Milliseconds are mandatory.</Tooltip>
		</h2>
		<div class="text-purple-500 text-2xs grow">{cronError}</div>
		<div class="flex flex-row items-end max-w-5xl">
			<label class="text-xs min-w-max mr-2 self-center" for="cron-schedule">CRON expression</label>
			<input
				class="inline-block"
				type="text"
				id="cron-schedule"
				name="cron-schedule"
				bind:value={scheduleInput}
			/>
		</div>
		<div class="flex flex-row text-xs text-blue-500 gap-3 pl-28">
			<button
				on:click={() => {
					scheduleInput = '0 */15 * * *'
					cronError = ''
				}}>every 15 min</button
			>
			<button
				on:click={() => {
					scheduleInput = '0 0 * * * *'
					cronError = ''
				}}>every hour</button
			>
			<button
				on:click={() => {
					scheduleInput = '0 0 8 * * *'
					cronError = ''
				}}>once a day at 8AM</button
			>
		</div>
		<div class="flex flex-row-reverse mt-2 ">
			<div>
				<button
					type="submit"
					disabled={!allowSchedule}
					class="{allowSchedule ? 'default-button' : 'default-button-disabled'} w-min px-6"
					on:click={scheduleScript}
				>
					{edit ? 'Save' : 'Schedule'}
				</button>
			</div>
		</div>

		{#if preview && preview.length > 0}
			<div class="text-sm text-gray-700 border mt-6 p-2 rounded-md">
				<div class="flex flex-row justify-between">
					The next 10 runs of this script will be:
					<button
						on:click={() => {
							preview = []
						}}
					>
						<svg
							class="w-4 h-4"
							fill="none"
							stroke="currentColor"
							viewBox="0 0 24 24"
							xmlns="http://www.w3.org/2000/svg"
						>
							<path
								stroke-linecap="round"
								stroke-linejoin="round"
								stroke-width="2"
								d="M6 18L18 6M6 6l12 12"
							/>
						</svg>
					</button>
				</div>
				<ul class="list-disc mx-12">
					{#each preview as p}
						<li class="mx-2 text-gray-700 text-sm">{displayDate(p)}</li>
					{/each}
				</ul>
			</div>
		{/if}
	</div>
</CenteredPage>

<style>
	.selected:hover {
		@apply border border-gray-400 rounded-md border-opacity-50;
	}
</style>
