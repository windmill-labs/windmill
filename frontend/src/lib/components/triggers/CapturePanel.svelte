<script lang="ts">
	import {
		Circle,
		CircleStop,
		Clipboard,
		Info,
		Mail,
		Play,
		Route,
		Save,
		Trash2,
		Unplug,
		Webhook
	} from 'lucide-svelte'
	import { base32 } from 'rfc4648'
	import ToggleButton from '../common/toggleButton-v2/ToggleButton.svelte'
	import ToggleButtonGroup from '../common/toggleButton-v2/ToggleButtonGroup.svelte'
	import KafkaIcon from '../icons/KafkaIcon.svelte'
	import SchemaForm from '../SchemaForm.svelte'
	import { enterpriseLicense, workspaceStore } from '$lib/stores'
	import {
		CaptureService,
		SettingService,
		type Capture,
		type CaptureConfig,
		type CaptureTriggerKind
	} from '$lib/gen'
	import Button from '../common/button/Button.svelte'
	import { capitalize, copyToClipboard, isObject, sendUserToast, sleep } from '$lib/utils'
	import { createEventDispatcher, onDestroy } from 'svelte'
	import { captureTriggerKindToTriggerKind, type TriggerKind } from '../triggers'
	import Label from '../Label.svelte'
	import ClipboardPanel from '../details/ClipboardPanel.svelte'
	import CopyableCodeBlock from '../details/CopyableCodeBlock.svelte'
	import { bash } from 'svelte-highlight/languages'
	import Popover from '../Popover.svelte'
	import { isCloudHosted } from '$lib/cloud'
	import Alert from '../common/alert/Alert.svelte'
	import CustomPopover from '../CustomPopover.svelte'
	import { page } from '$app/stores'
	import { convert } from '@redocly/json-to-json-schema'
	import SchemaViewer from '../SchemaViewer.svelte'

	export let isFlow: boolean
	export let path: string
	export let hasPreprocessor: boolean
	export let canHavePreprocessor: boolean
	export let newItem: boolean

	const dispatch = createEventDispatcher<{
		openTriggers: {
			kind: TriggerKind
			config: Record<string, any>
		}
		applyArgs: {
			kind: 'main' | 'preprocessor'
			args: Record<string, any> | undefined
		}
		addPreprocessor: null
		updateSchema: {
			schema: any
			redirect: boolean
		}
	}>()

	let selected: CaptureTriggerKind = 'webhook'
	let args: Record<string, any> = {}
	let active = false
	let isValid = true
	let testKind: 'preprocessor' | 'main' = 'main'

	$: hasPreprocessor && (testKind = 'preprocessor')

	$: selected && (isValid = true)

	const schemas = {
		http: {
			$schema: 'http://json-schema.org/draft-07/schema#',
			type: 'object',
			properties: {
				route_path: {
					type: 'string',
					description: "':myparam' for path params",
					title: 'Custom route path',
					pattern: '^[\\w-:]+(/[\\w-:]+)*$',
					customErrorMessage: 'Invalid route path'
				}
			},
			required: ['route_path']
		},
		websocket: {
			$schema: 'http://json-schema.org/draft-07/schema#',
			type: 'object',
			properties: {
				url: {
					type: 'string',
					title: 'URL',
					pattern: '^(ws:|wss:)//[^\\s]+$',
					customErrorMessage: 'Invalid websocket URL'
				}
			},
			required: ['url']
		},
		kafka: {
			$schema: 'http://json-schema.org/draft-07/schema#',
			type: 'object',
			properties: {
				brokers: {
					type: 'array',
					items: {
						type: 'string'
					},
					nullable: false,
					title: 'Brokers'
				},
				security: {
					type: 'object',
					title: 'Security',
					oneOf: [
						{
							type: 'object',
							title: 'PLAINTEXT',
							properties: {
								label: {
									enum: ['PLAINTEXT'],
									type: 'string'
								}
							}
						},
						{
							type: 'object',
							order: ['mechanism', 'username', 'password'],
							title: 'SASL_PLAINTEXT',
							required: ['mechanism', 'username', 'password'],
							properties: {
								label: {
									enum: ['SASL_PLAINTEXT'],
									type: 'string'
								},
								password: {
									type: 'string',
									password: true
								},
								username: {
									type: 'string'
								},
								mechanism: {
									enum: ['PLAIN', 'SCRAM-SHA-256', 'SCRAM-SHA-512'],
									type: 'string',
									disableCreate: true
								}
							}
						},
						{
							type: 'object',
							title: 'SSL',
							properties: {
								label: {
									enum: ['SSL'],
									type: 'string'
								}
							}
						},
						{
							type: 'object',
							order: ['mechanism', 'username', 'password'],
							title: 'SASL_SSL',
							required: ['mechanism', 'username', 'password'],
							properties: {
								label: {
									enum: ['SASL_SSL'],
									type: 'string'
								},
								password: {
									type: 'string',
									password: true
								},
								username: {
									type: 'string'
								},
								mechanism: {
									enum: ['PLAIN', 'SCRAM-SHA-256', 'SCRAM-SHA-512'],
									type: 'string',
									disableCreate: true
								}
							}
						}
					]
				},
				topics: {
					type: 'array',
					items: {
						type: 'string'
					},
					nullable: false,
					title: 'Topics'
				},
				group_id: { type: 'string', title: 'Group ID' }
			},
			required: ['brokers', 'security', 'topics', 'group_id']
		}
	}

	async function setConfig() {
		await CaptureService.setCaptureConfig({
			requestBody: {
				trigger_kind: selected,
				path,
				is_flow: isFlow,
				trigger_config: args && Object.keys(args).length > 0 ? args : undefined
			},
			workspace: $workspaceStore!
		})
	}

	let emailDomain: string | undefined = undefined

	async function getEmailDomain() {
		emailDomain =
			((await SettingService.getGlobal({
				key: 'email_domain'
			})) as any) ?? undefined
	}

	getEmailDomain()

	function getEmailAddress(emailDomain: string | undefined) {
		const cleanedPath = path.replaceAll('/', '.')
		const plainPrefix = `capture+${$workspaceStore}+${(isFlow ? 'flow.' : '') + cleanedPath}`
		const encodedPrefix = base32
			.stringify(new TextEncoder().encode(plainPrefix), {
				pad: false
			})
			.toLowerCase()
		return `${encodedPrefix}@${emailDomain}`
	}
	$: emailAddress = getEmailAddress(emailDomain)

	function webhook() {
		return `${$page.url.origin}/api/w/${$workspaceStore}/capture_u/webhook/${
			isFlow ? 'flow' : 'script'
		}/${path}`
	}
	const webhookUrl = webhook()

	function getHttpRoute(route_path: string | undefined) {
		return `${$page.url.origin}/api/w/${$workspaceStore}/capture_u/http/${
			isFlow ? 'flow' : 'script'
		}/${path.replaceAll('/', '.')}/${route_path ?? ''}`
	}
	$: httpRoute = getHttpRoute(args?.route_path)

	let captures: Capture[] = []
	async function refreshCaptures() {
		captures = await CaptureService.listCaptures({
			workspace: $workspaceStore!,
			runnableKind: isFlow ? 'flow' : 'script',
			path
		})
	}
	refreshCaptures()

	let captureConfigs: {
		[key: string]: CaptureConfig
	} = {}
	async function getCaptureConfigs() {
		const captureConfigsList = await CaptureService.getCaptureConfigs({
			workspace: $workspaceStore!,
			runnableKind: isFlow ? 'flow' : 'script',
			path
		})

		captureConfigs = captureConfigsList.reduce((acc, c) => {
			acc[c.trigger_kind] = c
			return acc
		}, {})

		if ((selected === 'websocket' || selected === 'kafka') && active) {
			const config = captureConfigs[selected]
			if (config && config.error) {
				const serverEnabled = getServerEnabled(config)
				if (!serverEnabled) {
					sendUserToast('Capture was stopped because of error: ' + config.error, true)
					active = false
				}
			}
		}
	}
	getCaptureConfigs()

	async function capture() {
		let i = 0
		active = true
		while (active) {
			if (i % 3 === 0) {
				await CaptureService.pingCaptureConfig({
					workspace: $workspaceStore!,
					triggerKind: selected,
					runnableKind: isFlow ? 'flow' : 'script',
					path
				})
				await getCaptureConfigs()
			}
			refreshCaptures()
			i++
			await sleep(1000)
		}
	}

	let deleteLoading: number | null = null
	async function deleteCapture(id: number) {
		deleteLoading = id
		try {
			await CaptureService.deleteCapture({
				workspace: $workspaceStore!,
				id
			})
			refreshCaptures()
		} finally {
			deleteLoading = null
		}
	}

	function stopAndSetDefaultArgs() {
		active = false
		if (selected in captureConfigs) {
			const triggerConfig = captureConfigs[selected].trigger_config
			args = isObject(triggerConfig) ? triggerConfig : {}
		} else if (selected === 'kafka') {
			args = {
				brokers: [''],
				topics: [''],
				group_id: `windmill_consumer-${$workspaceStore}-${path.replaceAll('/', '__')}`
			}
		} else {
			args = {}
		}
	}
	$: selected && stopAndSetDefaultArgs()

	onDestroy(() => {
		active = false
	})

	function getServerEnabled(config: CaptureConfig) {
		return (
			!!config.last_server_ping &&
			new Date(config.last_server_ping).getTime() > new Date().getTime() - 15 * 1000
		)
	}

	let config: CaptureConfig | undefined
	$: config = captureConfigs[selected]

	$: cloudDisabled = (selected === 'websocket' || selected === 'kafka') && isCloudHosted()

	$: selectedCaptures = captures.filter((c) => c.trigger_kind === selected)
</script>

<div class="flex flex-col gap-4 p-2">
	<div class="flex flex-row gap-2 justify-between">
		<ToggleButtonGroup bind:selected>
			<ToggleButton value="webhook" label="Webhook" icon={Webhook} small />
			<ToggleButton value="http" label="HTTP" icon={Route} small />
			<ToggleButton value="websocket" label="Websocket" icon={Unplug} small />
			<ToggleButton
				value="kafka"
				label={'Kafka' + (!$enterpriseLicense ? ' (ee)' : '')}
				disabled={!$enterpriseLicense}
				icon={KafkaIcon}
			/>
			<ToggleButton value="email" label="Email" icon={Mail} />
		</ToggleButtonGroup>

		<div class="flex gap-1">
			{#if (selected === 'websocket' || selected === 'kafka') && config && active}
				{@const serverEnabled = getServerEnabled(config)}
				<div class="self-center">
					{#if serverEnabled}
						<Popover notClickable>
							<span class="flex h-4 w-4">
								<Circle class="text-green-600 relative inline-flex fill-current" size={12} />
							</span>
							<div slot="text"> Websocket is connected </div>
						</Popover>
					{:else}
						<Popover notClickable>
							<span class="flex h-4 w-4">
								<Circle
									class="text-red-600 animate-ping absolute inline-flex fill-current"
									size={12}
								/>
								<Circle class="text-red-600 relative inline-flex fill-current" size={12} />
							</span>
							<div slot="text">
								Websocket is not connected{config.error ? ': ' + config.error : ''}
							</div>
						</Popover>
					{/if}
				</div>
			{/if}
			<Button
				size="xs2"
				on:click={async () => {
					if (!active) {
						await setConfig()
						capture()
					} else {
						active = false
					}
				}}
				disabled={!isValid || cloudDisabled}
				color={active ? 'red' : 'dark'}
				startIcon={{ icon: active ? CircleStop : Play }}
			>
				Capture
			</Button>
			{#if newItem || cloudDisabled}
				<Popover notClickable>
					<Button size="xs2" disabled startIcon={{ icon: Save }} iconOnly wrapperClasses="h-full" />
					<svelte:fragment slot="text">
						{#if newItem}
							Deploy the runnable to enable trigger creation
						{:else if cloudDisabled}
							{capitalize(selected)} triggers are disabled in the multi-tenant cloud
						{/if}
					</svelte:fragment>
				</Popover>
			{:else}
				<Button
					size="xs2"
					on:click={() =>
						dispatch('openTriggers', {
							kind: captureTriggerKindToTriggerKind(selected),
							config: args
						})}
					startIcon={{ icon: Save }}
					iconOnly
				/>
			{/if}
		</div>
	</div>
	{#if cloudDisabled}
		<Alert title="Not compatible with multi-tenant cloud" type="warning" size="xs">
			{capitalize(selected)} triggers are disabled in the multi-tenant cloud.
		</Alert>
	{:else}
		{#if selected in schemas}
			{#key selected}
				<SchemaForm schema={schemas[selected]} bind:args bind:isValid />
			{/key}
		{/if}

		{#if selected === 'webhook'}
			<Label label="URL">
				<ClipboardPanel content={webhookUrl} />
			</Label>
			<Label label="Example curl">
				<CopyableCodeBlock
					code={`curl \\
-X POST ${webhookUrl} \\
-H 'Content-Type: application/json' \\
-d '{"foo": 42}'`}
					language={bash}
				/>
			</Label>
		{:else if selected === 'http'}
			<Label label="URL">
				<ClipboardPanel content={httpRoute} />
			</Label>
			<Label label="Example curl">
				<CopyableCodeBlock
					code={`curl \\
-X POST ${httpRoute} \\
-H 'Content-Type: application/json' \\
-d '{"foo": 42}'`}
					language={bash}
				/>
			</Label>
		{:else if selected === 'email'}
			<Label label="Email">
				<ClipboardPanel content={emailAddress} />
			</Label>
		{/if}

		<Label label="Captures">
			<svelte:fragment slot="action">
				{#if canHavePreprocessor}
					<div>
						<ToggleButtonGroup bind:selected={testKind}>
							<ToggleButton value="main" label={isFlow ? 'Flow' : 'Main'} small />
							<ToggleButton
								value="preprocessor"
								label="Preprocessor"
								small
								tooltip="When the runnable has a preprocessor, it receives additional information about the request"
							/>
						</ToggleButtonGroup>
					</div>
				{/if}
			</svelte:fragment>
			<div class="flex flex-col gap-1 mt-2">
				{#if selectedCaptures.length === 0}
					<div class="text-xs text-secondary">No {selected} captures yet</div>
				{:else}
					{#each selectedCaptures as capture}
						{@const payload = isObject(capture.payload) ? capture.payload : {}}
						{@const triggerExtra = isObject(capture.trigger_extra) ? capture.trigger_extra : {}}
						{@const payloadData =
							testKind === 'preprocessor'
								? {
										...payload,
										...triggerExtra
								  }
								: payload}
						{@const schema =
							isFlow && testKind === 'main'
								? { required: [], properties: {}, ...convert(payloadData) }
								: {}}
						<div class="flex flex-row gap-1">
							<div class="text-xs border p-2 rounded-md overflow-auto grow whitespace-nowrap">
								{JSON.stringify(payloadData)}
							</div>
							<Button
								size="xs2"
								color="light"
								variant="border"
								on:click={() => {
									copyToClipboard(JSON.stringify(payloadData))
								}}
								iconOnly
								startIcon={{ icon: Clipboard }}
							/>

							{#if isFlow && testKind === 'main'}
								<CustomPopover>
									<Button
										size="xs"
										color="light"
										variant="border"
										on:click={() => {
											dispatch('updateSchema', { schema, redirect: true })
										}}
										wrapperClasses="h-full"
									>
										Apply schema
									</Button>

									<svelte:fragment slot="overlay">
										{#if schema}
											<div class="min-w-[400px]">
												<SchemaViewer {schema} />
											</div>
										{/if}
									</svelte:fragment>
								</CustomPopover>
							{/if}

							{#if testKind === 'preprocessor' && !hasPreprocessor}
								<CustomPopover noPadding>
									<Button
										size="xs"
										color="dark"
										disabled
										endIcon={{
											icon: Info
										}}
										wrapperClasses="h-full"
									>
										Apply args
									</Button>
									<svelte:fragment slot="overlay">
										<div class="text-sm p-2 flex flex-col gap-1 items-start">
											<p>You need to add a preprocessor to use preprocessor captures as args</p>
											<Button
												size="xs"
												color="dark"
												on:click={() => {
													dispatch('addPreprocessor')
												}}
											>
												Add preprocessor
											</Button>
										</div>
									</svelte:fragment>
								</CustomPopover>
							{:else}
								<Button
									size="xs"
									color="dark"
									on:click={() => {
										if (isFlow && testKind === 'main') {
											dispatch('updateSchema', { schema, redirect: false })
										}
										dispatch('applyArgs', {
											kind: testKind,
											args: payloadData
										})
									}}
									disabled={testKind === 'preprocessor' && !hasPreprocessor}
								>
									{isFlow && testKind === 'main' ? 'Apply schema and args' : 'Apply args'}
								</Button>
							{/if}

							<Button
								size="xs2"
								color="red"
								iconOnly
								startIcon={{ icon: Trash2 }}
								loading={deleteLoading === capture.id}
								on:click={() => {
									deleteCapture(capture.id)
								}}
							/>
						</div>
					{/each}
				{/if}
			</div>
		</Label>
	{/if}
</div>
