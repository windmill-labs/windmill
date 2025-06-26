<script lang="ts">
	import { ScheduleService } from '$lib/gen'
	import { emptyString, formatCron, sendUserToast } from '$lib/utils'
	import Badge from './Badge.svelte'
	import { Button } from './common'
	import timezones from './timezones'
	import CronBuilder from './CronBuilder.svelte'
	import Label from './Label.svelte'
	import CronGen from './copilot/CronGen.svelte'
	import Select from './select/Select.svelte'
	import MultiSelect from './select/MultiSelect.svelte'
	import { safeSelectItems } from './select/utils.svelte'
	import { untrack } from 'svelte'

	interface Props {
		schedule: string
		// export let offset: number = -60 * Math.floor(new Date().getTimezoneOffset() / 60)
		timezone: string // = Intl.DateTimeFormat().resolvedOptions().timeZone
		disabled?: boolean
		validCRON?: boolean
		cronVersion?: string
	}

	let {
		schedule = $bindable(),
		timezone = $bindable(),
		disabled = false,
		validCRON = $bindable(true),
		cronVersion = $bindable('v2')
	}: Props = $props()

	let preview: string[] = $state([])
	// If the user has already entered a cron string, switching to the basic tab will override it.
	let executeEvery: 'second' | 'minute' | 'hour' | 'day-month' | 'month' | 'day-week' =
		$state('minute')

	let seconds = $state(30)
	let minutes = $state(30)
	let hours = $state(1)
	const daysOfMonthOptions: number[] = Array.from(Array(31).keys()).map((i) => i + 1)
	let daysOfMonth: number[] = $state([])
	// let lastDayOfMonth = false
	const monthsOfYearOptions: string[] = [
		'January',
		'February',
		'March',
		'April',
		'May',
		'June',
		'July',
		'August',
		'September',
		'October',
		'November',
		'December'
	]
	let monthsOfYear: string[] = $state([])
	const daysOfWeekOptions: string[] = [
		'Sunday',
		'Monday',
		'Tuesday',
		'Wednesday',
		'Thursday',
		'Friday',
		'Saturday'
	]
	let daysOfWeek: string[] = $state([])
	let UTCTime: string = $state('')

	async function handleScheduleInput(input: string, timezone: string): Promise<void> {
		try {
			preview = await ScheduleService.previewSchedule({
				requestBody: { schedule: formatCron(input), timezone, cron_version: cronVersion }
			})
			validCRON = true
		} catch (err) {
			if (err.status == 400 && err.body.includes('cron')) {
				validCRON = false
			} else {
				sendUserToast(err.body, true)
				validCRON = false
			}
		}
	}

	let nschedule = $state('')

	function formatDate(timezone) {
		try {
			return new Intl.DateTimeFormat('en-GB', {
				weekday: 'short',
				day: '2-digit',
				month: 'short',
				year: 'numeric',
				hour: 'numeric',
				minute: 'numeric',
				second: 'numeric',
				timeZone: timezone,
				timeZoneName: 'short'
			}).format
		} catch (ee) {
			sendUserToast(
				`Invalid timezone: ${timezone}. Update your browser's timezone preference`,
				true
			)
			return new Intl.DateTimeFormat('en-GB', {
				weekday: 'short',
				day: '2-digit',
				month: 'short',
				year: 'numeric',
				hour: 'numeric',
				minute: 'numeric',
				second: 'numeric',
				timeZone: 'Europe/Paris',
				timeZoneName: 'short'
			}).format
		}
	}

	const items = Object.keys(timezones)
		.map((key) => {
			return Object.keys(timezones[key])
				.map((subKey) => {
					return {
						value: subKey,
						label: subKey,
						group: timezones[key][subKey][1] as string
					}
				})
				.flat()
		})
		.flat()
	$effect(() => {
		schedule
		untrack(() => {
			!emptyString(schedule) && handleScheduleInput(schedule, timezone)
		})
	})
	$effect(() => {
		// CRON string format
		// sec  min   hour      day of month   month     day of week   year
		// 0    30    9,12,15   1,15           May-Aug   Mon,Wed,Fri   2018/2

		let s_daysOfMonth = ''
		// if (lastDayOfMonth) {
		// 	s_daysOfMonth = 'L'
		// } else
		if (daysOfMonth.length > 0) {
			s_daysOfMonth = daysOfMonth.join(',')
		} else {
			s_daysOfMonth = '*'
		}

		let s_months = ''
		if (monthsOfYear.length > 0) {
			s_months = monthsOfYear.map((m) => m.slice(0, 3).toLowerCase()).join(',')
		} else {
			s_months = '*'
		}

		let s_daysOfWeek = ''
		if (daysOfWeek.length > 0) {
			s_daysOfWeek = daysOfWeek.map((d) => d.slice(0, 3).toLowerCase()).join(',')
		} else {
			s_daysOfWeek = '*'
		}

		const s_AtUTCHours = parseInt(UTCTime.split(':')[0]) || '0'
		const s_AtUTCMinutes = parseInt(UTCTime.split(':')[1]) || '0'

		// If using the basic editor, set the cron string based on the selected options
		if (executeEvery === 'second') {
			if (seconds > 0) {
				nschedule = `*/${seconds} * * * * *`
			} else {
				nschedule = `* * * * * *`
			}
		} else if (executeEvery === 'minute') {
			if (minutes > 0) {
				nschedule = `0 */${minutes} * * * *`
			} else {
				nschedule = `* * * * * *`
			}
		} else if (executeEvery === 'hour') {
			if (hours > 0) {
				nschedule = `0 0 */${hours} * * *`
			} else {
				nschedule = `* * * * * *`
			}
		} else if (executeEvery === 'day-month') {
			nschedule = `0 ${s_AtUTCMinutes} ${s_AtUTCHours} ${s_daysOfMonth} * *`
		} else if (executeEvery === 'month') {
			nschedule = `0 ${s_AtUTCMinutes} ${s_AtUTCHours} ${s_daysOfMonth} ${s_months} *`
		} else if (executeEvery === 'day-week') {
			nschedule = `0 ${s_AtUTCMinutes} ${s_AtUTCHours} * * ${s_daysOfWeek}`
		}
	})
	let dateFormatter = $derived(formatDate(timezone))
</script>

<div class="w-full flex space-x-8">
	<div class="w-full flex flex-col gap-4">
		<Label label="Cron" class="font-semibold" primary={true}>
			{#snippet error()}
				{#if !validCRON}
					<div class="text-red-600 text-xs"> Invalid cron syntax </div>
				{/if}
			{/snippet}
			<div class="flex flex-row-reverse text-2xs text-tertiary -mt-1 hover:underline">
				<a
					class="text-tertiary"
					href="https://www.windmill.dev/docs/core_concepts/scheduling#cron-syntax"
					target="_blank">Croner</a
				>
			</div>
			<input
				class="inline-block"
				type="text"
				id="cron-schedule"
				name="cron-schedule"
				placeholder="0 0 */1 * * *"
				bind:value={schedule}
				{disabled}
			/>
		</Label>
		<Label label="Timezone" class="font-semibold" primary>
			{#if disabled}
				<div>
					<Badge><span class="text-primary dark:text-primary-inverse">{timezone}</span></Badge>
				</div>
			{:else}
				<Select
					{items}
					class="w-full"
					bind:value={timezone}
					groupBy={(item) => item.group}
					sortBy={(a, b) => {
						if (a.group[0] === '+' && b.group[0] === '-') return -1
						if (a.group[0] === '-' && b.group[0] === '+') return 1
						return a.group.localeCompare(b.group)
					}}
				/>
			{/if}
		</Label>

		{#if !disabled}
			<div class="flex flex-row gap-2 mb-2">
				<CronBuilder>
					{#snippet children({ close })}
						<div class="w-full flex flex-col">
							<div class="w-full flex flex-col gap-1">
								<div class="text-secondary text-sm leading-none">Execute schedule every</div>

								<div class="w-full flex gap-4">
									<div class="w-full flex flex-col gap-1 mb-2">
										<select
											{disabled}
											name="execute_every"
											id="execute_every"
											bind:value={executeEvery}
										>
											<option value="second">Second(s)</option>
											<option value="minute">Minute(s)</option>
											<option value="hour">Hour(s)</option>
											<option value="day-month">Day of the month</option>
											<option value="month">Month(s)</option>
											<option value="day-week">Day of the week</option>
										</select>
									</div>

									<div class="w-full flex flex-col gap-1 justify-center">
										{#if executeEvery == 'second'}
											<input {disabled} type="number" min="0" max="59" bind:value={seconds} />
											<small>Valid range 0-59</small>
										{:else if executeEvery == 'minute'}
											<input {disabled} type="number" min="0" max="59" bind:value={minutes} />
											<small>Valid range 0-59</small>
										{:else if executeEvery == 'hour'}
											<input {disabled} type="number" min="0" max="23" bind:value={hours} />
											<small>Valid range 0-23</small>
										{:else if executeEvery == 'day-month'}
											<!-- <div class="w-full flex">
										<label for="lastDayOfMonth" class="w-full flex items-center gap-2">
											<div class="flex">
												<input type="checkbox" id="lastDayOfMonth" bind:checked={lastDayOfMonth} />
											</div>
											<small> Last day of the month </small>
										</label>
									</div> -->
										{/if}
									</div>
								</div>
							</div>

							<div class="w-full flex flex-col gap-4">
								{#if executeEvery == 'month'}
									<div class="w-full flex flex-col">
										<MultiSelect
											disablePortal
											{disabled}
											bind:value={monthsOfYear}
											items={safeSelectItems(monthsOfYearOptions)}
											placeholder="Every month"
										/>
									</div>
								{/if}

								{#if executeEvery == 'day-week'}
									<div class="w-full flex flex-col">
										<MultiSelect
											disablePortal
											{disabled}
											bind:value={daysOfWeek}
											items={safeSelectItems(daysOfWeekOptions)}
											placeholder="Every day"
										/>
									</div>
								{/if}

								{#if executeEvery == 'day-month' || executeEvery == 'month'}
									<div class="w-full flex flex-col gap-1">
										{#if executeEvery == 'month'}
											<small class="font-bold">On day of the month</small>
										{/if}
										<div class="w-full flex gap-4">
											<div class="w-full flex">
												<MultiSelect
													disablePortal
													{disabled}
													bind:value={daysOfMonth}
													items={safeSelectItems(daysOfMonthOptions)}
													placeholder="Every day"
												/>
											</div>

											<!-- {#if executeEvery == 'month'}
										<div class="w-full flex">
											<label for="lastDayOfMonth" class="w-full flex items-center gap-2">
												<div class="flex">
													<input type="checkbox" id="lastDayOfMonth" bind:checked={lastDayOfMonth} />
												</div>
												<small> Last day of the month </small>
											</label>
										</div>
									{/if} -->
										</div>
										<small>Schedule will only execute on valid calendar days</small>
									</div>
								{/if}

								{#if executeEvery == 'day-month' || executeEvery == 'month' || executeEvery == 'day-week'}
									<div class="w-full flex flex-col gap-1">
										<small class="font-bold">At Time</small>
										<input
											{disabled}
											type="time"
											name="atUTCTime"
											id="atUTCTime"
											bind:value={UTCTime}
										/>
									</div>
								{/if}
							</div>

							<div class="w-full flex flex-col gap-1">
								<div class="text-secondary text-sm leading-none">Preview New Cron</div>

								<div class="flex p-2 px-4 rounded-md bg-surface-secondary">
									<span>{nschedule}</span>
								</div>
							</div>
						</div>

						<div class="mt-4">
							<Button
								color="dark"
								size="xs"
								on:click={() => {
									schedule = nschedule
									close()
								}}
							>
								Set cron schedule
							</Button>
						</div>
					{/snippet}
				</CronBuilder>
				<CronGen bind:schedule bind:cronVersion />
			</div>
		{/if}
	</div>

	<div class="w-full flex flex-col space-y-2">
		<div class="text-sm font-semibold leading-none">Estimated upcoming events ({timezone})</div>
		<div class="flex flex-col space-y-2">
			<div class="flex flex-col rounded-md p-4 border text-tertiary bg-surface-secondary gap-0.5">
				{#each preview as date}
					<span class="text-sm">{dateFormatter(new Date(date))}</span>
				{/each}
			</div>
		</div>
	</div>
</div>
