import {
	KafkaTriggerService,
	MqttTriggerService,
	AmqpTriggerService,
	NatsTriggerService,
	PostgresTriggerService,
	ScheduleService,
	SqsTriggerService,
	WebsocketTriggerService,
	NativeTriggerService,
	type GcpTrigger,
	type AzureTrigger,
	type KafkaTrigger,
	type PostgresTrigger,
	type Schedule,
	type TriggersCount,
	type HttpTrigger,
	HttpTriggerService,
	GcpTriggerService,
	AzureTriggerService,
	type EmailTrigger,
	EmailTriggerService,
	type NativeTrigger,
	type NativeServiceName
} from '$lib/gen'
import { enterpriseLicense } from '$lib/stores'

import {
	getLightConfig,
	newDraftTriggerPath,
	sortTriggers,
	triggerDraftKind,
	updateTriggersCount,
	type Trigger,
	type TriggerDraftTarget
} from './utils'
import { get, type Writable } from 'svelte/store'
import type { TriggerType } from './utils'
import type { UserExt } from '$lib/stores'
import type { ScheduleTrigger } from '../triggers'
import { canWrite, formatCron, generateRandomString } from '$lib/utils'

export class Triggers {
	#triggers = $state<Trigger[]>([])
	#selectedTriggerIndex = $state<number | undefined>(undefined)
	#selectedTrigger = $derived(
		this.#selectedTriggerIndex !== undefined
			? this.#triggers[this.#selectedTriggerIndex]
			: undefined
	)

	constructor(triggers: Trigger[] = [], selectedIndex?: number) {
		this.#triggers = triggers
		this.#selectedTriggerIndex = selectedIndex
	}

	get selectedTrigger(): Trigger | undefined {
		return this.#selectedTrigger
	}

	get selectedTriggerIndex(): number | undefined {
		return this.#selectedTriggerIndex
	}

	set selectedTriggerIndex(index: number | undefined) {
		if (index === undefined || index < 0 || index >= this.#triggers.length) {
			this.#selectedTriggerIndex = undefined
		} else {
			this.#selectedTriggerIndex = index
		}
	}

	get triggers(): Trigger[] {
		return this.#triggers
	}

	setTriggers(triggers: Trigger[]) {
		this.#triggers = triggers
	}

	setDraftConfig(triggerIndex: number, draftConfig: Record<string, any> | undefined) {
		if (triggerIndex === undefined || triggerIndex < 0 || triggerIndex >= this.#triggers.length) {
			return
		}
		this.#triggers[triggerIndex].draftConfig = draftConfig
	}

	getSelectedTriggerSnapshot(): number | undefined {
		return $state.snapshot(this.#selectedTriggerIndex)
	}

	/** Add an undeployed trigger to the editor's list. For the draft-backed kinds
	 * this writes the `trigger_*` draft row up front — draft rows are path-keyed,
	 * so the trigger needs a path before the user has typed one, and persisting
	 * immediately is what makes it survive a reload. Native kinds have no draft
	 * row yet and keep their config in `draftConfig` until deploy. */
	addDraftTrigger(
		triggersCountStore: Writable<TriggersCount | undefined>,
		type: TriggerType,
		target: TriggerDraftTarget,
		seedCfg?: Record<string, any>
	): number {
		const primaryScheduleExists = this.#triggers.some((t) => t.type === 'schedule' && t.isPrimary)
		const isPrimary = type === 'schedule' && !primaryScheduleExists
		const draftKind = triggerDraftKind(type)
		const { runnablePath, isFlow, workspace } = target

		// Reserve the path the draft row will be keyed at. The row itself is
		// written by the editor panel's autosave, which is what turns the form's
		// defaults into a complete config — seeding a partial one here would leave
		// required fields unset and crash the editors that don't default them.
		const path =
			draftKind && workspace
				? newDraftTriggerPath(
						runnablePath,
						type,
						this.#triggers.filter((t) => t.type === type).map((t) => t.path ?? ''),
						isPrimary
					)
				: undefined

		const newTrigger: Trigger = {
			id: generateRandomString(),
			type,
			path,
			isPrimary,
			isDraft: true,
			isNew: true,
			hasDraft: !!draftKind,
			// Until the row exists this is the editor's starting point, whatever the
			// kind — `TriggersWrapper` hands it to `openNew` as defaults. It carries
			// the runnable explicitly: the editors otherwise fall back to the fake
			// capture path an undeployed runnable is given.
			draftConfig: { ...seedCfg, script_path: runnablePath, is_flow: isFlow }
		}

		this.#triggers.push(newTrigger)

		updateTriggersCount(triggersCountStore, type, 'add', seedCfg)

		return this.#triggers.length - 1
	}

	deleteTrigger(
		triggersCountStore: Writable<TriggersCount | undefined>,
		triggerIndex: number
	): void {
		if (triggerIndex === undefined || triggerIndex < 0 || triggerIndex >= this.#triggers.length) {
			return
		}
		const { type } = this.#triggers[triggerIndex]

		this.#triggers = this.#triggers.filter((_, index) => index !== triggerIndex)

		updateTriggersCount(triggersCountStore, type, 'remove')
	}

	updateTriggers(
		remoteTriggers: any[],
		type: TriggerType,
		user: UserExt | undefined = undefined
	): number {
		const currentTriggers = this.#triggers
		// Preserve the editor-local config of the kinds that have no draft row
		// (native); every other kind's pending state lives in the backend rows.
		const configMap = new Map<string, Record<string, any>>(
			currentTriggers
				.filter((t) => t.type === type && t.draftConfig)
				.map((t) => [t.path ?? '', t.draftConfig!])
		)

		const backendTriggers = remoteTriggers.map((trigger) => ({
			type: type as TriggerType,
			path: trigger.path,
			isPrimary: type === 'schedule' && trigger.path === trigger.script_path,
			// `draft_only` rows are synthesized from a draft with no deployed
			// counterpart; `is_draft` also covers a draft layered on a deployed row.
			isDraft: !!trigger.draft_only,
			hasDraft: !!trigger.is_draft,
			draftPath: trigger.draft_path,
			canWrite: canWrite(trigger.path, trigger.extra_perms, user),
			draftConfig: configMap.get(trigger.path),
			lightConfig: getLightConfig(type, trigger)
		}))

		// A locally-added trigger survives only until the backend knows about it:
		// an `isNew` one until its editor's autosave creates the draft row, a
		// native one (no draft row at all) until it deploys.
		const backendPaths = new Set(remoteTriggers.map((t) => t.path))
		const filteredTriggers = currentTriggers.filter(
			(t) =>
				t.type !== type ||
				(t.isDraft &&
					!backendPaths.has(t.path ?? '') &&
					(t.isNew || !triggerDraftKind(t.type)))
		)
		const newTriggers = sortTriggers([...filteredTriggers, ...backendTriggers])
		this.#triggers = newTriggers

		return newTriggers.filter((t) => t.type === type).length
	}

	async fetchSchedules(
		triggersCountStore: Writable<TriggersCount | undefined>,
		workspaceId: string | undefined,
		path: string,
		isFlow: boolean,
		primarySchedule?: ScheduleTrigger | undefined | false,
		user: UserExt | undefined = undefined
	): Promise<void> {
		if (!workspaceId) return
		try {
			//First update the store with legacy primary schedule
			if (primarySchedule && !this.#triggers.some((s) => s.isPrimary)) {
				const primary = {
					type: 'schedule' as TriggerType,
					path,
					isPrimary: true,
					isDraft: false,
					draftConfig: {
						schedule: primarySchedule.cron ? formatCron(primarySchedule.cron) : undefined,
						args: primarySchedule.args,
						timezone: primarySchedule.timezone,
						summary: primarySchedule.summary,
						description: primarySchedule.description,
						enabled: primarySchedule.enabled
					}
				}
				this.#triggers = [...this.#triggers, primary]
			}

			const allDeployedSchedules: Schedule[] = await ScheduleService.listSchedules({
				workspace: workspaceId,
				path,
				isFlow,
				includeDraftOnly: true
			})

			const scheduleCount = this.updateTriggers(allDeployedSchedules, 'schedule', user)
			const updatedPrimarySchedule = this.#triggers.find((s) => s.isPrimary)
			triggersCountStore.update((triggersCount) => {
				return {
					...(triggersCount ?? {}),
					schedule_count: scheduleCount,
					primary_schedule: {
						schedule:
							updatedPrimarySchedule?.draftConfig?.schedule ??
							updatedPrimarySchedule?.lightConfig?.schedule
					}
				}
			})

			return
		} catch (error) {
			console.error('Failed to fetch schedules:', error)
			return
		}
	}

	async fetchWebsocketTriggers(
		triggersCountStore: Writable<TriggersCount | undefined>,
		workspaceId: string | undefined,
		path: string,
		isFlow: boolean,
		user: UserExt | undefined = undefined
	): Promise<void> {
		if (!workspaceId) return
		try {
			const wsTriggers = await WebsocketTriggerService.listWebsocketTriggers({
				workspace: workspaceId,
				path,
				isFlow,
				includeDraftOnly: true
			})
			const wsCount = this.updateTriggers(wsTriggers, 'websocket', user)
			triggersCountStore.update((triggersCount) => {
				return {
					...(triggersCount ?? {}),
					websocket_count: wsCount
				}
			})
		} catch (error) {
			console.error('Failed to fetch Websocket triggers:', error)
		}
	}

	async fetchPostgresTriggers(
		triggersCountStore: Writable<TriggersCount | undefined>,
		workspaceId: string | undefined,
		path: string,
		isFlow: boolean,
		user: UserExt | undefined = undefined
	): Promise<void> {
		if (!workspaceId) return
		try {
			const pgTriggers: PostgresTrigger[] = await PostgresTriggerService.listPostgresTriggers({
				workspace: workspaceId,
				path,
				isFlow,
				includeDraftOnly: true
			})
			const pgCount = this.updateTriggers(pgTriggers, 'postgres', user)
			triggersCountStore.update((triggersCount) => {
				return {
					...(triggersCount ?? {}),
					postgres_count: pgCount
				}
			})
		} catch (error) {
			console.error('Failed to fetch Postgres triggers:', error)
		}
	}

	async fetchKafkaTriggers(
		triggersCountStore: Writable<TriggersCount | undefined>,
		workspaceId: string | undefined,
		path: string,
		isFlow: boolean,
		user: UserExt | undefined = undefined
	): Promise<void> {
		if (!workspaceId) return
		try {
			const kafkaTriggers: KafkaTrigger[] = await KafkaTriggerService.listKafkaTriggers({
				workspace: workspaceId,
				path,
				isFlow,
				includeDraftOnly: true
			})
			const kafkaCount = this.updateTriggers(kafkaTriggers, 'kafka', user)
			triggersCountStore.update((triggersCount) => {
				return {
					...(triggersCount ?? {}),
					kafka_count: kafkaCount
				}
			})
		} catch (error) {
			console.error('Failed to fetch Kafka triggers:', error)
		}
	}

	async fetchNatsTriggers(
		triggersCountStore: Writable<TriggersCount | undefined>,
		workspaceId: string | undefined,
		path: string,
		isFlow: boolean,
		user: UserExt | undefined = undefined
	): Promise<void> {
		if (!workspaceId) return
		try {
			const natsTriggers = await NatsTriggerService.listNatsTriggers({
				workspace: workspaceId,
				path,
				isFlow,
				includeDraftOnly: true
			})
			const natsCount = this.updateTriggers(natsTriggers, 'nats', user)
			triggersCountStore.update((triggersCount) => {
				return {
					...(triggersCount ?? {}),
					nats_count: natsCount
				}
			})
		} catch (error) {
			console.error('Failed to fetch NATS triggers:', error)
		}
	}

	async fetchMqttTriggers(
		triggersCountStore: Writable<TriggersCount | undefined>,
		workspaceId: string | undefined,
		path: string,
		isFlow: boolean,
		user: UserExt | undefined = undefined
	): Promise<void> {
		if (!workspaceId) return
		try {
			const mqttTriggers = await MqttTriggerService.listMqttTriggers({
				workspace: workspaceId,
				path,
				isFlow,
				includeDraftOnly: true
			})
			const mqttCount = this.updateTriggers(mqttTriggers, 'mqtt', user)
			triggersCountStore.update((triggersCount) => {
				return {
					...(triggersCount ?? {}),
					mqtt_count: mqttCount
				}
			})
		} catch (error) {
			console.error('Failed to fetch MQTT triggers:', error)
		}
	}

	async fetchAmqpTriggers(
		triggersCountStore: Writable<TriggersCount | undefined>,
		workspaceId: string | undefined,
		path: string,
		isFlow: boolean,
		user: UserExt | undefined = undefined
	): Promise<void> {
		if (!workspaceId) return
		try {
			const amqpTriggers = await AmqpTriggerService.listAmqpTriggers({
				workspace: workspaceId,
				path,
				isFlow,
				includeDraftOnly: true
			})
			const amqpCount = this.updateTriggers(amqpTriggers, 'amqp', user)
			triggersCountStore.update((triggersCount) => {
				return {
					...(triggersCount ?? {}),
					amqp_count: amqpCount
				}
			})
		} catch (error) {
			console.error('Failed to fetch AMQP triggers:', error)
		}
	}

	async fetchSqsTriggers(
		triggersCountStore: Writable<TriggersCount | undefined>,
		workspaceId: string | undefined,
		path: string,
		isFlow: boolean,
		user: UserExt | undefined = undefined
	): Promise<void> {
		if (!workspaceId) return
		try {
			const sqsTriggers = await SqsTriggerService.listSqsTriggers({
				workspace: workspaceId,
				path,
				isFlow,
				includeDraftOnly: true
			})
			const sqsCount = this.updateTriggers(sqsTriggers, 'sqs', user)
			triggersCountStore.update((triggersCount) => {
				return {
					...(triggersCount ?? {}),
					sqs_count: sqsCount
				}
			})
		} catch (error) {
			console.error('Failed to fetch SQS triggers:', error)
		}
	}

	async fetchGcpTriggers(
		triggersCountStore: Writable<TriggersCount | undefined>,
		workspaceId: string | undefined,
		path: string,
		isFlow: boolean,
		user: UserExt | undefined = undefined
	): Promise<void> {
		if (!workspaceId) return
		try {
			const gcpTriggers: GcpTrigger[] = await GcpTriggerService.listGcpTriggers({
				workspace: workspaceId,
				path,
				isFlow,
				includeDraftOnly: true
			})
			const gcpCount = this.updateTriggers(gcpTriggers, 'gcp', user)
			triggersCountStore.update((triggersCount) => {
				return {
					...(triggersCount ?? {}),
					gcp_count: gcpCount
				}
			})
		} catch (error) {
			console.error('Failed to fetch GCP Pub/Sub triggers:', error)
		}
	}

	async fetchAzureTriggers(
		triggersCountStore: Writable<TriggersCount | undefined>,
		workspaceId: string | undefined,
		path: string,
		isFlow: boolean,
		user: UserExt | undefined = undefined
	): Promise<void> {
		if (!workspaceId) return
		try {
			const azureTriggers: AzureTrigger[] = await AzureTriggerService.listAzureTriggers({
				workspace: workspaceId,
				path,
				isFlow,
				includeDraftOnly: true
			})
			const azureCount = this.updateTriggers(azureTriggers, 'azure', user)
			triggersCountStore.update((triggersCount) => {
				return {
					...(triggersCount ?? {}),
					azure_count: azureCount
				}
			})
		} catch (error) {
			console.error('Failed to fetch Azure triggers:', error)
		}
	}

	async fetchHttpTriggers(
		triggersCountStore: Writable<TriggersCount | undefined>,
		workspaceId: string | undefined,
		path: string,
		isFlow: boolean,
		user: UserExt | undefined = undefined
	): Promise<void> {
		if (!workspaceId) return
		try {
			const httpTriggers: HttpTrigger[] = await HttpTriggerService.listHttpTriggers({
				workspace: workspaceId,
				path,
				isFlow,
				includeDraftOnly: true
			})
			const httpCount = this.updateTriggers(httpTriggers, 'http', user)
			triggersCountStore.update((triggersCount) => {
				return {
					...(triggersCount ?? {}),
					http_routes_count: httpCount
				}
			})
		} catch (error) {
			console.error('Failed to fetch HTTP triggers:', error)
		}
	}

	async fetchEmailTriggers(
		triggersCountStore: Writable<TriggersCount | undefined>,
		workspaceId: string | undefined,
		path: string,
		isFlow: boolean,
		user: UserExt | undefined = undefined
	): Promise<void> {
		if (!workspaceId) return
		try {
			const emailTriggers: EmailTrigger[] = await EmailTriggerService.listEmailTriggers({
				workspace: workspaceId,
				path,
				isFlow,
				includeDraftOnly: true
			})
			const emailCount = this.updateTriggers(emailTriggers, 'email', user)
			triggersCountStore.update((triggersCount) => {
				return {
					...(triggersCount ?? {}),
					email_count: emailCount
				}
			})
		} catch (error) {
			console.error('Failed to fetch email triggers:', error)
		}
	}

	async fetchNativeTriggers(
		triggersCountStore: Writable<TriggersCount | undefined>,
		serviceName: NativeServiceName,
		workspaceId: string | undefined,
		path: string,
		isFlow: boolean,
		user: UserExt | undefined = undefined
	): Promise<void> {
		if (!workspaceId) return
		try {
			const triggers: NativeTrigger[] = await NativeTriggerService.listNativeTriggers({
				workspace: workspaceId,
				serviceName,
				path,
				isFlow
			})
			// Convert to the trigger format used by updateTriggers
			const triggerData = triggers.map((t) => ({
				path: t.external_id,
				script_path: t.script_path,
				is_flow: t.is_flow,
				extra_perms: {},
				service_config: t.service_config
			}))
			const count = this.updateTriggers(triggerData, serviceName, user)
			const countProperty = `${serviceName}_count`
			triggersCountStore.update((triggersCount) => {
				return {
					...(triggersCount ?? {}),
					[countProperty]: count
				}
			})
		} catch (error) {
			console.debug(`Failed to fetch ${serviceName} triggers:`, error)
		}
	}

	async fetchTriggers(
		triggersCountStore: Writable<TriggersCount | undefined>,
		workspaceId: string | undefined,
		path: string,
		isFlow: boolean,
		primarySchedule: ScheduleTrigger | undefined | false = undefined,
		user: UserExt | undefined = undefined
	): Promise<void> {
		if (!workspaceId) return

		// Fetch each type of trigger
		await Promise.all([
			this.fetchSchedules(triggersCountStore, workspaceId, path, isFlow, primarySchedule, user),
			this.fetchHttpTriggers(triggersCountStore, workspaceId, path, isFlow, user),
			this.fetchWebsocketTriggers(triggersCountStore, workspaceId, path, isFlow, user),
			this.fetchPostgresTriggers(triggersCountStore, workspaceId, path, isFlow, user),
			this.fetchMqttTriggers(triggersCountStore, workspaceId, path, isFlow, user),
			this.fetchAmqpTriggers(triggersCountStore, workspaceId, path, isFlow, user),
			this.fetchNativeTriggers(triggersCountStore, 'nextcloud', workspaceId, path, isFlow, user),
			this.fetchNativeTriggers(triggersCountStore, 'google', workspaceId, path, isFlow, user),
			this.fetchNativeTriggers(triggersCountStore, 'github', workspaceId, path, isFlow, user),
			...(get(enterpriseLicense)
				? [
						this.fetchKafkaTriggers(triggersCountStore, workspaceId, path, isFlow, user),
						this.fetchSqsTriggers(triggersCountStore, workspaceId, path, isFlow, user),
						this.fetchGcpTriggers(triggersCountStore, workspaceId, path, isFlow, user),
						this.fetchAzureTriggers(triggersCountStore, workspaceId, path, isFlow, user),
						this.fetchEmailTriggers(triggersCountStore, workspaceId, path, isFlow, user),
						this.fetchNatsTriggers(triggersCountStore, workspaceId, path, isFlow, user)
					]
				: [])
		])
	}
}
