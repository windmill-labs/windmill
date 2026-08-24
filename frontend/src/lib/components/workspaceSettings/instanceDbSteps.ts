import type { CustomInstanceDb } from '$lib/gen'
import { runningFrom, type SetupStep } from '../wizards/SetupChecklist.svelte'

/**
 * The same checks as [`instanceDbSteps`], in the vocabulary the wizard's checklist speaks.
 * Nothing is reported until the call returns, so an unreported step is either the failure
 * (when the call errored) or simply not reached yet.
 */
export function instanceSetupSteps(
	dbname: string,
	status: CustomInstanceDb | undefined,
	running: boolean
): SetupStep[] {
	let firstUnreported = true
	const steps = instanceDbSteps(dbname, status).map((step): SetupStep => {
		if (step.status === 'OK') return { ...step, status: 'done' }
		if (step.status === 'FAIL') return { ...step, status: 'failed' }
		if (step.status === 'SKIP') return { ...step, status: 'skipped' }
		const failed = firstUnreported && !!status?.error
		firstUnreported = false
		return { ...step, status: failed ? 'failed' : 'pending' }
	})
	return runningFrom(steps, running)
}

/**
 * The checks `setup_custom_instance_db` reports, in the order it runs them. Shared so the
 * setup modal and the data table wizard describe the same failure the same way.
 */
export function instanceDbSteps(dbname: string, status: CustomInstanceDb | undefined) {
	return [
		{
			title: 'Super admin required',
			status: status?.logs.super_admin,
			description:
				'You need to be a super admin to create a new database in the Windmill PostgreSQL instance'
		},
		{
			title: 'Retrieve and parse database credentials',
			status: status?.logs.database_credentials,
			description:
				'Windmill uses the DATABASE_URL or DATABASE_URL_FILE environment variable to connect to the PostgreSQL instance. Make sure it is correctly set'
		},
		{
			title: 'Database name is valid',
			status: status?.logs.valid_dbname,
			description:
				'The database name must be alphanumeric (underscores and hyphens allowed) and cannot be named the same as the Windmill database (usually "windmill")'
		},
		{
			title:
				'Create database' +
				(status?.logs.created_database === 'SKIP' ? ' (already exists, skipped)' : ''),
			status: status?.logs.created_database,
			description: `In the Windmill PostgreSQL instance, run: CREATE DATABASE "${dbname}".`
		},
		{
			title: `Connect to the ${dbname} database`,
			status: status?.logs.db_connect,
			description:
				"Connect to the newly created database with the default admin user (the one in DATABASE_URL, usually 'postgres') to run the next commands"
		},
		{
			title: 'Grant permissions to custom_instance_user',
			status: status?.logs.grant_permissions,
			description:
				'Gives custom_instance_user the required permissions to use the database. custom_instance_user is already created during a migration and has an auto-generated password stored in global_settings.custom_instance_pg_databases.user_pwd. These are the commands : \n\n' +
				`GRANT CONNECT ON DATABASE "${dbname}" TO custom_instance_user;\n` +
				'GRANT USAGE ON SCHEMA public TO custom_instance_user;\n' +
				'GRANT CREATE ON SCHEMA public TO custom_instance_user;\n' +
				`GRANT CREATE ON DATABASE "${dbname}" TO custom_instance_user;\n` +
				'ALTER DEFAULT PRIVILEGES IN SCHEMA public \n' +
				'  	GRANT SELECT, INSERT, UPDATE, DELETE ON TABLES\n    TO custom_instance_user;\n' +
				'ALTER ROLE custom_instance_user CREATEROLE;'
		},
		{
			title: 'Grant replication to custom_instance_replication_user',
			status: status?.logs.replication_user,
			description:
				'Postgres triggers on custom-instance datatables connect as custom_instance_replication_user, whose password is stored in global_settings.custom_instance_replication_pwd. The role is cluster-wide, so it is created on the Windmill PostgreSQL instance rather than on this database : \n\n' +
				'ALTER ROLE custom_instance_replication_user REPLICATION;\n' +
				'GRANT custom_instance_user TO custom_instance_replication_user;\n\n' +
				'Setting REPLICATION requires a superuser on PostgreSQL 15 and older. Managed instances never grant one, so on AWS RDS Windmill falls back to GRANT rds_replication TO custom_instance_replication_user. The database stays usable for datatables if this step fails, but postgres triggers on them do not.' +
				(status?.logs.replication_user_error
					? `\n\nError: ${status.logs.replication_user_error}`
					: '')
		}
	]
}
