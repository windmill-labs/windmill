<script lang="ts">
    import { Alert, Button } from '$lib/components/common'
    import SchemaForm from '$lib/components/SchemaForm.svelte'
    import Toggle from '$lib/components/Toggle.svelte'
    import type { Schema } from '$lib/common'
    import { workspaceStore } from '$lib/stores'
    import { emptyString, tryEvery } from '$lib/utils'
    import { JobService, ResourceService, WorkspaceService } from '$lib/gen'
    import Icon from 'svelte-awesome'
    import { check } from 'svelte-awesome/icons'
    import { faRotate, faRotateRight, faTimes } from '@fortawesome/free-solid-svg-icons'

    export const workspaceSlackConnectionResource = 'f/slack_bot/bot_token'

	export let can_write: boolean;

    export let handlerPath: string | undefined
    export let handlerPathToSet: string
    export let handlerSchema: Schema | undefined
    export let handlerExtraArgs: Record<string, any>

    let slackConnectionToken: {value: string, label:string} | undefined
	let slackConnectionTestJob: {uuid: string, is_success: boolean, in_progress: boolean} | undefined

    async function loadSlackResources() {
		const nc = (
			await ResourceService.listResource({
				workspace: $workspaceStore!,
				resourceType: 'slack',
			})
		)
			// filter out custom user token, use only the one created by the workspace Slack connection
			.filter((x) => x.path == workspaceSlackConnectionResource)
			.map((x) => ({
				value: "$res:" + x.path,
				label: x.path
			}))
		if (nc.length == 1) {
			slackConnectionToken = nc[0]
		}
	}

    async function sendSlackMessage(channel: string): Promise<void> {
		let submitted_job = await WorkspaceService.runSlackMessageTestJob({
			workspace: $workspaceStore!,
			requestBody: {
				hub_script_path: handlerPath,
				channel: channel,
				test_msg: `This is a notification to test the connection between Slack and Windmill workspace '${$workspaceStore!}'`
			}
		})
		slackConnectionTestJob = {
			uuid: submitted_job.job_uuid!,
			in_progress: true,
			is_success: false
		}
		tryEvery({
			tryCode: async () => {
				const testResult = await JobService.getCompletedJob({
					workspace: $workspaceStore!,
					id: slackConnectionTestJob!.uuid
				})
				slackConnectionTestJob!.in_progress = false
				slackConnectionTestJob!.is_success = testResult.success
			},
			timeoutCode: async () => {				
				try {
					await JobService.cancelQueuedJob({
						workspace: $workspaceStore!,
						id: slackConnectionTestJob!.uuid,
						requestBody: {
							reason: 'Slack message not sent after after 5s'
						}
					})
				} catch (err) {
					console.error(err)
				}
			},
			interval: 500,
			timeout: 5000
		})
	}

	$: {
		if ($workspaceStore) {
			loadSlackResources()
		}
	}
</script>

<span class="w-full flex mb-3">
    <Toggle
        disabled={!can_write}
        checked={ !emptyString(handlerPath) }
        options={{ right: 'enable'}}
        size='xs'
        on:change={async (e) => handlerPath = e.detail ? handlerPathToSet : undefined}
    />
</span>
{#if slackConnectionToken !== undefined}
    <SchemaForm
        disabled={!can_write || emptyString(handlerPath)}
        schema={handlerSchema}
        schemaSkippedValues={['slack']}
        schemaFieldTooltip={{'channel': 'Slack channel name without the "#" - example: "windmill-alerts"'}}
        bind:args={handlerExtraArgs}
        shouldHideNoInputs
        class="text-xs"
    />
{/if}
{#if !emptyString(handlerPath) }
    {#if slackConnectionToken === undefined}
        <Alert type="error" title="Workspace not connected to Slack">
            <div class="flex flex-row gap-x-1 w-full items-center">
                <p class="text-clip grow min-w-0">
                    The workspace needs to be connected to Slack to use this feature. You can <a target="_blank" href="/workspace_settings?tab=slack">configure it here</a>. 
                </p>
                <Button
                    variant="border"
                    color="light"
                    on:click={loadSlackResources}
                >
                    <Icon scale={0.8} data={faRotateRight} />
                </Button>
            </div>
        </Alert>
    {:else}
        <Button
            disabled={emptyString(handlerExtraArgs['channel'])}
            btnClasses="w-32 text-center"
            color="dark"
            on:click={() => sendSlackMessage(handlerExtraArgs['channel'])}
            size="xs">Send test message</Button
        >
        {#if slackConnectionTestJob !== undefined}
            <p class="text-normal text-2xs mt-1 gap-2">
                {#if slackConnectionTestJob.in_progress}
                    <Icon scale={0.8} data={faRotate} class="mr-1" />
                {:else if slackConnectionTestJob.is_success}
                    <Icon scale={0.8} data={check} class="mr-1 text-green-600" />
                {:else}
                    <Icon scale={0.8} data={faTimes} class="mr-1 text-red-700" />
                {/if}
                Message sent via Windmill job <a target="_blank" href={`/run/${slackConnectionTestJob.uuid}?workspace=${$workspaceStore}`}>{slackConnectionTestJob.uuid}</a>
            </p>
        {/if}
    {/if}
{/if}
