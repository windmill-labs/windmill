import { WORKSPACE_SLACK_BOT_TOKEN_PATH } from '$lib/consts'

const AUTO_SLACK_ARG = '$res:' + WORKSPACE_SLACK_BOT_TOKEN_PATH

// `slack` belongs to the Slack built-in only while it holds the value the picker set; any other
// value is a custom handler's own argument and must survive handler switches and reloads.
export function syncSlackBotArg(
	args: Record<string, any>,
	isSlackBuiltin: boolean,
	wasSlackBuiltin: boolean
) {
	if (isSlackBuiltin) {
		args.slack = AUTO_SLACK_ARG
	} else if (wasSlackBuiltin && args.slack === AUTO_SLACK_ARG) {
		delete args.slack
	}
}
