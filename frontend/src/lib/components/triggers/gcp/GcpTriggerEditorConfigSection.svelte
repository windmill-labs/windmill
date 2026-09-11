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
	import { userStore, workspaceStore } from '$lib/stores'
	import { getTriggerWorkspace } from '$lib/components/triggers/triggerWorkspace'

	import { Button, Url } from '$lib/components/common'
	import TextInput from '$lib/components/text_input/TextInput.svelte'
	import { RefreshCw } from 'lucide-svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import TestingBadge from '../testingBadge.svelte'
	import Select from '$lib/components/select/Select.svelte'
	import { safeSelectItems } from '$lib/components/select/utils.svelte'

	// Declared before `DEFAULT_PUSH_CONFIG` / the `base_endpoint` prop default,
	// which call `getBaseUrl()` (a `wsId` reader) during component init.
	const triggerWs = getTriggerWorkspace()
	const wsId = $derived(triggerWs?.() ?? $workspaceStore)

	let topic_items: string[] = $state([])
	let subscription_items: string[] = $state([])
	let loadingTopic = $state(false)
	let loadingSubscription = $state(false)

	const DEFAULT_PUSH_CONFIG: PushConfig = {
		audience: getBaseUrl(),
		authenticate: false
	}

	async function loadAllPubSubTopicsFromProject() {
		// Listing is admin-only under application default credentials, and a non-admin viewing an
		// inherited trigger cannot change the topic anyway, so asking would only raise a 403 toast.
		if (!hasCredentials || blockedByAdminGate) {
			return
		}
		try {
			loadingTopic = true
			topic_items = usesDefaultCredentials
				? await GcpTriggerService.listGoogleTopicsWithDefaultCredentials({
						workspace: wsId!,
						projectId: project_id
					})
				: await GcpTriggerService.listGoogleTopics({
						workspace: wsId!,
						path: gcp_resource_path!,
						projectId: project_id
					})
		} catch (error) {
			sendUserToast(error.body, true)
		}
		loadingTopic = false
	}

	async function loadAllSubscriptionFromGooglePubSubTopic() {
		if (!hasCredentials || blockedByAdminGate || emptyStringTrimmed(topic_id)) {
			return
		}
		try {
			loadingSubscription = true
			const requestBody = { topic_id, project_id }
			subscription_items = usesDefaultCredentials
				? await GcpTriggerService.listAllTgoogleTopicSubscriptionsWithDefaultCredentials({
						workspace: wsId!,
						requestBody
					})
				: await GcpTriggerService.listAllTgoogleTopicSubscriptions({
						workspace: wsId!,
						path: gcp_resource_path!,
						requestBody
					})
		} catch (error) {
			sendUserToast(error.body, true)
		}
		loadingSubscription = false
	}

	/** Subscriptions come back fully qualified so cross-project ones survive the round trip. The
	 * project is what tells two same-named subscriptions apart, which is exactly the case a
	 * cross-project topic creates, so it stays in the label rather than being trimmed away. */
	function subscriptionLabel(name: string): string {
		const parts = name.split('/')
		const id = parts.pop() ?? name
		const project = parts.length >= 2 ? parts[1] : undefined
		return project ? `${id} (${project})` : id
	}

	interface Props {
		can_write?: boolean
		headless?: boolean
		isValid?: boolean
		gcp_resource_path?: string | undefined
		/** Authenticate as the server itself instead of with a `gcloud` resource. */
		use_default_credentials?: boolean
		/** Whether the config was *loaded* in that mode, as opposed to switched into it here. */
		loaded_uses_default_credentials?: boolean
		project_id?: string
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
		ack_deadline?: number
	}

	let {
		can_write = false,
		headless = false,
		isValid = $bindable(false),
		gcp_resource_path = $bindable(),
		use_default_credentials = $bindable(),
		loaded_uses_default_credentials = false,
		project_id = $bindable(),
		subscription_id = $bindable(''),
		topic_id = $bindable(''),
		delivery_type = $bindable('pull'),
		delivery_config = $bindable(),
		subscription_mode = $bindable('create_update'),
		base_endpoint = $bindable(getBaseUrl()),
		auto_acknowledge_msg = $bindable(true),
		ack_deadline = $bindable(),
		path = '',
		showTestingBadge = false,
		cloud_subscription_id = $bindable(''),
		create_update_subscription_id = $bindable('')
	}: Props = $props()

	/** Only workspace admins may point a trigger at the server's own GCP identity, which no
	 * resource ACL covers. The backend enforces this too; hiding it keeps a non-admin from
	 * building a config that cannot be saved. Someone who inherits such a trigger still sees the
	 * mode it is in. */
	const usesDefaultCredentials = $derived(use_default_credentials ?? false)
	// Keyed on the loaded mode, not the live one: reading the live mode would make the toggle a
	// one-way door, disabling itself the moment a non-admin switched an inherited ADC trigger away.
	const canUseDefaultCredentials = $derived(
		$userStore?.is_admin === true || loaded_uses_default_credentials
	)
	const hasCredentials = $derived(usesDefaultCredentials || !emptyStringTrimmed(gcp_resource_path))
	/** Saving re-provisions the subscription with the instance's credentials, so the backend runs
	 * the admin check on every write, not only when the mode is switched. A non-admin who inherits
	 * such a trigger can open it, so say why saving is unavailable instead of letting them hit a
	 * bare 403. */
	const blockedByAdminGate = $derived(usesDefaultCredentials && $userStore?.is_admin !== true)

	// One-shot on mount, so read the props rather than the derived: referencing `$derived` state
	// here captures its initial value anyway, and Svelte warns about it.
	if (gcp_resource_path || use_default_credentials) {
		loadAllPubSubTopicsFromProject()
	}

	function onCredentialsModeChange(useDefault: boolean) {
		use_default_credentials = useDefault
		gcp_resource_path = useDefault ? undefined : ''
		// The topic and subscription belong to the credentials that listed them. Keeping them
		// across a switch leaves the form valid and saveable against names the new credentials may
		// not have, or worse may have in a different project.
		topic_items = []
		subscription_items = []
		topic_id = ''
		subscription_id = ''
		cloud_subscription_id = ''
		create_update_subscription_id = ''
		if (useDefault) {
			loadAllPubSubTopicsFromProject()
		}
	}

	$effect(() => {
		isValid =
			hasCredentials &&
			!blockedByAdminGate &&
			!emptyStringTrimmed(topic_id) &&
			!emptyStringTrimmed(subscription_id)
	})
	$effect(() => {
		if (!delivery_type) {
			delivery_type = 'pull'
		} else if (delivery_type === 'push' && !delivery_config) {
			delivery_config = DEFAULT_PUSH_CONFIG
		}
	})
	function getBaseUrl() {
		return `${window.location.origin}${base}/api/gcp/w/${wsId!}`
	}

	$effect(() => {
		!base_endpoint && (base_endpoint = getBaseUrl())
	})

	$effect(() => {
		if (emptyStringTrimmed(subscription_id) && !emptyStringTrimmed(path)) {
			subscription_id = `windmill-${wsId!}-${path.replaceAll('/', '_')}`
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
				<div class="flex flex-col gap-3 mt-2">
					<ToggleButtonGroup
						selected={usesDefaultCredentials ? 'default' : 'resource'}
						on:selected={(e) => onCredentialsModeChange(e.detail === 'default')}
					>
						{#snippet children({ item })}
							<ToggleButton
								label="Service account"
								value="resource"
								tooltip="Authenticate with a service account key held in a GCP resource."
								showTooltipIcon
								{item}
							/>
							<ToggleButton
								label="Application default credentials"
								value="default"
								disabled={!canUseDefaultCredentials}
								tooltip={canUseDefaultCredentials
									? 'Authenticate as the Windmill server itself, using the credentials of its environment (workload identity, the metadata server, or GOOGLE_APPLICATION_CREDENTIALS).'
									: 'Workspace admins can authenticate as the Windmill server itself. Ask one to set this up.'}
								showTooltipIcon
								{item}
							/>
						{/snippet}
					</ToggleButtonGroup>

					{#if !usesDefaultCredentials}
						<ResourcePicker
							workspace={wsId}
							resourceType="gcloud"
							bind:value={
								() => gcp_resource_path,
								(v) => {
									gcp_resource_path = v
									loadAllPubSubTopicsFromProject()
								}
							}
						/>
					{/if}

					<Subsection
						label="Project ID"
						tooltip="The project topics and subscriptions are listed and created in. Leave empty to use the project of the credentials. Names given in full (projects/<project>/topics/<id>) are reached whatever this is set to."
					>
						<div class="mt-2">
							<!-- Typing does not refetch: every keystroke would be a Pub/Sub call for a
							     project id that is not finished being typed. The refresh button next to
							     the topic picker is what reloads the lists. -->
							<!-- Cleared means "unset", not an empty or whitespace-only string: that would
							     travel as `?project_id=`, dirty the config, and reach the column as a
							     value `empty_as_none` does not trim away. -->
							<TextInput
								bind:value={() => project_id ?? '', (v) => (project_id = emptyStringTrimmed(v) ? undefined : v)}
								inputProps={{
									placeholder: 'my-gcp-project',
									disabled: !can_write,
									autocomplete: 'off'
								}}
							/>
						</div>
					</Subsection>

					{#if blockedByAdminGate}
						<Alert title="Workspace admin required" type="info" size="xs">
							This trigger authenticates as the Windmill server. Saving changes to it needs
							workspace admin, because saving re-provisions the subscription with those credentials.
						</Alert>
					{/if}

					{#if hasCredentials}
						<TestTriggerConnection kind="gcp" args={{ gcp_resource_path, project_id }} />
					{/if}
				</div>
			</Subsection>

			{#if hasCredentials}
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
								variant="default"
								wrapperClasses="self-stretch"
								on:click={loadAllPubSubTopicsFromProject}
								startIcon={{ icon: RefreshCw }}
								iconOnly
							/>
						</div>
					</Subsection>
				</div>
			{/if}
			{#if hasCredentials && !emptyStringTrimmed(topic_id)}
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
											<p class="text-xs mb-2 text-primary">
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
										items={subscription_items.map((s) => ({
											value: s,
											label: subscriptionLabel(s)
										}))}
										placeholder="Choose a subscription"
									/>
									<Button
										disabled={!can_write}
										variant="default"
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
