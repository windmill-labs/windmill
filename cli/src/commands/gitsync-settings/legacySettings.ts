import process from "node:process";

import { colors } from "@cliffy/ansi/colors";
import { Confirm } from "@cliffy/prompt/confirm";
import * as wmill from "../../../gen/services.gen.ts";
import { GitSyncRepository } from "./types.ts";

// Shared migration function for legacy repositories
export async function handleLegacyRepositoryMigration(
  selectedRepo: any,
  gitSyncSettings: any,
  workspace: any,
  opts: { yes?: boolean },
  operationName: string = "operation"
): Promise<any> {
  if (selectedRepo.settings) {
    return selectedRepo; // Already migrated
  }

  // This repository is in legacy format - handle migration
  if (!gitSyncSettings.include_path || !gitSyncSettings.include_type) {
    throw new Error(
      `Repository "${selectedRepo.git_repo_resource_path}" has legacy format but workspace-level include_path or include_type is missing. This indicates corrupted git-sync settings.`
    );
  }

  const workspaceIncludePath = gitSyncSettings.include_path;
  const workspaceIncludeType = gitSyncSettings.include_type;

  if (!!process.stdout.isTTY && !opts.yes) {
    // Interactive mode - show migration prompt
    console.log(colors.yellow('\n⚠️  Legacy git-sync settings detected!'));
    console.log(`\nRepository "${selectedRepo.git_repo_resource_path}" has legacy settings format.`);
    console.log('The new format allows per-repository filter configuration.');
    if (operationName === "push") {
      console.log('This repository must be migrated before pushing settings.\n');
    } else {
      console.log('\n');
    }

    console.log(colors.bold('Current workspace-level settings:'));
    console.log(`  Include paths: ${workspaceIncludePath.join(', ')}`);
    console.log(`  Include types: ${workspaceIncludeType.join(', ')}\n`);

    // Show what the migration will do
    let finalIncludeType = [...workspaceIncludeType];
    if (selectedRepo.exclude_types_override && selectedRepo.exclude_types_override.length > 0) {
      const originalCount = finalIncludeType.length;
      finalIncludeType = finalIncludeType.filter(
        type => !selectedRepo.exclude_types_override.includes(type)
      );
      const excludedCount = originalCount - finalIncludeType.length;
      console.log(colors.yellow(`Repository excludes ${excludedCount} types: ${selectedRepo.exclude_types_override.join(', ')}`));
    }

    console.log(colors.bold('\nAfter migration, repository will have:'));
    console.log(`  Include paths: ${workspaceIncludePath.join(', ')}`);
    console.log(`  Include types: ${finalIncludeType.join(', ')}\n`);

    const confirm = await Confirm.prompt({
      message: operationName === "push"
        ? 'Do you want to migrate this repository before pushing?'
        : 'Do you want to migrate this repository?',
      default: true
    });

    if (!confirm) {
      const message = operationName === "push"
        ? '\n⚠️  Migration skipped. Cannot push to legacy repository.'
        : '\n⚠️  Migration skipped. You can migrate later via the UI.';
      console.log(colors.yellow(message));
      if (operationName === "push") {
        return null; // Signal to exit push operation
      }
      throw new Error('Migration cancelled by user');
    }

    // Perform the migration
    let migratedIncludeType = [...workspaceIncludeType];
    if (selectedRepo.exclude_types_override && selectedRepo.exclude_types_override.length > 0) {
      migratedIncludeType = migratedIncludeType.filter(
        type => !selectedRepo.exclude_types_override.includes(type)
      );
    }

    const migratedRepo = {
      ...selectedRepo,
      settings: {
        include_path: [...workspaceIncludePath],
        include_type: migratedIncludeType,
        exclude_path: [],
        extra_include_path: []
      }
    };

    // Remove the old field
    delete migratedRepo.exclude_types_override;

    // Update the backend with migrated repository
    const updatedRepositories = gitSyncSettings.repositories.map((repo: any) => {
      if (repo.git_repo_resource_path === selectedRepo.git_repo_resource_path) {
        return migratedRepo;
      }
      return repo;
    });

    await wmill.editWorkspaceGitSyncConfig({
      workspace: workspace.workspaceId,
      requestBody: {
        git_sync_settings: {
          repositories: updatedRepositories,
          // Keep workspace-level settings if other repos are still legacy
          ...(gitSyncSettings.repositories.some((r: any) => r.git_repo_resource_path !== selectedRepo.git_repo_resource_path && !r.settings) && {
            include_path: workspaceIncludePath,
            include_type: workspaceIncludeType
          })
        }
      }
    });

    console.log(colors.green('\n✓ Repository migration completed successfully!'));
    if (operationName === "push") {
      console.log('Now proceeding with push operation...\n');
    }
    return migratedRepo;

  } else {
    // Non-interactive mode - show error
    console.error(colors.red('\n❌ Legacy git-sync settings detected!'));
    console.error(`\nRepository "${selectedRepo.git_repo_resource_path}" has legacy settings format.`);
    if (operationName === "push") {
      console.error('This repository must be migrated before pushing settings.');
    }
    console.error('Please choose one of the following options:\n');
    console.error('1. Go to the Windmill UI > Workspace Settings > Git Sync');
    console.error('   Review and save this repository to migrate to the new format.\n');
    console.error('2. Run this command in interactive mode (with TTY) to migrate.');
    console.error(`   Example: wmill gitsync-settings ${operationName}\n`);
    if (operationName === "push") {
      console.error('3. Pull settings first to migrate: wmill gitsync-settings pull\n');
    } else {
      console.error('3. Push local settings to override backend settings:');
      console.error('   wmill gitsync-settings push\n');
    }
    process.exit(1);
  }
}