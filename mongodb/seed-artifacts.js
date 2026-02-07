#!/usr/bin/env node
// seed-artifacts.js — Batch-load all project artifacts into MongoDB
// Run: node mongodb/seed-artifacts.js
// Stores: skills, docs, jsx, idea-inventory, project metadata

const { MongoClient } = require('mongodb');
const fs = require('fs');
const path = require('path');
const os = require('os');

// ── .env loader ────────────────────────────────────────────────
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

// ── extract text from .docx (xml parse, no deps) ──────────────
function extractDocxText(docxPath) {
  try {
    const AdmZip = require('adm-zip'); // fallback if available
    const zip = new AdmZip(docxPath);
    const xml = zip.readAsText('word/document.xml');
    return xml.replace(/<[^>]+>/g, ' ').replace(/\s+/g, ' ').trim().slice(0, 50000);
  } catch {
    // If adm-zip not available, just store path reference
    return `[binary .docx — see ${docxPath}]`;
  }
}

// ── main ───────────────────────────────────────────────────────
async function main() {
  loadEnv();
  const uri = getUri();
  if (!uri) { console.error('No MongoDB URI'); process.exit(1); }

  const dbName = process.env.MONGODB_DB || 'claude_cage';
  const client = new MongoClient(uri, {
    serverSelectionTimeoutMS: 10000,
    connectTimeoutMS: 10000,
  });

  const ROOT = path.resolve(__dirname, '..');
  const now = new Date();
  const meta = { _ts: now, _host: os.hostname(), _seeded: true };

  // ── collect all artifacts ────────────────────────────────────
  const artifacts = [];

  // Text files
  const textFiles = [
    { path: '.claude/skills/tom-collab/SKILL.md', type: 'skill', project: 'tom-collab' },
    { path: '.claude/skills/tom-collab/DEPLOY.md', type: 'docs', project: 'tom-collab' },
    { path: '.claude/skills/tom-collab/references/idea-inventory.md', type: 'reference', project: 'tom-collab' },
    { path: '.claude/skills/atlas-cli/SKILL.md', type: 'skill', project: 'atlas-cli' },
    { path: '.claude/commands/atlas.md', type: 'command', project: 'atlas-cli' },
    { path: 'projects/gentlyos-workstation/GentlyWorkstation.jsx', type: 'code', project: 'gentlyos-workstation' },
    { path: 'mongodb/store.js', type: 'code', project: 'claude-cage' },
    { path: 'lib/mongodb.sh', type: 'code', project: 'claude-cage' },
    { path: 'CLAUDE.md', type: 'docs', project: 'claude-cage' },
  ];

  for (const f of textFiles) {
    const fullPath = path.join(ROOT, f.path);
    if (fs.existsSync(fullPath)) {
      const content = fs.readFileSync(fullPath, 'utf8');
      artifacts.push({
        name: path.basename(f.path),
        path: f.path,
        type: f.type,
        project: f.project,
        content: content.slice(0, 100000), // cap at 100KB
        size: content.length,
        ...meta,
      });
    }
  }

  // .docx files (store as metadata + truncated text)
  const docxFiles = [
    'projects/gentlyos-docs/Gently_Studio_Protocols.docx',
    'projects/gentlyos-docs/GentlyOS_Workspace_System.docx',
    'projects/gentlyos-docs/Google_Infrastructure_Research.docx',
  ];

  for (const f of docxFiles) {
    const fullPath = path.join(ROOT, f);
    if (fs.existsSync(fullPath)) {
      const stats = fs.statSync(fullPath);
      artifacts.push({
        name: path.basename(f),
        path: f,
        type: 'docx',
        project: 'gentlyos-docs',
        content: extractDocxText(fullPath),
        size: stats.size,
        ...meta,
      });
    }
  }

  // ── project metadata ─────────────────────────────────────────
  const projects = [
    { name: 'claude-cage', desc: 'Dockerized sandbox for Claude CLI & Desktop', status: 'active' },
    { name: 'tom-collab', desc: 'Tom Collaboration Engine skill', status: 'active' },
    { name: 'atlas-cli', desc: 'MongoDB Atlas CLI skill + /atlas command', status: 'active' },
    { name: 'gentlyos-workstation', desc: 'GentlyOS Workstation React UI prototype', status: 'prototype' },
    { name: 'gentlyos-docs', desc: 'Gently Studio specs, workspace system, security research', status: 'reference' },
    { name: 'headless-ubuntu-auto', desc: 'GPU server provisioning (2x RTX 3090)', status: 'active' },
    { name: 'Gently-nix', desc: 'NixOS provisioning ecosystem', status: 'active' },
  ];

  // ── insert ───────────────────────────────────────────────────
  try {
    await client.connect();
    const db = client.db(dbName);

    // Artifacts
    if (artifacts.length > 0) {
      const result = await db.collection('artifacts').insertMany(artifacts);
      console.log(`Inserted ${result.insertedCount} artifacts`);
    }

    // Projects
    for (const p of projects) {
      await db.collection('projects').updateOne(
        { name: p.name },
        { $set: { ...p, ...meta } },
        { upsert: true }
      );
    }
    console.log(`Upserted ${projects.length} projects`);

    // Seed event
    await db.collection('events').insertOne({
      type: 'seed',
      key: 'initial-load',
      value: {
        artifacts: artifacts.length,
        projects: projects.length,
        files: artifacts.map(a => a.path),
      },
      ...meta,
      _project: 'claude-cage',
    });
    console.log('Seed event logged');

    // Summary
    const counts = {};
    for (const col of ['artifacts', 'projects', 'events']) {
      counts[col] = await db.collection(col).countDocuments();
    }
    console.log('Collection counts:', JSON.stringify(counts));

  } catch (err) {
    console.error('MongoDB error:', err.message);
    process.exit(1);
  } finally {
    await client.close();
  }
}

main();
