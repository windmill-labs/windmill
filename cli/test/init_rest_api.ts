// Test implementation of REST API workspace initialization
export async function initializeWorkspaceViaAPI(baseUrl: string, workspace: string, token: string): Promise<void> {
  console.log('🔧 Initializing test workspace via REST API...');
  
  // Step 1: Create workspace via REST API
  try {
    const createWorkspaceResponse = await fetch(`${baseUrl}/api/workspaces/create`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({
        id: workspace,
        name: 'Test Workspace',
        username: 'admin'
      })
    });

    if (!createWorkspaceResponse.ok) {
      // Workspace might already exist, which is fine
      if (createWorkspaceResponse.status !== 409) {
        const errorText = await createWorkspaceResponse.text();
        console.warn(`Failed to create workspace: ${createWorkspaceResponse.status} - ${errorText}`);
      }
    } else {
      await createWorkspaceResponse.text(); // Consume response
      console.log('✅ Test workspace created');
    }
  } catch (error) {
    console.warn(`Workspace creation failed: ${error}`);
  }

  // Step 2: Set up git sync configuration via REST API  
  try {
    const gitSyncConfig = {
      git_sync_settings: {
        repositories: [{
          script_path: "f/**",
          git_repo_resource_path: "u/test/test_repo", 
          use_individual_branch: false,
          group_by_folder: false,
          settings: {
            include_path: ["f/**"],
            include_type: ["script", "flow", "app", "folder", "resource"],
            exclude_path: ["*.test.ts", "*.spec.ts"],
            extra_include_path: []
          }
        }]
      }
    };

    const gitSyncResponse = await fetch(`${baseUrl}/api/w/${workspace}/workspaces/edit_git_sync_config`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${token}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(gitSyncConfig)
    });

    if (!gitSyncResponse.ok) {
      const errorText = await gitSyncResponse.text();
      console.warn(`Failed to set git sync config: ${gitSyncResponse.status} - ${errorText}`);
    } else {
      await gitSyncResponse.text(); // Consume response
      console.log('✅ Git sync configuration set (EE features available)');
    }
  } catch (error) {
    console.warn(`Git sync config failed: ${error}`);
  }
  
  console.log('✅ Test workspace initialized via REST API');
}