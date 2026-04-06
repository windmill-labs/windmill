export const en = {
	// Sidebar - main nav
	'sidebar.home': 'Home',
	'sidebar.runs': 'Runs',
	'sidebar.variables': 'Variables',
	'sidebar.resources': 'Resources',
	'sidebar.assets': 'Assets',
	'sidebar.tutorials': 'Tutorials',

	// Sidebar - triggers section
	'sidebar.triggers': 'Triggers',
	'sidebar.schedules': 'Schedules',
	'sidebar.http': 'HTTP',
	'sidebar.websockets': 'WebSockets',
	'sidebar.postgres': 'Postgres',
	'sidebar.kafka': 'Kafka',
	'sidebar.nats': 'NATS',
	'sidebar.sqs': 'SQS',
	'sidebar.gcp_pubsub': 'GCP Pub/Sub',
	'sidebar.mqtt': 'MQTT',
	'sidebar.email': 'Email',

	// Sidebar - settings
	'sidebar.settings': 'Settings',
	'sidebar.account': 'Account',
	'sidebar.workspace': 'Workspace',
	'sidebar.instance': 'Instance',
	'sidebar.leave_workspace': 'Leave workspace',
	'sidebar.delete_fork': 'Delete Forked Workspace',
	'sidebar.workers': 'Workers',
	'sidebar.folders_groups': 'Folders & Groups',
	'sidebar.folders': 'Folders',
	'sidebar.groups': 'Groups',
	'sidebar.logs': 'Logs',
	'sidebar.audit_logs': 'Audit logs',
	'sidebar.service_logs': 'Service logs',
	'sidebar.critical_alerts': 'Critical alerts',

	// Sidebar - help
	'sidebar.help': 'Help',
	'sidebar.docs': 'Docs',
	'sidebar.feedbacks': 'Feedbacks',
	'sidebar.issues': 'Issues',
	'sidebar.changelog': 'Changelog',

	// User menu
	'user.account_settings': 'Account settings',
	'user.switch_theme': 'Switch theme',
	'user.sign_out': 'Sign out',
	'user.upgrade': 'Upgrade',
	'user.premium_plan': 'Premium plan',
	'user.admin_of_workspace': 'Admin of this workspace',
	'user.operator_in_workspace': 'Operator in this workspace',
	'user.language': 'Language',

	// Operator menu
	'operator.custom_http_routes': 'Custom HTTP routes',
	'operator.websocket_triggers': 'Websocket triggers',
	'operator.postgres_triggers': 'Postgres triggers',
	'operator.kafka_triggers': 'Kafka triggers',
	'operator.nats_triggers': 'NATS triggers',
	'operator.sqs_triggers': 'SQS triggers',
	'operator.gcp_triggers': 'GCP Pub/Sub triggers',
	'operator.mqtt_triggers': 'MQTT triggers',
	'operator.email_triggers': 'Email triggers',

	// Common actions
	'action.save': 'Save',
	'action.cancel': 'Cancel',
	'action.delete': 'Delete',
	'action.edit': 'Edit',
	'action.create': 'Create',
	'action.run': 'Run',
	'action.deploy': 'Deploy',
	'action.share': 'Share',
	'action.close': 'Close',
	'action.confirm': 'Confirm',
	'action.refresh': 'Refresh',

	// Page headers
	'page.home': 'Home',
	'page.variables': 'Variables',
	'page.resources': 'Resources',
	'page.schedules': 'Schedules',
	'page.workers': 'Workers',
	'page.folders': 'Folders',
	'page.groups': 'Groups'
} as const

export type MessageKey = keyof typeof en
