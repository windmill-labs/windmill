<script lang="ts">
	import { ScheduleService } from '$lib/gen'

	import { workspaceStore } from '$lib/stores'
	import { pathIsEmpty } from '$lib/utils'
	import { faPlus } from '@fortawesome/free-solid-svg-icons'
	import Icon from 'svelte-awesome'
	import FlowInput from './flows/FlowInput.svelte'
	import FlowSettings from './flows/FlowSettings.svelte'
	import { addModule, flowStore, mode } from './flows/flowStore'
	import ModuleStep from './ModuleStep.svelte'
	import Tooltip from './Tooltip.svelte'

	export let pathError = ''
	export let initialPath: string = ''

	export let scheduleArgs: Record<string, any> = {}
	export let scheduleEnabled = false
	export let scheduleCron: string = '0 */5 * * *'

	async function loadSchedule() {
		try {
			const schedule = await ScheduleService.getSchedule({
				workspace: $workspaceStore ?? '',
				path: initialPath
			})
			scheduleEnabled = schedule.enabled!
			scheduleCron = schedule.schedule
			scheduleArgs = scheduleArgs
			console.log(schedule.enabled, schedule.schedule)
		} catch (e) {
			console.log(`no primary schedule found for ${initialPath}`)
		}
	}

	$: if ($workspaceStore && initialPath != '') {
		loadSchedule()
	}

	let open = 0
	let args: Record<string, any> = {}
</script>

{#if $flowStore}
	<div class="flex space-y-8 flex-col items-center line">
		<FlowSettings
			bind:pathError
			bind:initialPath
			bind:scheduleArgs
			bind:scheduleCron
			bind:scheduleEnabled
			bind:open
		/>
		<FlowInput />
		{#each $flowStore?.value.modules as mod, i}
			<ModuleStep bind:open bind:mod bind:args {i} mode={$mode} />
			{#if i == 0 && $mode == 'pull'}
				<div class="flex justify-center bg-white shadow p-2">
					Starting from here, the flow for loop over items from step 1's result above &nbsp;
					<Tooltip>
						This flow being in 'Pull' mode, the rest of the flow will for loop over the list of
						items returned by the trigger script right above. Retrieve the item value using
						`flow_input._value`
					</Tooltip>
				</div>
			{/if}
		{/each}
		<button
			disabled={pathIsEmpty($flowStore.path)}
			class="default-button h-10 w-10 shadow"
			on:click={() => {
				addModule()
				open = $flowStore?.value.modules.length - 1
			}}
		>
			<Icon class="text-white mb-1" data={faPlus} />
			Add step {pathIsEmpty($flowStore?.path) ? '(pick a name first!)' : ''}
		</button>
	</div>
{:else}
	<h3>Loading flow</h3>
{/if}

<style>
	.line {
		background-image: linear-gradient(#e5e7eb, #e5e7eb);
		background-size: 2px 100%;
		background-repeat: no-repeat;
		background-position: center center;
	}
</style>
