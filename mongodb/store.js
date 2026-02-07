#!/usr/bin/env node
// store.js — MongoDB fire-and-forget store for claude-cage
// Usage:
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

      default:
        process.stderr.write(`Unknown command: ${cmd}\n`);
        process.stderr.write('Commands: put, log, get, search, aggregate, bulk, distinct, stats, ping, count\n');
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
