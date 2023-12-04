<script lang="ts">
	import { capitalize } from '$lib/utils'
	import DataTable from '$lib/components/table/DataTable.svelte'
	import Cell from '$lib/components/table/Cell.svelte'
	import { WorkspaceService, type User, UserService } from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { Button } from '../common'
	import Tooltip from '../Tooltip.svelte'
	import { Badge, ExternalLink } from 'lucide-svelte'
	import { twMerge } from 'tailwind-merge'

	export let plan: string | undefined
	export let customer_id: string | undefined

	let users: User[] | undefined = undefined

	let premiumInfo:
		| {
				premium: boolean
				usage: number
				seats: number
				authorNb: number
				operatorNb: number
				seatsFromUsers: number
				seatsFromComps: number
				usedSeats: number
		  }
		| undefined = undefined
	const plans = {
		Free: [
			'Users use their individual global free-tier quotas when doing executions in this workspace',
			'<b>1 000</b> free global executions per-user per month'
		],
		Team: [
			`<b>$10/mo</b> per seat`,
			`Every seat includes <b>10 000</b> executions`,
			`Every seat includes either 1 user OR 2 operators`
		],
		Enterprise: [
			`<b>Dedicated</b> and isolated database and workers available (EU/US/Asia)`,
			`<b>Dedicated</b> entire cluster available for (EU/US/Asia)`,
			`<b>SAML</b> support with group syncing`,
			`<b>SLA</b>`,
			`<b>Priority Support 24/7 with 3h response time and automation engineer assistance</b>`,
			`<b>Design partners for Roadmap</b>`,
			`<div class="mt-4">Self-hosted licenses also available</div>`
		]
	}

	$: {
		if ($workspaceStore) {
			loadPremiumInfo()
			listUsers()
		}
	}

	async function listUsers(): Promise<void> {
		users = await UserService.listUsers({ workspace: $workspaceStore! })
	}

	async function loadPremiumInfo() {
		const info = await WorkspaceService.getPremiumInfo({ workspace: $workspaceStore! })
		const authorNb = users?.filter((x) => !x.operator)?.length ?? 0
		const operatorNb = users?.filter((x) => x.operator)?.length ?? 0
		const seatsFromUsers = Math.ceil(authorNb + operatorNb / 2)
		const seatsFromComps = Math.ceil((info.usage ?? 0) / 10000)
		const usedSeats = Math.max(seatsFromUsers, seatsFromComps)
		premiumInfo = {
			...info,
			usage: info.usage ?? 0,
			seats: info.seats ?? 1,
			authorNb,
			operatorNb,
			seatsFromUsers,
			seatsFromComps,
			usedSeats
		}
	}
</script>

<div class="my-8" />
<div class="flex flex-col gap-1">
	<div class=" text-primary text-md font-semibold"> Plans </div>
</div>
{#if customer_id}
	<div class="mt-2 mb-2">
		<Button
			endIcon={{ icon: ExternalLink }}
			href="/api/w/{$workspaceStore}/workspaces/billing_portal">Customer Portal</Button
		>
		<p class="text-xs text-tertiary mt-1">
			See invoices, change billing information or subscription details
		</p>
	</div>
{/if}

<div class="text-xs my-4">
	{#if premiumInfo?.premium}
		<div class="flex flex-col gap-0.5">
			{#if plan}
				<div class="text-base inline font-bold leading-8 mb-2">
					Current plan: {capitalize(plan)} plan{plan === 'team'
						? ` (${premiumInfo.seats} seat${premiumInfo.seats > 1 ? 's' : ''})`
						: ''}
				</div>
			{:else}
				<div class="inline text-lg font-bold">Current plan: Free plan</div>
			{/if}

			{#if plan}
				<div class="w-full">
					<DataTable>
						<tbody class="divide-y">
							<tr>
								<Cell first>
									Authors
									<Tooltip>
										Actual pricing is calculated on the MAXIMUM number of users in a given billing
										period, see the customer portal for more info.
									</Tooltip>
								</Cell>
								<Cell last numeric>
									<div class="text-base">
										{premiumInfo.authorNb}
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
								<Cell first
									><div class="font-semibold">Minimum number of seats needed for users</div></Cell
								>
								<Cell last numeric>
									<div class="text-base font-bold">
										u = ceil({premiumInfo.authorNb} + {premiumInfo.operatorNb}/2) = {premiumInfo.seatsFromUsers}
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
								<Cell first
									><div class="font-semibold">Minimum number of seats needed for computations</div
									></Cell
								>
								<Cell last numeric>
									<div class="text-base font-bold">
										c = ceil({premiumInfo.usage} / 10 000) = {premiumInfo.seatsFromComps}
									</div>
								</Cell>
							</tr>
							<tr>
								<Cell first
									><div
										class={twMerge(
											'font-semibold',
											plan === 'team' && premiumInfo.usedSeats > premiumInfo.seats
												? 'text-red-500'
												: ''
										)}
										>Used seats <Tooltip
											>Highest between seats from authors + operators and seats from computations
										</Tooltip>{plan === 'team' && premiumInfo.usedSeats > premiumInfo.seats
											? ' > Paid seats'
											: ''}</div
									></Cell
								>
								<Cell last numeric>
									<div
										class={twMerge(
											'text-base font-bold',
											plan === 'team' && premiumInfo.usedSeats > premiumInfo.seats
												? 'text-red-500'
												: ''
										)}
									>
										max(u, c) = max({premiumInfo.seatsFromUsers}, {premiumInfo.seatsFromComps}) = {premiumInfo.usedSeats}{premiumInfo.usedSeats >
										premiumInfo.seats
											? ` > ${premiumInfo.seats}`
											: ''}
									</div>
								</Cell>
							</tr>
						</tbody>
					</DataTable>

					{#if plan === 'team' && premiumInfo.usedSeats > premiumInfo.seats}
						<p class="text-red-500 mt-2 text-right text-base"
							>You have exceeded your allowed number of seats, please update your plan in the
							customer portal.
						</p>
					{/if}
				</div>
			{/if}
		</div>
	{:else}
		This workspace is <b>NOT</b> on a team plan. Users use their global free-tier quotas when doing executions
		in this workspace. Upgrade to a Team or Enterprise plan to unlock unlimited executions in this workspace.
	{/if}
</div>

<div class="text-base font-bold leading-8 mb-2 pt-8"> All plans </div>

<div class="grid grid-cols-1 md:grid-cols-3 gap-4">
	{#each Object.entries(plans) as [planTitle, planDesc]}
		<div class="box p-4 text-xs flex flex-col h-full overflow-hidden prose-sm rounded-md">
			<h2 class="mb-4">{planTitle}</h2>
			<ul class="list-disc text-sm p-4">
				{#each planDesc as item}
					<li class="mt-2">{@html item}</li>
				{/each}
			</ul>

			<div class="grow" />
			{#if planTitle == 'Team'}
				{#if plan != 'team'}
					<div class="mt-4 mx-auto">
						{#if plan != 'enterprise'}
							<Button
								size="lg"
								color="dark"
								href="/api/w/{$workspaceStore}/workspaces/checkout?plan=team{premiumInfo?.usedSeats
									? `&seats=${premiumInfo.usedSeats}`
									: ''}"
							>
								Upgrade to the Team plan</Button
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
						<Button size="xs" color="dark" href="https://www.windmill.dev/pricing" target="_blank">
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
					<Badge class="mx-auto text-md font-semibold">Workspace is on the free plan</Badge>
				{/if}
			{/if}
		</div>
	{/each}
</div>
<div class="flex flex-col gap-1 my-8 w-full items-center">
	<div class="text-primary text-md font-semibold"> Frequently asked questions </div>

	<div class="flex flex-col gap-4">
		<div>
			<div class="text-sm mb-1 text-secondary"> What is an execution? </div>
			<div class="text-xs max-w-xl border-b pb-4 text-tertiary">
				The single credit-unit is called an "execution". An execution corresponds to a single job
				whose duration is less than 1s. For any additional seconds of computation, an additional
				execution is accounted for. Jobs are executed on one powerful virtual CPU with 2Gb of
				memory. Most jobs will take less than 200ms to execute.
			</div>
		</div>
		<div>
			<div class="text-sm mb-1 text-secondary">
				What is the difference between an author and an operator?
			</div>
			<div class="text-xs max-w-xl text-tertiary">
				An author can write scripts/flows/apps/variables/resources. An operator can only run/view
				them.
			</div>
		</div>
	</div>
</div>
