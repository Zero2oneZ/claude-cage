#!/usr/bin/env node
// store.js — MongoDB consolidated store for claude-cage / GentlyOS
//
// Replaces: Supabase, Redis, Qdrant, FAISS, ClickHouse, SQLite, NATS
// Into:     One MongoDB Atlas cluster
//
// ── ORIGINAL (backward-compatible) ──────────────────────────────
//   node store.js put <collection> '<json>'
//   node store.js log <type> <key> ['<value_json>']
//   node store.js get <collection> ['<query_json>'] [limit]
//   node store.js search <collection> '<query_text>' [limit]
//   node store.js aggregate <collection> '<pipeline_json>'
//   node store.js bulk <collection> '<docs_array_json>'
//   node store.js distinct <collection> '<field>' ['<query_json>']
//   node store.js ping
//   node store.js count <collection> ['<query_json>']
//
// ── CACHE (replaces Redis) ──────────────────────────────────────
//   node store.js cache-set <key> '<value_json>' [ttl_seconds]
//   node store.js cache-get <key>
//   node store.js cache-del <key>
//
// ── QUEUE (replaces SQLite durable queues, Redis queues) ────────
//   node store.js queue-push <queue> '<payload_json>' [priority]
//   node store.js queue-pop <queue>
//   node store.js queue-ack <task_id>
//   node store.js queue-fail <task_id> ['<error_msg>']
//   node store.js queue-stats <queue>
//
// ── AGENTS (replaces fileseed pattern from ScatterBrainz) ───────
//   node store.js agent-register '<agent_json>'
//   node store.js agent-heartbeat <agent_id> ['<status_json>']
//   node store.js agent-list
//   node store.js agent-get <agent_id>
//   node store.js agent-deregister <agent_id>
//
// ── VECTORS (replaces Qdrant, FAISS, pgvector) ──────────────────
//   node store.js vector-upsert <collection> '<doc_json>'
//   node store.js vector-search <collection> '<embedding_json>' [limit]
//
// ── TOOLS (replaces pgvector tool registry) ─────────────────────
//   node store.js tool-register '<tool_json>'
//   node store.js tool-search '<query>' [limit]
//   node store.js tool-list [category]
//   node store.js tool-telemetry <tool_id> '<metrics_json>'
//
// ── FEED (replaces Supabase feed_posts + realtime) ──────────────
//   node store.js feed-post '<post_json>'
//   node store.js feed-get [limit] [before_id]
//   node store.js feed-boost <post_id> [amount]
//
// ── RLAIF (replaces JSONL files for RL from AI Feedback) ────────
//   node store.js rlaif-capture '<episode_json>'
//   node store.js rlaif-export [split_ratio]
//   node store.js rlaif-stats
//
// ── PROFILES (replaces Supabase profiles) ───────────────────────
//   node store.js profile-upsert '<profile_json>'
//   node store.js profile-get <user_id>
//
// ── ANALYTICS (replaces ClickHouse) ─────────────────────────────
//   node store.js analytics-inc <metric> [amount]
//   node store.js analytics-get <metric> [window_hours]
//   node store.js analytics-top [limit]
//
// ── WATCH (replaces NATS, Redis pub/sub, Supabase Realtime) ─────
//   node store.js watch <collection> ['<pipeline_json>']
//
// Env:
//   MONGODB_URI              — full connection string (preferred)
//   MONGODB_CLUSTER0_ADMIN   — fallback: auto-prepends mongodb+srv://
//   MONGODB_DB               — database name (default: claude_cage)
//   CAGE_PROJECT             — project tag (default: claude-cage)

const { MongoClient } = require('mongodb');
const fs = require('fs');
const path = require('path');
const os = require('os');

// ── .env loader (flat key=value, no deps) ──────────────────────
function loadEnv() {
  const envPath = path.join(__dirname, '.env');
  if (!fs.existsSync(envPath)) return;
  for (const raw of fs.readFileSync(envPath, 'utf8').split('\n')) {
    const line = raw.trim();
    if (!line || line.startsWith('#')) continue;
    const eq = line.indexOf('=');
    if (eq < 1) continue;
    const key = line.slice(0, eq).trim();
    let val = line.slice(eq + 1).trim();
    // strip surrounding quotes
    val = val.replace(/^["']|["']$/g, '');
    if (!process.env[key]) process.env[key] = val;
  }
}

// ── resolve connection URI ─────────────────────────────────────
function getUri() {
  if (process.env.MONGODB_URI) return process.env.MONGODB_URI;

  const admin = process.env.MONGODB_CLUSTER0_ADMIN;
  if (admin) {
    // strip leading *, trailing quotes/spaces
    let clean = admin.replace(/^["'*\s]+|["'\s]+$/g, '');
    if (clean.startsWith('mongodb')) return clean;

    // Parse user@pass@host or user:pass@host format
    const parts = clean.split('@');
    if (parts.length >= 3) {
      // user@pass@host... → URL-encode user and pass separately
      const user = encodeURIComponent(parts[0]);
      const pass = encodeURIComponent(parts.slice(1, -1).join('@'));
      const host = parts[parts.length - 1];
      return `mongodb+srv://${user}:${pass}@${host}`;
    }
    if (parts.length === 2) {
      const colonIdx = parts[0].indexOf(':');
      if (colonIdx > 0) {
        const user = encodeURIComponent(parts[0].slice(0, colonIdx));
        const pass = encodeURIComponent(parts[0].slice(colonIdx + 1));
        return `mongodb+srv://${user}:${pass}@${parts[1]}`;
      }
    }
    return `mongodb+srv://${clean}`;
  }
  return null;
}

// ── main ───────────────────────────────────────────────────────
async function main() {
  loadEnv();

  const uri = getUri();
  if (!uri) {
    process.stderr.write('ERR: MONGODB_URI not set\n');
    process.exit(1);
  }

  const dbName = process.env.MONGODB_DB || 'claude_cage';
  const client = new MongoClient(uri, {
    serverSelectionTimeoutMS: 5000,
    connectTimeoutMS: 5000,
    socketTimeoutMS: 10000,
  });

  const [,, cmd, ...args] = process.argv;

  try {
    await client.connect();
    const db = client.db(dbName);

    switch (cmd) {

      // ── put: insert one document into any collection ─────────
      case 'put': {
        const collection = args[0];
        if (!collection) throw new Error('put requires <collection>');
        let doc;
        if (args[1]) {
          doc = JSON.parse(args[1]);
        } else {
          // read from stdin
          doc = JSON.parse(fs.readFileSync(0, 'utf8'));
        }
        doc._ts = doc._ts || new Date();
        doc._host = os.hostname();
        const result = await db.collection(collection).insertOne(doc);
        process.stdout.write(JSON.stringify({ ok: 1, id: result.insertedId }) + '\n');
        break;
      }

      // ── log: structured event insert into 'events' ──────────
      case 'log': {
        const [type, key, valueJson] = args;
        if (!type || !key) throw new Error('log requires <type> <key>');
        const doc = {
          type,
          key,
          value: valueJson ? JSON.parse(valueJson) : {},
          _ts: new Date(),
          _host: os.hostname(),
          _project: process.env.CAGE_PROJECT || 'claude-cage',
        };
        const result = await db.collection('events').insertOne(doc);
        process.stdout.write(JSON.stringify({ ok: 1, id: result.insertedId }) + '\n');
        break;
      }

      // ── get: query documents ────────────────────────────────
      case 'get': {
        const collection = args[0];
        if (!collection) throw new Error('get requires <collection>');
        const query = args[1] ? JSON.parse(args[1]) : {};
        const limit = parseInt(args[2], 10) || 10;
        const docs = await db.collection(collection)
          .find(query)
          .sort({ _ts: -1 })
          .limit(limit)
          .toArray();
        process.stdout.write(JSON.stringify(docs) + '\n');
        break;
      }

      // ── ping: test connectivity ─────────────────────────────
      case 'ping': {
        const result = await db.command({ ping: 1 });
        const collections = await db.listCollections().toArray();
        process.stdout.write(JSON.stringify({
          ok: result.ok,
          db: dbName,
          collections: collections.map(c => c.name),
          host: uri.replace(/\/\/[^@]*@/, '//***@'), // mask credentials
        }) + '\n');
        break;
      }

      // ── count: count docs in a collection ───────────────────
      case 'count': {
        const collection = args[0];
        if (!collection) throw new Error('count requires <collection>');
        const query = args[1] ? JSON.parse(args[1]) : {};
        const n = await db.collection(collection).countDocuments(query);
        process.stdout.write(JSON.stringify({ collection, count: n }) + '\n');
        break;
      }

      // ── search: Atlas vector search ($vectorSearch) ───────
      // Requires a vector_index on the collection with embedding field
      // Falls back to text search ($text) if no vector index
      case 'search': {
        const collection = args[0];
        if (!collection) throw new Error('search requires <collection>');
        const queryText = args[1];
        if (!queryText) throw new Error('search requires <query_text>');
        const limit = parseInt(args[2], 10) || 5;

        // Try Atlas $search (full-text) first — works without vector index
        try {
          const pipeline = [
            {
              $search: {
                index: 'default',
                text: { query: queryText, path: { wildcard: '*' } },
              },
            },
            { $limit: limit },
            {
              $project: {
                _id: 0,
                name: 1, type: 1, project: 1, path: 1, key: 1,
                content: { $substrBytes: ['$content', 0, 500] },
                score: { $meta: 'searchScore' },
                _ts: 1,
              },
            },
          ];
          const docs = await db.collection(collection).aggregate(pipeline).toArray();
          process.stdout.write(JSON.stringify(docs) + '\n');
        } catch (searchErr) {
          // Fallback: regex search on content/name/key fields
          const regex = new RegExp(queryText.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'i');
          const docs = await db.collection(collection)
            .find({ $or: [
              { content: regex }, { name: regex }, { key: regex },
              { type: regex }, { 'value': regex },
            ]})
            .sort({ _ts: -1 })
            .limit(limit)
            .toArray();
          // Truncate content field for readability
          for (const doc of docs) {
            if (doc.content && doc.content.length > 500) {
              doc.content = doc.content.slice(0, 500) + '...';
            }
          }
          process.stdout.write(JSON.stringify(docs) + '\n');
        }
        break;
      }

      // ── aggregate: run custom aggregation pipeline ────────
      case 'aggregate': {
        const collection = args[0];
        if (!collection) throw new Error('aggregate requires <collection>');
        const pipelineJson = args[1];
        if (!pipelineJson) throw new Error('aggregate requires <pipeline_json>');
        const pipeline = JSON.parse(pipelineJson);
        if (!Array.isArray(pipeline)) throw new Error('pipeline must be a JSON array');
        const docs = await db.collection(collection).aggregate(pipeline).toArray();
        process.stdout.write(JSON.stringify(docs) + '\n');
        break;
      }

      // ── bulk: batch insert documents ──────────────────────
      case 'bulk': {
        const collection = args[0];
        if (!collection) throw new Error('bulk requires <collection>');
        let docs;
        if (args[1]) {
          docs = JSON.parse(args[1]);
        } else {
          docs = JSON.parse(fs.readFileSync(0, 'utf8'));
        }
        if (!Array.isArray(docs)) throw new Error('bulk requires a JSON array');
        const now = new Date();
        const host = os.hostname();
        for (const doc of docs) {
          doc._ts = doc._ts || now;
          doc._host = doc._host || host;
        }
        const result = await db.collection(collection).insertMany(docs);
        process.stdout.write(JSON.stringify({ ok: 1, inserted: result.insertedCount }) + '\n');
        break;
      }

      // ── distinct: get unique values for a field ───────────
      case 'distinct': {
        const collection = args[0];
        if (!collection) throw new Error('distinct requires <collection>');
        const field = args[1];
        if (!field) throw new Error('distinct requires <field>');
        const query = args[2] ? JSON.parse(args[2]) : {};
        const values = await db.collection(collection).distinct(field, query);
        process.stdout.write(JSON.stringify({ field, values, count: values.length }) + '\n');
        break;
      }

      // ── stats: collection statistics ──────────────────────
      case 'stats': {
        const collections = await db.listCollections().toArray();
        const stats = {};
        for (const col of collections) {
          const n = await db.collection(col.name).countDocuments();
          stats[col.name] = n;
        }
        process.stdout.write(JSON.stringify({ db: dbName, collections: stats }) + '\n');
        break;
      }

      // ═══════════════════════════════════════════════════════
      // CACHE — replaces Redis (TTL-based auto-expiry)
      // ═══════════════════════════════════════════════════════

      case 'cache-set': {
        const key = args[0];
        if (!key) throw new Error('cache-set requires <key>');
        const value = args[1] ? JSON.parse(args[1]) : JSON.parse(fs.readFileSync(0, 'utf8'));
        const ttl = parseInt(args[2], 10) || 3600; // default 1 hour
        const expiresAt = new Date(Date.now() + ttl * 1000);
        const result = await db.collection('cache').updateOne(
          { _key: key },
          { $set: { _key: key, value, expiresAt, _ts: new Date(), _host: os.hostname() } },
          { upsert: true }
        );
        process.stdout.write(JSON.stringify({ ok: 1, key, ttl, expiresAt }) + '\n');
        break;
      }

      case 'cache-get': {
        const key = args[0];
        if (!key) throw new Error('cache-get requires <key>');
        const doc = await db.collection('cache').findOne(
          { _key: key, expiresAt: { $gt: new Date() } }
        );
        if (doc) {
          process.stdout.write(JSON.stringify(doc.value) + '\n');
        } else {
          process.stdout.write('null\n');
        }
        break;
      }

      case 'cache-del': {
        const key = args[0];
        if (!key) throw new Error('cache-del requires <key>');
        const result = await db.collection('cache').deleteOne({ _key: key });
        process.stdout.write(JSON.stringify({ ok: 1, deleted: result.deletedCount }) + '\n');
        break;
      }

      // ═══════════════════════════════════════════════════════
      // QUEUE — replaces SQLite durable queues, Redis queues
      // Atomic pop via findOneAndUpdate (no double-processing)
      // ═══════════════════════════════════════════════════════

      case 'queue-push': {
        const queue = args[0];
        if (!queue) throw new Error('queue-push requires <queue>');
        const payload = args[1] ? JSON.parse(args[1]) : JSON.parse(fs.readFileSync(0, 'utf8'));
        const priority = parseInt(args[2], 10) || 0;
        const doc = {
          queue,
          payload,
          priority,
          status: 'pending',    // pending → processing → completed | failed
          attempts: 0,
          maxAttempts: 3,
          createdAt: new Date(),
          _ts: new Date(),
          _host: os.hostname(),
        };
        const result = await db.collection('tasks').insertOne(doc);
        process.stdout.write(JSON.stringify({ ok: 1, id: result.insertedId, queue }) + '\n');
        break;
      }

      case 'queue-pop': {
        const queue = args[0];
        if (!queue) throw new Error('queue-pop requires <queue>');
        // Atomic: find oldest pending, set to processing
        const doc = await db.collection('tasks').findOneAndUpdate(
          {
            queue,
            status: 'pending',
            $expr: { $lt: ['$attempts', '$maxAttempts'] },
          },
          {
            $set: { status: 'processing', startedAt: new Date() },
            $inc: { attempts: 1 },
          },
          { sort: { priority: -1, createdAt: 1 }, returnDocument: 'after' }
        );
        if (doc) {
          process.stdout.write(JSON.stringify(doc) + '\n');
        } else {
          process.stdout.write('null\n');
        }
        break;
      }

      case 'queue-ack': {
        const taskId = args[0];
        if (!taskId) throw new Error('queue-ack requires <task_id>');
        const { ObjectId } = require('mongodb');
        const result = await db.collection('tasks').updateOne(
          { _id: new ObjectId(taskId) },
          { $set: { status: 'completed', completedAt: new Date() } }
        );
        process.stdout.write(JSON.stringify({ ok: 1, modified: result.modifiedCount }) + '\n');
        break;
      }

      case 'queue-fail': {
        const taskId = args[0];
        if (!taskId) throw new Error('queue-fail requires <task_id>');
        const errorMsg = args[1] || 'unknown error';
        const { ObjectId } = require('mongodb');
        // If under max attempts, reset to pending for retry
        const task = await db.collection('tasks').findOne({ _id: new ObjectId(taskId) });
        const newStatus = task && task.attempts < task.maxAttempts ? 'pending' : 'failed';
        const result = await db.collection('tasks').updateOne(
          { _id: new ObjectId(taskId) },
          { $set: { status: newStatus, lastError: errorMsg, failedAt: new Date() } }
        );
        process.stdout.write(JSON.stringify({ ok: 1, status: newStatus }) + '\n');
        break;
      }

      case 'queue-stats': {
        const queue = args[0];
        if (!queue) throw new Error('queue-stats requires <queue>');
        const pipeline = [
          { $match: { queue } },
          { $group: { _id: '$status', count: { $sum: 1 } } },
        ];
        const results = await db.collection('tasks').aggregate(pipeline).toArray();
        const stats = { queue };
        for (const r of results) stats[r._id] = r.count;
        process.stdout.write(JSON.stringify(stats) + '\n');
        break;
      }

      // ═══════════════════════════════════════════════════════
      // AGENTS — replaces fileseed (ScatterBrainz pattern)
      // Agent registry with health monitoring
      // ═══════════════════════════════════════════════════════

      case 'agent-register': {
        const agentDoc = args[0] ? JSON.parse(args[0]) : JSON.parse(fs.readFileSync(0, 'utf8'));
        if (!agentDoc.id) throw new Error('agent requires id field');
        const now = new Date();
        const setFields = {
          ...agentDoc,
          status: agentDoc.status || 'active',
          lastHeartbeat: now,
          _ts: now,
          _host: os.hostname(),
        };
        const result = await db.collection('agents').updateOne(
          { id: agentDoc.id },
          {
            $set: setFields,
            $setOnInsert: { registeredAt: now, heartbeatCount: 0 },
          },
          { upsert: true }
        );
        process.stdout.write(JSON.stringify({ ok: 1, id: agentDoc.id, upserted: !!result.upsertedId }) + '\n');
        break;
      }

      case 'agent-heartbeat': {
        const agentId = args[0];
        if (!agentId) throw new Error('agent-heartbeat requires <agent_id>');
        const statusDoc = args[1] ? JSON.parse(args[1]) : {};
        const result = await db.collection('agents').updateOne(
          { id: agentId },
          {
            $set: {
              lastHeartbeat: new Date(),
              health: statusDoc,
              _ts: new Date(),
            },
            $inc: { heartbeatCount: 1 },
          }
        );
        if (result.matchedCount === 0) {
          process.stderr.write(`Agent ${agentId} not registered\n`);
          process.exit(1);
        }
        process.stdout.write(JSON.stringify({ ok: 1, id: agentId }) + '\n');
        break;
      }

      case 'agent-list': {
        const staleThreshold = new Date(Date.now() - 90000); // 90s = stale
        const agents = await db.collection('agents')
          .find({})
          .sort({ lastHeartbeat: -1 })
          .toArray();
        for (const a of agents) {
          a._healthy = a.lastHeartbeat > staleThreshold;
        }
        process.stdout.write(JSON.stringify(agents) + '\n');
        break;
      }

      case 'agent-get': {
        const agentId = args[0];
        if (!agentId) throw new Error('agent-get requires <agent_id>');
        const agent = await db.collection('agents').findOne({ id: agentId });
        process.stdout.write(JSON.stringify(agent) + '\n');
        break;
      }

      case 'agent-deregister': {
        const agentId = args[0];
        if (!agentId) throw new Error('agent-deregister requires <agent_id>');
        const result = await db.collection('agents').updateOne(
          { id: agentId },
          { $set: { status: 'deregistered', deregisteredAt: new Date() } }
        );
        process.stdout.write(JSON.stringify({ ok: 1, id: agentId }) + '\n');
        break;
      }

      // ═══════════════════════════════════════════════════════
      // VECTORS — replaces Qdrant, FAISS, pgvector
      // Uses Atlas $vectorSearch for kNN similarity
      // ═══════════════════════════════════════════════════════

      case 'vector-upsert': {
        const collection = args[0];
        if (!collection) throw new Error('vector-upsert requires <collection>');
        const doc = args[1] ? JSON.parse(args[1]) : JSON.parse(fs.readFileSync(0, 'utf8'));
        if (!doc.embedding || !Array.isArray(doc.embedding)) {
          throw new Error('doc must have embedding array');
        }
        doc._ts = new Date();
        doc._host = os.hostname();
        if (doc.doc_id) {
          await db.collection(collection).updateOne(
            { doc_id: doc.doc_id },
            { $set: doc },
            { upsert: true }
          );
        } else {
          await db.collection(collection).insertOne(doc);
        }
        process.stdout.write(JSON.stringify({ ok: 1, dims: doc.embedding.length }) + '\n');
        break;
      }

      case 'vector-search': {
        const collection = args[0];
        if (!collection) throw new Error('vector-search requires <collection>');
        const embedding = JSON.parse(args[1]);
        if (!Array.isArray(embedding)) throw new Error('embedding must be an array');
        const limit = parseInt(args[2], 10) || 5;
        try {
          // Atlas Vector Search
          const pipeline = [
            {
              $vectorSearch: {
                index: 'vector_index',
                path: 'embedding',
                queryVector: embedding,
                numCandidates: limit * 10,
                limit,
              },
            },
            {
              $project: {
                embedding: 0, // exclude large vector from results
                score: { $meta: 'vectorSearchScore' },
              },
            },
          ];
          const docs = await db.collection(collection).aggregate(pipeline).toArray();
          process.stdout.write(JSON.stringify(docs) + '\n');
        } catch (e) {
          // Fallback: brute-force cosine similarity (for dev/local)
          process.stderr.write(`Atlas vector search unavailable, using brute-force: ${e.message}\n`);
          const allDocs = await db.collection(collection)
            .find({ embedding: { $exists: true } })
            .limit(1000)
            .toArray();
          const scored = allDocs.map(doc => {
            let dot = 0, normA = 0, normB = 0;
            for (let i = 0; i < embedding.length; i++) {
              dot += embedding[i] * (doc.embedding[i] || 0);
              normA += embedding[i] * embedding[i];
              normB += (doc.embedding[i] || 0) * (doc.embedding[i] || 0);
            }
            const denom = Math.sqrt(normA) * Math.sqrt(normB);
            doc.score = denom > 0 ? dot / denom : 0;
            delete doc.embedding;
            return doc;
          });
          scored.sort((a, b) => b.score - a.score);
          process.stdout.write(JSON.stringify(scored.slice(0, limit)) + '\n');
        }
        break;
      }

      // ═══════════════════════════════════════════════════════
      // TOOLS — replaces pgvector tool registry (xZero2oneZx)
      // Tool discovery, registration, telemetry
      // ═══════════════════════════════════════════════════════

      case 'tool-register': {
        const tool = args[0] ? JSON.parse(args[0]) : JSON.parse(fs.readFileSync(0, 'utf8'));
        if (!tool.name) throw new Error('tool requires name field');
        const now = new Date();
        const setFields = {
          ...tool,
          status: tool.status || 'active',
          _ts: now,
          _host: os.hostname(),
        };
        const result = await db.collection('tools').updateOne(
          { name: tool.name },
          {
            $set: setFields,
            $setOnInsert: { registeredAt: now, lastUsed: null, useCount: 0, avgLatencyMs: 0 },
          },
          { upsert: true }
        );
        process.stdout.write(JSON.stringify({ ok: 1, name: tool.name, upserted: !!result.upsertedId }) + '\n');
        break;
      }

      case 'tool-search': {
        const query = args[0];
        if (!query) throw new Error('tool-search requires <query>');
        const limit = parseInt(args[1], 10) || 10;
        const regex = new RegExp(query.replace(/[.*+?^${}()|[\]\\]/g, '\\$&'), 'i');
        const docs = await db.collection('tools')
          .find({
            status: 'active',
            $or: [
              { name: regex }, { description: regex },
              { category: regex }, { tags: regex },
            ],
          })
          .sort({ useCount: -1, _ts: -1 })
          .limit(limit)
          .toArray();
        process.stdout.write(JSON.stringify(docs) + '\n');
        break;
      }

      case 'tool-list': {
        const category = args[0];
        const query = category ? { category, status: 'active' } : { status: 'active' };
        const docs = await db.collection('tools')
          .find(query)
          .sort({ category: 1, useCount: -1 })
          .toArray();
        process.stdout.write(JSON.stringify(docs) + '\n');
        break;
      }

      case 'tool-telemetry': {
        const toolId = args[0];
        if (!toolId) throw new Error('tool-telemetry requires <tool_id>');
        const metrics = args[1] ? JSON.parse(args[1]) : {};
        // Update tool usage stats
        await db.collection('tools').updateOne(
          { name: toolId },
          {
            $set: { lastUsed: new Date() },
            $inc: { useCount: 1 },
          }
        );
        // Log telemetry event
        const result = await db.collection('tool_telemetry').insertOne({
          tool: toolId,
          ...metrics,
          _ts: new Date(),
          _host: os.hostname(),
        });
        process.stdout.write(JSON.stringify({ ok: 1 }) + '\n');
        break;
      }

      // ═══════════════════════════════════════════════════════
      // FEED — replaces Supabase feed_posts + realtime
      // Living feed with charge/decay (matches gently-feed)
      // ═══════════════════════════════════════════════════════

      case 'feed-post': {
        const post = args[0] ? JSON.parse(args[0]) : JSON.parse(fs.readFileSync(0, 'utf8'));
        const doc = {
          ...post,
          charge: post.charge || 1.0,       // initial energy
          boosts: 0,
          createdAt: new Date(),
          _ts: new Date(),
          _host: os.hostname(),
          _project: process.env.CAGE_PROJECT || 'claude-cage',
        };
        const result = await db.collection('feed').insertOne(doc);
        process.stdout.write(JSON.stringify({ ok: 1, id: result.insertedId }) + '\n');
        break;
      }

      case 'feed-get': {
        const limit = parseInt(args[0], 10) || 20;
        const { ObjectId } = require('mongodb');
        const beforeId = args[1];
        const query = beforeId ? { _id: { $lt: new ObjectId(beforeId) } } : {};
        // Score = charge * decay(age) + boost_bonus
        // We approximate with a sort by charge * recency
        const docs = await db.collection('feed')
          .find(query)
          .sort({ createdAt: -1 })
          .limit(limit)
          .toArray();
        // Apply charge decay: charge * e^(-age_hours/24)
        const now = Date.now();
        for (const doc of docs) {
          const ageHours = (now - doc.createdAt.getTime()) / 3600000;
          doc._effectiveCharge = doc.charge * Math.exp(-ageHours / 24) + doc.boosts * 0.1;
        }
        docs.sort((a, b) => b._effectiveCharge - a._effectiveCharge);
        process.stdout.write(JSON.stringify(docs) + '\n');
        break;
      }

      case 'feed-boost': {
        const postId = args[0];
        if (!postId) throw new Error('feed-boost requires <post_id>');
        const amount = parseInt(args[1], 10) || 1;
        const { ObjectId } = require('mongodb');
        const result = await db.collection('feed').updateOne(
          { _id: new ObjectId(postId) },
          {
            $inc: { boosts: amount, charge: amount * 0.5 },
            $set: { _ts: new Date() },
          }
        );
        process.stdout.write(JSON.stringify({ ok: 1, modified: result.modifiedCount }) + '\n');
        break;
      }

      // ═══════════════════════════════════════════════════════
      // RLAIF — replaces JSONL files (project-stalker pattern)
      // Reinforcement Learning from AI Feedback episodes
      // ═══════════════════════════════════════════════════════

      case 'rlaif-capture': {
        const episode = args[0] ? JSON.parse(args[0]) : JSON.parse(fs.readFileSync(0, 'utf8'));
        const doc = {
          ...episode,
          capturedAt: new Date(),
          validated: !!(episode.validate && episode.validate.tests_passed),
          _ts: new Date(),
          _host: os.hostname(),
        };
        const result = await db.collection('rlaif_episodes').insertOne(doc);
        process.stdout.write(JSON.stringify({ ok: 1, id: result.insertedId }) + '\n');
        break;
      }

      case 'rlaif-export': {
        const splitRatio = parseFloat(args[0]) || 0.8;
        const episodes = await db.collection('rlaif_episodes')
          .find({ validated: true })
          .sort({ capturedAt: 1 })
          .toArray();
        const splitIdx = Math.floor(episodes.length * splitRatio);
        const train = episodes.slice(0, splitIdx);
        const validation = episodes.slice(splitIdx);
        // Write JSONL files
        const trainPath = path.join(os.tmpdir(), `rlaif_train_${Date.now()}.jsonl`);
        const valPath = path.join(os.tmpdir(), `rlaif_val_${Date.now()}.jsonl`);
        fs.writeFileSync(trainPath, train.map(e => JSON.stringify(e)).join('\n') + '\n');
        fs.writeFileSync(valPath, validation.map(e => JSON.stringify(e)).join('\n') + '\n');
        process.stdout.write(JSON.stringify({
          ok: 1,
          total: episodes.length,
          train: { count: train.length, path: trainPath },
          validation: { count: validation.length, path: valPath },
        }) + '\n');
        break;
      }

      case 'rlaif-stats': {
        const pipeline = [
          {
            $group: {
              _id: null,
              total: { $sum: 1 },
              validated: { $sum: { $cond: ['$validated', 1, 0] } },
              avgDuration: { $avg: '$validate.duration_ms' },
              providers: { $addToSet: '$model_provider' },
            },
          },
        ];
        const [stats] = await db.collection('rlaif_episodes').aggregate(pipeline).toArray();
        process.stdout.write(JSON.stringify(stats || { total: 0 }) + '\n');
        break;
      }

      // ═══════════════════════════════════════════════════════
      // PROFILES — replaces Supabase profiles table
      // ═══════════════════════════════════════════════════════

      case 'profile-upsert': {
        const profile = args[0] ? JSON.parse(args[0]) : JSON.parse(fs.readFileSync(0, 'utf8'));
        if (!profile.user_id) throw new Error('profile requires user_id');
        const result = await db.collection('profiles').updateOne(
          { user_id: profile.user_id },
          {
            $set: { ...profile, updatedAt: new Date(), _ts: new Date() },
            $setOnInsert: { createdAt: new Date() },
          },
          { upsert: true }
        );
        process.stdout.write(JSON.stringify({ ok: 1, user_id: profile.user_id }) + '\n');
        break;
      }

      case 'profile-get': {
        const userId = args[0];
        if (!userId) throw new Error('profile-get requires <user_id>');
        const profile = await db.collection('profiles').findOne({ user_id: userId });
        process.stdout.write(JSON.stringify(profile) + '\n');
        break;
      }

      // ═══════════════════════════════════════════════════════
      // ANALYTICS — replaces ClickHouse for metrics/counters
      // Time-bucketed counters with aggregation
      // ═══════════════════════════════════════════════════════

      case 'analytics-inc': {
        const metric = args[0];
        if (!metric) throw new Error('analytics-inc requires <metric>');
        const amount = parseInt(args[1], 10) || 1;
        const now = new Date();
        const hourBucket = new Date(now.getFullYear(), now.getMonth(), now.getDate(), now.getHours());
        const result = await db.collection('analytics').updateOne(
          { metric, bucket: hourBucket },
          {
            $inc: { value: amount },
            $set: { _ts: now },
            $setOnInsert: { metric, bucket: hourBucket, createdAt: now },
          },
          { upsert: true }
        );
        process.stdout.write(JSON.stringify({ ok: 1, metric, amount }) + '\n');
        break;
      }

      case 'analytics-get': {
        const metric = args[0];
        if (!metric) throw new Error('analytics-get requires <metric>');
        const windowHours = parseInt(args[1], 10) || 24;
        const since = new Date(Date.now() - windowHours * 3600000);
        const docs = await db.collection('analytics')
          .find({ metric, bucket: { $gte: since } })
          .sort({ bucket: 1 })
          .toArray();
        const total = docs.reduce((sum, d) => sum + d.value, 0);
        process.stdout.write(JSON.stringify({ metric, windowHours, total, buckets: docs }) + '\n');
        break;
      }

      case 'analytics-top': {
        const limit = parseInt(args[0], 10) || 10;
        const since = new Date(Date.now() - 24 * 3600000);
        const pipeline = [
          { $match: { bucket: { $gte: since } } },
          { $group: { _id: '$metric', total: { $sum: '$value' } } },
          { $sort: { total: -1 } },
          { $limit: limit },
        ];
        const docs = await db.collection('analytics').aggregate(pipeline).toArray();
        process.stdout.write(JSON.stringify(docs) + '\n');
        break;
      }

      // ═══════════════════════════════════════════════════════
      // WATCH — replaces NATS, Redis pub/sub, Supabase Realtime
      // MongoDB Change Streams (requires replica set / Atlas)
      // ═══════════════════════════════════════════════════════

      case 'watch': {
        const collection = args[0];
        if (!collection) throw new Error('watch requires <collection>');
        const pipeline = args[1] ? JSON.parse(args[1]) : [];
        const changeStream = db.collection(collection).watch(pipeline, {
          fullDocument: 'updateLookup',
        });
        process.stderr.write(`Watching ${collection} for changes (Ctrl+C to stop)...\n`);
        changeStream.on('change', (change) => {
          process.stdout.write(JSON.stringify({
            op: change.operationType,
            ns: `${change.ns.db}.${change.ns.coll}`,
            id: change.documentKey?._id,
            doc: change.fullDocument,
            ts: new Date(),
          }) + '\n');
        });
        changeStream.on('error', (err) => {
          process.stderr.write(`Watch error: ${err.message}\n`);
          process.exit(1);
        });
        // Keep alive until killed
        await new Promise(() => {});
        break;
      }

      default:
        process.stderr.write(`Unknown command: ${cmd}\n`);
        process.stderr.write([
          'ORIGINAL:  put, log, get, search, aggregate, bulk, distinct, stats, ping, count',
          'CACHE:     cache-set, cache-get, cache-del',
          'QUEUE:     queue-push, queue-pop, queue-ack, queue-fail, queue-stats',
          'AGENTS:    agent-register, agent-heartbeat, agent-list, agent-get, agent-deregister',
          'VECTORS:   vector-upsert, vector-search',
          'TOOLS:     tool-register, tool-search, tool-list, tool-telemetry',
          'FEED:      feed-post, feed-get, feed-boost',
          'RLAIF:     rlaif-capture, rlaif-export, rlaif-stats',
          'PROFILES:  profile-upsert, profile-get',
          'ANALYTICS: analytics-inc, analytics-get, analytics-top',
          'WATCH:     watch',
        ].join('\n') + '\n');
        process.exit(1);
    }
  } catch (err) {
    process.stderr.write(`MongoDB error: ${err.message}\n`);
    process.exit(1);
  } finally {
    await client.close();
  }
}

main();
