<script lang="ts">
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import Label from '$lib/components/Label.svelte'
	import { base } from '$lib/base'

	$: console.log('dbg base', base)
</script>

<div class="flex flex-col w-full gap-4 text-sm">
	<Alert
		title="Scheduled Poll"
		size="xs"
		documentationLink="https://www.windmill.dev/docs/flows/flow_trigger"
	>
		Sheduled Poll is a trigger that polls an external API at a given interval. It is useful for
		triggering jobs based on the change of external states. Under the hood, it is a script triggered
		by a schedule, and returning a list of the changed states so we can iterate over them.
	</Alert>

	<Label label="Setting a scheduled poll">
		<ul class="pt-4 space-y-2">
			<li>
				1. Chose a trigger script using the script picker. <img
					class="shadow-lg border rounded"
					alt="script picker"
					src="{base}/script-picker.png"
				/>
			</li>

			<li>
				2. Modify the primary schedule to your needs. <img
					class="shadow-lg rounded"
					alt="set schedule"
					src="{base}/set-primary-schedule.png"
				/></li
			>

			<li>
				3. Do something with the returned states. <img
					class="shadow-lg rounded"
					alt="create action"
					src="{base}/create_action.png"
				/>
			</li>
		</ul>
	</Label>
</div>
