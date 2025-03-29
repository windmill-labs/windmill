<script lang="ts">
	import Section from '$lib/components/Section.svelte'
	import CaptureSection, { type CaptureInfo } from '../CaptureSection.svelte'
	import CaptureTable from '../CaptureTable.svelte'
	import Required from '$lib/components/Required.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import { emptyStringTrimmed } from '$lib/utils'
	import TestTriggerConnection from '../TestTriggerConnection.svelte'
	import Subsection from '$lib/components/Subsection.svelte'
	import type { DeliveryType, PushConfig } from '$lib/gen'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import { getHttpRoute } from '../http/utils'
	import { workspaceStore } from '$lib/stores'
	import { Badge } from '$lib/components/common'

	export let can_write: boolean = false
	export let headless: boolean = false
	export let showCapture: boolean = false
	export let captureTable: CaptureTable | undefined = undefined
	export let captureInfo: CaptureInfo | undefined = undefined
	export let isValid: boolean = false
	export let gcp_resource_path = ''
	export let subscription_id = ''
	export let delivery_type: DeliveryType = 'push'
	export let delivery_config: PushConfig = { route_path: '', audience: '' }

	function isDeliveryTypeValid() {
		if (delivery_type === 'push' && emptyStringTrimmed(delivery_config.route_path)) {
			return false
		}
		return true
	}

	$: isValid = !emptyStringTrimmed(gcp_resource_path) && !emptyStringTrimmed(subscription_id) && isDeliveryTypeValid()
	$: fullRoute = getHttpRoute(delivery_config.route_path, false, $workspaceStore ?? '', 'gcp/push')
</script>

<div>
	{#if showCapture && captureInfo}
		<CaptureSection
			captureType="gcp"
			disabled={!isValid}
			{captureInfo}
			on:captureToggle
			on:applyArgs
			on:updateSchema
			on:addPreprocessor
			on:testWithArgs
			bind:captureTable
		/>
	{/if}
	<Section label="GCP" {headless}>
		<div class="flex flex-col w-full gap-4">
			<Subsection label="Connection setup">
				<div class="flex flex-col gap-3">
					<div class="flex flex-col gap-1">
						<p class="text-xs mb-1 text-tertiary">
							Select a gcp resource. <Required required={true} />
						</p>
						<ResourcePicker
							resourceType="google_cloud_auth_service_account"
							bind:value={gcp_resource_path}
						/>
						{#if isValid}
							<TestTriggerConnection kind="gcp" args={{ gcp_resource_path, subscription_id }} />
						{/if}
					</div>
				</div>
			</Subsection>
			<div class="flex flex-col gap-1">
				<p class="text-xs mb-1 text-tertiary">
					Enter the ID of an existing Pub/Sub subscription to connect with. This form does not
					create a new subscription. <Required required={true} />
				</p>
				<input
					type="text"
					autocomplete="off"
					placeholder="susbcription_id"
					bind:value={subscription_id}
					disabled={!can_write}
				/>
			</div>
			{#if !emptyStringTrimmed(subscription_id)}
				<div class="flex flex-col gap-2">
					<Subsection label="Delivery type">
						<div class="flex flex-col">
							<p class="text-xs mb-1 text-tertiary">
								Select the delivery type that matches the configuration of the existing Pub/Sub
								subscription id you entered above. <Required required={true} />
							</p>
							<ToggleButtonGroup bind:selected={delivery_type} let:item>
								<ToggleButton
									label="Pull"
									tooltip="Connect to an existing Pub/Sub subscription using the pull delivery type, where your service polls for messages."
									value="pull"
									showTooltipIcon
									{item}
								/>
								<ToggleButton
									label="Push"
									tooltip="Connect to an existing Pub/Sub subscription using the push delivery type, where Pub/Sub sends messages to a choosen endpoint."
									showTooltipIcon
									value="push"
									{item}
								/>
							</ToggleButtonGroup>
						</div>
					</Subsection>
					{#if delivery_type === 'push'}
						<Subsection label="Route path">
							<div class="flex flex-col w-full gap-3">
								<div class="flex flex-col">
									<p class="text-xs mb-1 text-tertiary">
										Enter the relative route path that matches the push endpoint configured for the
										existing Pub/Sub subscription. This is where Pub/Sub will send messages. <Required
											required={true}
										/>
									</p>
									<input
										type="text"
										autocomplete="off"
										placeholder="route_path"
										bind:value={delivery_config.route_path}
										disabled={!can_write}
									/>
								</div>
								<div class="flex justify-start w-full">
									<Badge
										color="gray"
										class="center-center !bg-surface-secondary !text-tertiary !w-[90px] !h-[24px] rounded-r-none border"
									>
										Full endpoint
									</Badge>
									<input
										type="text"
										readonly
										value={fullRoute}
										size={fullRoute.length || 50}
										class="font-mono !text-xs max-w-[calc(100%-70px)] !w-auto !h-[24px] !py-0 !border-l-0 !rounded-l-none"
										on:focus={({ currentTarget }) => {
											currentTarget.select()
										}}
									/>
								</div>
							</div>
						</Subsection>
						<Subsection label="Audience">
							<p class="text-xs mb-2 text-tertiary">
								Provide the expected audience value for verifying OIDC tokens in push requests.
								If left empty, the full URL of the endpoint will be used as the default audience: <code>{fullRoute}</code>.
							</p>
							<input
								type="text"
								autocomplete="off"
								placeholder="audience"
								bind:value={delivery_config.audience}
								disabled={!can_write}
							/>
						</Subsection>
					{/if}
				</div>
			{/if}
		</div>
	</Section>
</div>
