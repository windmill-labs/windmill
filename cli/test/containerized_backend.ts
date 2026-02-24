/**
 * Containerized Backend Test Utilities
 * Manages real Windmill EE backend containers for CLI testing
 */

import { mkdtemp, rm } from "node:fs/promises";
import { tmpdir } from "node:os";
import { join } from "node:path";

async function runCommand(cmd: string, args: string[], opts?: { cwd?: string, env?: Record<string, string> }): Promise<{ code: number, stdout: string, stderr: string }> {
  const proc = Bun.spawn([cmd, ...args], {
    stdout: 'pipe',
    stderr: 'pipe',
    cwd: opts?.cwd,
    env: { ...process.env, ...opts?.env },
  });
  const [code, stdout, stderr] = await Promise.all([
    proc.exited,
    new Response(proc.stdout).text(),
    new Response(proc.stderr).text(),
  ]);
  return { code, stdout, stderr };
}

export interface ContainerConfig {
  composeFile?: string;
  baseUrl?: string;
  workspace?: string;
  token?: string;
  username?: string;
  password?: string;
  timeout?: number;
  testConfigDir?: string;
}

export class ContainerizedBackend {
  private config: Required<ContainerConfig>;
  private isRunning = false;

  constructor(config: Partial<ContainerConfig> = {}) {
    this.config = {
      composeFile: config.composeFile || new URL('./docker-compose.test.yml', import.meta.url).pathname,
      baseUrl: config.baseUrl || 'http://localhost:8001',
      workspace: config.workspace || 'test', // Use test workspace
      token: config.token || '',
      username: config.username || 'admin@windmill.dev',
      password: config.password || 'changeme',
      timeout: config.timeout || 120000, // 2 minutes
      testConfigDir: config.testConfigDir || '' // Will be created if empty
    };
  }

  /**
   * Get the base URL of the backend
   */
  get baseUrl(): string {
    return this.config.baseUrl;
  }

  /**
   * Get the workspace ID
   */
  get workspace(): string {
    return this.config.workspace;
  }

  /**
   * Get the authentication token
   */
  get token(): string {
    return this.config.token;
  }

  /**
   * Get the test config directory
   */
  get testConfigDir(): string {
    return this.config.testConfigDir;
  }

  /**
   * Start the containerized backend (database + server + worker)
   */
  async start(): Promise<void> {
    if (this.isRunning) {
      return;
    }

    console.log('🚀 Starting containerized Windmill backend...');
    
    // Create isolated test config directory if not provided
    if (!this.config.testConfigDir) {
      this.config.testConfigDir = await mkdtemp(join(tmpdir(), 'wmill_test_config_'));
      console.log(`📁 Created test config directory: ${this.config.testConfigDir}`);
    }
    
    // Start containers with EE license key
    const startResult = await runCommand('docker', ['compose', '-f', this.config.composeFile, 'up', '-d'], {
      env: {
        ...(process.env.EE_LICENSE_KEY && { EE_LICENSE_KEY: process.env.EE_LICENSE_KEY })
      }
    });

    if (startResult.code !== 0) {
      throw new Error(`Failed to start containers: ${startResult.stderr}`);
    }

    // Wait for services to be healthy
    await this.waitForHealthy();
    
    // Verify API is responsive
    await this.waitForAPI();
    
    // Authenticate and get a proper token
    await this.authenticate();
    
    this.isRunning = true;
    console.log('✅ Containerized backend is ready!');
    console.log(`   Server: ${this.config.baseUrl}`);
    console.log(`   Workspace: ${this.config.workspace}`);
  }

  /**
   * Stop the containerized backend
   */
  async stop(): Promise<void> {
    if (!this.isRunning) {
      return;
    }

    console.log('🛑 Stopping containerized backend...');
    
    await runCommand('docker', ['compose', '-f', this.config.composeFile, 'down', '-v'], {
      env: {
        ...(process.env.EE_LICENSE_KEY && { EE_LICENSE_KEY: process.env.EE_LICENSE_KEY })
      }
    });
    
    // Clean up test config directory if we created it
    if (this.config.testConfigDir && this.config.testConfigDir.includes('wmill_test_config_')) {
      try {
        await rm(this.config.testConfigDir, { recursive: true });
        console.log(`🗑️  Cleaned up test config directory: ${this.config.testConfigDir}`);
      } catch (error) {
        console.warn(`⚠️  Failed to clean up test config directory: ${error}`);
      }
    }
    
    this.isRunning = false;
    console.log('✅ Backend stopped');
  }

  /**
   * Reset backend to clean state (clear workspace content, keep settings)
   */
  async reset(): Promise<void> {
    console.log('🔄 Fast API-based reset...');
    
    try {
      // Delete all workspace content via Windmill API
      await Promise.all([
        this.deleteAllScripts(),
        this.deleteAllFlows(),
        this.deleteAllApps(),
        this.deleteAllResources(),
        this.deleteAllVariables(),
        this.deleteAllFolders()
      ]);
      
      console.log('✅ Workspace reset complete');
    } catch (error) {
      console.error('Reset failed:', error instanceof Error ? error.message : String(error));
      // Don't throw - tests can continue with existing state
    }
  }

  /**
   * Delete all scripts in workspace
   */
  private async deleteAllScripts(): Promise<void> {
    try {
      const response = await fetch(`${this.config.baseUrl}/api/w/${this.config.workspace}/scripts/list`, {
        headers: { 'Authorization': `Bearer ${this.config.token}` }
      });
      
      if (!response.ok) {
        console.warn(`Failed to list scripts for deletion: ${response.status} ${response.statusText}`);
        return;
      }
      
      const scripts = await response.json();
      console.log(`Deleting ${scripts.length} scripts...`);
      
      const deletionResults = [];
      
      for (const script of scripts) {
        try {
          const deleteResponse = await fetch(`${this.config.baseUrl}/api/w/${this.config.workspace}/scripts/delete/p/${encodeURIComponent(script.path)}`, {
            method: 'POST',
            headers: { 'Authorization': `Bearer ${this.config.token}` }
          });
          
          const responseText = await deleteResponse.text();
          
          if (!deleteResponse.ok) {
            console.warn(`Failed to delete script ${script.path}: ${deleteResponse.status} ${deleteResponse.statusText} - ${responseText}`);
            deletionResults.push({ path: script.path, success: false, error: `${deleteResponse.status}: ${responseText}` });
          } else {
            console.log(`Successfully deleted script: ${script.path}`);
            deletionResults.push({ path: script.path, success: true });
          }
        } catch (error) {
          console.warn(`Error deleting script ${script.path}:`, error instanceof Error ? error.message : String(error));
          deletionResults.push({ path: script.path, success: false, error: error instanceof Error ? error.message : String(error) });
        }
      }
      
      const successCount = deletionResults.filter(r => r.success).length;
      const failureCount = deletionResults.filter(r => !r.success).length;
      
      console.log(`Script deletion summary: ${successCount} successful, ${failureCount} failed`);
      
      if (failureCount > 0) {
        console.warn('Failed script deletions:', deletionResults.filter(r => !r.success));
      }
      
      // Verify deletion by checking if scripts still exist
      await this.verifyScriptDeletion();
      
    } catch (error) {
      console.error('Failed to delete scripts:', error instanceof Error ? error.message : String(error));
      throw error; // Don't silently ignore major failures
    }
  }

  /**
   * Verify that scripts were actually deleted
   */
  private async verifyScriptDeletion(): Promise<void> {
    try {
      const response = await fetch(`${this.config.baseUrl}/api/w/${this.config.workspace}/scripts/list`, {
        headers: { 'Authorization': `Bearer ${this.config.token}` }
      });
      
      if (response.ok) {
        const remainingScripts = await response.json();
        if (remainingScripts.length > 0) {
          console.warn(`Warning: ${remainingScripts.length} scripts still exist after deletion attempt:`, 
            remainingScripts.map((s: any) => s.path));
          
          // Attempt to delete remaining scripts with retry
          for (const script of remainingScripts) {
            await this.retryScriptDeletion(script.path);
          }
        }
      }
    } catch (error) {
      console.warn('Could not verify script deletion:', error instanceof Error ? error.message : String(error));
    }
  }

  /**
   * Retry script deletion with exponential backoff
   */
  private async retryScriptDeletion(scriptPath: string, maxRetries: number = 3): Promise<void> {
    for (let attempt = 1; attempt <= maxRetries; attempt++) {
      try {
        const deleteResponse = await fetch(`${this.config.baseUrl}/api/w/${this.config.workspace}/scripts/delete/${encodeURIComponent(scriptPath)}`, {
          method: 'DELETE',
          headers: { 'Authorization': `Bearer ${this.config.token}` }
        });
        
        const responseText = await deleteResponse.text();
        
        if (deleteResponse.ok) {
          console.log(`Successfully deleted script on retry ${attempt}: ${scriptPath}`);
          return;
        } else if (deleteResponse.status === 404) {
          console.log(`Script already deleted: ${scriptPath}`);
          return;
        } else {
          console.warn(`Retry ${attempt} failed for script ${scriptPath}: ${deleteResponse.status} - ${responseText}`);
        }
      } catch (error) {
        console.warn(`Retry ${attempt} error for script ${scriptPath}:`, error instanceof Error ? error.message : String(error));
      }
      
      if (attempt < maxRetries) {
        const delay = Math.pow(2, attempt) * 1000; // Exponential backoff
        await new Promise(resolve => setTimeout(resolve, delay));
      }
    }
    
    console.error(`Failed to delete script after ${maxRetries} retries: ${scriptPath}`);
  }

  /**
   * Delete all flows in workspace
   */
  private async deleteAllFlows(): Promise<void> {
    try {
      const response = await fetch(`${this.config.baseUrl}/api/w/${this.config.workspace}/flows/list`, {
        headers: { 'Authorization': `Bearer ${this.config.token}` }
      });
      
      if (!response.ok) return;
      
      const flows = await response.json();
      
      for (const flow of flows) {
        try {
          const deleteResponse = await fetch(`${this.config.baseUrl}/api/w/${this.config.workspace}/flows/delete/${flow.path}`, {
            method: 'DELETE',
            headers: { 'Authorization': `Bearer ${this.config.token}` }
          });
          await deleteResponse.text(); // Consume response
        } catch {
          // Ignore individual deletion failures
        }
      }
    } catch {
      // Ignore listing failures
    }
  }

  /**
   * Delete all apps in workspace
   */
  private async deleteAllApps(): Promise<void> {
    try {
      const response = await fetch(`${this.config.baseUrl}/api/w/${this.config.workspace}/apps/list`, {
        headers: { 'Authorization': `Bearer ${this.config.token}` }
      });
      
      if (!response.ok) return;
      
      const apps = await response.json();
      
      for (const app of apps) {
        try {
          const deleteResponse = await fetch(`${this.config.baseUrl}/api/w/${this.config.workspace}/apps/delete/${app.path}`, {
            method: 'DELETE',
            headers: { 'Authorization': `Bearer ${this.config.token}` }
          });
          await deleteResponse.text(); // Consume response
        } catch {
          // Ignore individual deletion failures
        }
      }
    } catch {
      // Ignore listing failures
    }
  }

  /**
   * Delete all resources in workspace (except git repo)
   */
  private async deleteAllResources(): Promise<void> {
    try {
      const response = await fetch(`${this.config.baseUrl}/api/w/${this.config.workspace}/resources/list`, {
        headers: { 'Authorization': `Bearer ${this.config.token}` }
      });
      
      if (!response.ok) return;
      
      const resources = await response.json();
      
      for (const resource of resources) {
        // Keep git repo resource
        if (resource.path === 'u/test/test_repo') continue;
        
        try {
          const deleteResponse = await fetch(`${this.config.baseUrl}/api/w/${this.config.workspace}/resources/delete/${resource.path}`, {
            method: 'DELETE',
            headers: { 'Authorization': `Bearer ${this.config.token}` }
          });
          await deleteResponse.text(); // Consume response
        } catch {
          // Ignore individual deletion failures
        }
      }
    } catch {
      // Ignore listing failures
    }
  }

  /**
   * Delete all variables in workspace (except git repo)
   */
  private async deleteAllVariables(): Promise<void> {
    try {
      const response = await fetch(`${this.config.baseUrl}/api/w/${this.config.workspace}/variables/list`, {
        headers: { 'Authorization': `Bearer ${this.config.token}` }
      });
      
      if (!response.ok) return;
      
      const variables = await response.json();
      
      for (const variable of variables) {
        // Keep git repo resource
        if (variable.path === 'u/test/test_repo') continue;
        
        try {
          const deleteResponse = await fetch(`${this.config.baseUrl}/api/w/${this.config.workspace}/variables/delete/${variable.path}`, {
            method: 'DELETE',
            headers: { 'Authorization': `Bearer ${this.config.token}` }
          });
          await deleteResponse.text(); // Consume response
        } catch {
          // Ignore individual deletion failures
        }
      }
    } catch {
      // Ignore listing failures
    }
  }

  /**
   * Delete all folders in workspace
   */
  private async deleteAllFolders(): Promise<void> {
    try {
      const response = await fetch(`${this.config.baseUrl}/api/w/${this.config.workspace}/folders/list`, {
        headers: { 'Authorization': `Bearer ${this.config.token}` }
      });
      
      if (!response.ok) return;
      
      const folders = await response.json();
      
      for (const folder of folders) {
        try {
          const deleteResponse = await fetch(`${this.config.baseUrl}/api/w/${this.config.workspace}/folders/delete/${folder.name}`, {
            method: 'DELETE',
            headers: { 'Authorization': `Bearer ${this.config.token}` }
          });
          await deleteResponse.text(); // Consume response
        } catch {
          // Ignore individual deletion failures
        }
      }
    } catch {
      // Ignore listing failures
    }
  }

  /**
   * Setup test environment for CLI testing
   */
  async seedTestData(): Promise<void> {
    console.log('🌱 Setting up test environment...');
    
    // Try to set up git sync settings if EE is available
    try {
      const gitSyncConfig = {
        git_sync_settings: {
          repositories: [{
            settings: {
              exclude_path: ["*.test.ts", "*.spec.ts"],
              include_path: ["f/**"],
              include_type: ["script", "flow", "app", "folder", "resource"],
              extra_include_path: []
            },
            script_path: "f/**",
            group_by_folder: false,
            use_individual_branch: false,
            git_repo_resource_path: "u/test/test_repo"
          }]
        }
      };

      await this.updateGitSyncConfig(gitSyncConfig);
      console.log('✅ Git sync configuration set (EE features available)');
    } catch (error) {
      console.log('⚠️  Git sync not available (CE edition) - CLI tests will run without git sync');
    }

    // Create minimal test data (skip scripts/flows due to path validation issues)
    console.log('📝 Creating test apps and resources...');
    await this.createTestApp();
    await this.createTestResources();
    await this.createTestVariables();
    await this.createTestGroups();
    console.log('✅ Test data created');
  }

  /**
   * Create a realistic test script
   */
  private async createTestScript(): Promise<void> {
    const scriptContent = `// Test script for CLI sync operations
import { Resource } from "https://deno.land/x/windmill@v1.85.0/mod.ts";

type PostgresqlResource = {
  host: string;
  port: number;
  user: string;
  dbname: string;
  sslmode: string;
  password: string;
  root_certificate_pem: string;
};

export async function main(
  database: Resource<"postgresql">,
  query: string = "SELECT version();",
  limit: number = 100
) {
  // Simple database query example
  const result = await database.query(query);
  return {
    query,
    rows: result.rows.slice(0, limit),
    rowCount: result.rows.length,
    timestamp: new Date().toISOString()
  };
}`;

    const script = {
      path: 'f/test_database_query',
      summary: 'Test Database Query Script',
      description: 'A realistic test script that demonstrates database operations for CLI sync testing',
      content: scriptContent,
      language: 'deno',
      schema: {
        $schema: 'https://json-schema.org/draft/2020-12/schema',
        type: 'object',
        properties: {
          database: {
            type: 'object',
            format: 'resource-postgresql',
            description: 'PostgreSQL database connection'
          },
          query: {
            type: 'string',
            description: 'SQL query to execute',
            default: 'SELECT version();'
          },
          limit: {
            type: 'number',
            description: 'Maximum number of rows to return',
            default: 100,
            minimum: 1,
            maximum: 1000
          }
        },
        required: ['database']
      }
    };

    const response = await fetch(`${this.config.baseUrl}/api/w/${this.config.workspace}/scripts/create`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${this.config.token}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(script)
    });

    if (!response.ok) {
      const errorText = await response.text();
      console.warn(`Failed to create test script: ${response.status} - ${errorText}`);
    } else {
      await response.text(); // Consume response to avoid leak
      console.log('  ✅ Created test script: f/test_database_query');
    }
  }

  /**
   * Create a realistic test flow
   */
  private async createTestFlow(): Promise<void> {
    const flow = {
      path: 'f/test_data_processing',
      summary: 'Test Data Processing Flow',
      description: 'A multi-step flow that demonstrates data processing workflow for CLI sync testing',
      value: {
        modules: [
          {
            id: 'a',
            value: {
              type: 'rawscript',
              content: `export async function main(input_data: string[] = ["test1", "test2", "test3"]) {
  // Step 1: Validate and clean input data
  const cleaned = input_data
    .filter(item => item && item.trim().length > 0)
    .map(item => item.trim().toLowerCase());
  
  console.log(\`Processed \${cleaned.length} items\`);
  return { cleaned_data: cleaned, count: cleaned.length };
}`,
              language: 'deno'
            }
          },
          {
            id: 'b',
            value: {
              type: 'rawscript',
              content: `export async function main(processed_data: { cleaned_data: string[], count: number }) {
  // Step 2: Transform and enrich data
  const enriched = processed_data.cleaned_data.map((item, index) => ({
    id: index + 1,
    value: item,
    processed_at: new Date().toISOString(),
    length: item.length
  }));
  
  return { 
    enriched_data: enriched, 
    total_items: processed_data.count,
    avg_length: enriched.reduce((sum, item) => sum + item.length, 0) / enriched.length
  };
}`,
              language: 'deno'
            }
          },
          {
            id: 'c',
            value: {
              type: 'rawscript',
              content: `export async function main(final_data: any) {
  // Step 3: Generate summary report
  const report = {
    summary: "Data processing completed successfully",
    processed_at: new Date().toISOString(),
    total_records: final_data.total_items,
    average_length: Math.round(final_data.avg_length * 100) / 100,
    data_sample: final_data.enriched_data.slice(0, 3)
  };
  
  console.log("Processing report:", JSON.stringify(report, null, 2));
  return report;
}`,
              language: 'deno'
            }
          }
        ]
      },
      schema: {
        $schema: 'https://json-schema.org/draft/2020-12/schema',
        type: 'object',
        properties: {
          input_data: {
            type: 'array',
            items: { type: 'string' },
            description: 'Array of strings to process',
            default: ['test1', 'test2', 'test3']
          }
        }
      }
    };

    const response = await fetch(`${this.config.baseUrl}/api/w/${this.config.workspace}/flows/create`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${this.config.token}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(flow)
    });

    if (!response.ok) {
      const errorText = await response.text();
      console.warn(`Failed to create test flow: ${response.status} - ${errorText}`);
    } else {
      await response.text(); // Consume response to avoid leak
      console.log('  ✅ Created test flow: f/test_data_processing');
    }
  }

  /**
   * Create a realistic test app
   */
  private async createTestApp(): Promise<void> {
    const app = {
      path: 'f/test_dashboard',
      summary: 'Test Dashboard App',
      policy: {
        execution_mode: 'viewer'
      },
      value: {
        grid: [
          {
            "3": {
              id: 'title',
              data: {
                type: 'displaycomponent',
                configuration: {},
                componentInput: {
                  type: 'static',
                  value: 'Test Dashboard'
                }
              }
            }
          },
          {
            "12": {
              id: 'table',
              data: {
                type: 'tablecomponent',
                configuration: {},
                componentInput: {
                  type: 'runnable',
                  runnable: {
                    type: 'runnableByName',
                    name: 'f/test_database_query'
                  }
                }
              }
            }
          }
        ],
        fullscreen: false,
        unusedInlineScripts: []
      }
    };

    const response = await fetch(`${this.config.baseUrl}/api/w/${this.config.workspace}/apps/create`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${this.config.token}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(app)
    });

    if (!response.ok) {
      const errorText = await response.text();
      console.warn(`Failed to create test app: ${response.status} - ${errorText}`);
    } else {
      await response.text(); // Consume response to avoid leak
      console.log('  ✅ Created test app: f/test_dashboard');
    }
  }

  /**
   * Create test resources
   */
  private async createTestResources(): Promise<void> {
    // Create PostgreSQL test database
    const testDb = {
      path: 'u/admin/test_database',
      description: 'Test PostgreSQL database for CLI sync testing',
      resource_type: 'postgresql',
      value: {
        host: 'localhost',
        port: 5432,
        user: 'testuser',
        dbname: 'testdb',
        sslmode: 'disable',
        password: 'testpassword',
        root_certificate_pem: ''
      }
    };

    const dbResponse = await fetch(`${this.config.baseUrl}/api/w/${this.config.workspace}/resources/create`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${this.config.token}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(testDb)
    });

    if (!dbResponse.ok) {
      const errorText = await dbResponse.text();
      console.warn(`Failed to create test resource: ${dbResponse.status} - ${errorText}`);
    } else {
      await dbResponse.text(); // Consume response to avoid leak
      console.log('  ✅ Created test resource: u/admin/test_database');
    }

    // Create only the primary git repository resource (u/test/test_repo)
    // Multi-repo tests will create additional repositories as needed
    const primaryGitRepo = {
      path: 'u/test/test_repo',
      description: 'Primary test git repository for backend code',
      resource_type: 'git',
      value: {
        url: 'https://github.com/windmill-labs/windmill-test-repo.git',
        branch: 'main',
        token: ''
      }
    };

    const gitResponse = await fetch(`${this.config.baseUrl}/api/w/${this.config.workspace}/resources/create`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${this.config.token}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(primaryGitRepo)
    });

    if (!gitResponse.ok) {
      const errorText = await gitResponse.text();
      console.warn(`Failed to create git resource ${primaryGitRepo.path}: ${gitResponse.status} - ${errorText}`);
    } else {
      await gitResponse.text(); // Consume response to avoid leak
      console.log(`  ✅ Created git resource: ${primaryGitRepo.path}`);
    }
  }

  /**
   * Create additional git repository resource for multi-repo tests
   */
  async createAdditionalGitRepo(repoPath: string, description: string): Promise<void> {
    const gitRepo = {
      path: repoPath,
      description: description,
      resource_type: 'git',
      value: {
        url: 'https://github.com/windmill-labs/windmill-frontend-test.git',
        branch: 'main',
        token: ''
      }
    };

    const gitResponse = await fetch(`${this.config.baseUrl}/api/w/${this.config.workspace}/resources/create`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${this.config.token}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(gitRepo)
    });

    if (!gitResponse.ok) {
      const errorText = await gitResponse.text();
      console.warn(`Failed to create git resource ${gitRepo.path}: ${gitResponse.status} - ${errorText}`);
    } else {
      await gitResponse.text(); // Consume response to avoid leak
      console.log(`  ✅ Created additional git resource: ${gitRepo.path}`);
    }
  }

  /**
   * Create test variables for sync testing
   */
  private async createTestVariables(): Promise<void> {
    // Create a regular variable
    const testVariable = {
      path: 'u/admin/test_config',
      value: 'production',
      description: 'Test configuration variable',
      is_secret: false
    };

    const response = await fetch(`${this.config.baseUrl}/api/w/${this.config.workspace}/variables/create`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${this.config.token}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(testVariable)
    });

    if (!response.ok) {
      const errorText = await response.text();
      console.warn(`Failed to create test variable: ${response.status} - ${errorText}`);
    } else {
      await response.text(); // Consume response to avoid leak
      console.log('  ✅ Created test variable: u/admin/test_config');
    }
  }

  /**
   * Create test groups for sync testing
   */
  private async createTestGroups(): Promise<void> {
    // Create a test group for sync testing
    const testGroup = {
      name: 'test_group',
      summary: 'Test group for CLI sync operations'
    };

    const response = await fetch(`${this.config.baseUrl}/api/w/${this.config.workspace}/groups/create`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${this.config.token}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(testGroup)
    });

    if (!response.ok) {
      const errorText = await response.text();
      console.warn(`Failed to create test group: ${response.status} - ${errorText}`);
    } else {
      await response.text(); // Consume response to avoid leak
      console.log('  ✅ Created test group: test_group');
    }
  }

  /**
   * Get current workspace settings from backend
   */
  async getWorkspaceSettings(): Promise<any> {
    const response = await fetch(`${this.config.baseUrl}/api/w/${this.config.workspace}/workspaces/get_settings`, {
      headers: {
        'Authorization': `Bearer ${this.config.token}`
      }
    });
    
    if (!response.ok) {
      throw new Error(`Failed to get workspace settings: ${response.status} ${response.statusText}`);
    }
    
    return await response.json();
  }

  /**
   * List all scripts in workspace with full content
   */
  async listAllScripts(): Promise<Array<{path: string, content: string, summary: string, description?: string}>> {
    // First get the list of scripts
    const response = await fetch(`${this.config.baseUrl}/api/w/${this.config.workspace}/scripts/list`, {
      headers: { 'Authorization': `Bearer ${this.config.token}` }
    });
    
    if (!response.ok) {
      throw new Error(`Failed to list scripts: ${response.status}`);
    }
    
    const scriptList = await response.json();
    
    // Then fetch full details (including content) for each script
    const scriptsWithContent = [];
    for (const script of scriptList) {
      try {
        const detailResponse = await fetch(`${this.config.baseUrl}/api/w/${this.config.workspace}/scripts/get/p/${script.path}`, {
          headers: { 'Authorization': `Bearer ${this.config.token}` }
        });
        
        if (detailResponse.ok) {
          const fullScript = await detailResponse.json();
          scriptsWithContent.push(fullScript);
        } else {
          // If we can't get details, include the basic info without content
          console.warn(`Could not fetch details for script ${script.path}: ${detailResponse.status}`);
          scriptsWithContent.push(script);
        }
      } catch (error) {
        console.warn(`Error fetching script ${script.path}:`, error);
        scriptsWithContent.push(script);
      }
    }
    
    return scriptsWithContent;
  }

  /**
   * List all apps in workspace  
   */
  async listAllApps(): Promise<Array<{path: string, summary: string, value: any}>> {
    const response = await fetch(`${this.config.baseUrl}/api/w/${this.config.workspace}/apps/list`, {
      headers: { 'Authorization': `Bearer ${this.config.token}` }
    });
    
    if (!response.ok) {
      throw new Error(`Failed to list apps: ${response.status}`);
    }
    
    return await response.json();
  }

  /**
   * List all resources in workspace
   */
  async listAllResources(): Promise<Array<{path: string, description?: string, resource_type: string, value: any}>> {
    const response = await fetch(`${this.config.baseUrl}/api/w/${this.config.workspace}/resources/list`, {
      headers: { 'Authorization': `Bearer ${this.config.token}` }
    });
    
    if (!response.ok) {
      throw new Error(`Failed to list resources: ${response.status}`);
    }
    
    return await response.json();
  }

  /**
   * List all variables in workspace
   */
  async listAllVariables(): Promise<Array<{path: string, description?: string, value: any, is_secret: boolean}>> {
    const response = await fetch(`${this.config.baseUrl}/api/w/${this.config.workspace}/variables/list`, {
      headers: { 'Authorization': `Bearer ${this.config.token}` }
    });
    
    if (!response.ok) {
      throw new Error(`Failed to list variables: ${response.status}`);
    }
    
    return await response.json();
  }

  /**
   * Update git-sync configuration via API
   */
  async updateGitSyncConfig(config: any): Promise<void> {
    const response = await fetch(`${this.config.baseUrl}/api/w/${this.config.workspace}/workspaces/edit_git_sync_config`, {
      method: 'POST',
      headers: {
        'Authorization': `Bearer ${this.config.token}`,
        'Content-Type': 'application/json'
      },
      body: JSON.stringify(config)
    });
    
    if (!response.ok) {
      const errorText = await response.text();
      throw new Error(`Failed to update git-sync config: ${response.status} ${response.statusText} - ${errorText}`);
    }
    
    // Consume the response body to avoid resource leak
    await response.text();
  }

  /**
   * Create CLI command with proper authentication
   */
  createCLICommand(args: string[], workingDir: string, opts?: { workspace?: string; token?: string }): { cmd: string[], cwd: string } {
    const workspace = opts?.workspace || this.config.workspace;
    const fullArgs = [
      '--base-url', this.config.baseUrl,
      '--workspace', workspace,
      '--token', opts?.token || this.config.token,
      '--config-dir', this.config.testConfigDir,
      ...args
    ];

    const useNode = process.env["TEST_CLI_RUNTIME"] === "node";
    const cliDir = new URL('..', import.meta.url).pathname;
    const entrypoint = useNode
      ? new URL('../npm/esm/main.js', import.meta.url).pathname
      : new URL('../src/main.ts', import.meta.url).pathname;
    const runtime = useNode ? 'node' : 'bun';
    const runtimeArgs = useNode ? [entrypoint] : ['run', entrypoint];

    console.log('CLI Command:', [runtime, ...runtimeArgs, ...fullArgs].join(' '));

    return {
      cmd: [runtime, ...runtimeArgs, ...fullArgs],
      cwd: workingDir,
    };
  }

  /**
   * Run CLI command and return result
   */
  async runCLICommand(args: string[], workingDir: string, opts?: { workspace?: string; token?: string }): Promise<{
    stdout: string;
    stderr: string;
    code: number;
  }> {
    const { cmd, cwd } = this.createCLICommand(args, workingDir, opts);
    const proc = Bun.spawn(cmd, {
      stdout: 'pipe',
      stderr: 'pipe',
      cwd,
    });
    const [code, stdout, stderr] = await Promise.all([
      proc.exited,
      new Response(proc.stdout).text(),
      new Response(proc.stderr).text(),
    ]);
    return { stdout, stderr, code };
  }

  /**
   * Authenticate with the API and get a proper token
   */
  private async authenticate(): Promise<void> {
    console.log('🔑 Authenticating with backend...');
    
    // First, initialize the database with our workspace and user
    await this.initializeTestData();
    
    // Step 1: Login to get session token
    const loginResponse = await fetch(`${this.config.baseUrl}/api/auth/login`, {
      method: 'POST',
      headers: {
        'Content-Type': 'application/json'
      },
      body: JSON.stringify({
        email: this.config.username,
        password: this.config.password
      })
    });
    
    if (!loginResponse.ok) {
      throw new Error(`Login failed: ${loginResponse.status} ${loginResponse.statusText}`);
    }
    
    // The session token is returned as plain text
    const sessionToken = await loginResponse.text();
    
    // Use the session token directly (API token creation has issues)
    this.config.token = sessionToken;
    
    // Step 2: Verify the session token works
    const testResponse = await fetch(`${this.config.baseUrl}/api/w/${this.config.workspace}/workspaces/get_settings`, {
      headers: {
        'Authorization': `Bearer ${this.config.token}`
      }
    });
    
    if (!testResponse.ok) {
      const errorText = await testResponse.text();
      throw new Error(`Token verification failed: ${testResponse.status} ${testResponse.statusText} - ${errorText}`);
    }
    
    // Consume response body to avoid leak
    await testResponse.text();
    
    console.log('✅ Authentication successful');
  }

  /**
   * Initialize test workspace and user data via REST API
   */
  private async initializeTestData(): Promise<void> {
    console.log('🔧 Initializing test workspace...');
    
    const initSQL = `
      -- Create the test workspace if it doesn't exist (using INSERT ... WHERE NOT EXISTS)
      INSERT INTO workspace (id, name, owner, deleted, premium)
      SELECT '${this.config.workspace}', 'Test Workspace', 'admin@windmill.dev', false, false
      WHERE NOT EXISTS (SELECT 1 FROM workspace WHERE id = '${this.config.workspace}');

      -- Create admin user for the test workspace
      INSERT INTO usr (workspace_id, email, username, is_admin, created_at)
      SELECT '${this.config.workspace}', 'admin@windmill.dev', 'admin', true, NOW()
      WHERE NOT EXISTS (SELECT 1 FROM usr WHERE workspace_id = '${this.config.workspace}' AND email = 'admin@windmill.dev');

      -- Create a test token for CLI authentication
      DELETE FROM token WHERE workspace_id = '${this.config.workspace}' AND owner = 'admin@windmill.dev' AND label = 'CLI Test Token';
      INSERT INTO token (workspace_id, owner, label, token, created_at, expiration)
      VALUES ('${this.config.workspace}', 'admin@windmill.dev', 'CLI Test Token', '${this.config.token}', NOW(), NOW() + INTERVAL '1 year');

      -- Set up workspace settings with git-sync configuration for testing
      INSERT INTO workspace_settings (workspace_id, git_sync)
      VALUES ('${this.config.workspace}', '{
        "include_path": ["f/**", "g/**"],
        "include_type": ["script", "flow", "app", "folder", "resource"],
        "repositories": [{
          "exclude_types_override": [],
          "script_path": "f/**",
          "git_repo_resource_path": "u/test/test_repo",
          "use_individual_branch": false,
          "group_by_folder": false,
          "settings": {
            "include_path": ["f/**"],
            "exclude_path": ["*.test.ts", "*.spec.ts"],
            "extra_include_path": [],
            "include_type": ["script", "flow", "app", "folder", "resource"]
          }
        }],
        "exclude_path": ["*.test.ts", "*.spec.ts"],
        "extra_include_path": []
      }'::jsonb)
      ON CONFLICT (workspace_id) DO UPDATE SET git_sync = EXCLUDED.git_sync;

      -- Create test folders for proper structure  
      INSERT INTO folder (workspace_id, name, display_name, owners)
      SELECT '${this.config.workspace}', 'f', 'Scripts and Flows', ARRAY['admin@windmill.dev']
      WHERE NOT EXISTS (SELECT 1 FROM folder WHERE workspace_id = '${this.config.workspace}' AND name = 'f');
      
      INSERT INTO folder (workspace_id, name, display_name, owners)
      SELECT '${this.config.workspace}', 'g', 'General Scripts', ARRAY['admin@windmill.dev']
      WHERE NOT EXISTS (SELECT 1 FROM folder WHERE workspace_id = '${this.config.workspace}' AND name = 'g');
      
      INSERT INTO folder (workspace_id, name, display_name, owners)
      SELECT '${this.config.workspace}', 'u', 'User Scripts', ARRAY['admin@windmill.dev']
      WHERE NOT EXISTS (SELECT 1 FROM folder WHERE workspace_id = '${this.config.workspace}' AND name = 'u');

      -- Create the 'all' group if it doesn't exist
      INSERT INTO group_ (workspace_id, name, summary)
      SELECT '${this.config.workspace}', 'all', 'All users group'
      WHERE NOT EXISTS (SELECT 1 FROM group_ WHERE workspace_id = '${this.config.workspace}' AND name = 'all');

      -- Grant all permissions to admin user
      INSERT INTO usr_to_group (workspace_id, usr, group_)
      SELECT '${this.config.workspace}', 'admin@windmill.dev', 'all'
      WHERE NOT EXISTS (SELECT 1 FROM usr_to_group WHERE workspace_id = '${this.config.workspace}' AND usr = 'admin@windmill.dev' AND group_ = 'all');

      -- Add test resource for git repository (mocked)
      INSERT INTO resource (workspace_id, path, description, resource_type, value, extra_perms)
      VALUES ('${this.config.workspace}', 'u/test/test_repo', 'Test Git Repository', 'git_repository', '{
        "git_url": "https://github.com/test/test-repo.git",
        "branch": "main", 
        "git_username": "test-user",
        "git_token": "test-token-123"
      }'::jsonb, '{}'::jsonb)
      ON CONFLICT (workspace_id, path) DO UPDATE SET value = EXCLUDED.value;

      -- EE license key is now handled via environment variable only

      -- Create workspace encryption key for sync operations
      INSERT INTO workspace_key (workspace_id, kind, key)
      VALUES ('${this.config.workspace}', 'cloud', 'test_encryption_key_for_sync_operations_123456789abcdef')
      ON CONFLICT (workspace_id, kind) DO UPDATE SET key = EXCLUDED.key;
    `;

    const result = await runCommand('docker', ['compose', '-f', this.config.composeFile, 'exec', '-T', 'test_db',
             'psql', '-U', 'postgres', '-d', 'windmill_test', '-c', initSQL], {
      env: {
        ...(process.env.EE_LICENSE_KEY && { EE_LICENSE_KEY: process.env.EE_LICENSE_KEY })
      }
    });

    if (result.code !== 0) {
      throw new Error(`Failed to initialize test data: ${result.stderr}`);
    }
    
    console.log('✅ Test workspace initialized');
    
    // Verify license key was stored
    const checkResult = await runCommand('docker', ['compose', '-f', this.config.composeFile, 'exec', '-T', 'test_db',
             'psql', '-U', 'postgres', '-d', 'windmill_test', '-c',
             "SELECT name, value FROM global_settings WHERE name = 'license_key';"], {
      env: {
        EE_LICENSE_KEY: process.env.EE_LICENSE_KEY || 'REMOVED_HARDCODED_LICENSE'
      }
    });

    if (checkResult.code === 0) {
      console.log('License key in database:', checkResult.stdout.trim());
    }
  }

  // Private helper methods

  private async waitForHealthy(): Promise<void> {
    console.log('⏳ Waiting for containers to be healthy...');
    
    const maxAttempts = 30;
    let attempts = 0;
    
    while (attempts < maxAttempts) {
      const result = await runCommand('docker', ['compose', '-f', this.config.composeFile, 'ps', '--format', 'json'], {
        env: {
          ...(process.env.EE_LICENSE_KEY && { EE_LICENSE_KEY: process.env.EE_LICENSE_KEY })
        }
      });

      if (result.code === 0) {
        const output = result.stdout;
        if (output.trim()) {
          const containers = output.trim().split('\n').map(line => JSON.parse(line));
          
          const allHealthy = containers.every(container => 
            container.State === 'running' && 
            (container.Health === 'healthy' || container.Health === '' || container.Health === undefined)
          );
          
          if (allHealthy) {
            return;
          }
        }
      }
      
      attempts++;
      await new Promise(resolve => setTimeout(resolve, 2000));
    }
    
    throw new Error('Containers failed to become healthy within timeout');
  }

  private async waitForAPI(): Promise<void> {
    console.log('⏳ Waiting for API to be responsive...');
    
    const maxAttempts = 30;
    let attempts = 0;
    
    while (attempts < maxAttempts) {
      try {
        const response = await fetch(`${this.config.baseUrl}/api/version`, {
          signal: AbortSignal.timeout(5000)
        });
        
        if (response.ok) {
          const version = await response.text();
          console.log(`📡 API ready (version: ${version})`);
          return;
        } else {
          // Consume response body even on non-OK responses to avoid leak
          await response.text();
        }
      } catch (error) {
        // Continue trying
      }
      
      attempts++;
      await new Promise(resolve => setTimeout(resolve, 2000));
    }
    
    throw new Error('API failed to respond within timeout');
  }

  /**
   * Quick health check for the backend
   */
  async checkHealth(): Promise<void> {
    try {
      const response = await fetch(`${this.config.baseUrl}/api/version`);
      if (!response.ok) {
        throw new Error(`Health check failed: ${response.status}`);
      }
      await response.text(); // Consume response to avoid leak
    } catch (error) {
      throw new Error(`Backend health check failed: ${error instanceof Error ? error.message : String(error)}`);
    }
  }

}

// Global backend instance for persistent containers
let globalBackend: ContainerizedBackend | null = null;

// Convenience function for tests with persistent backend
export async function withContainerizedBackend<T>(
  testFn: (backend: ContainerizedBackend, tempDir: string) => Promise<T>
): Promise<T> {
  // Reuse global backend if it exists and is healthy
  if (!globalBackend) {
    globalBackend = new ContainerizedBackend();
    await globalBackend.start();
  } else {
    // Quick health check
    try {
      await globalBackend.checkHealth();
    } catch {
      // Backend is unhealthy, restart it
      await globalBackend.stop();
      globalBackend = new ContainerizedBackend();
      await globalBackend.start();
    }
  }
  
  const tempDir = await mkdtemp(join(tmpdir(), 'windmill_cli_test_'));

  try {
    await globalBackend.reset();
    await globalBackend.seedTestData();

    return await testFn(globalBackend, tempDir);
  } finally {
    await rm(tempDir, { recursive: true });
  }
}

// Cleanup function for test suites
export async function cleanupGlobalBackend(): Promise<void> {
  if (globalBackend) {
    await globalBackend.stop();
    globalBackend = null;
  }
}