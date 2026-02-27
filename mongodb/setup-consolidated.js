#!/usr/bin/env node
/**
 * setup-consolidated.js — Create all collections, indexes, and vector search configs
 *
 * Consolidates: Supabase, Redis, Qdrant, FAISS, ClickHouse, SQLite, NATS → MongoDB Atlas
 *
 * Collections created:
 *   EXISTING: events, artifacts, projects, nodes, embeddings, blueprints
 *   CACHE:    cache (TTL index — auto-expires, replaces Redis)
 *   QUEUE:    tasks (durable queue — replaces SQLite/Redis queues)
 *   AGENTS:   agents (registry + health — replaces fileseed/ScatterBrainz)
 *   VECTORS:  embeddings, knowledge, tools (vector search — replaces Qdrant/FAISS/pgvector)
 *   FEED:     feed (living feed — replaces Supabase feed_posts)
 *   RLAIF:    rlaif_episodes (RL from AI Feedback — replaces JSONL files)
 *   PROFILES: profiles (user profiles — replaces Supabase profiles)
 *   ANALYTICS: analytics (time-bucketed metrics — replaces ClickHouse)
 *   TOOLS:    tools, tool_telemetry (tool registry — replaces pgvector tool_registry)
 *   MESSAGES: messages (chat — replaces Supabase messages)
 *   TREASURY: treasury (token economics cache)
 *
 * Run: node mongodb/setup-consolidated.js
 */

const { MongoClient } = require('mongodb');
const fs = require('fs');
const path = require('path');

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

// ── index definitions ──────────────────────────────────────────
const INDEXES = {
  // ── EXISTING (ensure they exist) ───────────────
  events: [
    { key: { type: 1, _ts: -1 }, name: 'type_ts' },
    { key: { key: 1 }, name: 'key' },
    { key: { _project: 1, _ts: -1 }, name: 'project_ts' },
  ],
  artifacts: [
    { key: { name: 1, project: 1 }, name: 'name_project' },
    { key: { type: 1 }, name: 'type' },
    { key: { project: 1 }, name: 'project' },
  ],
  projects: [
    { key: { name: 1 }, name: 'name', options: { unique: true } },
  ],
  nodes: [
    { key: { id: 1 }, name: 'id', options: { unique: true, sparse: true } },
    { key: { _project: 1 }, name: 'project' },
    { key: { 'metadata.department': 1 }, name: 'department' },
  ],
  embeddings: [
    { key: { doc_id: 1 }, name: 'doc_id', options: { unique: true, sparse: true } },
    { key: { source_type: 1 }, name: 'source_type' },
    { key: { embedded_at: -1 }, name: 'embedded_at' },
    { key: { blueprint_id: 1 }, name: 'blueprint_id', options: { sparse: true } },
  ],
  blueprints: [
    { key: { id: 1 }, name: 'id', options: { unique: true, sparse: true } },
    { key: { 'metadata.status': 1 }, name: 'status' },
  ],

  // ── CACHE (replaces Redis) ─────────────────────
  cache: [
    { key: { _key: 1 }, name: 'key', options: { unique: true } },
    { key: { expiresAt: 1 }, name: 'ttl', options: { expireAfterSeconds: 0 } }, // TTL index!
  ],

  // ── QUEUE (replaces SQLite/Redis durable queues) ──
  tasks: [
    { key: { queue: 1, status: 1, priority: -1, createdAt: 1 }, name: 'queue_pop' },
    { key: { queue: 1, status: 1 }, name: 'queue_status' },
    { key: { status: 1, completedAt: 1 }, name: 'completed_cleanup' },
  ],

  // ── AGENTS (replaces fileseed/ScatterBrainz) ──────
  agents: [
    { key: { id: 1 }, name: 'id', options: { unique: true } },
    { key: { status: 1, lastHeartbeat: -1 }, name: 'status_heartbeat' },
    { key: { department: 1 }, name: 'department' },
    { key: { 'sephira': 1 }, name: 'sephira' },
  ],

  // ── TOOLS (replaces pgvector tool_registry) ────────
  tools: [
    { key: { name: 1 }, name: 'name', options: { unique: true } },
    { key: { category: 1, status: 1 }, name: 'category_status' },
    { key: { status: 1, useCount: -1 }, name: 'popular' },
    { key: { tags: 1 }, name: 'tags' },
  ],
  tool_telemetry: [
    { key: { tool: 1, _ts: -1 }, name: 'tool_ts' },
    { key: { _ts: -1 }, name: 'ts' },
  ],

  // ── KNOWLEDGE (replaces pgvector knowledge graph) ──
  knowledge: [
    { key: { doc_id: 1 }, name: 'doc_id', options: { unique: true, sparse: true } },
    { key: { source: 1 }, name: 'source' },
    { key: { domain: 1 }, name: 'domain' },
    { key: { tags: 1 }, name: 'tags' },
  ],

  // ── FEED (replaces Supabase feed_posts) ────────────
  feed: [
    { key: { createdAt: -1 }, name: 'created_desc' },
    { key: { author: 1, createdAt: -1 }, name: 'author_feed' },
    { key: { charge: -1 }, name: 'charge' },
  ],

  // ── RLAIF (replaces JSONL files) ───────────────────
  rlaif_episodes: [
    { key: { capturedAt: -1 }, name: 'captured_desc' },
    { key: { validated: 1 }, name: 'validated' },
    { key: { model_provider: 1 }, name: 'provider' },
    { key: { repo_id: 1 }, name: 'repo' },
  ],

  // ── PROFILES (replaces Supabase profiles) ──────────
  profiles: [
    { key: { user_id: 1 }, name: 'user_id', options: { unique: true } },
    { key: { wallet: 1 }, name: 'wallet', options: { sparse: true } },
    { key: { email: 1 }, name: 'email', options: { sparse: true } },
  ],

  // ── MESSAGES (replaces Supabase messages) ──────────
  messages: [
    { key: { channel: 1, createdAt: -1 }, name: 'channel_ts' },
    { key: { sender: 1, createdAt: -1 }, name: 'sender_ts' },
    { key: { type: 1 }, name: 'type' },
  ],

  // ── ANALYTICS (replaces ClickHouse) ────────────────
  analytics: [
    { key: { metric: 1, bucket: 1 }, name: 'metric_bucket', options: { unique: true } },
    { key: { bucket: -1 }, name: 'bucket_desc' },
  ],

  // ── TREASURY (token economics state cache) ─────────
  treasury: [
    { key: { token: 1 }, name: 'token', options: { unique: true } },
    { key: { updatedAt: -1 }, name: 'updated' },
  ],

  // ── STRATEGIES (replaces FAISS strategy search) ────
  strategies: [
    { key: { name: 1 }, name: 'name', options: { unique: true } },
    { key: { type: 1 }, name: 'type' },
    { key: { tags: 1 }, name: 'tags' },
    { key: { performance: -1 }, name: 'performance' },
  ],
};

// ── Atlas Search index definitions (must be created via API) ──
const ATLAS_SEARCH_INDEXES = [
  {
    collection: 'embeddings',
    name: 'vector_index',
    type: 'vectorSearch',
    definition: {
      fields: [{
        type: 'vector',
        path: 'embedding',
        numDimensions: 384,
        similarity: 'cosine',
      }],
    },
  },
  {
    collection: 'knowledge',
    name: 'vector_index',
    type: 'vectorSearch',
    definition: {
      fields: [{
        type: 'vector',
        path: 'embedding',
        numDimensions: 384,
        similarity: 'cosine',
      }],
    },
  },
  {
    collection: 'tools',
    name: 'vector_index',
    type: 'vectorSearch',
    definition: {
      fields: [{
        type: 'vector',
        path: 'embedding',
        numDimensions: 384,
        similarity: 'cosine',
      }],
    },
  },
  {
    collection: 'strategies',
    name: 'vector_index',
    type: 'vectorSearch',
    definition: {
      fields: [{
        type: 'vector',
        path: 'embedding',
        numDimensions: 384,
        similarity: 'cosine',
      }],
    },
  },
  {
    collection: 'artifacts',
    name: 'default',
    type: 'search',
    definition: {
      mappings: {
        dynamic: true,
        fields: {
          name: { type: 'string', analyzer: 'lucene.standard' },
          type: { type: 'string', analyzer: 'lucene.keyword' },
          content: { type: 'string', analyzer: 'lucene.standard' },
        },
      },
    },
  },
];

// ── main ───────────────────────────────────────────────────────
async function setup() {
  loadEnv();
  const uri = getUri();
  if (!uri) {
    console.error('Error: MONGODB_URI or MONGODB_CLUSTER0_ADMIN required');
    process.exit(1);
  }

  const dbName = process.env.MONGODB_DB || 'claude_cage';
  const client = new MongoClient(uri, {
    serverSelectionTimeoutMS: 10000,
    connectTimeoutMS: 10000,
  });

  try {
    await client.connect();
    const db = client.db(dbName);
    console.log(`Connected to ${dbName}\n`);

    // ── Create collections ──────────────────────────
    const existing = (await db.listCollections().toArray()).map(c => c.name);
    let created = 0;
    for (const collName of Object.keys(INDEXES)) {
      if (!existing.includes(collName)) {
        await db.createCollection(collName);
        console.log(`  + Created collection: ${collName}`);
        created++;
      }
    }
    console.log(`\nCollections: ${created} created, ${Object.keys(INDEXES).length} total\n`);

    // ── Create indexes ──────────────────────────────
    let indexCount = 0;
    for (const [collName, indexes] of Object.entries(INDEXES)) {
      const coll = db.collection(collName);
      for (const idx of indexes) {
        try {
          await coll.createIndex(idx.key, {
            name: idx.name,
            ...(idx.options || {}),
          });
          indexCount++;
        } catch (e) {
          if (e.code === 85 || e.code === 86) {
            // Index already exists (possibly with different options) — skip
          } else {
            console.warn(`  ! ${collName}.${idx.name}: ${e.message}`);
          }
        }
      }
      console.log(`  ${collName}: ${indexes.length} indexes`);
    }
    console.log(`\nTotal indexes created/verified: ${indexCount}\n`);

    // ── Atlas Search indexes (print instructions) ───
    console.log('═══════════════════════════════════════════════════');
    console.log('ATLAS SEARCH INDEXES (create via Atlas UI or API):');
    console.log('═══════════════════════════════════════════════════\n');

    // Try to create via the driver (Atlas 7.0+ supports this)
    let searchCreated = 0;
    for (const idx of ATLAS_SEARCH_INDEXES) {
      try {
        const coll = db.collection(idx.collection);
        // Atlas driver method (MongoDB 7.0+)
        await coll.createSearchIndex({
          name: idx.name,
          type: idx.type,
          definition: idx.definition,
        });
        console.log(`  + ${idx.collection}/${idx.name} (${idx.type}) — created via driver`);
        searchCreated++;
      } catch (e) {
        // Expected to fail on older Atlas tiers
        console.log(`  ~ ${idx.collection}/${idx.name} (${idx.type}) — create manually:`);
        console.log(`    ${JSON.stringify(idx.definition)}\n`);
      }
    }

    if (searchCreated < ATLAS_SEARCH_INDEXES.length) {
      console.log('\nTo create vector search indexes manually:');
      console.log('  1. Atlas → Database → Browse Collections → [collection]');
      console.log('  2. Click "Search Indexes" → "Create Index"');
      console.log('  3. Use the JSON definitions printed above\n');
    }

    // ── Consolidation map ───────────────────────────
    console.log('═══════════════════════════════════════════════════');
    console.log('CONSOLIDATION MAP — What MongoDB Replaces:');
    console.log('═══════════════════════════════════════════════════');
    const map = [
      ['Redis cache',           'cache collection + TTL index (auto-expiry)'],
      ['Redis pub/sub',         'MongoDB Change Streams (watch command)'],
      ['Supabase PostgreSQL',   'profiles, messages, feed collections'],
      ['Supabase Realtime',     'MongoDB Change Streams'],
      ['Supabase RLS',          'Application-layer access control'],
      ['Qdrant vector DB',      'Atlas Vector Search ($vectorSearch)'],
      ['FAISS vector search',   'Atlas Vector Search + brute-force fallback'],
      ['pgvector (PostgreSQL)', 'Atlas Vector Search on tools/knowledge'],
      ['ClickHouse analytics',  'analytics collection + aggregation pipeline'],
      ['SQLite durable queues', 'tasks collection + atomic findOneAndUpdate'],
      ['NATS message bus',      'MongoDB Change Streams (watch command)'],
      ['JSONL files (RLAIF)',   'rlaif_episodes collection'],
      ['fileseed (ScatterBrainz)', 'agents collection + heartbeat'],
    ];
    for (const [from, to] of map) {
      console.log(`  ${from.padEnd(25)} → ${to}`);
    }

    // ── Final stats ─────────────────────────────────
    console.log('\n═══════════════════════════════════════════════════');
    console.log('FINAL COLLECTION STATS:');
    console.log('═══════════════════════════════════════════════════');
    const allCols = await db.listCollections().toArray();
    for (const col of allCols.sort((a, b) => a.name.localeCompare(b.name))) {
      const count = await db.collection(col.name).countDocuments();
      console.log(`  ${col.name.padEnd(20)} ${count} docs`);
    }
    console.log(`\n  Total collections: ${allCols.length}`);
    console.log('\nSetup complete.');

  } catch (err) {
    console.error('Setup error:', err.message);
    process.exit(1);
  } finally {
    await client.close();
  }
}

setup();
