#!/usr/bin/env deno run --allow-all

/**
 * Unified test runner for all CLI tests
 * All tests are now independent and don't require local Windmill setup
 */

import { parseArgs } from "https://deno.land/std@0.224.0/cli/parse_args.ts";

const COLORS = {
  reset: '\x1b[0m',
  green: '\x1b[32m',
  red: '\x1b[31m',
  yellow: '\x1b[33m',
  blue: '\x1b[34m',
  magenta: '\x1b[35m',
  cyan: '\x1b[36m'
};

interface TestSuite {
  name: string;
  file: string;
  description: string;
  type: 'unit' | 'integration';
}

const TEST_SUITES: TestSuite[] = [
  {
    name: 'Utils',
    file: 'test/utils.test.ts',
    description: 'Unit tests for utility functions (23 tests)',
    type: 'unit'
  },
  {
    name: 'Config Resolution',
    file: 'test/sync_config_resolution.test.ts',
    description: 'Configuration resolution and wmill.yaml parsing tests (7 tests)',
    type: 'unit'
  },
  {
    name: 'Containerized',
    file: 'test/containerized_backend.test.ts',
    description: 'Real backend integration tests (24 tests) - Complete CLI coverage',
    type: 'integration'
  }
];

async function runTestSuite(suite: TestSuite, verbose: boolean = false): Promise<{
  name: string;
  passed: boolean;
  output: string;
  duration: number;
}> {
  const startTime = Date.now();
  
  console.log(`${COLORS.blue}Running ${suite.name}...${COLORS.reset}`);
  
  const cmd = new Deno.Command('deno', {
    args: ['test', '--allow-all', suite.file],
    stdout: 'piped',
    stderr: 'piped'
  });

  const result = await cmd.output();
  const duration = Date.now() - startTime;
  const output = new TextDecoder().decode(result.stdout) + new TextDecoder().decode(result.stderr);
  
  const passed = result.code === 0;
  
  if (verbose || !passed) {
    console.log(output);
  }
  
  return {
    name: suite.name,
    passed,
    output,
    duration
  };
}

async function main() {
  const args = parseArgs(Deno.args, {
    boolean: ['help', 'verbose', 'unit-only', 'integration-only'],
    string: ['filter'],
    alias: {
      h: 'help',
      v: 'verbose',
      f: 'filter'
    }
  });

  if (args.help) {
    console.log(`
${COLORS.cyan}Windmill CLI Test Runner${COLORS.reset}

Usage: deno run --allow-all test/runner.ts [options]

Options:
  -h, --help              Show this help message
  -v, --verbose           Show detailed output for all tests
  --unit-only             Run only unit tests
  --integration-only      Run only integration tests
  -f, --filter <name>     Run only tests matching the filter

Test Suites:
${TEST_SUITES.map(suite => 
  `  ${COLORS.yellow}${suite.name}${COLORS.reset} (${suite.type})\n    ${suite.description}`
).join('\n')}

Examples:
  deno run --allow-all test/runner.ts                    # Run all tests
  deno run --allow-all test/runner.ts --unit-only        # Run only unit tests
  deno run --allow-all test/runner.ts --filter Settings  # Run tests with 'Settings' in name

Note: All tests are now independent and don't require local Windmill setup!
`);
    return;
  }

  let suitesToRun = TEST_SUITES;

  // Apply filters
  if (args['unit-only']) {
    suitesToRun = suitesToRun.filter(suite => suite.type === 'unit');
  } else if (args['integration-only']) {
    suitesToRun = suitesToRun.filter(suite => suite.type === 'integration');
  }

  if (args.filter) {
    suitesToRun = suitesToRun.filter(suite => 
      suite.name.toLowerCase().includes(args.filter.toLowerCase())
    );
  }

  console.log(`${COLORS.magenta}🧪 Windmill CLI Test Suite${COLORS.reset}`);
  console.log(`Running ${suitesToRun.length} test suite(s)...\n`);

  const results = [];
  let totalPassed = 0;
  let totalFailed = 0;

  for (const suite of suitesToRun) {
    const result = await runTestSuite(suite, args.verbose);
    results.push(result);
    
    if (result.passed) {
      totalPassed++;
      console.log(`${COLORS.green}✅ ${result.name} (${result.duration}ms)${COLORS.reset}`);
    } else {
      totalFailed++;
      console.log(`${COLORS.red}❌ ${result.name} (${result.duration}ms)${COLORS.reset}`);
    }
  }

  console.log(`\n${COLORS.cyan}📊 Summary${COLORS.reset}`);
  console.log(`${COLORS.green}Passed: ${totalPassed}${COLORS.reset}`);
  console.log(`${COLORS.red}Failed: ${totalFailed}${COLORS.reset}`);
  console.log(`Total: ${totalPassed + totalFailed}`);

  if (totalFailed > 0) {
    console.log(`\n${COLORS.yellow}Failed Test Suites:${COLORS.reset}`);
    results.filter(r => !r.passed).forEach(result => {
      console.log(`  ${COLORS.red}• ${result.name}${COLORS.reset}`);
    });
    
    console.log(`\n${COLORS.yellow}💡 Tips:${COLORS.reset}`);
    console.log(`  • All tests are independent and don't require local Windmill setup`);
    console.log(`  • Use --verbose to see detailed output for failed tests`);
    console.log(`  • Check the mock server responses if integration tests fail`);
  } else {
    console.log(`\n${COLORS.green}🎉 All tests passed!${COLORS.reset}`);
    console.log(`${COLORS.green}✨ Ready for CI/CD deployment${COLORS.reset}`);
  }

  if (totalFailed > 0) {
    Deno.exit(1);
  }
}

if (import.meta.main) {
  await main();
}