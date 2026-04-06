import type { MessageKey } from './en'

export const zh: Record<MessageKey, string> = {
	// Sidebar - main nav
	'sidebar.home': '首页',
	'sidebar.runs': '运行',
	'sidebar.variables': '变量',
	'sidebar.resources': '资源',
	'sidebar.assets': '资产',
	'sidebar.tutorials': '教程',

	// Sidebar - triggers section
	'sidebar.triggers': '触发器',
	'sidebar.schedules': '定时任务',
	'sidebar.http': 'HTTP',
	'sidebar.websockets': 'WebSockets',
	'sidebar.postgres': 'Postgres',
	'sidebar.kafka': 'Kafka',
	'sidebar.nats': 'NATS',
	'sidebar.sqs': 'SQS',
	'sidebar.gcp_pubsub': 'GCP Pub/Sub',
	'sidebar.mqtt': 'MQTT',
	'sidebar.email': '邮件',

	// Sidebar - settings
	'sidebar.settings': '设置',
	'sidebar.account': '账户',
	'sidebar.workspace': '工作区',
	'sidebar.instance': '实例',
	'sidebar.leave_workspace': '退出',
	'sidebar.delete_fork': '删除分支',
	'sidebar.workers': 'Workers',
	'sidebar.folders_groups': '文件夹和分组',
	'sidebar.folders': '文件夹',
	'sidebar.groups': '分组',
	'sidebar.logs': '日志',
	'sidebar.audit_logs': '审计日志',
	'sidebar.service_logs': '服务日志',
	'sidebar.critical_alerts': '严重告警',

	// Sidebar - help
	'sidebar.help': '帮助',
	'sidebar.docs': '文档',
	'sidebar.feedbacks': '反馈',
	'sidebar.issues': 'Issues',
	'sidebar.changelog': '更新日志',

	// User menu
	'user.account_settings': '账户设置',
	'user.switch_theme': '切换主题',
	'user.sign_out': '退出登录',
	'user.upgrade': '升级',
	'user.premium_plan': '高级版',
	'user.admin_of_workspace': '管理员',
	'user.operator_in_workspace': '操作员',
	'user.language': '语言',

	// Operator menu
	'operator.custom_http_routes': 'HTTP路由',
	'operator.websocket_triggers': 'WebSocket触发器',
	'operator.postgres_triggers': 'Postgres触发器',
	'operator.kafka_triggers': 'Kafka触发器',
	'operator.nats_triggers': 'NATS触发器',
	'operator.sqs_triggers': 'SQS触发器',
	'operator.gcp_triggers': 'GCP触发器',
	'operator.mqtt_triggers': 'MQTT触发器',
	'operator.email_triggers': '邮件触发器',

	// Common actions
	'action.save': '保存',
	'action.cancel': '取消',
	'action.delete': '删除',
	'action.edit': '编辑',
	'action.create': '创建',
	'action.run': '运行',
	'action.deploy': '部署',
	'action.share': '共享',
	'action.close': '关闭',
	'action.confirm': '确认',
	'action.refresh': '刷新',

	// Page headers
	'page.home': '首页',
	'page.variables': '变量',
	'page.resources': '资源',
	'page.schedules': '定时任务',
	'page.workers': 'Workers',
	'page.folders': '文件夹',
	'page.groups': '分组'
}
