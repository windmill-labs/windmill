import { JobService, ScheduleService } from "$lib/gen"
import { sendUserToast } from "$lib/utils"

export async function runScheduleNow(
    path: string,
    schedulePath: string,
    isFlow: boolean,
    workspace_id: string
): Promise<void> {
    try {
        const runByPath = isFlow ? JobService.runFlowByPath : JobService.runScriptByPath
        const args = (
            await ScheduleService.getSchedule({
                workspace: workspace_id,
                path: schedulePath
            })
        ).args
        const run = await runByPath({
            path,
            requestBody: args ?? {},
            workspace: workspace_id
        })

        sendUserToast(`Schedule ${path} will run now`, false, [
            {
                label: 'Go to the run page',
                callback: () => window.open('/run/' + run + '?workspace=' + workspace_id, '_blank')
            }
        ])
    } catch (err) {
        sendUserToast(`Cannot run schedule now: ${err.body}`, true)
    }
}