<script lang="ts">
	import { Badge, Button } from './common'
	import { sendUserToast } from '$lib/toast'
	import { base } from '$lib/base'
	import Toggle from './Toggle.svelte'
	import { ListOrdered, PenBox } from 'lucide-svelte'
	import JobArgs from './JobArgs.svelte'
	import Tooltip from './Tooltip.svelte'
	import ScheduleEditor from '$lib/components/triggers/schedules/ScheduleEditor.svelte'
	import { type Schedule } from '$lib/gen'

	export let schedule: any
	export let can_write: boolean
	export let path: string
	export let isFlow: boolean
	export let scheduleEditor: ScheduleEditor
	export let setScheduleEnabled: (path: string, enabled: boolean) => void

	$: schedule = typeof schedule === 'boolean' ? undefined : (schedule as Schedule | undefined)
</script>

<div class="flex flex-col gap-2 grow w-full">
	<div class="grid grid-cols-3 w-full">
		<div class="flex justify-start">
			<Badge color="indigo" small>
				Primary&nbsp;
				<Tooltip light>
					Share the same path as the script or flow it is attached to and its path get renamed
					whenever the source path is renamed
				</Tooltip>
			</Badge>
		</div>
		<div class="flex justify-center">
			<div class="flex flex-row gap-2 items-center">
				<input
					size="9"
					class="!text-xs !h-6 !text-primary"
					type="text"
					id="cron-schedule"
					name="cron-schedule"
					placeholder="*/30 * * * *"
					value={schedule?.schedule ?? ''}
					disabled={true}
				/>
				<Toggle
					checked={schedule?.enabled ?? false}
					on:change={(e) => {
						if (can_write) {
							setScheduleEnabled(path, e.detail)
						} else {
							sendUserToast('not enough permission', true)
						}
					}}
					options={{
						right: 'On'
					}}
					size="xs"
					textClass="text-primary font-normal text-xs"
				/>
			</div>
		</div>
		<div class="flex justify-end">
			<div class="flex flex-row gap-2 items-center">
				<Button size={'xs'} variant="border" color="light" href={`${base}/runs/${path}`}>
					<span>Runs</span>
					<ListOrdered size={14} />
				</Button>
				<Button
					size="xs"
					color="dark"
					on:click={() => scheduleEditor?.openEdit(path ?? '', isFlow)}
				>
					<PenBox size={14} />
				</Button>
			</div>
		</div>
	</div>

	{#if Object.keys(schedule?.args ?? {}).length > 0}
		<div class="">
			<JobArgs args={schedule?.args ?? {}} />
		</div>
	{:else}
		<div class="text-xs text-tertiary"> No arguments </div>
	{/if}
</div>
