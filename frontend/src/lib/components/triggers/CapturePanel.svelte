<script lang="ts">
	import {
		Circle,
		CircleStop,
		Clipboard,
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
	import { copyToClipboard, sendUserToast, sleep } from '$lib/utils'
	import { createEventDispatcher, onDestroy } from 'svelte'
	import { captureTriggerKindToTriggerKind, type TriggerKind } from '../triggers'
	import Label from '../Label.svelte'
	import ClipboardPanel from '../details/ClipboardPanel.svelte'
	import CopyableCodeBlock from '../details/CopyableCodeBlock.svelte'
	import { bash } from 'svelte-highlight/languages'
	import Popover from '../Popover.svelte'

	export let isFlow: boolean
	export let path: string
	export let hasPreprocessor: boolean
	export let newItem = false

	const dispatch = createEventDispatcher<{
		openTrigger: {
			kind: TriggerKind
			config: Record<string, any>
		}
		test: {
			kind: 'main' | 'preprocessor'
			args: Record<string, any> | undefined
		}
	}>()

	let selected: CaptureTriggerKind = 'webhook'
	let args: Record<string, any> = {}
	let hostname = 'http://localhost:3000'
	let active = false
	let isValid = true
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
		return `${hostname}/api/w/${$workspaceStore}/capture_u/webhook/${
			isFlow ? 'flow' : 'script'
		}/${path}`
	}
	const webhookUrl = webhook()

	function getHttpRoute(route_path: string | undefined) {
		return `${hostname}/api/w/${$workspaceStore}/capture_u/http/${
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
			args = captureConfigs[selected].trigger_config
		} else if (selected === 'kafka') {
			args = {
				brokers: [''],
				topics: [''],
				group_id: `windmill_consumer-${$workspaceStore}-${path.replaceAll('/', '__')}`
			}
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
</script>

<div class="flex flex-col gap-4 p-2">
	<div class="flex flex-row gap-2 justify-between">
		<ToggleButtonGroup bind:selected>
			<ToggleButton value="webhook" label="Webhook" icon={Webhook} />
			<ToggleButton value="http" label="HTTP" icon={Route} />
			<ToggleButton value="websocket" label="Websocket" icon={Unplug} />
			<ToggleButton
				value="kafka"
				label={'Kafka' + (!$enterpriseLicense ? ' (ee)' : '')}
				disabled={!$enterpriseLicense}
				icon={KafkaIcon}
			/>
			<ToggleButton value="email" label="Email" icon={Mail} />
		</ToggleButtonGroup>

		<div class="flex gap-2">
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
				size="xs"
				on:click={async () => {
					if (!active) {
						await setConfig()
						capture()
					} else {
						active = false
					}
				}}
				disabled={!isValid}
				color={active ? 'red' : 'dark'}
				startIcon={{ icon: active ? CircleStop : Play }}
			>
				Capture
			</Button>
			<Button
				size="xs"
				on:click={() =>
					dispatch('openTrigger', {
						kind: captureTriggerKindToTriggerKind(selected),
						config: args
					})}
				startIcon={{ icon: Save }}
				disabled={newItem}
			>
				Save
			</Button>
		</div>
	</div>

	{#if selected in schemas}
		<SchemaForm schema={schemas[selected]} bind:args bind:isValid />
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
		<div class="flex flex-col gap-1">
			{#if captures.length === 0}
				<div class="text-xs text-secondary">No {selected} captures yet</div>
			{:else}
				{#each captures.filter((c) => c.trigger_kind === selected) as capture}
					{@const payloadData = hasPreprocessor
						? {
								...capture.payload,
								...(capture.trigger_extra ?? {})
						  }
						: capture.payload}
					<div class="flex flex-row gap-1">
						<div class="text-xs border p-2 rounded-md overflow-auto grow whitespace-nowrap">
							{JSON.stringify(payloadData)}
						</div>
						<Button
							size="xs2"
							color="dark"
							on:click={() => {
								copyToClipboard(JSON.stringify(payloadData))
							}}
							iconOnly
							startIcon={{ icon: Clipboard }}
						/>
						<Button
							size="xs2"
							color="dark"
							on:click={() => {
								dispatch('test', {
									kind: 'main',
									args: payloadData
								})
							}}
						>
							Test on main
						</Button>

						{#if hasPreprocessor}
							<Button
								size="xs2"
								color="dark"
								on:click={() => {
									dispatch('test', {
										kind: 'preprocessor',
										args: payloadData
									})
								}}
							>
								Test on preprocessor
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
</div>
