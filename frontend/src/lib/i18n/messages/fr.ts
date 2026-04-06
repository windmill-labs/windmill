import type { MessageKey } from './en'

export const fr: Record<MessageKey, string> = {
	// Sidebar - main nav
	'sidebar.home': 'Accueil',
	'sidebar.runs': 'Exécutions',
	'sidebar.variables': 'Variables',
	'sidebar.resources': 'Ressources',
	'sidebar.assets': 'Assets',
	'sidebar.tutorials': 'Tutoriels',

	// Sidebar - triggers section
	'sidebar.triggers': 'Déclencheurs',
	'sidebar.schedules': 'Plannings',
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
	'sidebar.settings': 'Paramètres',
	'sidebar.account': 'Compte',
	'sidebar.workspace': 'Espace',
	'sidebar.instance': 'Instance',
	'sidebar.leave_workspace': 'Quitter',
	'sidebar.delete_fork': 'Supprimer le fork',
	'sidebar.workers': 'Workers',
	'sidebar.folders_groups': 'Dossiers & Groupes',
	'sidebar.folders': 'Dossiers',
	'sidebar.groups': 'Groupes',
	'sidebar.logs': 'Journaux',
	'sidebar.audit_logs': 'Audit',
	'sidebar.service_logs': 'Services',
	'sidebar.critical_alerts': 'Alertes critiques',

	// Sidebar - help
	'sidebar.help': 'Aide',
	'sidebar.docs': 'Docs',
	'sidebar.feedbacks': 'Retours',
	'sidebar.issues': 'Issues',
	'sidebar.changelog': 'Changelog',

	// User menu
	'user.account_settings': 'Mon compte',
	'user.switch_theme': 'Changer de thème',
	'user.sign_out': 'Déconnexion',
	'user.upgrade': 'Passer en premium',
	'user.premium_plan': 'Plan premium',
	'user.admin_of_workspace': 'Admin de cet espace',
	'user.operator_in_workspace': 'Opérateur',
	'user.language': 'Langue',

	// Operator menu
	'operator.custom_http_routes': 'Routes HTTP',
	'operator.websocket_triggers': 'Déclencheurs WS',
	'operator.postgres_triggers': 'Déclencheurs PG',
	'operator.kafka_triggers': 'Déclencheurs Kafka',
	'operator.nats_triggers': 'Déclencheurs NATS',
	'operator.sqs_triggers': 'Déclencheurs SQS',
	'operator.gcp_triggers': 'Déclencheurs GCP',
	'operator.mqtt_triggers': 'Déclencheurs MQTT',
	'operator.email_triggers': 'Déclencheurs email',

	// Common actions
	'action.save': 'Enregistrer',
	'action.cancel': 'Annuler',
	'action.delete': 'Supprimer',
	'action.edit': 'Modifier',
	'action.create': 'Créer',
	'action.run': 'Lancer',
	'action.deploy': 'Déployer',
	'action.share': 'Partager',
	'action.close': 'Fermer',
	'action.confirm': 'Confirmer',
	'action.refresh': 'Actualiser',
	'action.duplicate': 'Dupliquer',
	'action.fork': 'Fork',
	'action.rename': 'Renommer',
	'action.move': 'Déplacer',
	'action.copy_path': 'Copier le chemin',
	'action.export': 'Exporter',
	'action.undo': 'Annuler',
	'action.redo': 'Rétablir',
	'action.test': 'Tester',
	'action.view': 'Voir',
	'action.next': 'Suivant',
	'action.previous': 'Précédent',
	'action.remove': 'Retirer',
	'action.enable': 'Activer',
	'action.disable': 'Désactiver',
	'action.see_permissions': 'Voir les permissions',
	'action.deploy_to_prod': 'Déployer en prod/staging',
	'action.refresh_token': 'Rafraîchir le token',
	'action.view_runs': 'Voir les exécutions',
	'action.view_code': 'Voir le code',
	'action.audit_logs': "Journaux d'audit",
	'action.schedule': 'Planifier',
	'action.versions': 'Versions',
	'action.delete_draft': 'Supprimer le Draft',
	'action.publish_to_hub': 'Publier sur le Hub',
	'action.run_now': 'Lancer maintenant',
	'action.manage': 'Gérer',

	// Page headers
	'page.home': 'Accueil',
	'page.variables': 'Variables',
	'page.resources': 'Ressources',
	'page.schedules': 'Plannings',
	'page.workers': 'Workers',
	'page.folders': 'Dossiers',
	'page.groups': 'Groupes',
	'page.runs': 'Exécutions',

	// Home page
	'home.create_a': 'Créer un',
	'home.workspace': 'Espace',
	'home.hub': 'Hub',
	'home.scripts': 'Scripts',
	'home.flows': 'Flows',
	'home.apps': 'Apps',
	'home.hub_flow': 'Flow du Hub',
	'home.hub_app': 'App du Hub',
	'home.view_on_hub': 'Voir sur le Hub',
	'home.content': 'Contenu',

	// Variables page
	'variables.new_variable': 'Nouvelle variable',
	'variables.new_contextual': 'Nouvelle var. contextuelle',
	'variables.contextual': 'Contextuelle',
	'variables.custom_contextual': 'Variables contextuelles',
	'variables.contextual_vars': 'Variables contextuelles',
	'variables.no_variables': 'Aucune variable trouvée',
	'variables.path': 'Chemin',
	'variables.value': 'Valeur',
	'variables.description': 'Description',

	// Resources page
	'resources.new_resource': 'Nouvelle ressource',

	// Schedules page
	'schedules.duplicate': 'Dupliquer le planning',
	'schedules.view_script': 'Voir le script',
	'schedules.view_flow': 'Voir le flow',

	// Runs page
	'runs.cancel_jobs': 'Annuler les jobs',
	'runs.rerun_jobs': 'Relancer les jobs',
	'runs.cancel_all': 'Annuler tous les jobs filtrés',
	'runs.rerun_all': 'Relancer tous les jobs filtrés',
	'runs.force_cancel': "Forcer l'annulation",

	// Editor - common
	'editor.save': 'Enregistrer',
	'editor.deploy': 'Déployer',
	'editor.draft': 'Draft',
	'editor.test': 'Tester',
	'editor.test_and_record': 'Tester & enregistrer',
	'editor.history': 'Historique',
	'editor.versions_history': 'Historique des versions',
	'editor.deployment_history': 'Historique des déploiements',
	'editor.edit_in_yaml': 'Éditer en YAML',
	'editor.flow_settings': 'Paramètres du flow',
	'editor.export': 'Exporter',
	'editor.diff': 'Diff',
	'editor.editor': 'Éditeur',
	'editor.preview': 'Aperçu',
	'editor.main': 'Principal',
	'editor.preprocessor': 'Préprocesseur',
	'editor.diagram': 'Diagramme',
	'editor.save_to_workspace': "Enregistrer dans l'espace",
	'editor.exit_see_details': 'Quitter & voir détails',
	'editor.edit_in_fork': 'Éditer dans le fork',
	'editor.save_anyway': 'Enregistrer quand même',
	'editor.invite_others': "Inviter d'autres",

	// Flow editor
	'flow.test_flow': 'Tester le flow',
	'flow.test_flow_record': 'Tester & enregistrer',
	'flow.build_a_flow': 'Créer un flow',
	'flow.fix_broken_flow': 'Réparer un flow',
	'flow.reset_tutorials': 'Réinitialiser les tutoriels',
	'flow.skip_tutorials': 'Passer les tutoriels',
	'flow.create_group': 'Créer un groupe',
	'flow.test_iteration': 'Tester une itération',
	'flow.skip_failures': 'Ignorer les erreurs',
	'flow.run_parallel': 'Lancer en parallèle',

	// App editor
	'app.public_url': 'URL publique',
	'app.app_inputs': "Entrées de l'app",
	'app.schedule_reports': 'Planifier les rapports',
	'app.troubleshoot': 'Dépannage',
	'app.lazy_mode': 'Mode lazy',
	'app.hub_export': 'Export Hub',

	// Status labels
	'status.default': 'Défaut',
	'status.primary': 'Principal',
	'status.modified': 'Modifié',
	'status.draft': 'Draft',

	// Tabs - common
	'tab.workspace': 'Espace',
	'tab.contextual': 'Contextuel',
	'tab.hub': 'Hub',
	'tab.scripts': 'Scripts',
	'tab.flows': 'Flows',
	'tab.apps': 'Apps',

	// Row actions (scripts/flows/apps)
	'row.view_json': 'Voir JSON/YAML',
	'row.move_rename': 'Déplacer/Renommer',
	'row.duplicate_fork': 'Dupliquer/Fork',
	'row.go_to_public_page': 'Aller à la page publique',
	'row.deployments': 'Déploiements',

	// Confirmation dialogs
	'confirm.delete_forever': 'Supprimer définitivement',
	'confirm.empty_trashbin': 'Vider la corbeille',
	'confirm.discard_changes': 'Abandonner les modifications',
	'confirm.override': 'Écraser',

	// Groups/Folders
	'groups.manage_group': 'Gérer le groupe',
	'groups.new_group': 'Nouveau groupe',
	'folders.manage_folder': 'Gérer le dossier',
	'folders.new_folder': 'Nouveau dossier',

	// Triggers
	'triggers.suspend_job': "Suspendre l'exécution",
	'triggers.more_triggers': 'Plus de déclencheurs',
	'triggers.scheduled_poll': 'Sondage planifié',

	// Misc
	'misc.all_workspaces': 'Tous les espaces',
	'misc.instance_settings': "Paramètres d'instance",
	'misc.search': 'Rechercher',
	'misc.filter': 'Filtrer',
	'misc.tree_view': 'Vue arborescente',
	'misc.loading': 'Chargement...'
}
