import type { MessageKey } from './en'

export const ko: Record<MessageKey, string> = {
	// Sidebar - main nav
	'sidebar.home': '홈',
	'sidebar.runs': '실행',
	'sidebar.variables': '변수',
	'sidebar.resources': '리소스',
	'sidebar.assets': '에셋',
	'sidebar.tutorials': '튜토리얼',

	// Sidebar - triggers section
	'sidebar.triggers': '트리거',
	'sidebar.schedules': '스케줄',
	'sidebar.http': 'HTTP',
	'sidebar.websockets': 'WebSockets',
	'sidebar.postgres': 'Postgres',
	'sidebar.kafka': 'Kafka',
	'sidebar.nats': 'NATS',
	'sidebar.sqs': 'SQS',
	'sidebar.gcp_pubsub': 'GCP Pub/Sub',
	'sidebar.mqtt': 'MQTT',
	'sidebar.email': '이메일',

	// Sidebar - settings
	'sidebar.settings': '설정',
	'sidebar.account': '계정',
	'sidebar.workspace': '워크스페이스',
	'sidebar.instance': '인스턴스',
	'sidebar.leave_workspace': '나가기',
	'sidebar.delete_fork': '포크 삭제',
	'sidebar.workers': '워커',
	'sidebar.folders_groups': '폴더 및 그룹',
	'sidebar.folders': '폴더',
	'sidebar.groups': '그룹',
	'sidebar.logs': '로그',
	'sidebar.audit_logs': '감사 로그',
	'sidebar.service_logs': '서비스 로그',
	'sidebar.critical_alerts': '심각한 알림',

	// Sidebar - help
	'sidebar.help': '도움말',
	'sidebar.docs': '문서',
	'sidebar.feedbacks': '피드백',
	'sidebar.issues': 'Issues',
	'sidebar.changelog': '변경 내역',

	// User menu
	'user.account_settings': '계정 설정',
	'user.switch_theme': '테마 전환',
	'user.sign_out': '로그아웃',
	'user.upgrade': '업그레이드',
	'user.premium_plan': '프리미엄 플랜',
	'user.admin_of_workspace': '관리자',
	'user.operator_in_workspace': '운영자',
	'user.language': '언어',

	// Operator menu
	'operator.custom_http_routes': 'HTTP 라우트',
	'operator.websocket_triggers': 'WebSocket 트리거',
	'operator.postgres_triggers': 'Postgres 트리거',
	'operator.kafka_triggers': 'Kafka 트리거',
	'operator.nats_triggers': 'NATS 트리거',
	'operator.sqs_triggers': 'SQS 트리거',
	'operator.gcp_triggers': 'GCP 트리거',
	'operator.mqtt_triggers': 'MQTT 트리거',
	'operator.email_triggers': '이메일 트리거',

	// Common actions
	'action.save': '저장',
	'action.cancel': '취소',
	'action.delete': '삭제',
	'action.edit': '편집',
	'action.create': '생성',
	'action.run': '실행',
	'action.deploy': '배포',
	'action.share': '공유',
	'action.close': '닫기',
	'action.confirm': '확인',
	'action.refresh': '새로고침',

	// Page headers
	'page.home': '홈',
	'page.variables': '변수',
	'page.resources': '리소스',
	'page.schedules': '스케줄',
	'page.workers': '워커',
	'page.folders': '폴더',
	'page.groups': '그룹'
}
