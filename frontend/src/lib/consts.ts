function strToBool(someStr) {
	return (someStr || 'true') === 'true'
}

export const DEFAULT_WEBHOOK_TYPE: 'async' | 'sync' = import.meta.env.VITE_DEFAULT_WEBHOOK_TYPE || 'async'

export const HOME_SHOW_HUB = strToBool(import.meta.env.VITE_HOME_SHOW_HUB)

export const HOME_SHOW_CREATE_FLOW = strToBool(import.meta.env.VITE_HOME_SHOW_CREATE_FLOW)
export const HOME_SEARCH_SHOW_FLOW = strToBool(import.meta.env.VITE_HOME_SEARCH_SHOW_FLOW)

export const HOME_SEARCH_PLACEHOLDER = import.meta.env.VITE_HOME_SEARCH_PLACEHOLDER || 'Search Scripts, Flows & Apps'

export const SIDEBAR_SHOW_SCHEDULES = strToBool(import.meta.env.VITE_SIDEBAR_SHOW_SCHEDULES)

export const SCRIPT_SHOW_PSQL = strToBool(import.meta.env.VITE_SCRIPT_SHOW_PSQL)
export const SCRIPT_SHOW_GO = strToBool(import.meta.env.VITE_SCRIPT_SHOW_GO)
export const SCRIPT_SHOW_BASH = strToBool(import.meta.env.VITE_SCRIPT_SHOW_BASH)

export const WORKSPACE_SHOW_SLACK_CMD = strToBool(import.meta.env.VITE_WORKSPACE_SHOW_SLACK_CMD)
export const WORKSPACE_SHOW_WEBHOOK_CLI_SYNC = strToBool(import.meta.env.VITE_WORKSPACE_SHOW_WEBHOOK_CLI_SYNC)

export const SCRIPT_VIEW_SHOW_PUBLISH_TO_HUB = strToBool(import.meta.env.VITE_SCRIPT_VIEW_SHOW_PUBLISH_TO_HUB)
export const SCRIPT_VIEW_SHOW_SCHEDULE = strToBool(import.meta.env.VITE_SCRIPT_VIEW_SHOW_SCHEDULE)
