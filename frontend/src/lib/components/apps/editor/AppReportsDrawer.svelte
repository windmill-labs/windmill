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
		RawScript,
		ScheduleService,
		ScriptService,
		WorkspaceService
	} from '$lib/gen'
	import { workspaceStore } from '$lib/stores'
	import { emptyString, formatCron, sendUserToast } from '$lib/utils'
	import SchemaForm from '$lib/components/SchemaForm.svelte'
	import Button from '$lib/components/common/button/Button.svelte'
	import Toggle from '$lib/components/Toggle.svelte'
	import { Save } from 'lucide-svelte'
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
	let selectedTab: 'email' | 'slack' | 'discord' | 'custom' = 'custom'
	let customPath: string | undefined = undefined
	let customPathSchema: Record<string, any> = {}
	let args: Record<string, any> = {}

	$: customPath
		? ScriptService.getScriptByPath({
				path: customPath,
				workspace: $workspaceStore!
		  }).then((script) => {
				customPathSchema = script.schema
					? {
							...script.schema,
							properties: Object.fromEntries(
								Object.entries(script.schema.properties ?? {}).filter(
									([key, _]) => key !== 'pdf' && key !== 'app_path'
								)
							)
					  }
					: {}
		  })
		: (customPathSchema = {})

	$: selectedTab === 'slack' &&
		$enterpriseLicense &&
		!appReportingEnabled &&
		setWorkspaceSlackResource()

	let rerenderingSlackArgs = false
	async function setWorkspaceSlackResource() {
		const settings = await WorkspaceService.getSettings({
			workspace: $workspaceStore!
		})
		if (settings.slack_name) {
			args['slack'] = '$res:f/slack_bot/bot_token'
			rerenderingSlackArgs = true // force rerendering of schema form for resource picker to take into account args
		}
	}

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
			appReportingStartupDuration ??= schedule.args?.startup_duration
			args = schedule.args
				? Object.fromEntries(
						Object.entries(schedule.args).filter(
							([key, _]) => key !== 'app_path' && key !== 'startup_duration'
						)
				  )
				: {}

			selectedTab =
				flow.value.modules[1]?.value.type === 'script'
					? 'custom'
					: schedule.args?.smtp
					? 'email'
					: schedule.args?.discord_webhook
					? 'discord'
					: schedule.args?.slack
					? 'slack'
					: 'custom'

			customPath =
				flow.value.modules[1]?.value.type === 'script'
					? flow.value.modules[1]?.value.path
					: undefined

			appReportingEnabled = true
		} catch (err) {}
	}

	getAppReportingInfo()

	async function disableAppReporting(skipToast = false) {
		const flowPath = appPath + '_reports'
		await ScheduleService.deleteSchedule({
			workspace: $workspaceStore!,
			path: flowPath
		})
		await FlowService.deleteFlowByPath({
			workspace: $workspaceStore!,
			path: flowPath
		})
		if (!skipToast) {
			appReportingEnabled = false
			sendUserToast('App reporting disabled')
		}
	}

	const pdfPreviewScript = `import puppeteer from \'puppeteer-core\';
export async function main(app_path: string, startup_duration = 5) {
  const browser = await puppeteer.launch({ headless: \'new\', executablePath: \'/usr/bin/chromium\', args: [\'--no-sandbox\'] });
  const page = await browser.newPage();
  await page.setCookie({
    "name": "token",
    "value": Bun.env["WM_TOKEN"],
    "domain": Bun.env["WM_BASE_URL"]?.replace(/https?:\\/\\//, \'\')
  })
  await page.goto(Bun.env["WM_BASE_URL"] + \'/apps/get/\' + app_path + \'?workspace=\' + Bun.env["WM_WORKSPACE"]);
  await new Promise((resolve, _) => {
      setTimeout(resolve, startup_duration * 1000)
  })
  await page.$eval("#sidebar", el => el.remove())
  await page.$eval("#content", el => el.classList.remove("md:pl-12"))
  await page.$eval("#app-edit-btn", el => el.remove())
  const elem = await page.$(\'#app-content\');
  const { height } = await elem.boundingBox();
  await page.setViewport({ width: 1200, height });
  await new Promise((resolve, _) => {
      setTimeout(resolve, 200)
  })
  const pdf = await page.pdf({
      printBackground: true,
      width: 1200,
      height
  });
  await browser.close();
  return Buffer.from(pdf).toString(\'base64\');
}`

	const notificationScripts = {
		discord: {
			script: `type DiscordWebhook = {
  webhook_url: string;
};
export async function main(discord_webhook: DiscordWebhook, pdf: string, app_path: string) {
    const formData = new FormData();
    formData.append(\'files[0]\', new Blob([Buffer.from(pdf, \'base64\')], {
      type: "application/pdf"
}), "report.pdf")
  formData.append(\'payload_json\', JSON.stringify({"content": "App report of " + app_path}))
  const response = await fetch(discord_webhook.webhook_url, {
      method: \'POST\',
    body: formData
  })
  return response.text()
}`,
			schema: {
				type: 'object',
				properties: {
					discord_webhook: {
						type: 'object',
						format: 'resource-discord_webhook',
						properties: {},
						required: []
					}
				},
				required: ['discord_webhook']
			}
		},
		slack: {
			script: `type Slack = {
  token: string;
};
export async function main(
  slack: Slack,
  channel: string,
  pdf: string,
  app_path: string,
) {
  const formData = new FormData();
  formData.append("token", slack.token);
  formData.append("file", new Blob([Buffer.from(pdf, 'base64')]), "report.pdf");
  formData.append("channels", channel);
  formData.append("initial_comment", "App report of " + app_path)
  return await (
    await fetch("https://slack.com/api/files.upload", {
      method: "POST",
      body: formData,
    })
  ).json();
}`,
			schema: {
				type: 'object',
				properties: {
					slack: {
						type: 'object',
						format: 'resource-slack',
						properties: {},
						required: []
					},
					channel: {
						type: 'string',
						default: ''
					}
				},
				required: ['slack', 'channel']
			}
		},
		email: {
			script: `import nodemailer from 'nodemailer'
type Smtp = {
  host: string;
  port: number;
  user: string;
  password: string;
};
export async function main(
  smtp: Smtp,
  pdf: string,
  to_email: string,
  from_email: string,
  app_path: string,
) {

  const transporter = nodemailer.createTransport({
    host: smtp.host,
    port: smtp.port,
    secure: true,
    auth: {
      user: smtp.user,
      pass: smtp.password,
    },
  });

  return await transporter.sendMail({
    from: from_email,
    to: to_email,
    subject: "App report of " + app_path,
    text: "App report of " + app_path + "\\n\\n",
    attachments: [
      {
        filename: 'report.pdf',
        content: Buffer.from(pdf, 'base64')
      },
    ]
  });
}`,

			schema: {
				type: 'object',
				properties: {
					smtp: {
						type: 'object',
						format: 'resource-smtp',
						properties: {},
						required: []
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

	async function updateAppReporting() {
		await disableAppReporting(true)
		await enableAppReporting(true)
		sendUserToast('App reporting updated')
	}

	async function enableAppReporting(skipToast = false) {
		const flowPath = appPath + '_reports'

		const inputTransforms: {
			[key: string]: {
				expr: string
				type: 'javascript'
			}
		} = {
			app_path: {
				type: 'javascript',
				expr: 'flow_input.app_path'
			},
			pdf: {
				type: 'javascript',
				expr: 'results.a'
			},
			...Object.fromEntries(
				Object.keys(args).map((key) => [
					key,
					{
						type: 'javascript',
						expr: `flow_input.${key}`
					}
				])
			)
		}
		await FlowService.createFlow({
			workspace: $workspaceStore!,
			requestBody: {
				summary: appPath + ' - Reports flow',
				value: {
					modules: [
						{
							id: 'a',
							value: {
								type: 'rawscript',
								tag: 'chromium',
								content: pdfPreviewScript,
								language: RawScript.language.BUN,
								input_transforms: {
									app_path: {
										expr: 'flow_input.app_path',
										type: 'javascript'
									},
									startup_duration: {
										expr: 'flow_input.startup_duration',
										type: 'javascript'
									}
								}
							}
						},
						{
							id: 'b',
							value:
								selectedTab === 'custom'
									? {
											type: 'script',
											path: customPath || '',
											input_transforms: inputTransforms
									  }
									: {
											type: 'rawscript',
											content: notificationScripts[selectedTab].script,
											language: RawScript.language.BUN,
											input_transforms: inputTransforms
									  }
						}
					]
				},
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
						...(selectedTab === 'custom'
							? customPathSchema.properties
							: notificationScripts[selectedTab].schema.properties)
					},
					required: [
						'app_path',
						'startup_duration',
						...(selectedTab === 'custom'
							? customPathSchema.required
							: notificationScripts[selectedTab].schema.required)
					],
					type: 'object'
				},
				path: flowPath
			}
		})

		await ScheduleService.createSchedule({
			workspace: $workspaceStore!,
			requestBody: {
				path: flowPath,
				schedule: formatCron(appReportingSchedule.cron),
				timezone: appReportingSchedule.timezone,
				script_path: flowPath,
				is_flow: true,
				args: {
					app_path: appPath,
					startup_duration: appReportingStartupDuration,
					...args
				},
				enabled: true
			}
		})
		appReportingEnabled = true
		if (!skipToast) {
			sendUserToast('App reporting enabled')
		}
	}
</script>

<Drawer bind:open>
	<DrawerContent title="Schedule Reports" on:close={() => (open = false)}
		><svelte:fragment slot="actions">
			<div class="mr-4 center-center -mt-2">
				<Toggle
					checked={appReportingEnabled}
					options={{ right: 'enable', left: 'disable' }}
					on:change={async () => {
						if (appReportingEnabled) {
							disableAppReporting()
						} else {
							enableAppReporting()
						}
					}}
					disabled={emptyString(appReportingSchedule.cron) ||
						(selectedTab === 'custom' && emptyString(customPath)) ||
						(selectedTab !== 'custom' &&
							Object.keys(notificationScripts[selectedTab].schema.properties).some((key) =>
								emptyString(args[key])
							))}
				/>
			</div>
			<Button
				color="dark"
				startIcon={{ icon: Save }}
				size="sm"
				on:click={async () => {
					appReportingEnabled ? await updateAppReporting() : await enableAppReporting()
					open = false
				}}
				disabled={emptyString(appReportingSchedule.cron) ||
					(selectedTab === 'custom' && emptyString(customPath)) ||
					(selectedTab !== 'custom' &&
						Object.keys(notificationScripts[selectedTab].schema.properties).some((key) =>
							emptyString(args[key])
						))}
			>
				{appReportingEnabled ? 'Update' : 'Save and enable'}
			</Button>
		</svelte:fragment>
		<div class="flex flex-col gap-8">
			<Alert type="info" title="Scheduled PDF reports"
				>Send a PDF preview of the app at a given schedule. Enabling this feature will create a flow
				and a schedule in your workspace.
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

			<Section label="Notification">
				<Tabs bind:selected={selectedTab}>
					<Tab value="custom">Custom</Tab>
					<Tab value="email" disabled={!$enterpriseLicense}>
						<div class="flex flex-row gap-1 items-center"
							>Email{!$enterpriseLicense ? ' (EE only)' : ''}</div
						>
					</Tab>
					<Tab value="slack" disabled={!$enterpriseLicense}
						>Slack{!$enterpriseLicense ? ' (EE only)' : ''}</Tab
					>
					<Tab value="discord" disabled={!$enterpriseLicense}
						>Discord{!$enterpriseLicense ? ' (EE only)' : ''}</Tab
					>
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
						Pick a script that does whatever with the PDF report.

						<br />

						The script chosen is passed the parameters `pdf: string` and `app_path: string` where
						`pdf` is the base64 encoded PDF report and `app_path` is the path of the app being
						reported.
					</div>
				{/if}
				<div class="w-full pt-4">
					{#key rerenderingSlackArgs}
						<SchemaForm
							bind:args
							schema={selectedTab !== 'custom'
								? notificationScripts[selectedTab].schema
								: customPathSchema}
						/>
					{/key}
				</div>
			</Section>
		</div>
	</DrawerContent>
</Drawer>
