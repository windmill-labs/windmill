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
	'action.duplicate': '复制',
	'action.fork': 'Fork',
	'action.rename': '重命名',
	'action.move': '移动',
	'action.copy_path': '复制路径',
	'action.export': '导出',
	'action.undo': '撤销',
	'action.redo': '重做',
	'action.test': '测试',
	'action.view': '查看',
	'action.next': '下一步',
	'action.previous': '上一步',
	'action.remove': '移除',
	'action.enable': '启用',
	'action.disable': '禁用',
	'action.see_permissions': '查看权限',
	'action.deploy_to_prod': '部署到生产/预发布',
	'action.refresh_token': '刷新令牌',
	'action.view_runs': '查看运行',
	'action.view_code': '查看代码',
	'action.audit_logs': '审计日志',
	'action.schedule': '定时',
	'action.versions': '版本',
	'action.delete_draft': '删除Draft',
	'action.publish_to_hub': '发布到Hub',
	'action.run_now': '立即运行',
	'action.manage': '管理',

	// Page headers
	'page.home': '首页',
	'page.variables': '变量',
	'page.resources': '资源',
	'page.schedules': '定时任务',
	'page.workers': 'Workers',
	'page.folders': '文件夹',
	'page.groups': '分组',
	'page.runs': '运行',

	// Home page
	'home.create_a': '创建',
	'home.workspace': '工作区',
	'home.hub': 'Hub',
	'home.scripts': '脚本',
	'home.flows': '流程',
	'home.apps': '应用',
	'home.hub_flow': 'Hub流程',
	'home.hub_app': 'Hub应用',
	'home.view_on_hub': '在Hub上查看',
	'home.content': '内容',

	// Variables page
	'variables.new_variable': '新建变量',
	'variables.new_contextual': '新建上下文变量',
	'variables.contextual': '上下文',
	'variables.custom_contextual': '自定义上下文变量',
	'variables.contextual_vars': '上下文变量',
	'variables.no_variables': '未找到变量',
	'variables.path': '路径',
	'variables.value': '值',
	'variables.description': '描述',

	// Resources page
	'resources.new_resource': '新建资源',

	// Schedules page
	'schedules.duplicate': '复制定时任务',
	'schedules.view_script': '查看脚本',
	'schedules.view_flow': '查看流程',

	// Runs page
	'runs.cancel_jobs': '取消任务',
	'runs.rerun_jobs': '重新运行任务',
	'runs.cancel_all': '取消所有符合筛选条件的任务',
	'runs.rerun_all': '重新运行所有符合筛选条件的任务',
	'runs.force_cancel': '强制取消',

	// Editor - common
	'editor.save': '保存',
	'editor.deploy': '部署',
	'editor.draft': 'Draft',
	'editor.test': '测试',
	'editor.test_and_record': '测试并记录',
	'editor.history': '历史',
	'editor.versions_history': '版本历史',
	'editor.deployment_history': '部署历史',
	'editor.edit_in_yaml': '以YAML编辑',
	'editor.flow_settings': '流程设置',
	'editor.export': '导出',
	'editor.diff': '差异对比',
	'editor.editor': '编辑器',
	'editor.preview': '预览',
	'editor.main': '主程序',
	'editor.preprocessor': '预处理器',
	'editor.diagram': '流程图',
	'editor.save_to_workspace': '保存到工作区',
	'editor.exit_see_details': '退出并查看详情',
	'editor.edit_in_fork': '在分支中编辑',
	'editor.save_anyway': '仍然保存',
	'editor.invite_others': '邀请其他人',

	// Flow editor
	'flow.test_flow': '测试流程',
	'flow.test_flow_record': '测试流程并记录',
	'flow.build_a_flow': '创建流程',
	'flow.fix_broken_flow': '修复异常流程',
	'flow.reset_tutorials': '重置教程',
	'flow.skip_tutorials': '跳过教程',
	'flow.create_group': '创建分组',
	'flow.test_iteration': '测试迭代',
	'flow.skip_failures': '跳过失败',
	'flow.run_parallel': '并行运行',

	// App editor
	'app.public_url': '公开URL',
	'app.app_inputs': '应用输入',
	'app.schedule_reports': '定时报告',
	'app.troubleshoot': '排障面板',
	'app.lazy_mode': '懒加载模式',
	'app.hub_export': 'Hub导出',

	// Status labels
	'status.default': '默认',
	'status.primary': '主要',
	'status.modified': '已修改',
	'status.draft': 'Draft',

	// Tabs - common
	'tab.workspace': '工作区',
	'tab.contextual': '上下文',
	'tab.hub': 'Hub',
	'tab.scripts': '脚本',
	'tab.flows': '流程',
	'tab.apps': '应用',

	// Row actions (scripts/flows/apps)
	'row.view_json': '查看JSON/YAML',
	'row.move_rename': '移动/重命名',
	'row.duplicate_fork': '复制/Fork',
	'row.go_to_public_page': '前往公开页面',
	'row.deployments': '部署记录',

	// Confirmation dialogs
	'confirm.delete_forever': '永久删除',
	'confirm.empty_trashbin': '清空回收站',
	'confirm.discard_changes': '放弃更改',
	'confirm.override': '覆盖',

	// Groups/Folders
	'groups.manage_group': '管理分组',
	'groups.new_group': '新建分组',
	'folders.manage_folder': '管理文件夹',
	'folders.new_folder': '新建文件夹',

	// Triggers
	'triggers.suspend_job': '暂停任务执行',
	'triggers.more_triggers': '更多触发器',
	'triggers.scheduled_poll': '定时轮询',

	// Misc
	'misc.all_workspaces': '所有工作区',
	'misc.instance_settings': '实例设置',
	'misc.search': '搜索',
	'misc.filter': '筛选',
	'misc.tree_view': '树形视图',
	'misc.loading': '加载中...'
}
