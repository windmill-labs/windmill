<script lang="ts">
	import { Badge, Button } from './common'
	import { sendUserToast } from '$lib/toast'
	import { base } from '$lib/base'
	import Toggle from './Toggle.svelte'
	import { ListOrdered, PenBox } from 'lucide-svelte'
	import JobArgs from './JobArgs.svelte'
	import Tooltip from './Tooltip.svelte'
	import ScheduleEditor from './ScheduleEditor.svelte'
	import { type Schedule } from '$lib/gen'

	export let light = false
	export let schedule: Schedule
	export let can_write: boolean
	export let path: string
	export let isFlow: boolean
	export let scheduleEditor: ScheduleEditor
	export let setScheduleEnabled: (path: string, enabled: boolean) => void
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
					class={light ? '!text-xs !h-6 !text-primary' : 'text-sm bg-red-400'}
					type="text"
					id="cron-schedule"
					name="cron-schedule"
					placeholder="*/30 * * * *"
					value={schedule.schedule}
					disabled={true}
				/>
				<Toggle
					checked={schedule.enabled}
					on:change={(e) => {
						if (can_write) {
							setScheduleEnabled(path, e.detail)
						} else {
							sendUserToast('not enough permission', true)
						}
					}}
					options={light
						? undefined
						: {
								right: 'On'
						  }}
					size="xs"
					textClass="text-primary font-normal text-xs"
				/>
			</div>
		</div>
		<div class="flex justify-end">
			<div class="flex flex-row gap-2 items-center">
				{#if !light}
					<Button
						size={light ? 'xs2' : 'xs'}
						variant="border"
						color="light"
						href={`${base}/runs/${path}`}
						spacingSize={light ? 'xs3' : undefined}
					>
						<span class={light ? 'text-2xs px-1 font-normal' : ''}>Runs</span>
						<ListOrdered size={light ? 12 : 14} />
					</Button>
				{/if}
				<Button
					size={light ? 'xs2' : 'xs'}
					color="dark"
					on:click={() => scheduleEditor?.openEdit(path ?? '', isFlow)}
				>
					<PenBox size={light ? 12 : 14} />
				</Button>
			</div>
		</div>
	</div>

	{#if !light}
		{#if Object.keys(schedule?.args ?? {}).length > 0}
			<div class="">
				<JobArgs args={schedule.args ?? {}} />
			</div>
		{:else}
			<div class="text-xs text-tertiary"> No arguments </div>
		{/if}
	{/if}
</div>
