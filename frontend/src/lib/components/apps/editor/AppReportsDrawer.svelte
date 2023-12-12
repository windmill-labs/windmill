<script lang="ts">
	import { enterpriseLicense } from '$lib/stores'
	import CronInput from '$lib/components/CronInput.svelte'
	import ScriptPicker from '$lib/components/ScriptPicker.svelte'
	import Section from '$lib/components/Section.svelte'
	import Alert from '$lib/components/common/alert/Alert.svelte'
	import Drawer from '$lib/components/common/drawer/Drawer.svelte'
	import DrawerContent from '$lib/components/common/drawer/DrawerContent.svelte'
	import Tab from '$lib/components/common/tabs/Tab.svelte'
	import Tabs from '$lib/components/common/tabs/Tabs.svelte'
	import {
		FlowService,
		JobService,
		RawScript,
		ScheduleService,
		SettingService,
		WorkspaceService
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { emptyString, formatCron, sendUserToast, tryEvery } from '$lib/utils'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { RotateCw, Save } from 'lucide-svelte'
	import { CUSTOM_TAGS_SETTING, WORKSPACE_SLACK_BOT_TOKEN_PATH } from '$lib/consts'
	import { loadSchemaFromPath } from '$lib/infer'
	export let appPath: string
	export let open = false

	let appReportingEnabled = false
	let appReportingStartupDuration = 5
	let appReportingSchedule: {
		cron: string
		timezone: string
	} = {
		cron: '0 0 12 * *',
		timezone: Intl.DateTimeFormat().resolvedOptions().timeZone
	}
	let selectedTab: 'email' | 'slack' | 'discord' | 'custom' = $enterpriseLicense
		? 'slack'
		: 'custom'

	let screenshotKind: 'pdf' | 'png' = 'pdf'

	let customPath: string | undefined = undefined
	let customPathSchema: Record<string, any> = {}
	let args: Record<string, any> = {}
	let areArgsValid = true

	$: customPath
		? loadSchemaFromPath(customPath).then((schema) => {
				customPathSchema = schema
					? {
							...schema,
							properties: Object.fromEntries(
								Object.entries(schema.properties ?? {}).filter(
									([key, _]) => key !== 'screenshot' && key !== 'app_path' && key !== 'kind'
								)
							)
					  }
					: {}
		  })
		: (customPathSchema = {})

	let isSlackConnectedWorkspace = false
	async function getWorspaceSlackSetting() {
		const settings = await WorkspaceService.getSettings({
			workspace: $workspaceStore!
		})
		if (settings.slack_name) {
			isSlackConnectedWorkspace = true
		} else {
			isSlackConnectedWorkspace = false
		}
	}
	getWorspaceSlackSetting()

	async function getAppReportingInfo() {
		const flowPath = appPath + '_reports'
		try {
			const flow = await FlowService.getFlowByPath({
				workspace: $workspaceStore!,
				path: flowPath
			})

			const schedule = await ScheduleService.getSchedule({
				workspace: $workspaceStore!,
				path: flowPath
			})

			appReportingSchedule = {
				cron: schedule.schedule,
				timezone: schedule.timezone
			}
			appReportingStartupDuration = schedule.args?.startup_duration ?? appReportingStartupDuration
			screenshotKind = schedule.args?.kind ?? screenshotKind

			args = schedule.args
				? Object.fromEntries(
						Object.entries(schedule.args).filter(
							([key, _]) => key !== 'app_path' && key !== 'startup_duration' && key !== 'kind'
						)
				  )
				: {}

			selectedTab =
				flow.value.modules[1]?.value.type === 'script'
					? flow.value.modules[1].value.path === notificationScripts.email.path
						? 'email'
						: flow.value.modules[1].value.path === notificationScripts.slack.path
						? 'slack'
						: flow.value.modules[1].value.path === notificationScripts.discord.path
						? 'discord'
						: 'custom'
					: 'custom'

			customPath =
				selectedTab === 'custom' &&
				flow.value.modules[1]?.value.type === 'script' &&
				!flow.value.modules[1].value.path.startsWith('hub/')
					? flow.value.modules[1].value.path
					: undefined

			appReportingEnabled = true
		} catch (err) {}
	}

	$: appPath && getAppReportingInfo()

	async function disableAppReporting() {
		const flowPath = appPath + '_reports'
		await ScheduleService.deleteSchedule({
			workspace: $workspaceStore!,
			path: flowPath
		})
		await FlowService.deleteFlowByPath({
			workspace: $workspaceStore!,
			path: flowPath
		})
		appReportingEnabled = false
		sendUserToast('App reporting disabled')
	}

	const appPreviewScript = `import puppeteer from 'puppeteer-core';
import dayjs from 'dayjs';
export async function main(app_path: string, startup_duration = 5, kind: 'pdf' | 'png' = 'pdf') {
  const browser = await puppeteer.launch({ headless: 'new', executablePath: '/usr/bin/chromium', args: ['--no-sandbox'] });
  const page = await browser.newPage();
  await page.setCookie({
    "name": "token",
    "value": Bun.env["WM_TOKEN"],
    "domain": Bun.env["WM_BASE_URL"]?.replace(/https?:\\/\\//, '')
  })
  page
    .on('console', msg =>
      console.log(dayjs().format("HH:mm:ss") + " " + msg.type().substr(0, 3).toUpperCase() + " " + msg.text()))
    .on('pageerror', ({ msg }) => console.log(dayjs().format("HH:mm:ss") + " " + msg));
  await page.setViewport({ width: 1200, height: 2000 });
  await page.goto(Bun.env["WM_BASE_URL"] + '/apps/get/' + app_path + '?workspace=' + Bun.env["WM_WORKSPACE"] + "&hideRefreshBar=true&hideEditBtn=true");
	await page.waitForSelector("#app-content", { timeout: 20000 })
  await new Promise((resolve, _) => {
		setTimeout(resolve, startup_duration * 1000)
  })
  await page.$eval("#sidebar", el => el.remove())
  await page.$eval("#content", el => el.classList.remove("md:pl-12"))
	await page.$$eval(".app-component-refresh-btn", els => els.forEach(el => el.remove()))
	await page.$$eval(".app-table-footer-btn", els => els.forEach(el => el.remove()))
  const elem = await page.$('#app-content');
  const { height } = await elem.boundingBox();
  await page.setViewport({ width: 1200, height });
  await new Promise((resolve, _) => {
		setTimeout(resolve, 500)
  })
  const screenshot = kind === "pdf" ? await page.pdf({
		printBackground: true,
		width: 1200,
		height
  }) : await page.screenshot({
		fullPage: true,
		type: "png",
    captureBeyondViewport: false
	});
  await browser.close();
  return Buffer.from(screenshot).toString('base64');
}`

	const notificationScripts = {
		discord: {
			path: 'hub/7838/discord',
			schema: {
				type: 'object',
				properties: {
					discord_webhook: {
						type: 'object',
						format: 'resource-discord_webhook',
						properties: {},
						required: [],
						description: ''
					}
				},
				required: ['discord_webhook']
			}
		},
		slack: {
			path: 'hub/7836/slack', // if to be updated, also update it in in backend/windmill-queue/src/jobs.rs
			schema: {
				type: 'object',
				properties: {
					channel: {
						type: 'string',
						default: ''
					}
				},
				required: ['channel']
			}
		},
		email: {
			path: 'hub/7837/smtp',
			schema: {
				type: 'object',
				properties: {
					smtp: {
						type: 'object',
						format: 'resource-smtp',
						properties: {},
						required: [],
						description: ''
					},
					from_email: {
						type: 'string',
						default: ''
					},
					to_email: {
						type: 'string',
						default: ''
					}
				},
				required: ['smtp', 'from_email', 'to_email']
			}
		}
	}

	function getFlowArgs() {
		return {
			app_path: appPath,
			startup_duration: appReportingStartupDuration,
			kind: screenshotKind,
			...args,
			...(selectedTab === 'slack'
				? {
						slack: '$res:' + WORKSPACE_SLACK_BOT_TOKEN_PATH
				  }
				: {})
		}
	}

	function getFlowValue() {
		const notifInputTransforms: {
			[key: string]: {
				expr: string
				type: 'javascript'
			}
		} = {
			app_path: {
				type: 'javascript',
				expr: 'flow_input.app_path'
			},
			screenshot: {
				type: 'javascript',
				expr: 'results.a'
			},
			kind: {
				type: 'javascript',
				expr: 'flow_input.kind'
			},
			...Object.fromEntries(
				Object.keys(args).map((key) => [
					key,
					{
						type: 'javascript',
						expr: `flow_input.${key}`
					}
				])
			),
			...(selectedTab === 'slack'
				? {
						slack: {
							type: 'javascript',
							expr: 'flow_input.slack'
						}
				  }
				: {})
		}

		const value = {
			modules: [
				{
					id: 'a',
					value: {
						type: 'rawscript' as const,
						tag: 'chromium',
						content: appPreviewScript,
						language: RawScript.language.BUN,
						input_transforms: {
							app_path: {
								expr: 'flow_input.app_path',
								type: 'javascript' as const
							},
							startup_duration: {
								expr: 'flow_input.startup_duration',
								type: 'javascript' as const
							},
							kind: {
								expr: 'flow_input.kind',
								type: 'javascript' as const
							}
						}
					}
				},
				{
					id: 'b',
					value: {
						type: 'script' as const,
						path:
							selectedTab === 'custom' ? customPath || '' : notificationScripts[selectedTab].path,
						input_transforms: notifInputTransforms
					}
				}
			]
		}

		return value
	}

	async function enableAppReporting() {
		const flowPath = appPath + '_reports'

		try {
			// will only work if the user is super admin
			const customTags = ((await SettingService.getGlobal({
				key: CUSTOM_TAGS_SETTING
			})) ?? []) as string[]

			if (!customTags.includes('chromium')) {
				await SettingService.setGlobal({
					key: CUSTOM_TAGS_SETTING,
					requestBody: {
						value: [...customTags, 'chromium']
					}
				})
			}
		} catch (err) {}

		await FlowService.deleteFlowByPath({
			workspace: $workspaceStore!,
			path: flowPath
		})

		await FlowService.createFlow({
			workspace: $workspaceStore!,
			requestBody: {
				summary: appPath + ' - Reports flow',
				value: getFlowValue(),
				schema: {
					$schema: 'https://json-schema.org/draft/2020-12/schema',
					properties: {
						app_path: {
							description: '',
							type: 'string',
							default: null,
							format: ''
						},
						startup_duration: {
							description: '',
							type: 'integer',
							default: 5,
							format: ''
						},
						kind: {
							description: '',
							type: 'string',
							enum: ['pdf', 'png'],
							default: 'pdf',
							format: ''
						},
						...(selectedTab === 'custom'
							? customPathSchema.properties
							: notificationScripts[selectedTab].schema.properties),
						...(selectedTab === 'slack'
							? {
									slack: {
										description: '',
										type: 'object',
										format: 'resource-slack',
										properties: {},
										required: []
									}
							  }
							: {})
					},
					required: [
						'app_path',
						'startup_duration',
						'kind',
						...(selectedTab === 'custom'
							? customPathSchema.required
							: notificationScripts[selectedTab].schema.required),
						...(selectedTab === 'slack' ? ['slack'] : [])
					],
					type: 'object'
				},
				path: flowPath
			}
		})

		await ScheduleService.deleteSchedule({
			workspace: $workspaceStore!,
			path: flowPath
		})

		await ScheduleService.createSchedule({
			workspace: $workspaceStore!,
			requestBody: {
				path: flowPath,
				schedule: formatCron(appReportingSchedule.cron),
				timezone: appReportingSchedule.timezone,
				script_path: flowPath,
				is_flow: true,
				args: getFlowArgs(),
				enabled: true
			}
		})
		appReportingEnabled = true
	}

	let testLoading = false
	async function testReport() {
		try {
			testLoading = true
			const jobId = await JobService.runFlowPreview({
				workspace: $workspaceStore!,
				requestBody: {
					args: getFlowArgs(),
					value: getFlowValue()
				}
			})
			tryEvery({
				tryCode: async () => {
					let testResult = await JobService.getCompletedJob({
						workspace: $workspaceStore!,
						id: jobId
					})
					testLoading = false
					sendUserToast(
						testResult.success
							? 'Report sent successfully'
							: 'Report error: ' + testResult.result?.['error']?.['message'],
						!testResult.success
					)
				},
				timeoutCode: async () => {
					testLoading = false
					sendUserToast('Reports flow did not return after 30s', true)
					try {
						await JobService.cancelQueuedJob({
							workspace: $workspaceStore!,
							id: jobId,
							requestBody: {
								reason: 'Reports flow did not return after 30s'
							}
						})
					} catch (err) {
						console.error(err)
					}
				},
				interval: 500,
				timeout: 30000
			})
		} catch (err) {
			sendUserToast('Could not test reports flow: ' + err, true)
			testLoading = false
		}
	}

	let disabled = true
	$: disabled =
		emptyString(appReportingSchedule.cron) ||
		(selectedTab === 'custom' && emptyString(customPath)) ||
		(selectedTab === 'slack' && !isSlackConnectedWorkspace) ||
		!areArgsValid
</script>

<Drawer bind:open size="800px">
	<DrawerContent on:close={() => (open = false)}
		title="Schedule Reports"
		tooltip="Send a PDF or PNG preview of any app at a given schedule"
		documentationLink="https://www.windmill.dev/docs/apps/schedule_reports"
		><svelte:fragment slot="actions">
			<div class="mr-4 center-center -mt-2">
				<Toggle
					checked={appReportingEnabled}
					options={{ right: 'enable', left: 'disable' }}
					on:change={async () => {
						if (appReportingEnabled) {
							disableAppReporting()
						} else {
							await enableAppReporting()
							sendUserToast('App reporting enabled')
						}
					}}
					disabled={disabled && !appReportingEnabled}
				/>
			</div>
			<Button
				color="dark"
				startIcon={{ icon: Save }}
				size="sm"
				on:click={async () => {
					await enableAppReporting()
					sendUserToast('App reporting updated')
					open = false
				}}
				{disabled}
			>
				{appReportingEnabled ? 'Update' : 'Save and enable'}
			</Button>
		</svelte:fragment>
		<div class="flex flex-col gap-8">
			<Alert type="info" title="Scheduled PDF/PNG reports"
				>Send a PDF or PNG preview of the app at a given schedule. Enabling this feature will create
				a flow and a schedule in your workspace.
				<br /><br />
				For the flow to be executed, you need to set the WORKER_GROUP environment variable of one of
				your workers to "reports" or add the tag "chromium" to one of your worker groups.
			</Alert>

			<Section label="Reporting schedule">
				<CronInput
					bind:schedule={appReportingSchedule.cron}
					bind:timezone={appReportingSchedule.timezone}
				/>
			</Section>

			<Section
				label="Startup duration in seconds"
				tooltip="The number of seconds to wait before capturing a preview to ensure that all startup scripts
		have been executed."
			>
				<div class="w-full pt-2">
					<input
						type="number"
						class="text-sm w-full font-semibold"
						bind:value={appReportingStartupDuration}
					/>
				</div>
			</Section>

			<Section label="Screenshot kind">
				<div class="w-full pt-2">
					<select class="text-sm w-full font-semibold" bind:value={screenshotKind}>
						<option value="pdf">PDF</option>
						<option value="png">PNG</option>
					</select>
				</div></Section
			>

			<Section label="Notification">
				<Tabs bind:selected={selectedTab}>
					{#if !$enterpriseLicense}
						<Tab value="custom">Custom</Tab>
					{/if}
					<Tab value="slack" disabled={!$enterpriseLicense}
						>Slack{!$enterpriseLicense ? ' (EE only)' : ''}</Tab
					>
					<Tab value="discord" disabled={!$enterpriseLicense}
						>Discord{!$enterpriseLicense ? ' (EE only)' : ''}</Tab
					>
					<Tab value="email" disabled={!$enterpriseLicense}>
						<div class="flex flex-row gap-1 items-center"
							>Email{!$enterpriseLicense ? ' (EE only)' : ''}
						</div>
					</Tab>
					{#if $enterpriseLicense}
						<Tab value="custom">Custom</Tab>
					{/if}
				</Tabs>
				{#if selectedTab === 'custom'}
					<div class="pt-2">
						<ScriptPicker
							on:select={(ev) => {
								customPath = ev.detail.path
							}}
							initialPath={customPath}
							allowRefresh
						/>
					</div>
					<div class="prose text-2xs text-tertiary mt-2">
						Pick a script that does whatever with the PDF/PNG report.

						<br />

						The script chosen is passed the parameters `screenshot: string`, `kind: 'pdf' | 'png'`,
						`app_path: string` where `screenshot` is the base64 encoded PDF/PNG report, `kind` is
						the type of the screenshot, and `app_path` is the path of the app being reported.
					</div>
				{/if}
				{#if selectedTab === 'slack'}
					<div class="pt-4">
						{#if isSlackConnectedWorkspace}
							<Alert type="info" title="Will use the Slack resource linked to the workspace" />
						{:else}
							<Alert type="error" title="Workspace not connected to Slack">
								<div class="flex flex-row gap-x-1 w-full items-center">
									<p class="text-clip grow min-w-0">
										The workspace needs to be connected to Slack to use this feature. You can <a
											target="_blank"
											href="/workspace_settings?tab=slack">configure it here</a
										>.
									</p>
									<Button
										variant="border"
										color="light"
										on:click={getWorspaceSlackSetting}
										startIcon={{ icon: RotateCw }}
									/>
								</div>
							</Alert>
						{/if}
					</div>
				{/if}
				<div class="w-full pt-4">
					{#if selectedTab !== 'custom' || customPath !== undefined}
						{#key selectedTab + JSON.stringify(customPathSchema ?? {})}
							<SchemaForm
								bind:isValid={areArgsValid}
								bind:args
								schema={selectedTab !== 'custom'
									? notificationScripts[selectedTab].schema
									: customPathSchema}
							/>
						{/key}
					{/if}
				</div>
				<Button
					loading={testLoading}
					{disabled}
					on:click={testReport}
					size="xs"
					color="dark"
					btnClasses="w-auto"
				>
					Send test report
				</Button>
			</Section>
		</div>
	</DrawerContent>
</Drawer>
