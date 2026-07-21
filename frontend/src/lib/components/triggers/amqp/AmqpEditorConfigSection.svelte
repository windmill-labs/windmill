<script lang="ts">
	import Section from '$lib/components/Section.svelte'
	import Required from '$lib/components/Required.svelte'
	import { Plus, X } from 'lucide-svelte'
	import Subsection from '$lib/components/Subsection.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import type { AmqpExchange, AmqpOptions } from '$lib/gen'
	import Button from '$lib/components/common/button/Button.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { fade } from 'svelte/transition'
	import { emptyStringTrimmed } from '$lib/utils'
	import TestTriggerConnection from '../TestTriggerConnection.svelte'
	import Tooltip from '$lib/components/Tooltip.svelte'
	import TestingBadge from '../testingBadge.svelte'
	import { workspaceStore } from '$lib/stores'
	import { getTriggerWorkspace } from '$lib/components/triggers/triggerWorkspace'

	interface Props {
		can_write?: boolean
		amqp_resource_path?: string
		queue_name?: string
		exchange?: AmqpExchange | undefined
		options?: AmqpOptions | undefined
		isValid?: boolean
		showTestingBadge?: boolean
	}

	let {
		can_write = false,
		amqp_resource_path = $bindable(''),
		queue_name = $bindable(''),
		exchange = $bindable(undefined),
		options = $bindable(undefined),
		isValid = $bindable(false),
		showTestingBadge = false
	}: Props = $props()

	const triggerWs = getTriggerWorkspace()
	const wsId = $derived(triggerWs?.() ?? $workspaceStore)

	// Toggling the exchange binding on/off. Off means the queue is consumed
	// directly with no exchange binding.
	let bindExchange = $state(false)
	$effect(() => {
		bindExchange = !!exchange
	})

	$effect(() => {
		isValid = !emptyStringTrimmed(amqp_resource_path) && !emptyStringTrimmed(queue_name)
	})
</script>

<div class="flex flex-col gap-12">
	<Section label="AMQP">
		{#snippet header()}
			{#if showTestingBadge}
				<TestingBadge />
			{/if}
		{/snippet}
		<div class="flex flex-col w-full gap-12">
			<Subsection label="Connection setup">
				<ResourcePicker
					workspace={wsId}
					resourceType="amqp"
					disabled={!can_write}
					bind:value={amqp_resource_path}
				/>
				{#if !emptyStringTrimmed(amqp_resource_path)}
					<TestTriggerConnection kind="amqp" args={{ amqp_resource_path }} />
				{/if}
			</Subsection>

			<Subsection label="Queue">
				<p class="text-xs text-primary mb-2">
					Name of the queue to consume messages from<Required required={true} />
				</p>
				<input
					type="text"
					bind:value={queue_name}
					disabled={!can_write}
					placeholder="queue name"
					autocomplete="off"
				/>
			</Subsection>

			<Subsection label="Exchange binding">
				<Toggle
					textClass="font-normal text-sm"
					color="nord"
					size="xs"
					checked={bindExchange}
					disabled={!can_write}
					on:change={(ev) => {
						if (ev.detail) {
							exchange = { exchange_name: '', routing_keys: [] }
						} else {
							exchange = undefined
						}
					}}
					options={{
						right: 'Bind the queue to an exchange',
						rightTooltip:
							'Declare the queue and bind it to an exchange for each routing key, so messages published to the exchange are routed to it.'
					}}
					class="py-1"
				/>

				{#if exchange}
					<div class="flex flex-col gap-2 mt-2">
						<label class="flex flex-col w-full gap-1">
							<span class="text-secondary text-sm">Exchange name</span>
							<input
								type="text"
								bind:value={exchange.exchange_name}
								disabled={!can_write}
								placeholder="exchange name"
								autocomplete="off"
							/>
						</label>

						<span class="text-secondary text-sm mt-2">
							Routing keys
							<Tooltip>The routing keys used to bind the queue to the exchange.</Tooltip>
						</span>
						{#each exchange.routing_keys ?? [] as _, i}
							<div class="flex w-full gap-2 items-center">
								<input
									type="text"
									bind:value={exchange.routing_keys![i]}
									disabled={!can_write}
									placeholder="routing key"
									autocomplete="off"
								/>
								<button
									transition:fade|local={{ duration: 100 }}
									class="rounded-full p-1 bg-surface-secondary duration-200 hover:bg-surface-hover"
									aria-label="Clear"
									onclick={() => {
										if (exchange) {
											exchange.routing_keys = (exchange.routing_keys ?? []).filter(
												(_, index) => index !== i
											)
										}
									}}
								>
									<X size={14} />
								</button>
							</div>
						{/each}
						<div class="flex items-baseline">
							<Button
								variant="default"
								size="xs"
								btnClasses="mt-1"
								disabled={!can_write}
								on:click={() => {
									if (exchange) {
										exchange.routing_keys = [...(exchange.routing_keys ?? []), '']
									}
								}}
								startIcon={{ icon: Plus }}
							>
								Add routing key
							</Button>
						</div>
					</div>
				{/if}
			</Subsection>
		</div>
	</Section>
</div>
