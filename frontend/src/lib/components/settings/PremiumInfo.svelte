<script lang="ts">
	import { base } from '$lib/base'
	import { capitalize, pluralize, sendUserToast } from '$lib/utils'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import { WorkspaceService, type User, UserService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Button } from '../common'
	import Tooltip from '../Tooltip.svelte'
	import { ExternalLink, Pen, X } from 'lucide-svelte'
	import Section from '../Section.svelte'
	import Range from '../Range.svelte'
	import Label from '../Label.svelte'
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
		const seatsFromUsers = Math.ceil(developerNb + operatorNb / 2)
		const seatsFromExtraComps = Math.max(Math.ceil((info.usage ?? 0) / 10000) - seatsFromUsers, 0)
		const usedSeats = seatsFromUsers + seatsFromExtraComps
		premiumInfo = {
			...info,
			usage: info.usage ?? 0,
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

	let estimatedDevs = 1
	let estimatedOps = 0

	$: estimatedSeats = estimatedDevs + Math.ceil(estimatedOps / 2)

	let estimatedExecs = 1

	function updateExecs() {
		if (estimatedExecs < estimatedSeats) {
			estimatedExecs = estimatedSeats
		}
	}
	$: estimatedSeats && updateExecs()
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
		<div class=" text-primary text-lg font-semibold">
			{#if premiumInfo?.premium && plan}
				Plan: {capitalize(plan)} plan{plan === 'team' ? ' (usage-based)' : ''}
			{:else}
				Plan: Free plan
			{/if}
		</div>
		{#if premiumInfo?.status === 'past_due'}
			<p class="text-red-500 text-base">
				Your last invoice is unpaid. Please update your payment method in the customer portal to
				prevent account downgrade and the interruption of your job executions.
			</p>
		{/if}
	</div>
</div>
{#if customer_id}
	<div class="mt-2">
		<Button
			endIcon={{ icon: ExternalLink }}
			color="dark"
			href="{base}/api/w/{$workspaceStore}/workspaces/billing_portal">Customer Portal</Button
		>
		<p class="text-xs text-tertiary mt-1">
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
									Developers
									<Tooltip>
										Actual pricing is calculated on the MAXIMUM number of users in a given billing
										period, see the customer portal for more info.
									</Tooltip>
								</Cell>
								<Cell last numeric>
									<div class="text-base">
										{premiumInfo.developerNb}
									</div>
								</Cell>
							</tr>
							<tr>
								<Cell first>
									Operators
									<Tooltip>
										Actual pricing is calculated on the MAXIMUM number of operators in a given
										billing period, see the customer portal for more info.
									</Tooltip>
								</Cell>
								<Cell last numeric>
									<div class="text-base">
										{premiumInfo.operatorNb}
									</div>
								</Cell>
							</tr>
							<tr>
								<Cell first><div class="font-semibold">Number of seats for users</div></Cell>
								<Cell last numeric>
									<div class="text-base font-bold">
										u = ceil({premiumInfo.developerNb} + {premiumInfo.operatorNb}/2) = {premiumInfo.seatsFromUsers}
									</div>
								</Cell>
							</tr>
							<tr>
								<Cell first>Computations executed this month</Cell>
								<Cell last numeric>
									<div class="text-base">
										{premiumInfo.usage}
									</div>
								</Cell>
							</tr>
							<tr>
								<Cell first>Included computations with users (10k per user seat)</Cell>
								<Cell last numeric>
									<div class="text-base">
										- {premiumInfo.seatsFromUsers * 10000}
									</div>
								</Cell>
							</tr>
							<tr>
								<Cell first
									><div class="font-semibold">Number of seats for extra computations</div></Cell
								>
								<Cell last numeric>
									<div class="text-base font-bold">
										c = ceil({Math.max(0, premiumInfo.usage - premiumInfo.seatsFromUsers * 10000)} /
										10 000) = {premiumInfo.seatsFromExtraComps}
									</div>
								</Cell>
							</tr>
							<tr>
								<Cell first
									><div class="font-semibold"
										>Used seats <Tooltip
											>Highest between seats from developers + operators and seats from computations
										</Tooltip></div
									></Cell
								>
								<Cell last numeric>
									<div class="text-base font-bold">
										u + c = {premiumInfo.usedSeats}
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
	<div class="border p-4 rounded-md" transition:slide>
		<Label label="Number of developers">
			<Range min={1} max={20} bind:value={estimatedDevs} hideInput />
		</Label>
		<Label label="Number of operators">
			<Range min={0} max={20} bind:value={estimatedOps} hideInput />
		</Label>
		<Label label="Number of executions">
			<Range
				min={estimatedSeats}
				max={100}
				bind:value={estimatedExecs}
				format={(v) => `${v * 10}k`}
				hideInput
			/>
		</Label>
		<div class="mt-4 text-base">
			<div class="flex flex-row justify-between">
				<div>Seats for users = {estimatedDevs} devs + {estimatedOps} ops / 2</div>
				<div>{pluralize(estimatedSeats, 'seat')}</div>
			</div>
			<div class="flex flex-row justify-between">
				<div
					>Extra computations = {estimatedExecs * 10}k execs - {estimatedSeats * 10}k included with
					users</div
				>
				<div
					>{(estimatedExecs - estimatedSeats) * 10}k extra execs = {pluralize(
						estimatedExecs - estimatedSeats,
						'seat'
					)}</div
				>
			</div>
			<div class="flex flex-row justify-between font-medium items-center">
				<div>Total</div>
				<div class="flex flex-col items-end">
					<div>
						{pluralize(estimatedSeats + (estimatedExecs - estimatedSeats), 'seat')} = {(estimatedSeats +
							(estimatedExecs - estimatedSeats)) *
							10}$ / month
					</div>
					{#if premiumInfo?.premium && plan === 'team'}
						<button
							class="text-xs text-blue-500 underline"
							on:click={() => {
								newThresholdAlertAmount = (estimatedSeats + (estimatedExecs - estimatedSeats)) * 10
								thresholdAlertOpen = true
							}}
						>
							Setup threshold email alert
						</button>
					{/if}
				</div>
			</div>
			{#if (estimatedSeats + (estimatedExecs - estimatedSeats)) * 10 > 700}
				<a
					class="mt-4 text-teal-600 font-semibold text-center block underline"
					href="https://www.windmill.dev/pricing"
					target="_blank"
				>
					You should consider subscribing to Cloud Enterprise
				</a>
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
				{#each planDesc as item}
					<li class="mt-2">{@html item}</li>
				{/each}
			</ul>

			<div class="grow"></div>
			{#if planTitle == 'Team'}
				{#if plan != 'team'}
					<div class="mt-4 mx-auto">
						{#if plan != 'enterprise'}
							<Button
								size="xs"
								color="bg-blue-500 text-white"
								href="{base}/api/w/{$workspaceStore}/workspaces/checkout?plan=team"
							>
								Upgrade to Team plan</Button
							>
						{:else}
							<div class="mx-auto font-semibold text-center">
								Cancel your plan in the customer portal then upgrade to a team plan
							</div>
						{/if}
					</div>
				{:else}
					<div class="mx-auto text-md font-semibold">Workspace is on the team plan</div>
				{/if}
			{:else if planTitle == 'Enterprise'}
				{#if plan != 'enterprise'}
					<div class="mt-4 mx-auto">
						<Button
							size="xs"
							color="bg-teal-600 text-white"
							href="https://www.windmill.dev/pricing"
							target="_blank"
						>
							See more
						</Button>
					</div>
				{:else}
					<div class="mx-auto text-md font-semibold">Workspace is on enterprise plan</div>
				{/if}
			{:else if planTitle === 'Free'}
				{#if plan}
					<div class="mx-auto font-semibold text-center">
						Cancel your plan in the customer portal to downgrade to the free plan
					</div>
				{:else}
					<div class="mx-auto font-semibold text-center"> Workspace is on the free plan </div>
				{/if}
			{/if}
		</div>
	{/each}
</div>
<div class="flex flex-col gap-1 my-8 w-full items-center">
	<div class="text-primary text-md font-semibold"> Frequently asked questions </div><br />
	<div class="flex flex-col gap-4">
		<div>
			<div class="text-sm mb-1 text-secondary font-medium"> What is an execution? </div>
			<div class="text-xs max-w-xl border-b pb-4 text-tertiary">
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
			<div class="text-xs max-w-xl text-tertiary">
				A developer can write scripts/flows/apps/variables/resources. An operator can only run/view
				them.
			</div>
		</div>
	</div>
</div>
