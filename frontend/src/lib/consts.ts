function strToBool(someStr) {
	return (someStr || 'true') === 'true'
}

export const DEFAULT_WEBHOOK_TYPE: 'async' | 'sync' = import.meta.env.VITE_DEFAULT_WEBHOOK_TYPE || 'async'

export const HOME_SHOW_HUB = strToBool(import.meta.env.VITE_HOME_SHOW_HUB)

export const HOME_SHOW_CREATE_FLOW = strToBool(import.meta.env.VITE_HOME_SHOW_CREATE_FLOW)
export const HOME_SEARCH_SHOW_FLOW = strToBool(import.meta.env.VITE_HOME_SEARCH_SHOW_FLOW)

export const HOME_SHOW_CREATE_APP = strToBool(import.meta.env.VITE_HOME_SHOW_CREATE_APP)

export const HOME_SEARCH_PLACEHOLDER = import.meta.env.VITE_HOME_SEARCH_PLACEHOLDER || 'Search Scripts, Flows & Apps'

export const SIDEBAR_SHOW_SCHEDULES = strToBool(import.meta.env.VITE_SIDEBAR_SHOW_SCHEDULES)

export const SCRIPT_SHOW_PSQL = strToBool(import.meta.env.VITE_SCRIPT_SHOW_PSQL)
export const SCRIPT_SHOW_GO = strToBool(import.meta.env.VITE_SCRIPT_SHOW_GO)
export const SCRIPT_SHOW_BASH = strToBool(import.meta.env.VITE_SCRIPT_SHOW_BASH)

export const WORKSPACE_SHOW_SLACK_CMD = strToBool(import.meta.env.VITE_WORKSPACE_SHOW_SLACK_CMD)
export const WORKSPACE_SHOW_WEBHOOK_CLI_SYNC = strToBool(import.meta.env.VITE_WORKSPACE_SHOW_WEBHOOK_CLI_SYNC)

export const SCRIPT_VIEW_SHOW_PUBLISH_TO_HUB = strToBool(import.meta.env.VITE_SCRIPT_VIEW_SHOW_PUBLISH_TO_HUB)
export const SCRIPT_VIEW_SHOW_SCHEDULE = strToBool(import.meta.env.VITE_SCRIPT_VIEW_SHOW_SCHEDULE)
export const SCRIPT_VIEW_SHOW_EXAMPLE_CURL = strToBool(import.meta.env.VITE_SCRIPT_VIEW_SHOW_EXAMPLE_CURL)
export const SCRIPT_VIEW_SHOW_CREATE_TOKEN_BUTTON = strToBool(import.meta.env.VITE_SCRIPT_VIEW_SHOW_CREATE_TOKEN_BUTTON)
export const SCRIPT_VIEW_SHOW_RUN_FROM_CLI = strToBool(import.meta.env.VITE_SCRIPT_VIEW_SHOW_RUN_FROM_CLI)
export const SCRIPT_VIEW_SHOW_SCHEDULE_RUN_LATER = strToBool(import.meta.env.VITE_SCRIPT_VIEW_SHOW_SCHEDULE_RUN_LATER)

export const SCRIPT_VIEW_WEBHOOK_INFO_TIP = import.meta.env.VITE_SCRIPT_VIEW_WEBHOOK_INFO_TIP || `Pass the input as a json payload, the token as a Bearer token (header: 'Authorization:
Bearer XXXX') or as query arg \`?token=XXX\`, and pass as header: 'Content-Type:
application/json'`
export const SCRIPT_VIEW_WEBHOOK_INFO_LINK = import.meta.env.VITE_SCRIPT_VIEW_WEBHOOK_INFO_LINK || 'https://docs.windmill.dev/docs/core_concepts/webhooks'

export const SCRIPT_EDITOR_SHOW_EXPLORE_OTHER_SCRIPTS = strToBool(import.meta.env.VITE_SCRIPT_EDITOR_SHOW_EXPLORE_OTHER_SCRIPTS)

export const SCRIPT_CUSTOMISE_SHOW_KIND = strToBool(import.meta.env.VITE_SCRIPT_CUSTOMISE_SHOW_KIND)
