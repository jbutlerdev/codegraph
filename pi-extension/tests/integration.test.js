/**
 * CodeGraph Extension Integration Tests
 * 
 * Tests the CLI commands that the pi extension tools wrap.
 * Run with: node tests/integration.test.js
 */

const { execSync, spawnSync } = require('child_process');
const path = require('path');

const CODEGRAPH_BIN = process.env.CODEGRAPH_BIN || 
  path.join(__dirname, '../../target/release/codegraph');

// Use full path for local repos (more reliable than name resolution)
const TEST_REPO_PATH = '/data/jbutler/git/jbutlerdev/codegraph';
const TEST_REPO_ID = '08bca880-5eef-401d-998d-1f2c68b60def';

// Helper to run codegraph CLI - uses spawn for proper quoting
function runCodegraph(args) {
  try {
    // Use spawnSync with shell: false for proper argument handling
    const result = spawnSync(CODEGRAPH_BIN, args, {
      encoding: 'utf-8',
      timeout: 60000,
    });
    const output = (result.stdout || '') + (result.stderr || '');
    return { 
      success: result.status === 0, 
      output: output, 
      error: result.error ? result.error.message : null,
      status: result.status
    };
  } catch (e) {
    return { success: false, output: '', error: e.message };
  }
}

// Assertions
function assert(condition, message) {
  if (!condition) {
    throw new Error(`ASSERTION FAILED: ${message}`);
  }
}

function assertContains(output, text, message) {
  if (!output.includes(text)) {
    throw new Error(`${message}\nExpected to contain: "${text}"\nActual output:\n${output.substring(0, 500)}`);
  }
}

function assertNotContains(output, text, message) {
  if (output.includes(text)) {
    throw new Error(`${message}\nExpected NOT to contain: "${text}"\nActual output:\n${output.substring(0, 500)}`);
  }
}

// Tests
async function testListRepos() {
  console.log('\n📋 Test: list_repos');
  const result = runCodegraph(['ls']);
  assert(result.success, 'ls command should succeed');
  assertContains(result.output, 'REPOS', 'Should show REPOS header');
  assertContains(result.output, 'codegraph', 'Should show codegraph repo');
  console.log('  ✅ list_repos works');
}

async function testSearch() {
  console.log('\n🔍 Test: search');
  
  // Search for "knowledge graph" should find files
  const result = runCodegraph(['search', 'knowledge graph', '--repo', TEST_REPO_ID]);
  assert(result.success, 'search command should succeed');
  assertContains(result.output, 'SEARCH RESULTS', 'Should show SEARCH RESULTS header');
  console.log('  ✅ search works');
}

async function testEntityDefines() {
  console.log('\n📍 Test: entity defines (class)');
  
  // Find where Database struct is defined - entity_type is positional argument
  const result = runCodegraph(['defines', 'class', 'Database', '--repo', TEST_REPO_ID]);
  assert(result.success, 'defines command should succeed');
  assertContains(result.output, 'DEFINITION', 'Should show DEFINITION header');
  console.log('  Output preview:', result.output.substring(0, 150).replace(/\n/g, ' ').trim());
  console.log('  ✅ entity defines works');
}

async function testEntityDefinesAll() {
  console.log('\n📍 Test: entity defines --all');
  
  // Find with all usages
  const result = runCodegraph(['defines', 'class', 'Database', '--repo', TEST_REPO_ID, '--all']);
  assert(result.success, 'defines --all command should succeed');
  assertContains(result.output, 'COMPLETE VIEW', 'Should show COMPLETE VIEW header');
  console.log('  ✅ entity defines --all works');
}

async function testEntityUses() {
  console.log('\n📦 Test: entity uses');
  
  // Find files that reference Error
  const result = runCodegraph(['uses', 'class', 'Error', '--repo', TEST_REPO_ID]);
  assert(result.success, 'uses command should succeed');
  assertContains(result.output, 'USAGES', 'Should show USAGES header');
  console.log('  ✅ entity uses works');
}

async function testEntityFunction() {
  console.log('\n⚡ Test: entity defines (function)');
  
  // Find where main is defined
  const result = runCodegraph(['defines', 'function', 'main', '--repo', TEST_REPO_ID]);
  assert(result.success, 'defines function command should succeed');
  assertContains(result.output, 'DEFINITION', 'Should show DEFINITION header');
  console.log('  ✅ entity defines (function) works');
}

async function testEntityModule() {
  console.log('\n📦 Test: entity defines (module)');
  
  // Find where a module is defined
  const result = runCodegraph(['defines', 'module', 'db', '--repo', TEST_REPO_ID]);
  assert(result.success, 'defines module command should succeed');
  assertContains(result.output, 'DEFINITION', 'Should show DEFINITION header');
  console.log('  ✅ entity defines (module) works');
}

async function testFileDeps() {
  console.log('\n📄 Test: file deps');
  
  // Get deps of a source file
  const result = runCodegraph(['deps', '--repo', TEST_REPO_ID, '--file', 'src/main.rs']);
  assert(result.success, 'deps command should succeed');
  assertContains(result.output, 'DEPENDENCIES', 'Should show DEPENDENCIES header');
  console.log('  ✅ file deps works');
}

async function testFileDependents() {
  console.log('\n🔗 Test: file dependents');
  
  // Get files that depend on a file
  const result = runCodegraph(['dependents', '--repo', TEST_REPO_ID, '--file', 'src/error.rs']);
  assert(result.success, 'dependents command should succeed');
  assertContains(result.output, 'DEPENDENTS', 'Should show DEPENDENTS header');
  console.log('  ✅ file dependents works');
}

async function testCat() {
  console.log('\n📖 Test: file cat');
  
  // View file metadata
  const result = runCodegraph(['cat', '--repo', TEST_REPO_PATH, '--file', 'Cargo.toml']);
  assert(result.success, 'cat command should succeed');
  assertContains(result.output, 'FILE', 'Should show FILE header');
  assertContains(result.output, 'codegraph', 'Should show package name');
  console.log('  ✅ file cat works');
}

async function testCatWithContent() {
  console.log('\n📖 Test: file cat with content');
  
  // View file with content
  const result = runCodegraph(['cat', '--repo', TEST_REPO_PATH, '--file', 'Cargo.toml', '--content', '--range', '1-10']);
  assert(result.success, 'cat --content command should succeed');
  assertContains(result.output, 'CONTENT', 'Should show CONTENT section');
  assertContains(result.output, '[package]', 'Should show TOML content');
  console.log('  ✅ file cat --content works');
}

async function testGrep() {
  console.log('\n🔎 Test: grep');
  
  // Search for pattern in Rust files
  const result = runCodegraph(['grep', '--repo', TEST_REPO_PATH, '--glob', '*.rs', 'pub fn']);
  assert(result.success, 'grep command should succeed');
  assertContains(result.output, 'GREP', 'Should show GREP header');
  console.log('  ✅ grep works');
}

async function testShortRepoId() {
  console.log('\n🔑 Test: short repo ID');
  
  // Get short ID from full ID
  const shortId = TEST_REPO_ID.substring(0, 8);
  
  // Try using short ID
  const result = runCodegraph(['cat', '--repo', shortId, '--file', 'Cargo.toml']);
  assert(result.success, 'Short repo ID should work');
  assertContains(result.output, 'codegraph', 'Should find results with short ID');
  console.log('  ✅ short repo ID works');
}

async function testEntityNameMatching() {
  console.log('\n🏷️  Test: entity name matching (no line numbers)');
  
  // Search without line number suffix - should find Config
  const result = runCodegraph(['defines', 'class', 'Config', '--repo', TEST_REPO_ID]);
  assert(result.success, 'Entity name matching should work');
  console.log('  ✅ entity name matching works');
}

async function testLookup() {
  console.log('\n🔍 Test: lookup');
  
  // Lookup entities
  const result = runCodegraph(['lookup', 'Database', '--repo', TEST_REPO_ID]);
  assert(result.success, 'lookup command should succeed');
  assertContains(result.output, 'LOOKUP RESULTS', 'Should show LOOKUP RESULTS header');
  console.log('  ✅ lookup works');
}

async function testTopClasses() {
  console.log('\n📊 Test: top classes');
  
  // Find most depended-upon classes
  const result = runCodegraph(['top', 'class', '--repo', TEST_REPO_ID]);
  assert(result.success, 'top command should succeed');
  assertContains(result.output, 'MOST DEPENDED-UPON ENTITIES', 'Should show MOST DEPENDED-UPON ENTITIES header');
  assertContains(result.output, 'refs', 'Should show reference counts');
  console.log('  ✅ top classes works');
}

async function testTopFunctions() {
  console.log('\n⚡ Test: top functions');
  
  // Find most depended-upon functions
  const result = runCodegraph(['top', 'function', '--repo', TEST_REPO_ID]);
  assert(result.success, 'top function command should succeed');
  assertContains(result.output, 'MOST DEPENDED-UPON ENTITIES', 'Should show MOST DEPENDED-UPON ENTITIES header');
  console.log('  ✅ top functions works');
}

async function testTopModules() {
  console.log('\n📦 Test: top modules');
  
  // Find most depended-upon modules
  const result = runCodegraph(['top', 'module', '--repo', TEST_REPO_ID]);
  assert(result.success, 'top module command should succeed');
  assertContains(result.output, 'MOST DEPENDED-UPON ENTITIES', 'Should show MOST DEPENDED-UPON ENTITIES header');
  console.log('  ✅ top modules works');
}

async function testTopWithLimit() {
  console.log('\n🔢 Test: top with limit');
  
  // Find top 3 classes
  const result = runCodegraph(['top', 'class', '--repo', TEST_REPO_ID, '--limit', '3']);
  assert(result.success, 'top with limit should succeed');
  assertContains(result.output, 'MOST DEPENDED-UPON ENTITIES', 'Should show header');
  console.log('  ✅ top with limit works');
}

async function testStatsImproved() {
  console.log('\n📈 Test: stats (improved)');
  
  // Stats should show more info now
  const result = runCodegraph(['stats']);
  assert(result.success, 'stats command should succeed');
  assertContains(result.output, 'STATISTICS', 'Should show STATISTICS header');
  // New stats should include entity counts
  assertContains(result.output, 'Total Classes', 'Should show Total Classes');
  assertContains(result.output, 'Total Functions', 'Should show Total Functions');
  assertContains(result.output, 'TOP ENTITIES BY USAGE', 'Should show top entities');
  console.log('  ✅ stats (improved) works');
}

async function testShortRepoIdTop() {
  console.log('\n🔑 Test: short repo ID with top');
  
  // Use short ID
  const shortId = TEST_REPO_ID.substring(0, 8);
  const result = runCodegraph(['top', 'class', '--repo', shortId]);
  assert(result.success, 'Short repo ID should work with top');
  assertContains(result.output, 'MOST DEPENDED-UPON ENTITIES', 'Should find results with short ID');
  console.log('  ✅ short repo ID with top works');
}

// Main test runner
async function runTests() {
  console.log('═══════════════════════════════════════════════════════');
  console.log('  CodeGraph Integration Tests');
  console.log('═══════════════════════════════════════════════════════');
  console.log(`\nUsing binary: ${CODEGRAPH_BIN}`);
  console.log(`Test repo ID: ${TEST_REPO_ID}`);
  console.log(`Test repo path: ${TEST_REPO_PATH}`);

  const tests = [
    { name: 'list_repos', fn: testListRepos },
    { name: 'search', fn: testSearch },
    { name: 'entity_defines', fn: testEntityDefines },
    { name: 'entity_defines_all', fn: testEntityDefinesAll },
    { name: 'entity_uses', fn: testEntityUses },
    { name: 'entity_function', fn: testEntityFunction },
    { name: 'entity_module', fn: testEntityModule },
    { name: 'file_deps', fn: testFileDeps },
    { name: 'file_dependents', fn: testFileDependents },
    { name: 'cat', fn: testCat },
    { name: 'cat_content', fn: testCatWithContent },
    { name: 'grep', fn: testGrep },
    { name: 'short_repo_id', fn: testShortRepoId },
    { name: 'entity_name_matching', fn: testEntityNameMatching },
    { name: 'lookup', fn: testLookup },
    { name: 'top_classes', fn: testTopClasses },
    { name: 'top_functions', fn: testTopFunctions },
    { name: 'top_modules', fn: testTopModules },
    { name: 'top_with_limit', fn: testTopWithLimit },
    { name: 'stats_improved', fn: testStatsImproved },
    { name: 'short_repo_id_top', fn: testShortRepoIdTop },
  ];

  let passed = 0;
  let failed = 0;

  for (const test of tests) {
    try {
      await test.fn();
      passed++;
    } catch (e) {
      console.log(`\n  ❌ ${test.name}: ${e.message}`);
      failed++;
    }
  }

  console.log('\n═══════════════════════════════════════════════════════');
  console.log(`  Results: ${passed} passed, ${failed} failed`);
  console.log('═══════════════════════════════════════════════════════\n');

  process.exit(failed > 0 ? 1 : 0);
}

// Run if executed directly
if (require.main === module) {
  runTests().catch(e => {
    console.error('Test runner error:', e);
    process.exit(1);
  });
}

module.exports = { runTests };
