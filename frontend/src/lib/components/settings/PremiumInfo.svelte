<script lang="ts">
	import { base } from '$lib/base'
	import { capitalize, pluralize, sendUserToast } from '$lib/utils'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import { WorkspaceService, type User, UserService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Button } from '../common'
	import Tooltip from '../Tooltip.svelte'
	import SettingsPageHeader from '$lib/components/settings/SettingsPageHeader.svelte'
	import { ExternalLink, Pen, X } from 'lucide-svelte'
	import Section from '../Section.svelte'
	import Range from '../Range.svelte'
	import Modal from '../common/modal/Modal.svelte'
	import { slide } from 'svelte/transition'

	export let plan: string | undefined
	export let customer_id: string | undefined

	let users: User[] | undefined = undefined

	let premiumInfo:
		| {
				premium: boolean
				usage: number
				status?: string
				developerNb: number
				operatorNb: number
				seatsFromUsers: number
				seatsFromExtraComps: number
				usedSeats: number
				owner: string
				is_past_due: boolean
				max_tolerated_executions?: number
		  }
		| undefined = undefined
	const plans = {
		Free: [
			'Users use their individual global free-tier quotas when doing executions in this workspace',
			'<b>1 000</b> free global executions per-user per month',
			'<b>1 000</b> free executions per workspace per month'
		],
		Team: [
			`<b>$10/mo</b> per seat`,
			`Every seat includes <b>10 000</b> executions`,
			`Every seat includes either 1 developer OR 2 operators`
		],
		Enterprise: [
			`Dedicated and isolated database and workers available (EU/US/Asia)`,
			`Dedicated entire cluster available for (EU/US/Asia)`,
			`SAML support with group syncing`,
			`SLA & Priority Support 24/7 with 3h response time and automation engineer assistance`,
			`Design partners for Roadmap`,
			`<div class="mt-4">Self-hosted licenses also available</div>`
		]
	}

	$: {
		if ($workspaceStore) {
			loadPremiumInfo()
			listUsers()
			getThresholdAlert()
		}
	}

	async function listUsers(): Promise<void> {
		users = await UserService.listUsers({ workspace: $workspaceStore! })
	}

	async function loadPremiumInfo() {
		const info = await WorkspaceService.getPremiumInfo({ workspace: $workspaceStore! })
		const developerNb = users?.filter((x) => !x.operator)?.length ?? 0
		const operatorNb = users?.filter((x) => x.operator)?.length ?? 0
		const usage = info.usage ?? 0

		const seatsFromUsers = Math.ceil(developerNb + operatorNb / 2)
		const seatsFromExtraComps = Math.max(Math.ceil(usage / 10000) - seatsFromUsers, 0)
		const usedSeats = seatsFromUsers + seatsFromExtraComps
		premiumInfo = {
			...info,
			usage,
			owner: info.owner,
			developerNb,
			operatorNb,
			seatsFromUsers,
			seatsFromExtraComps,
			usedSeats
		}
	}

	let thresholdAlert:
		| {
				threshold_alert_amount?: number
				last_alert_sent?: string
		  }
		| undefined = undefined
	let newThresholdAlertAmount: number | undefined = undefined

	let thresholdAlertOpen = false
	async function getThresholdAlert() {
		thresholdAlert = await WorkspaceService.getThresholdAlert({ workspace: $workspaceStore! })
	}

	async function setThresholdAlert() {
		if (thresholdAlert) {
			await WorkspaceService.setThresholdAlert({
				workspace: $workspaceStore!,
				requestBody: {
					threshold_alert_amount: newThresholdAlertAmount
				}
			})
			await getThresholdAlert()
			sendUserToast('Threshold alert updated')
		}
	}

	let estimatedDevsRaw = 1
	let estimatedOps = 0

	$: estimatedDevs = Math.max(1, estimatedDevsRaw)
	$: estimatedSeats = estimatedDevs + Math.ceil(estimatedOps / 2)

	let estimatedExecs = 1

	function updateExecs() {
		if (estimatedExecs < estimatedSeats) {
			estimatedExecs = estimatedSeats
		}
	}
	$: estimatedSeats && updateExecs()

	const formatNumber = (value: number) => value.toLocaleString('en-US')
</script>

<Modal bind:open={thresholdAlertOpen} title="Threshold alert">
	<div class="flex flex-col gap-4">
		<label class="block">
			<span class="text-secondary text-sm">Threshold amount in $</span>
			<input type="number" bind:value={newThresholdAlertAmount} />
		</label>
	</div>

	<svelte:fragment slot="actions">
		<Button
			size="sm"
			on:click={() => {
				setThresholdAlert()
				thresholdAlertOpen = false
			}}
		>
			Save
		</Button>
	</svelte:fragment>
</Modal>

<div class="flex flex-col gap-4 mt-8">
	<div class="flex flex-col gap-1">
		<SettingsPageHeader
			title={premiumInfo?.premium && plan
				? `Plan: ${capitalize(plan)} plan${plan === 'team' ? ' (usage-based)' : ''}`
				: 'Plan: Free plan'}
			class="mb-0"
		/>
		{#if premiumInfo?.status === 'past_due'}
			<p class="text-red-500 text-base">
				{#if premiumInfo.max_tolerated_executions === undefined || premiumInfo.usage > premiumInfo.max_tolerated_executions}
					Your last invoice is unpaid, you cannot run any more jobs. Please update your payment
					method in the Customer Portal to continue running jobs.
				{:else}
					Your last invoice is unpaid. Please update your payment method in the Customer Portal to
					prevent the interruption of your job executions.
				{/if}
			</p>
		{/if}
	</div>
</div>
{#if customer_id}
	<div class="mt-2">
		<Button
			endIcon={{ icon: ExternalLink }}
			variant="accent"
			href="{base}/api/w/{$workspaceStore}/workspaces/billing_portal">Customer Portal</Button
		>
		<p class="text-xs text-primary mt-1">
			See invoices, change billing information or subscription details.
		</p>
	</div>
{/if}

<div class="text-xs">
	{#if premiumInfo?.premium}
		<div class="flex flex-col gap-8 my-8">
			{#if plan}
				<div class="flex flex-col gap-1.5">
					<p class="font-semibold text-sm">Billing threshold email alert</p>
					<div class="flex flex-row gap-0.5 items-center">
						<p class="text-base text-secondary mr-0.5"
							>{thresholdAlert?.threshold_alert_amount
								? thresholdAlert?.threshold_alert_amount + '$'
								: 'Not set'}</p
						>
						<Button
							on:click={() => {
								newThresholdAlertAmount = thresholdAlert?.threshold_alert_amount ?? 10
								thresholdAlertOpen = true
							}}
							size="xs"
							spacingSize="xs2"
							variant="border"
							color="light"
							iconOnly
							startIcon={{
								icon: Pen
							}}
						/>
						{#if thresholdAlert?.threshold_alert_amount}
							<Button
								on:click={() => {
									if (thresholdAlert) {
										newThresholdAlertAmount = undefined
										setThresholdAlert()
									}
								}}
								variant="border"
								size="xs"
								spacingSize="xs2"
								color="light"
								iconOnly
								startIcon={{
									icon: X
								}}
							/>
						{/if}
					</div>
					<p class="italic text-xs">
						An email notification will be sent to {premiumInfo.owner} if the specified threshold amount
						is exceeded during a given month.
					</p>
				</div>
				<div class="w-full">
					<DataTable>
						<tbody class="divide-y">
							<tr>
								<Cell first>
									<div class="flex flex-col gap-0.5">
										<div class="font-medium">Developers</div>
										<p class="text-xs text-secondary">
											Calculated on the MAXIMUM number of users in a given billing
											period, see the Customer Portal for more info.
										</p>
									</div>
								</Cell>
								<Cell last numeric>
									<div class="text-sm text-secondary">
										{formatNumber(premiumInfo.developerNb)}
									</div>
								</Cell>
							</tr>
							<tr>
								<Cell first>
									<div class="flex flex-col gap-0.5">
										<div class="font-medium">Operators</div>
										<p class="text-xs text-secondary">
											Calculated on the MAXIMUM number of operators in a given
											billing period, see the Customer Portal for more info.
										</p>
									</div>
								</Cell>
								<Cell last numeric>
									<div class="text-sm text-secondary">
										{formatNumber(premiumInfo.operatorNb)}
									</div>
								</Cell>
							</tr>
							<tr class="bg-slate-50 dark:bg-slate-900/40">
								<Cell first>
									<div class="flex flex-col gap-0.5">
										<div class="font-semibold text-sm">Seats from users</div>
										<p class="text-xs text-secondary">
											1 developer = 1 seat, 2 operators = 1 seat.
										</p>
										<p class="text-[11px] text-secondary font-mono">
											u = ceil({formatNumber(premiumInfo.developerNb)} + {formatNumber(premiumInfo.operatorNb)}/2)
											= {formatNumber(premiumInfo.seatsFromUsers)}
										</p>
									</div>
								</Cell>
								<Cell last numeric>
									<div class="text-base font-bold text-primary">
										{formatNumber(premiumInfo.seatsFromUsers)}
									</div>
								</Cell>
							</tr>
							<tr>
								<Cell first>
									<div class="flex flex-col gap-0.5">
										<div class="font-semibold">Executions this month</div>
										<p class="text-xs text-secondary">
											One execution equals one job
											up to 1 second on a worker with 2GB of memory, with each additional
											second counting as an extra execution.
										</p>
									</div>
								</Cell>
								<Cell last numeric>
									<div class="text-sm text-secondary">
										{formatNumber(premiumInfo.usage)}
									</div>
								</Cell>
							</tr>
							<tr>
								<Cell first>
									<div class="flex flex-col gap-0.5">
										<div class="font-semibold">Included executions from user seats</div>
										<p class="text-xs text-secondary">
											Each seat includes 10,000 executions per month.
										</p>
									</div>
								</Cell>
								<Cell last numeric>
									<div class="text-sm text-secondary">
										- {formatNumber(premiumInfo.seatsFromUsers * 10000)}
									</div>
								</Cell>
							</tr>
							<tr class="bg-slate-50 dark:bg-slate-900/40">
								<Cell first>
									<div class="flex flex-col gap-0.5">
										<div class="font-semibold text-sm">Extra seats from computations</div>
										<p class="text-xs text-secondary">
											Additional seats created by executions beyond the included quota.
										</p>
										<p class="text-[11px] text-secondary font-mono">
											c = ceil(max(0, {formatNumber(premiumInfo.usage)} -
											{formatNumber(premiumInfo.seatsFromUsers * 10000)}) / 10,000) =
											{formatNumber(premiumInfo.seatsFromExtraComps)}
										</p>
									</div>
								</Cell>
								<Cell last numeric>
									<div class="text-base font-bold text-primary">
										{formatNumber(premiumInfo.seatsFromExtraComps)}
									</div>
								</Cell>
							</tr>
							<tr class="bg-slate-100 dark:bg-slate-900/60">
								<Cell first>
									<div class="flex flex-col gap-0.5">
										<div class="font-semibold text-sm flex items-center gap-1">
											Used seats (billed)
										</div>
										<p class="text-xs text-secondary">
											Highest between seats from 'Developers + Operators' and 'Seats from executions'.
											This is the number of seats used for billing this month.
										</p>
										<p class="text-[11px] text-secondary font-mono">
											u + c = {formatNumber(premiumInfo.usedSeats)}
										</p>
									</div>
								</Cell>
								<Cell last numeric>
									<div class="text-base font-extrabold text-primary">
										{formatNumber(premiumInfo.usedSeats)}
									</div>
								</Cell>
							</tr>
						</tbody>
					</DataTable>
				</div>
			{/if}
		</div>
	{:else}
		<div class="mb-8 mt-2">
			This workspace is <b>not</b> on a team plan. Users use their global free-tier quotas when doing
			executions in this workspace. Upgrade to a Team or Enterprise plan to unlock unlimited executions
			in this workspace.
		</div>
	{/if}
</div>

<Section collapsable label="Cost estimator">
	<div class="border rounded-md p-4 md:p-5 space-y-5" transition:slide>
		<div class="flex flex-col gap-1">
			<div class="text-sm font-semibold text-primary">Estimate your monthly cost</div>
			<p class="text-xs text-secondary max-w-xl">
				This is a rough estimate based on your expected team size and workload. Actual billing is based
				on the maximum number of users and executions in a given month.
			</p>
		</div>

		<div class="grid gap-6 md:grid-cols-[minmax(0,1.4fr)_minmax(0,1fr)] items-start">
			<!-- Inputs -->
			<div class="space-y-4">
				<div class="space-y-1.5">
					<div class="flex items-center justify-between gap-2">
						<div class="text-sm font-medium text-primary">Developers</div>
						<div class="text-xs text-secondary">
							<span class="font-semibold">{estimatedDevs}</span> dev{estimatedDevs === 1 ? '' : 's'}
						</div>
					</div>
					<Range min={1} max={20} bind:value={estimatedDevsRaw} hideInput />
				</div>

				<div class="space-y-1.5">
					<div class="flex items-center justify-between gap-2">
						<div class="text-sm font-medium text-primary">Operators</div>
						<div class="text-xs text-secondary">
							<span class="font-semibold">{estimatedOps}</span> operator{estimatedOps === 1 ? '' : 's'}
						</div>
					</div>
					<Range min={0} max={20} bind:value={estimatedOps} hideInput />
					<p class="text-[11px] text-secondary">
						2 operators = 1 seat
					</p>
				</div>

				<div class="space-y-1.5">
					<div class="flex items-center justify-between gap-2">
						<div class="flex items-center gap-1.5">
							<div class="text-sm font-medium text-primary">Monthly executions</div>
							<Tooltip>
								One execution equals one job up to 1 second on a virtual CPU with 2 GB of memory, with
								each additional second counting as an extra execution.
							</Tooltip>
						</div>
						<div class="text-xs text-secondary">
							<span class="font-semibold">{estimatedExecs * 10}k</span> executions / month
						</div>
					</div>
					<Range
						min={estimatedSeats}
						max={100}
						bind:value={estimatedExecs}
						format={(v) => `${v * 10}k`}
						hideInput
					/>
					<p class="text-[11px] text-secondary">
						Each seat includes 10k executions per month.
					</p>
				</div>
			</div>

			<!-- Breakdown -->
			{#if estimatedSeats}
				<div
					class="rounded-md bg-slate-50 dark:bg-slate-900/40 border border-slate-200 dark:border-slate-800 p-4 space-y-3 text-sm"
				>
					<div class="flex items-center justify-between">
						<div class="text-secondary">Seats from users</div>
						<div class="font-medium">
							{pluralize(estimatedSeats, 'seat')}
						</div>
					</div>

					<div class="flex items-center justify-between text-xs text-secondary">
						<div>
							Based on {estimatedDevs} dev{estimatedDevs === 1 ? '' : 's'} and
							{` ${estimatedOps} operator${estimatedOps === 1 ? '' : 's'}`}
						</div>
					</div>

					<hr class="border-slate-200 dark:border-slate-800" />

					<div class="flex items-center justify-between">
						<div class="text-secondary">Included executions</div>
						<div class="font-medium">{estimatedSeats * 10}k</div>
					</div>

					<div class="flex items-center justify-between">
						<div class="text-secondary">Extra executions</div>
						<div class="font-medium">
							{(estimatedExecs - estimatedSeats) * 10}k
						</div>
					</div>

					<div class="flex items-center justify-between text-xs text-secondary">
						<div>Charged as additional seats</div>
						<div>
							≈ {pluralize(estimatedExecs - estimatedSeats, 'seat')}
						</div>
					</div>

					<hr class="border-slate-200 dark:border-slate-800" />

					<div class="flex items-center justify-between font-semibold text-base">
						<div>Total estimated seats</div>
						<div class="text-right">
							<div>
								{pluralize(estimatedSeats + (estimatedExecs - estimatedSeats), 'seat')}
							</div>
							<div class="text-primary text-sm">
								{(estimatedSeats + (estimatedExecs - estimatedSeats)) * 10}$ / month
							</div>
						</div>
					</div>

					{#if premiumInfo?.premium && plan === 'team'}
						<button
							class="mt-1 text-[11px] text-blue-500 underline self-start"
							on:click={() => {
								newThresholdAlertAmount = (estimatedSeats + (estimatedExecs - estimatedSeats)) * 10
								thresholdAlertOpen = true
							}}
						>
							Use this amount for a threshold email alert
						</button>
					{/if}

					<div class="mt-2 h-4 text-[11px]">
						{#if (estimatedSeats + (estimatedExecs - estimatedSeats)) * 10 > 700}
							<a
								class="text-teal-600 font-semibold underline block text-right"
								href="https://www.windmill.dev/pricing"
								target="_blank"
							>
								Higher usage? Explore Cloud Enterprise
							</a>
						{/if}
					</div>
				</div>
			{/if}
		</div>
	</div>
</Section>

<div class="text-base font-bold leading-8 mb-2 pt-8"> All plans </div>

<div class="grid grid-cols-1 md:grid-cols-3 gap-4">
	{#each Object.entries(plans) as [planTitle, planDesc]}
		<div class="box p-4 text-xs flex flex-col h-full overflow-hidden prose-sm rounded-md">
			<h2
				class="mb-4 {planTitle === 'Team'
					? 'text-blue-500'
					: planTitle === 'Enterprise'
						? 'text-teal-600'
						: ''}"
			>
				{planTitle}
			</h2>
			<ul class="list-disc text-sm p-4">
				{#each planDesc as item, i}
					{#if planTitle === 'Team' && i === 1}
						<li class="mt-2">
							Every seat includes <b>10 000</b> executions
							<Tooltip>
								One execution equals one job up to 1 second on a virtual CPU with 2 GB of memory, with
								each additional second counting as an extra execution.
							</Tooltip>
						</li>
					{:else}
						<li class="mt-2">{@html item}</li>
					{/if}
				{/each}
			</ul>

			<div class="grow"></div>
			<div class="mt-4 mx-auto flex flex-col items-center text-center min-h-[2.5rem]">
				{#if planTitle == 'Team'}
					{#if plan != 'team'}
						{#if plan != 'enterprise'}
							<Button
								size="xs"
								color="bg-blue-500 text-white"
								href="{base}/api/w/{$workspaceStore}/workspaces/checkout?plan=team"
							>
								Upgrade to Team plan</Button
							>
						{:else}
							<div class="text-md font-semibold">
								Cancel your plan in the Customer Portal then upgrade to a team plan
							</div>
						{/if}
					{:else}
						<div class="text-md font-semibold">
							Workspace is on the team plan
						</div>
					{/if}
				{:else if planTitle == 'Enterprise'}
					{#if plan != 'enterprise'}
						<Button
							size="xs"
							color="bg-teal-600 text-white"
							href="https://www.windmill.dev/pricing"
							target="_blank"
						>
							See more
						</Button>
					{:else}
						<div class="text-md font-semibold">
							Workspace is on enterprise plan
						</div>
					{/if}
				{:else if planTitle === 'Free'}
					{#if plan}
						<div class="text-md font-semibold">
							Cancel your plan in the Customer Portal to downgrade to the free plan
						</div>
					{:else}
						<div class="font-semibold">
							Workspace is on the free plan
						</div>
					{/if}
				{/if}
			</div>
		</div>
	{/each}
</div>
<div class="flex flex-col gap-1 my-8 w-full items-center">
	<div class="text-primary text-md font-semibold"> Frequently asked questions </div><br />
	<div class="flex flex-col gap-4">
		<div>
			<div class="text-sm mb-1 text-secondary font-medium"> What is an execution? </div>
			<div class="text-xs max-w-xl border-b pb-4 text-primary">
				The single credit-unit is called an "execution". An execution corresponds to a single job
				whose duration is less than 1s. For any additional seconds of computation, an additional
				execution is accounted for. Jobs are executed on one powerful virtual CPU with 2Gb of
				memory. Most jobs will take less than 200ms to execute.
			</div>
		</div>
		<div>
			<div class="text-sm mb-1 text-secondary font-medium">
				What is the difference between a developer and an operator?
			</div>
			<div class="text-xs max-w-xl text-primary">
				A developer can write scripts/flows/apps/variables/resources. An operator can only run/view
				them.
			</div>
		</div>
	</div>
</div>
