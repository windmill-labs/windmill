<script lang="ts">
	import { ScheduleService } from '$lib/gen'

	import { workspaceStore } from '$lib/stores'
	import { pathIsEmpty } from '$lib/utils'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'
	import { Button } from 'flowbite-svelte'
	import Icon from 'svelte-awesome'
	import FlowSettings from './flows/FlowSettings.svelte'
	import { addModule, flowStore } from './flows/flowStore'
	import FlowTimeline from './flows/FlowTimeline.svelte'
	import { flowToMode } from './flows/utils'

	export let pathError = ''
	export let initialPath: string = ''

	export let scheduleArgs: Record<string, any> = {}
	export let scheduleEnabled = false
	export let scheduleCron: string = '0 */5 * * *'
	export let previewArgs: Record<string, any> = {}

	let scheduleLoaded = false

	async function loadSchedule() {
		if (!scheduleLoaded) {
			scheduleLoaded = true

			const existsSchedule = await ScheduleService.existsSchedule({
				workspace: $workspaceStore ?? '',
				path: initialPath
			})
			if (existsSchedule) {
				const schedule = await ScheduleService.getSchedule({
					workspace: $workspaceStore ?? '',
					path: initialPath
				})

				scheduleEnabled = schedule.enabled!
				scheduleCron = schedule.schedule
				scheduleArgs = schedule.args ?? {}
				previewArgs = JSON.parse(JSON.stringify(scheduleArgs))
			}
		}
	}

	$: if ($flowStore && $workspaceStore && initialPath != '') {
		loadSchedule()
	}

	let open = 0
</script>

{#if $flowStore}
	<div class="flex space-y-8 flex-col items-center">
		<FlowTimeline
			bind:args={previewArgs}
			bind:open
			modules={flowToMode($flowStore, 'pull').value.modules}
		>
			<div slot="settings">
				<FlowSettings
					bind:pathError
					bind:initialPath
					bind:scheduleArgs
					{previewArgs}
					bind:scheduleCron
					bind:scheduleEnabled
					bind:open
				/>
			</div>
		</FlowTimeline>

		<Button
			disabled={pathIsEmpty($flowStore.path)}
			class="blue-button"
			on:click={() => {
				addModule()
				open = $flowStore?.value.modules.length - 1
			}}
		>
			<Icon class="text-white mr-2" data={faPlus} />
			Add step {pathIsEmpty($flowStore?.path) ? '(pick a name first!)' : ''}
		</Button>
	</div>
{:else}
	<h3>Loading flow</h3>
{/if}
