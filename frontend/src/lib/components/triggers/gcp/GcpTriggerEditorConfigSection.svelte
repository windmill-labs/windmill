<script lang="ts">
	import Section from '$lib/components/Section.svelte'
	import Required from '$lib/components/Required.svelte'
	import ResourcePicker from '$lib/components/ResourcePicker.svelte'
	import { emptyStringTrimmed, sendUserToast } from '$lib/utils'
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
	import { base } from '$lib/base'
	import Toggle from '$lib/components/Toggle.svelte'
	import { workspaceStore } from '$lib/stores'

	import { Button, Url } from '$lib/components/common'
	import { RefreshCw } from 'lucide-svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import TestingBadge from '../testingBadge.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import { safeSelectItems } from '$lib/components/select/utils.svelte'

	let topic_items: string[] = $state([])
	let subscription_items: string[] = $state([])
	let loadingTopic = $state(false)
	let loadingSubscription = $state(false)

	const DEFAULT_PUSH_CONFIG: PushConfig = {
		audience: getBaseUrl(),
		authenticate: false
	}

	async function loadAllPubSubTopicsFromProject() {
		if (!emptyStringTrimmed(gcp_resource_path)) {
			try {
				loadingTopic = true
				topic_items = await GcpTriggerService.listGoogleTopics({
					workspace: $workspaceStore!,
					path: gcp_resource_path
				})
			} catch (error) {
				sendUserToast(error.body, true)
			}
			loadingTopic = false
		}
	}

	async function loadAllSubscriptionFromGooglePubSubTopic() {
		if (!emptyStringTrimmed(gcp_resource_path) && !emptyStringTrimmed(topic_id)) {
			try {
				loadingSubscription = true
				subscription_items = await GcpTriggerService.listAllTgoogleTopicSubscriptions({
					workspace: $workspaceStore!,
					path: gcp_resource_path,
					requestBody: {
						topic_id
					}
				})
			} catch (error) {
				sendUserToast(error.body, true)
			}
			loadingSubscription = false
		}
	}

	interface Props {
		can_write?: boolean
		headless?: boolean
		isValid?: boolean
		gcp_resource_path?: string
		subscription_id?: string
		topic_id?: string
		delivery_type?: DeliveryType | undefined
		delivery_config: PushConfig | undefined
		subscription_mode?: SubscriptionMode
		base_endpoint?: string
		path?: string
		showTestingBadge?: boolean
		cloud_subscription_id?: string
		create_update_subscription_id?: string
		auto_acknowledge_msg: boolean
	}

	let {
		can_write = false,
		headless = false,
		isValid = $bindable(false),
		gcp_resource_path = $bindable(''),
		subscription_id = $bindable(''),
		topic_id = $bindable(''),
		delivery_type = $bindable('pull'),
		delivery_config = $bindable(),
		subscription_mode = $bindable('create_update'),
		base_endpoint = $bindable(getBaseUrl()),
		auto_acknowledge_msg = $bindable(true),
		path = '',
		showTestingBadge = false,
		cloud_subscription_id = $bindable(''),
		create_update_subscription_id = $bindable('')
	}: Props = $props()

	if (gcp_resource_path) {
		loadAllPubSubTopicsFromProject()
	}

	$effect(() => {
		isValid =
			!emptyStringTrimmed(gcp_resource_path) &&
			!emptyStringTrimmed(topic_id) &&
			!emptyStringTrimmed(subscription_id)
	})
	$effect(() => {
		if (!delivery_type) {
			delivery_type = 'pull'
		}
	})
	$effect(() => {
		if (!delivery_config) {
			delivery_config = DEFAULT_PUSH_CONFIG
		}
	})

	function getBaseUrl() {
		return `${window.location.origin}${base}/api/gcp/w/${$workspaceStore!}`
	}

	$effect(() => {
		!base_endpoint && (base_endpoint = getBaseUrl())
	})

	$effect(() => {
		if (emptyStringTrimmed(subscription_id) && !emptyStringTrimmed(path)) {
			subscription_id = `windmill-${$workspaceStore!}-${path.replaceAll('/', '_')}`
		}
	})
</script>

<div>
	<Section label="GCP Pub/Sub" {headless}>
		{#snippet header()}
			{#if showTestingBadge}
				<TestingBadge />
			{/if}
		{/snippet}
		<div class="flex flex-col w-full gap-4">
			<Subsection label="Connection setup">
				<div class="flex flex-col gap-1 mt-2">
					<ResourcePicker
						resourceType="gcloud"
						bind:value={
							() => gcp_resource_path,
							(v) => {
								gcp_resource_path = v
								loadAllPubSubTopicsFromProject()
							}
						}
					/>
					{#if !emptyStringTrimmed(gcp_resource_path)}
						<TestTriggerConnection kind="gcp" args={{ gcp_resource_path }} />
					{/if}
				</div>
			</Subsection>

			{#if gcp_resource_path}
				<div class="flex flex-col gap-1">
					<Subsection
						label="Topic"
						tooltip="Select the Pub/Sub topic that this subscription will be attached to. Messages published to this topic will be delivered to your subscription."
					>
						<div class="flex flex-row gap-1 mt-2">
							<Select
								loading={loadingTopic}
								disablePortal
								clearable
								class="grow shrink"
								bind:value={
									() => topic_id,
									(t) => {
										topic_id = t
										loadAllSubscriptionFromGooglePubSubTopic()
									}
								}
								items={safeSelectItems(topic_items)}
								placeholder="Choose a topic"
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
					</Subsection>
				</div>
			{/if}
			{#if !emptyStringTrimmed(gcp_resource_path) && !emptyStringTrimmed(topic_id)}
				<Section
					label="Subscription"
					tooltip="Choose whether to create or update a Pub/Sub subscription, or link an existing one from your Google Cloud project."
					documentationLink="https://www.windmill.dev/docs/core_concepts/gcp_triggers#subscription-setup"
				>
					<div class="flex flex-col gap-3">
						<ToggleButtonGroup
							bind:selected={subscription_mode}
							on:selected={(e) => {
								if (e.detail === 'existing' && subscription_items.length === 0) {
									loadAllSubscriptionFromGooglePubSubTopic()
								}
							}}
						>
							{#snippet children({ item })}
								<ToggleButton
									label="Create/Update"
									value="create_update"
									tooltip="Create a new subscription or update an existing one with custom settings"
									showTooltipIcon
									{item}
								/>
								<ToggleButton
									label="Existing subscription"
									value="existing"
									tooltip="Select an existing subscription from GCP Pub/Sub"
									showTooltipIcon
									{item}
								/>
							{/snippet}
						</ToggleButtonGroup>

						{#if subscription_mode === 'create_update'}
							<Subsection
								label="Subscription id"
								tooltip="The unique identifier for the Pub/Sub subscription."
							>
								<div class="mt-2">
									<input
										type="text"
										autocomplete="off"
										placeholder="Enter subscription ID (leave empty to auto-generate)"
										bind:value={create_update_subscription_id}
										oninput={(event) => {
											subscription_id = event?.currentTarget.value
										}}
									/>
								</div>
							</Subsection>
							<div class="flex flex-col gap-2">
								<Subsection
									label="Delivery type"
									tooltip="Select the delivery type for the Pub/Sub subscription. If the subscription already exists and you want to keep it as-is, choose the same delivery type as in Google Cloud. You can switch the type here if the API allows it — otherwise, make the change directly in Google Cloud."
								>
									<div class="flex flex-col gap-2 mt-2">
										<ToggleButtonGroup bind:selected={delivery_type}>
											{#snippet children({ item })}
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
											{/snippet}
										</ToggleButtonGroup>
									</div>
								</Subsection>
								{#if delivery_type === 'push' && delivery_config}
									<div class="flex flex-col gap-3 mt-1">
										<div class="flex gap-2">
											<Url url={`${base_endpoint}/${path}`} label="Production URL" />
										</div>
										<Subsection label="Authenticate">
											<p class="text-xs mb-2 text-tertiary">
												Enable Google Cloud authentication for push delivery using a verified token.<Required
													required={true}
												/>
											</p>
											<Toggle bind:checked={delivery_config.authenticate} />
										</Subsection>
										{#if delivery_config.authenticate}
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
									</div>
								{:else if delivery_type === 'pull'}
									<Subsection
										label="Auto-acknowledge messages"
										tooltip="When enabled (recommended), Windmill automatically acknowledges Pub/Sub messages after successful processing. When disabled, your script/flow must explicitly acknowledge each message."
									>
										<div class="mt-2">
											<Toggle bind:checked={auto_acknowledge_msg} />
										</div>
										{#if !auto_acknowledge_msg}
											<div class="mt-3">
												<Alert size="xs" type="warning" title="Manual Acknowledgment Required">
													You must acknowledge each message in your script/flow code using the `ack_id` provided in the payload data. If messages are not acknowledged within the acknowledgment deadline (default: 10 seconds), GCP will automatically redeliver them, causing Windmill to reprocess the same messages repeatedly.
												</Alert>
											</div>
										{/if}
									</Subsection>
								{/if}
							</div>
						{:else if subscription_mode === 'existing'}
							<div class="flex flex-col gap-3">
								<div class="flex gap-1">
									<Select
										loading={loadingSubscription}
										disablePortal
										clearable
										class="grow shrink"
										bind:value={
											() => cloud_subscription_id,
											(t) => ((subscription_id = t), (cloud_subscription_id = t))
										}
										onClear={() => (subscription_id = '')}
										items={safeSelectItems(subscription_items)}
										placeholder="Choose a subscription"
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
								<Alert title="Push Subscription URL Requirements" type="warning">
									If the subscription uses <strong>push delivery</strong>, its endpoint URL must
									match the following format: <strong>{`${base_endpoint}/${path}`}/*</strong>,
									meaning it must start with
									<strong>{`${base_endpoint}/${path}`}</strong> followed by any path segment.
								</Alert>
							</div>
						{/if}
					</div>
				</Section>
			{/if}
		</div>
	</Section>
</div>
