#!/usr/bin/env node
// seed-all.js — Dynamic project discovery & universal seeder
// Scans projects/ + root-level project dirs, detects types, seeds artifacts + nodes
// Run: node mongodb/seed-all.js

const { MongoClient } = require('mongodb');
const fs = require('fs');
const path = require('path');
const os = require('os');

// ── .env loader (reused from seed-artifacts.js) ─────────────────
function loadEnv() {
  const envPath = path.join(__dirname, '.env');
  if (!fs.existsSync(envPath)) return;
  for (const raw of fs.readFileSync(envPath, 'utf8').split('\n')) {
    const line = raw.trim();
    if (!line || line.startsWith('#')) continue;
    const eq = line.indexOf('=');
    if (eq < 1) continue;
    const key = line.slice(0, eq).trim();
    let val = line.slice(eq + 1).trim().replace(/^["']|["']$/g, '');
    if (!process.env[key]) process.env[key] = val;
  }
}

function getUri() {
  if (process.env.MONGODB_URI) return process.env.MONGODB_URI;
  const admin = process.env.MONGODB_CLUSTER0_ADMIN;
  if (admin) {
    let clean = admin.replace(/^["'*\s]+|["'\s]+$/g, '');
    if (clean.startsWith('mongodb')) return clean;
    const parts = clean.split('@');
    if (parts.length >= 3) {
      const user = encodeURIComponent(parts[0]);
      const pass = encodeURIComponent(parts.slice(1, -1).join('@'));
      const host = parts[parts.length - 1];
      return `mongodb+srv://${user}:${pass}@${host}`;
    }
    return `mongodb+srv://${clean}`;
  }
  return null;
}

// ── project type detection ──────────────────────────────────────
const TYPE_MARKERS = [
  { file: 'Cargo.toml', type: 'rust' },
  { file: 'flake.nix', type: 'nix' },
  { file: 'package.json', type: 'node' },
  { file: 'go.mod', type: 'go' },
  { file: 'pyproject.toml', type: 'python' },
  { file: 'requirements.txt', type: 'python' },
  { file: 'Makefile', type: 'generic' },
];

function detectProjectType(projectPath) {
  for (const marker of TYPE_MARKERS) {
    if (fs.existsSync(path.join(projectPath, marker.file))) {
      return marker.type;
    }
  }
  return 'unknown';
}

// ── key files to seed as artifacts ──────────────────────────────
const KEY_FILES = [
  'CLAUDE.md',
  'README.md',
  'Cargo.toml',
  'package.json',
  'flake.nix',
  'tree.json',
  'Makefile',
  'docker-compose.yml',
];

const MAX_ARTIFACT_SIZE = 100 * 1024; // 100KB cap

// ── count crates (Rust workspaces) ──────────────────────────────
function countCrates(projectPath) {
  const cargoToml = path.join(projectPath, 'Cargo.toml');
  if (!fs.existsSync(cargoToml)) return 0;
  const content = fs.readFileSync(cargoToml, 'utf8');
  const match = content.match(/\[workspace\][\s\S]*?members\s*=\s*\[([\s\S]*?)\]/);
  if (match) {
    return match[1].split(',').filter(s => s.trim().replace(/['"]/g, '')).length;
  }
  // Single crate
  return 1;
}

// ── discover projects ───────────────────────────────────────────
function discoverProjects(root) {
  const projects = [];
  const projectsDir = path.join(root, 'projects');

  // Scan projects/ subdirectories
  if (fs.existsSync(projectsDir)) {
    for (const entry of fs.readdirSync(projectsDir, { withFileTypes: true })) {
      if (!entry.isDirectory()) continue;
      if (entry.name.startsWith('.')) continue;
      const fullPath = path.join(projectsDir, entry.name);
      projects.push({
        name: entry.name,
        path: path.relative(root, fullPath),
        fullPath,
        source: 'projects/',
      });
    }
  }

  // Root-level project dirs (gentlyos/, web/, etc.) — only if they contain code
  const rootSkip = new Set([
    'projects', 'node_modules', '.git', '.claude', 'docker', 'config',
    'security', 'bin', 'lib', 'audit', 'mongodb',
  ]);
  for (const entry of fs.readdirSync(root, { withFileTypes: true })) {
    if (!entry.isDirectory()) continue;
    if (entry.name.startsWith('.')) continue;
    if (rootSkip.has(entry.name)) continue;
    const fullPath = path.join(root, entry.name);
    // Must have at least one code-like file
    const hasCode = fs.readdirSync(fullPath).some(f =>
      /\.(js|ts|sh|py|rs|nix|toml|yaml|yml|json|jsx|tsx|md)$/.test(f)
    );
    if (hasCode) {
      projects.push({
        name: entry.name,
        path: entry.name,
        fullPath,
        source: 'root',
      });
    }
  }

  // claude-cage itself (the root project)
  projects.push({
    name: 'claude-cage',
    path: '.',
    fullPath: root,
    source: 'root',
  });

  return projects;
}

// ── main ────────────────────────────────────────────────────────
async function main() {
  loadEnv();
  const uri = getUri();
  if (!uri) { console.error('No MongoDB URI found'); process.exit(1); }

  const dbName = process.env.MONGODB_DB || 'claude_cage';
  const client = new MongoClient(uri, {
    serverSelectionTimeoutMS: 10000,
    connectTimeoutMS: 10000,
  });

  const ROOT = path.resolve(__dirname, '..');
  const now = new Date();
  const meta = { _ts: now, _host: os.hostname(), _seeded: true };

  const discovered = discoverProjects(ROOT);
  console.log(`Discovered ${discovered.length} projects`);

  const allArtifacts = [];
  const allProjects = [];
  let nodesSeeded = 0;

  for (const proj of discovered) {
    const type = detectProjectType(proj.fullPath);
    const crateCount = type === 'rust' ? countCrates(proj.fullPath) : 0;

    allProjects.push({
      name: proj.name,
      path: proj.path,
      type,
      source: proj.source,
      status: 'active',
      crate_count: crateCount,
    });

    console.log(`  ${proj.name} (${type}) — ${proj.path}`);

    // Collect key files as artifacts
    for (const keyFile of KEY_FILES) {
      const filePath = path.join(proj.fullPath, keyFile);
      if (!fs.existsSync(filePath)) continue;

      const stat = fs.statSync(filePath);
      if (!stat.isFile()) continue;

      const content = fs.readFileSync(filePath, 'utf8');
      allArtifacts.push({
        name: keyFile,
        path: path.join(proj.path, keyFile),
        type: keyFile.endsWith('.md') ? 'docs' : keyFile.endsWith('.json') ? 'config' : 'code',
        project: proj.name,
        content: content.slice(0, MAX_ARTIFACT_SIZE),
        size: content.length,
        ...meta,
      });
    }
  }

  // ── insert into MongoDB ─────────────────────────────────────
  try {
    await client.connect();
    const db = client.db(dbName);

    // Upsert projects
    for (const p of allProjects) {
      await db.collection('projects').updateOne(
        { name: p.name },
        { $set: { ...p, ...meta } },
        { upsert: true }
      );
    }
    console.log(`\nUpserted ${allProjects.length} projects`);

    // Insert artifacts (clear stale seeded artifacts first, then insert fresh)
    if (allArtifacts.length > 0) {
      await db.collection('artifacts').deleteMany({ _seeded: true });
      const result = await db.collection('artifacts').insertMany(allArtifacts);
      console.log(`Inserted ${result.insertedCount} artifacts`);
    }

    // Seed tree.json nodes for any project that has one
    for (const proj of discovered) {
      const treePath = path.join(proj.fullPath, 'tree.json');
      if (!fs.existsSync(treePath)) continue;

      try {
        const tree = JSON.parse(fs.readFileSync(treePath, 'utf8'));
        if (tree.nodes && Array.isArray(tree.nodes)) {
          for (const node of tree.nodes) {
            await db.collection('nodes').updateOne(
              { id: node.id },
              { $set: { ...node, _project: proj.name, ...meta } },
              { upsert: true }
            );
            nodesSeeded++;
          }
          console.log(`Seeded ${tree.nodes.length} nodes from ${proj.name}/tree.json`);
        }
      } catch (err) {
        console.warn(`  Warning: could not parse ${proj.name}/tree.json: ${err.message}`);
      }
    }

    // Log seed:all event
    await db.collection('events').insertOne({
      type: 'seed',
      key: 'seed:all',
      value: {
        projects: allProjects.length,
        artifacts: allArtifacts.length,
        nodes: nodesSeeded,
        project_names: allProjects.map(p => p.name),
      },
      ...meta,
      _project: 'claude-cage',
    });

    // Summary
    const counts = {};
    for (const col of ['projects', 'artifacts', 'nodes', 'events']) {
      counts[col] = await db.collection(col).countDocuments();
    }
    console.log('\nCollection totals:', JSON.stringify(counts));

  } catch (err) {
    console.error('MongoDB error:', err.message);
    process.exit(1);
  } finally {
    await client.close();
  }
}

main();
