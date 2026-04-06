import type { MessageKey } from './en'

export const es: Record<MessageKey, string> = {
	// Sidebar - main nav
	'sidebar.home': 'Inicio',
	'sidebar.runs': 'Ejecuciones',
	'sidebar.variables': 'Variables',
	'sidebar.resources': 'Recursos',
	'sidebar.assets': 'Assets',
	'sidebar.tutorials': 'Tutoriales',

	// Sidebar - triggers section
	'sidebar.triggers': 'Disparadores',
	'sidebar.schedules': 'Programados',
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
	'sidebar.settings': 'Ajustes',
	'sidebar.account': 'Cuenta',
	'sidebar.workspace': 'Espacio',
	'sidebar.instance': 'Instancia',
	'sidebar.leave_workspace': 'Salir',
	'sidebar.delete_fork': 'Eliminar fork',
	'sidebar.workers': 'Workers',
	'sidebar.folders_groups': 'Carpetas y Grupos',
	'sidebar.folders': 'Carpetas',
	'sidebar.groups': 'Grupos',
	'sidebar.logs': 'Registros',
	'sidebar.audit_logs': 'Auditoría',
	'sidebar.service_logs': 'Servicios',
	'sidebar.critical_alerts': 'Alertas críticas',

	// Sidebar - help
	'sidebar.help': 'Ayuda',
	'sidebar.docs': 'Docs',
	'sidebar.feedbacks': 'Comentarios',
	'sidebar.issues': 'Issues',
	'sidebar.changelog': 'Changelog',

	// User menu
	'user.account_settings': 'Mi cuenta',
	'user.switch_theme': 'Cambiar tema',
	'user.sign_out': 'Cerrar sesión',
	'user.upgrade': 'Mejorar plan',
	'user.premium_plan': 'Plan premium',
	'user.admin_of_workspace': 'Admin',
	'user.operator_in_workspace': 'Operador',
	'user.language': 'Idioma',

	// Operator menu
	'operator.custom_http_routes': 'Rutas HTTP',
	'operator.websocket_triggers': 'Triggers WebSocket',
	'operator.postgres_triggers': 'Triggers Postgres',
	'operator.kafka_triggers': 'Triggers Kafka',
	'operator.nats_triggers': 'Triggers NATS',
	'operator.sqs_triggers': 'Triggers SQS',
	'operator.gcp_triggers': 'Triggers GCP',
	'operator.mqtt_triggers': 'Triggers MQTT',
	'operator.email_triggers': 'Triggers email',

	// Common actions
	'action.save': 'Guardar',
	'action.cancel': 'Cancelar',
	'action.delete': 'Eliminar',
	'action.edit': 'Editar',
	'action.create': 'Crear',
	'action.run': 'Ejecutar',
	'action.deploy': 'Desplegar',
	'action.share': 'Compartir',
	'action.close': 'Cerrar',
	'action.confirm': 'Confirmar',
	'action.refresh': 'Actualizar',
	'action.duplicate': 'Duplicar',
	'action.fork': 'Fork',
	'action.rename': 'Renombrar',
	'action.move': 'Mover',
	'action.copy_path': 'Copiar ruta',
	'action.export': 'Exportar',
	'action.undo': 'Deshacer',
	'action.redo': 'Rehacer',
	'action.test': 'Probar',
	'action.view': 'Ver',
	'action.next': 'Siguiente',
	'action.previous': 'Anterior',
	'action.remove': 'Quitar',
	'action.enable': 'Activar',
	'action.disable': 'Desactivar',
	'action.see_permissions': 'Ver permisos',
	'action.deploy_to_prod': 'Desplegar a prod/staging',
	'action.refresh_token': 'Renovar token',
	'action.view_runs': 'Ver ejecuciones',
	'action.view_code': 'Ver código',
	'action.audit_logs': 'Auditoría',
	'action.schedule': 'Programar',
	'action.versions': 'Versiones',
	'action.delete_draft': 'Eliminar Draft',
	'action.publish_to_hub': 'Publicar en Hub',
	'action.run_now': 'Ejecutar ahora',
	'action.manage': 'Gestionar',

	// Page headers
	'page.home': 'Inicio',
	'page.variables': 'Variables',
	'page.resources': 'Recursos',
	'page.schedules': 'Programados',
	'page.workers': 'Workers',
	'page.folders': 'Carpetas',
	'page.groups': 'Grupos',
	'page.runs': 'Ejecuciones',

	// Home page
	'home.create_a': 'Crear un',
	'home.workspace': 'Espacio',
	'home.hub': 'Hub',
	'home.scripts': 'Scripts',
	'home.flows': 'Flujos',
	'home.apps': 'Apps',
	'home.hub_flow': 'Flujo de Hub',
	'home.hub_app': 'App de Hub',
	'home.view_on_hub': 'Ver en el Hub',
	'home.content': 'Contenido',

	// Variables page
	'variables.new_variable': 'Nueva variable',
	'variables.new_contextual': 'Nueva var. contextual',
	'variables.contextual': 'Contextual',
	'variables.custom_contextual': 'Variables contextuales',
	'variables.contextual_vars': 'Variables contextuales',
	'variables.no_variables': 'No se encontraron variables',
	'variables.path': 'Ruta',
	'variables.value': 'Valor',
	'variables.description': 'Descripción',

	// Resources page
	'resources.new_resource': 'Nuevo recurso',

	// Schedules page
	'schedules.duplicate': 'Duplicar programa',
	'schedules.view_script': 'Ver Script',
	'schedules.view_flow': 'Ver Flujo',

	// Runs page
	'runs.cancel_jobs': 'Cancelar trabajos',
	'runs.rerun_jobs': 'Reiniciar trabajos',
	'runs.cancel_all': 'Cancelar todos según filtros',
	'runs.rerun_all': 'Reiniciar todos según filtros',
	'runs.force_cancel': 'Forzar cancelación',

	// Editor - common
	'editor.save': 'Guardar',
	'editor.deploy': 'Desplegar',
	'editor.draft': 'Draft',
	'editor.test': 'Probar',
	'editor.test_and_record': 'Probar y grabar',
	'editor.history': 'Historial',
	'editor.versions_history': 'Historial de versiones',
	'editor.deployment_history': 'Historial de despliegues',
	'editor.edit_in_yaml': 'Editar en YAML',
	'editor.flow_settings': 'Ajustes del flujo',
	'editor.export': 'Exportar',
	'editor.diff': 'Diff',
	'editor.editor': 'Editor',
	'editor.preview': 'Vista previa',
	'editor.main': 'Principal',
	'editor.preprocessor': 'Preprocesador',
	'editor.diagram': 'Diagrama',
	'editor.save_to_workspace': 'Guardar en espacio',
	'editor.exit_see_details': 'Salir y ver detalles',
	'editor.edit_in_fork': 'Editar en fork',
	'editor.save_anyway': 'Guardar de todos modos',
	'editor.invite_others': 'Invitar a otros',

	// Flow editor
	'flow.test_flow': 'Probar flujo',
	'flow.test_flow_record': 'Probar flujo y grabar',
	'flow.build_a_flow': 'Crear un flujo',
	'flow.fix_broken_flow': 'Reparar flujo roto',
	'flow.reset_tutorials': 'Reiniciar tutoriales',
	'flow.skip_tutorials': 'Omitir tutoriales',
	'flow.create_group': 'Crear grupo',
	'flow.test_iteration': 'Probar iteración',
	'flow.skip_failures': 'Omitir errores',
	'flow.run_parallel': 'Ejecutar en paralelo',

	// App editor
	'app.public_url': 'URL pública',
	'app.app_inputs': 'Entradas de app',
	'app.schedule_reports': 'Programar informes',
	'app.troubleshoot': 'Panel de diagnóstico',
	'app.lazy_mode': 'Modo lazy',
	'app.hub_export': 'Exportar a Hub',

	// Status labels
	'status.default': 'Por defecto',
	'status.primary': 'Primario',
	'status.modified': 'Modificado',
	'status.draft': 'Draft',

	// Tabs - common
	'tab.workspace': 'Espacio',
	'tab.contextual': 'Contextual',
	'tab.hub': 'Hub',
	'tab.scripts': 'Scripts',
	'tab.flows': 'Flujos',
	'tab.apps': 'Apps',

	// Row actions (scripts/flows/apps)
	'row.view_json': 'Ver JSON/YAML',
	'row.move_rename': 'Mover/Renombrar',
	'row.duplicate_fork': 'Duplicar/Fork',
	'row.go_to_public_page': 'Ir a página pública',
	'row.deployments': 'Despliegues',

	// Confirmation dialogs
	'confirm.delete_forever': 'Eliminar para siempre',
	'confirm.empty_trashbin': 'Vaciar papelera',
	'confirm.discard_changes': 'Descartar cambios',
	'confirm.override': 'Sobrescribir',

	// Groups/Folders
	'groups.manage_group': 'Gestionar grupo',
	'groups.new_group': 'Nuevo grupo',
	'folders.manage_folder': 'Gestionar carpeta',
	'folders.new_folder': 'Nueva carpeta',

	// Triggers
	'triggers.suspend_job': 'Suspender ejecución',
	'triggers.more_triggers': 'Más disparadores',
	'triggers.scheduled_poll': 'Sondeo programado',

	// Misc
	'misc.all_workspaces': 'Todos los espacios',
	'misc.instance_settings': 'Ajustes de instancia',
	'misc.search': 'Buscar',
	'misc.filter': 'Filtrar',
	'misc.tree_view': 'Vista de árbol',
	'misc.loading': 'Cargando...'
}
