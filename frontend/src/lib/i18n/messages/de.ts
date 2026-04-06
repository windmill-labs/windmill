import type { MessageKey } from './en'

export const de: Record<MessageKey, string> = {
	// Sidebar - main nav
	'sidebar.home': 'Start',
	'sidebar.runs': 'Ausführungen',
	'sidebar.variables': 'Variablen',
	'sidebar.resources': 'Ressourcen',
	'sidebar.assets': 'Assets',
	'sidebar.tutorials': 'Tutorials',

	// Sidebar - triggers section
	'sidebar.triggers': 'Trigger',
	'sidebar.schedules': 'Zeitpläne',
	'sidebar.http': 'HTTP',
	'sidebar.websockets': 'WebSockets',
	'sidebar.postgres': 'Postgres',
	'sidebar.kafka': 'Kafka',
	'sidebar.nats': 'NATS',
	'sidebar.sqs': 'SQS',
	'sidebar.gcp_pubsub': 'GCP Pub/Sub',
	'sidebar.mqtt': 'MQTT',
	'sidebar.email': 'E-Mail',

	// Sidebar - settings
	'sidebar.settings': 'Einstellungen',
	'sidebar.account': 'Konto',
	'sidebar.workspace': 'Workspace',
	'sidebar.instance': 'Instanz',
	'sidebar.leave_workspace': 'Verlassen',
	'sidebar.delete_fork': 'Fork löschen',
	'sidebar.workers': 'Workers',
	'sidebar.folders_groups': 'Ordner & Gruppen',
	'sidebar.folders': 'Ordner',
	'sidebar.groups': 'Gruppen',
	'sidebar.logs': 'Protokolle',
	'sidebar.audit_logs': 'Audit',
	'sidebar.service_logs': 'Dienste',
	'sidebar.critical_alerts': 'Kritische Alerts',

	// Sidebar - help
	'sidebar.help': 'Hilfe',
	'sidebar.docs': 'Doku',
	'sidebar.feedbacks': 'Feedback',
	'sidebar.issues': 'Issues',
	'sidebar.changelog': 'Changelog',

	// User menu
	'user.account_settings': 'Kontoeinstellungen',
	'user.switch_theme': 'Design wechseln',
	'user.sign_out': 'Abmelden',
	'user.upgrade': 'Upgrade',
	'user.premium_plan': 'Premium-Plan',
	'user.admin_of_workspace': 'Admin',
	'user.operator_in_workspace': 'Operator',
	'user.language': 'Sprache',

	// Operator menu
	'operator.custom_http_routes': 'HTTP-Routen',
	'operator.websocket_triggers': 'WebSocket-Trigger',
	'operator.postgres_triggers': 'Postgres-Trigger',
	'operator.kafka_triggers': 'Kafka-Trigger',
	'operator.nats_triggers': 'NATS-Trigger',
	'operator.sqs_triggers': 'SQS-Trigger',
	'operator.gcp_triggers': 'GCP-Trigger',
	'operator.mqtt_triggers': 'MQTT-Trigger',
	'operator.email_triggers': 'E-Mail-Trigger',

	// Common actions
	'action.save': 'Speichern',
	'action.cancel': 'Abbrechen',
	'action.delete': 'Löschen',
	'action.edit': 'Bearbeiten',
	'action.create': 'Erstellen',
	'action.run': 'Ausführen',
	'action.deploy': 'Deployen',
	'action.share': 'Teilen',
	'action.close': 'Schließen',
	'action.confirm': 'Bestätigen',
	'action.refresh': 'Aktualisieren',
	'action.duplicate': 'Duplizieren',
	'action.fork': 'Fork',
	'action.rename': 'Umbenennen',
	'action.move': 'Verschieben',
	'action.copy_path': 'Pfad kopieren',
	'action.export': 'Exportieren',
	'action.undo': 'Rückgängig',
	'action.redo': 'Wiederholen',
	'action.test': 'Testen',
	'action.view': 'Ansehen',
	'action.next': 'Weiter',
	'action.previous': 'Zurück',
	'action.remove': 'Entfernen',
	'action.enable': 'Aktivieren',
	'action.disable': 'Deaktivieren',
	'action.see_permissions': 'Berechtigungen',
	'action.deploy_to_prod': 'Auf Prod deployen',
	'action.refresh_token': 'Token erneuern',
	'action.view_runs': 'Ausführungen',
	'action.view_code': 'Code ansehen',
	'action.audit_logs': 'Audit-Logs',
	'action.schedule': 'Planen',
	'action.versions': 'Versionen',
	'action.delete_draft': 'Draft löschen',
	'action.publish_to_hub': 'Im Hub veröffentlichen',
	'action.run_now': 'Jetzt ausführen',
	'action.manage': 'Verwalten',

	// Page headers
	'page.home': 'Start',
	'page.variables': 'Variablen',
	'page.resources': 'Ressourcen',
	'page.schedules': 'Zeitpläne',
	'page.workers': 'Workers',
	'page.folders': 'Ordner',
	'page.groups': 'Gruppen',
	'page.runs': 'Ausführungen',

	// Home page
	'home.create_a': 'Erstellen:',
	'home.workspace': 'Workspace',
	'home.hub': 'Hub',
	'home.scripts': 'Skripte',
	'home.flows': 'Flows',
	'home.apps': 'Apps',
	'home.hub_flow': 'Hub-Flow',
	'home.hub_app': 'Hub-App',
	'home.view_on_hub': 'Im Hub ansehen',
	'home.content': 'Inhalte',

	// Variables page
	'variables.new_variable': 'Neue Variable',
	'variables.new_contextual': 'Neue Kontextvariable',
	'variables.contextual': 'Kontext',
	'variables.custom_contextual': 'Eigene Kontextvariablen',
	'variables.contextual_vars': 'Kontextvariablen',
	'variables.no_variables': 'Keine Variablen gefunden',
	'variables.path': 'Pfad',
	'variables.value': 'Wert',
	'variables.description': 'Beschreibung',

	// Resources page
	'resources.new_resource': 'Neue Ressource',

	// Schedules page
	'schedules.duplicate': 'Zeitplan duplizieren',
	'schedules.view_script': 'Skript ansehen',
	'schedules.view_flow': 'Flow ansehen',

	// Runs page
	'runs.cancel_jobs': 'Jobs abbrechen',
	'runs.rerun_jobs': 'Jobs neu starten',
	'runs.cancel_all': 'Alle gefilterten Jobs abbrechen',
	'runs.rerun_all': 'Alle gefilterten Jobs neu starten',
	'runs.force_cancel': 'Erzwungen abbrechen',

	// Editor - common
	'editor.save': 'Speichern',
	'editor.deploy': 'Deployen',
	'editor.deploy_stay': 'Bereitstellen & Hier bleiben',
	'editor.draft': 'Draft',
	'editor.test': 'Testen',
	'editor.test_and_record': 'Testen & aufzeichnen',
	'editor.cancel': 'Abbrechen',
	'editor.settings': 'Einstellungen',
	'editor.history': 'Verlauf',
	'editor.versions_history': 'Versionsverlauf',
	'editor.deployment_history': 'Deployment-Verlauf',
	'editor.edit_in_yaml': 'In YAML bearbeiten',
	'editor.flow_settings': 'Flow-Einstellungen',
	'editor.export': 'Exportieren',
	'editor.export_yaml_json': 'Als YAML/JSON exportieren',
	'editor.diff': 'Diff',
	'editor.show_diff': 'Diff anzeigen',
	'editor.editor': 'Editor',
	'editor.preview': 'Vorschau',
	'editor.main': 'Main',
	'editor.preprocessor': 'Preprocessor',
	'editor.diagram': 'Diagramm',
	'editor.save_to_workspace': 'Im Workspace speichern',
	'editor.save_initial_draft': 'Erstentwurf speichern',
	'editor.exit_see_details': 'Beenden & Details',
	'editor.edit_in_fork': 'Im Fork bearbeiten',
	'editor.save_anyway': 'Trotzdem speichern',
	'editor.invite_others': 'Andere einladen',
	'editor.deployment_message': 'Bereitstellungsnachricht',
	'editor.test_to_see_result': 'Testen, um das Ergebnis hier zu sehen',
	'editor.no_logs_available': 'Noch keine Logs verfügbar',
	'editor.waiting_for_job': 'Warten auf Jobstart...',

	// Flow editor
	'flow.test_flow': 'Flow testen',
	'flow.test_flow_record': 'Flow testen & aufzeichnen',
	'flow.build_a_flow': 'Flow erstellen',
	'flow.fix_broken_flow': 'Flow reparieren',
	'flow.reset_tutorials': 'Tutorials zurücksetzen',
	'flow.skip_tutorials': 'Tutorials überspringen',
	'flow.create_group': 'Gruppe erstellen',
	'flow.test_iteration': 'Iteration testen',
	'flow.skip_failures': 'Fehler überspringen',
	'flow.run_parallel': 'Parallel ausführen',

	// App editor
	'app.public_url': 'Öffentliche URL',
	'app.app_inputs': 'App-Eingaben',
	'app.schedule_reports': 'Berichte planen',
	'app.troubleshoot': 'Fehlerbehebung',
	'app.lazy_mode': 'Lazy-Modus',
	'app.hub_export': 'Hub-Export',
	'app.debug_runs': 'Debug-Ausführungen',

	// Status labels
	'status.default': 'Standard',
	'status.primary': 'Primär',
	'status.modified': 'Geändert',
	'status.draft': 'Draft',

	// Tabs - common
	'tab.workspace': 'Workspace',
	'tab.contextual': 'Kontext',
	'tab.hub': 'Hub',
	'tab.scripts': 'Skripte',
	'tab.flows': 'Flows',
	'tab.apps': 'Apps',

	// Row actions (scripts/flows/apps)
	'row.view_json': 'JSON/YAML ansehen',
	'row.move_rename': 'Verschieben/Umbenennen',
	'row.duplicate_fork': 'Duplizieren/Fork',
	'row.go_to_public_page': 'Öffentliche Seite',
	'row.deployments': 'Deployments',

	// Confirmation dialogs
	'confirm.delete_forever': 'Endgültig löschen',
	'confirm.empty_trashbin': 'Papierkorb leeren',
	'confirm.discard_changes': 'Änderungen verwerfen',
	'confirm.override': 'Überschreiben',
	'confirm.unsaved_changes_detected': 'Nicht gespeicherte Änderungen erkannt',
	'confirm.discard_changes_question':
		'Sind Sie sicher, dass Sie die vorgenommenen Änderungen verwerfen möchten?',
	'confirm.leave_anyway': 'Trotzdem verlassen',
	'confirm.override_anyway': 'Trotzdem überschreiben',
	'confirm.new_version_deployed':
		'Eine neue Version wurde bereitgestellt, während Sie diese bearbeitet haben.',

	// Groups/Folders
	'groups.manage_group': 'Gruppe verwalten',
	'groups.new_group': 'Neue Gruppe',
	'folders.manage_folder': 'Ordner verwalten',
	'folders.new_folder': 'Neuer Ordner',

	// Triggers
	'triggers.suspend_job': 'Job pausieren',
	'triggers.more_triggers': 'Weitere Trigger',
	'triggers.scheduled_poll': 'Geplante Abfrage',

	// Misc
	'misc.all_workspaces': 'Alle Workspaces',
	'misc.instance_settings': 'Instanzeinstellungen',
	'misc.search': 'Suchen',
	'misc.filter': 'Filtern',
	'misc.tree_view': 'Baumansicht',
	'misc.loading': 'Laden...'
}
