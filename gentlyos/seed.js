#!/usr/bin/env node
// gentlyos/seed.js — Seed GentlyOS documents, tree, and schema into MongoDB
// Usage: node gentlyos/seed.js

const path = require('path');
const fs = require('fs');

// Reuse the store's connection logic
const STORE = path.join(__dirname, '..', 'mongodb', 'store.js');
const ROOT = path.join(__dirname, '..');

// Simple .docx text extractor (reads word/document.xml from zip)
function extractDocxText(filePath) {
  try {
    const AdmZip = require('adm-zip');
    const zip = new AdmZip(filePath);
    const xml = zip.readAsText('word/document.xml');
    // Extract text between <w:t> tags
    const texts = [];
    const regex = /<w:t[^>]*>([^<]*)<\/w:t>/g;
    let match;
    while ((match = regex.exec(xml)) !== null) {
      texts.push(match[1]);
    }
    return texts.join(' ');
  } catch (e) {
    // Fallback: read as binary and extract visible text
    const buf = fs.readFileSync(filePath);
    const str = buf.toString('utf8');
    const texts = [];
    const regex = /<w:t[^>]*>([^<]*)<\/w:t>/g;
    let match;
    while ((match = regex.exec(str)) !== null) {
      texts.push(match[1]);
    }
    return texts.join(' ') || '[binary content — extraction failed]';
  }
}

async function seed() {
  const { execSync } = require('child_process');

  function store(cmd, ...args) {
    const escaped = args.map(a => typeof a === 'string' ? a : JSON.stringify(a));
    const full = `node "${STORE}" ${cmd} ${escaped.map(a => `'${a.replace(/'/g, "'\\''")}'`).join(' ')}`;
    try {
      return execSync(full, { timeout: 15000, encoding: 'utf8' });
    } catch (e) {
      console.error(`  store error: ${e.message.split('\n')[0]}`);
      return null;
    }
  }

  console.log('GentlyOS Seed — Loading documents, tree, and schema into MongoDB\n');

  // 1. Seed the 4 Google docs
  const docs = [
    { file: 'GentlyOS_Virtual_Organization_System.docx', name: 'virtual-org-system', type: 'design-doc' },
    { file: 'GentlyOS_Workspace_System.docx', name: 'workspace-system', type: 'design-doc' },
    { file: 'Gently_Studio_Protocols.docx', name: 'studio-protocols', type: 'design-doc' },
    { file: 'Google_Infrastructure_Research.docx', name: 'google-infra-research', type: 'research' },
  ];

  for (const doc of docs) {
    const filePath = path.join(ROOT, doc.file);
    if (!fs.existsSync(filePath)) {
      console.log(`  SKIP ${doc.file} (not found)`);
      continue;
    }
    const text = extractDocxText(filePath);
    const entry = {
      name: doc.name,
      type: doc.type,
      project: 'gentlyos',
      file: doc.file,
      content: text.substring(0, 50000), // Cap at 50k chars for MongoDB
      content_length: text.length,
      _ts: new Date().toISOString(),
    };
    store('put', 'artifacts', JSON.stringify(entry));
    console.log(`  DOC  ${doc.name} (${text.length} chars)`);
  }

  // 2. Seed the universal node schema
  const schema = fs.readFileSync(path.join(__dirname, 'universal-node.schema.json'), 'utf8');
  store('put', 'artifacts', JSON.stringify({
    name: 'universal-node-schema',
    type: 'schema',
    project: 'gentlyos',
    content: schema,
    _ts: new Date().toISOString(),
  }));
  console.log('  SCHEMA universal-node.schema.json');

  // 3. Seed the tree (both as one artifact and individual nodes)
  const treeRaw = fs.readFileSync(path.join(__dirname, 'tree.json'), 'utf8');
  const tree = JSON.parse(treeRaw);

  store('put', 'artifacts', JSON.stringify({
    name: 'gentlyos-tree',
    type: 'tree',
    project: 'gentlyos',
    content: treeRaw.substring(0, 50000),
    node_count: tree.nodes.length,
    _ts: new Date().toISOString(),
  }));
  console.log(`  TREE  ${tree.nodes.length} nodes`);

  // 4. Seed each node individually into a 'nodes' collection
  for (const node of tree.nodes) {
    store('put', 'nodes', JSON.stringify({
      ...node,
      project: 'gentlyos',
      _ts: new Date().toISOString(),
    }));
  }
  console.log(`  NODES ${tree.nodes.length} individual nodes seeded`);

  // 5. Seed the SKILL.md
  const skillPath = path.join(ROOT, 'SKILL.md');
  if (fs.existsSync(skillPath)) {
    const skill = fs.readFileSync(skillPath, 'utf8');
    store('put', 'artifacts', JSON.stringify({
      name: 'tom-collaboration-engine',
      type: 'skill',
      project: 'gentlyos',
      content: skill.substring(0, 50000),
      content_length: skill.length,
      _ts: new Date().toISOString(),
    }));
    console.log(`  SKILL tom-collaboration-engine (${skill.length} chars)`);
  }

  // 6. Seed sephirot mapping
  store('put', 'artifacts', JSON.stringify({
    name: 'sephirot-mapping',
    type: 'mapping',
    project: 'gentlyos',
    content: JSON.stringify(tree.sephirot_mapping, null, 2),
    _ts: new Date().toISOString(),
  }));
  console.log('  MAP   sephirot → departments');

  // 7. Log the seed event
  store('log', 'seed', 'gentlyos', JSON.stringify({
    docs: docs.length,
    nodes: tree.nodes.length,
    artifacts: docs.length + 4,
  }));

  console.log('\nDone. Everything queryable via: node mongodb/store.js get <collection>');
}

seed().catch(e => { console.error(e); process.exit(1); });
