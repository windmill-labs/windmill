<script lang="ts">
	import Section from '$lib/components/Section.svelte'
	import CaptureSection, { type CaptureInfo } from '../CaptureSection.svelte'
	import CaptureTable from '../CaptureTable.svelte'
	import Required from '$lib/components/Required.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import { emptyStringTrimmed } from '$lib/utils'
	import TestTriggerConnection from '../TestTriggerConnection.svelte'
	import Subsection from '$lib/components/Subsection.svelte'
	import {
		GcpTriggerService,
		type DeliveryType,
		type PushConfig,
		type SubscriptionMode
	} from '$lib/gen'
	import ToggleButtonGroup from '$lib/components/common/toggleButton-v2/ToggleButtonGroup.svelte'
	import ToggleButton from '$lib/components/common/toggleButton-v2/ToggleButton.svelte'
	import Label from '$lib/components/Label.svelte'
	import ClipboardPanel from '$lib/components/details/ClipboardPanel.svelte'
	import { base } from '$lib/base'
	import Toggle from '$lib/components/Toggle.svelte'
	import { SELECT_INPUT_DEFAULT_STYLE } from '$lib/defaults'
	import DarkModeObserver from '$lib/components/DarkModeObserver.svelte'
	import Select from '$lib/components/apps/svelte-select/lib/Select.svelte'
	import { workspaceStore } from '$lib/stores'

	import { Button } from '$lib/components/common'
	import { RefreshCw } from 'lucide-svelte'

	export let can_write: boolean = false
	export let headless: boolean = false
	export let showCapture: boolean = false
	export let captureTable: CaptureTable | undefined = undefined
	export let captureInfo: CaptureInfo | undefined = undefined
	export let isValid: boolean = false
	export let gcp_resource_path = ''
	export let subscription_id = ''
	export let topic_id = ''
	export let delivery_type: DeliveryType | undefined
	export let delivery_config: PushConfig | undefined
	export let subscription_mode: SubscriptionMode = 'create_update'
	let topic_items: string[] = []
	let subscription_items: string[] = []
	let darkMode = false
	let base_endpoint = `${window.location.origin}${base}`

	const DEFAULT_PUSH_CONFIG: PushConfig = {
		route_path: '',
		audience: '',
		authenticate: false,
		base_endpoint
	}

	async function loadAllPubSubTopicsFromProject() {
		if (!emptyStringTrimmed(gcp_resource_path)) {
			topic_items = await GcpTriggerService.listGoogleTopics({
				workspace: $workspaceStore!,
				path: gcp_resource_path
			})
		}
	}

	async function loadAllSubscriptionFromGooglePubSubTopic() {
		if (!emptyStringTrimmed(gcp_resource_path) && !emptyStringTrimmed(topic_id)) {
			subscription_items = await GcpTriggerService.listAllTgoogleTopicSubscriptions({
				workspace: $workspaceStore!,
				path: gcp_resource_path,
				requestBody: {
					topic_id
				}
			})
		}
	}

	let cloud_subscription_id = subscription_id
	let create_update_subscription_id = subscription_id

	$: isValid = !emptyStringTrimmed(gcp_resource_path) && !emptyStringTrimmed(topic_id)
	$: if (!delivery_type) {
		delivery_type = 'pull'
	}
	$: !delivery_config && (delivery_config = DEFAULT_PUSH_CONFIG)
</script>

<DarkModeObserver bind:darkMode />

<div>
	{#if showCapture && captureInfo}
		{@const captureURL = `${location.origin}${base}/api/w/${$workspaceStore}/capture_u/gcp/${
			captureInfo.isFlow ? 'flow' : 'script'
		}/${captureInfo.path.replaceAll('/', '.')}/${delivery_config?.route_path ?? ''}`}
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
		>
			{#if delivery_type === 'push'}
				<Label label="URL">
					<ClipboardPanel content={captureURL} disabled={!captureInfo.active} />
				</Label>
			{/if}
		</CaptureSection>
	{/if}
	<Section label="GCP" {headless}>
		<div class="flex flex-col w-full gap-4">
			<Subsection label="Connection setup">
				<div class="flex flex-col gap-3">
					<div class="flex flex-col gap-1">
						<p class="text-xs mb-1 text-tertiary">
							Select a GCP resource.<Required required={true} />
						</p>
						<ResourcePicker
							on:change={() => {
								loadAllPubSubTopicsFromProject()
							}}
							resourceType="gcloud"
							bind:value={gcp_resource_path}
						/>
						{#if !emptyStringTrimmed(gcp_resource_path)}
							<TestTriggerConnection kind="gcp" args={{ gcp_resource_path }} />
						{/if}
					</div>
				</div>
			</Subsection>

			<div class="flex flex-col gap-1">
				<p class="text-xs mb-1 text-tertiary">
					Enter the ID of an existing Pub/Sub topic you'd like to connect to.<Required
						required={true}
					/>
				</p>
				<div class="flex gap-1">
					<Select
						class="grow shrink max-w-full"
						on:change={(e) => {
							topic_id = e.detail.value
							loadAllSubscriptionFromGooglePubSubTopic()
						}}
						on:clear={() => {
							topic_id = ''
						}}
						value={topic_id}
						items={topic_items}
						placeholder="Choose a topic"
						inputStyles={SELECT_INPUT_DEFAULT_STYLE.inputStyles}
						containerStyles={darkMode
							? SELECT_INPUT_DEFAULT_STYLE.containerStylesDark
							: SELECT_INPUT_DEFAULT_STYLE.containerStyles}
						portal={false}
					/>
					<Button
						disabled={!can_write}
						variant="border"
						color="light"
						wrapperClasses="self-stretch"
						on:click={loadAllPubSubTopicsFromProject}
						startIcon={{ icon: RefreshCw }}
						iconOnly
					/>
				</div>
			</div>
			{#if !emptyStringTrimmed(gcp_resource_path) && !emptyStringTrimmed(topic_id)}
				<ToggleButtonGroup bind:selected={subscription_mode} let:item>
					<ToggleButton
						label="Create/Update"
						value="create_update"
						tooltip="Create a new subscription or update an existing one with custom settings"
						showTooltipIcon
						{item}
					/>
					<ToggleButton
						label="Existing Subscription"
						value="existing"
						tooltip="Connect to an existing subscription in GCP Pub/Sub"
						showTooltipIcon
						{item}
					/>
				</ToggleButtonGroup>
				<div class="flex flex-col gap-1">
					{#if subscription_mode === 'create_update'}
						<p class="text-xs mb-1 text-tertiary">
							The ID of the subscription you want to create/update. if left empty it will be
							auto-generated by the server</p
						>
						<input
							type="text"
							autocomplete="off"
							placeholder="Enter subscription ID (leave empty to auto-generate)"
							bind:value={create_update_subscription_id}
							on:input={(event) => {
								subscription_id = event?.currentTarget.value
							}}
						/>

						<div class="flex flex-col gap-2">
							<Subsection
								label="Delivery type"
								tooltip="Select the delivery type for the Pub/Sub subscription. If the subscription already exists and you want to keep it as-is, choose the same delivery type as in Google Cloud. You can switch the type here if the API allows it — otherwise, make the change directly in Google Cloud."
							>
								<div class="flex flex-col gap-2 mt-2">
									<ToggleButtonGroup bind:selected={delivery_type} let:item>
										<ToggleButton
											label="Pull"
											tooltip="Create a subscription where your service will pull messages from the queue. Suitable for services that periodically check for new messages."
											value="pull"
											showTooltipIcon
											{item}
										/>
										<ToggleButton
											label="Push"
											tooltip="Windmill will auto-generate a push endpoint for this subscription. You must not modify this endpoint in Google Cloud, as it is managed internally by Windmill."
											showTooltipIcon
											value="push"
											{item}
										/>
									</ToggleButtonGroup>
								</div>
							</Subsection>
							{#if delivery_type === 'push' && delivery_config}
								<div class="flex flex-col gap-3 mt-1">
									{#if delivery_config.route_path}
										<Subsection
											label="URL"
											tooltip="The URL of the service that receives push messages."
										>
											<input
												type="text"
												autocomplete="off"
												placeholder="URL"
												disabled
												value={`${window.location.origin}${base}/${delivery_config.route_path ?? ''}`}
											/>
										</Subsection>
										<Subsection
											label="Audience"
											tooltip="Provide the expected audience value for verifying OIDC tokens in push requests. If
									left empty, the URL of the endpoint will be used as the default audience"
										>
											<input
												type="text"
												autocomplete="off"
												placeholder="audience"
												bind:value={delivery_config.audience}
												disabled={!can_write}
											/>
										</Subsection>
									{/if}
									<Subsection label="Authenticate">
										<p class="text-xs mb-2 text-tertiary">
											Enable Google Cloud authentication for push delivery using a verified token.<Required
												required={true}
											/>
										</p>
										<Toggle bind:checked={delivery_config.authenticate} />
									</Subsection>
								</div>
							{/if}
						</div>
					{:else if subscription_mode === 'existing'}
						<p class="text-xs mb-1 text-tertiary">
							The ID of the existing subscription.<Required required={true} />
						</p>
						<div class="flex gap-1">
							<Select
								class="grow shrink max-w-full"
								on:change={(e) => {
									subscription_id = e.detail.value
									cloud_subscription_id = e.detail.value
								}}
								on:clear={() => {
									subscription_id = ''
								}}
								value={cloud_subscription_id}
								items={subscription_items}
								placeholder="Choose a subscription"
								inputStyles={SELECT_INPUT_DEFAULT_STYLE.inputStyles}
								containerStyles={darkMode
									? SELECT_INPUT_DEFAULT_STYLE.containerStylesDark
									: SELECT_INPUT_DEFAULT_STYLE.containerStyles}
								portal={false}
							/>
							<Button
								disabled={!can_write}
								variant="border"
								color="light"
								wrapperClasses="self-stretch"
								on:click={loadAllSubscriptionFromGooglePubSubTopic}
								startIcon={{ icon: RefreshCw }}
								iconOnly
							/>
						</div>
					{/if}
				</div>
			{/if}
		</div>
	</Section>
</div>
