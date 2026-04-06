import type { MessageKey } from './en'

export const ja: Record<MessageKey, string> = {
	// Sidebar - main nav
	'sidebar.home': 'ホーム',
	'sidebar.runs': '実行',
	'sidebar.variables': '変数',
	'sidebar.resources': 'リソース',
	'sidebar.assets': 'アセット',
	'sidebar.tutorials': 'チュートリアル',

	// Sidebar - triggers section
	'sidebar.triggers': 'トリガー',
	'sidebar.schedules': 'スケジュール',
	'sidebar.http': 'HTTP',
	'sidebar.websockets': 'WebSockets',
	'sidebar.postgres': 'Postgres',
	'sidebar.kafka': 'Kafka',
	'sidebar.nats': 'NATS',
	'sidebar.sqs': 'SQS',
	'sidebar.gcp_pubsub': 'GCP Pub/Sub',
	'sidebar.mqtt': 'MQTT',
	'sidebar.email': 'メール',

	// Sidebar - settings
	'sidebar.settings': '設定',
	'sidebar.account': 'アカウント',
	'sidebar.workspace': 'ワークスペース',
	'sidebar.instance': 'インスタンス',
	'sidebar.leave_workspace': '退出',
	'sidebar.delete_fork': 'フォークを削除',
	'sidebar.workers': 'ワーカー',
	'sidebar.folders_groups': 'フォルダとグループ',
	'sidebar.folders': 'フォルダ',
	'sidebar.groups': 'グループ',
	'sidebar.logs': 'ログ',
	'sidebar.audit_logs': '監査ログ',
	'sidebar.service_logs': 'サービスログ',
	'sidebar.critical_alerts': '重要なアラート',

	// Sidebar - help
	'sidebar.help': 'ヘルプ',
	'sidebar.docs': 'ドキュメント',
	'sidebar.feedbacks': 'フィードバック',
	'sidebar.issues': 'Issues',
	'sidebar.changelog': '変更履歴',

	// User menu
	'user.account_settings': 'アカウント設定',
	'user.switch_theme': 'テーマ切替',
	'user.sign_out': 'ログアウト',
	'user.upgrade': 'アップグレード',
	'user.premium_plan': 'プレミアムプラン',
	'user.admin_of_workspace': '管理者',
	'user.operator_in_workspace': 'オペレーター',
	'user.language': '言語',

	// Operator menu
	'operator.custom_http_routes': 'HTTPルート',
	'operator.websocket_triggers': 'WebSocketトリガー',
	'operator.postgres_triggers': 'Postgresトリガー',
	'operator.kafka_triggers': 'Kafkaトリガー',
	'operator.nats_triggers': 'NATSトリガー',
	'operator.sqs_triggers': 'SQSトリガー',
	'operator.gcp_triggers': 'GCPトリガー',
	'operator.mqtt_triggers': 'MQTTトリガー',
	'operator.email_triggers': 'メールトリガー',

	// Common actions
	'action.save': '保存',
	'action.cancel': 'キャンセル',
	'action.delete': '削除',
	'action.edit': '編集',
	'action.create': '作成',
	'action.run': '実行',
	'action.deploy': 'デプロイ',
	'action.share': '共有',
	'action.close': '閉じる',
	'action.confirm': '確認',
	'action.refresh': '更新',

	// Page headers
	'page.home': 'ホーム',
	'page.variables': '変数',
	'page.resources': 'リソース',
	'page.schedules': 'スケジュール',
	'page.workers': 'ワーカー',
	'page.folders': 'フォルダ',
	'page.groups': 'グループ'
}
