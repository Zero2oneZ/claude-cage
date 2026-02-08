#!/usr/bin/env node
// kv-store.js — In-memory key-value store with HTTP API
// GET /k/:key — read, PUT /k/:key {value} — write, DELETE /k/:key — delete
// GET /keys — list all, GET /stats — store stats

const http = require('http');
const store = new Map();
const PORT = process.env.PORT || 3001;

function json(res, code, data) {
  res.writeHead(code, { 'Content-Type': 'application/json' });
  res.end(JSON.stringify(data));
}

function body(req) {
  return new Promise(r => {
    let d = '';
    req.on('data', c => d += c);
    req.on('end', () => r(d));
  });
}

const server = http.createServer(async (req, res) => {
  const url = new URL(req.url, `http://localhost:${PORT}`);
  const parts = url.pathname.split('/').filter(Boolean);

  if (parts[0] === 'k' && parts[1]) {
    const key = decodeURIComponent(parts[1]);

    if (req.method === 'GET') {
      if (!store.has(key)) return json(res, 404, { error: 'not found' });
      return json(res, 200, { key, value: store.get(key) });
    }

    if (req.method === 'PUT') {
      const raw = await body(req);
      try {
        const { value } = JSON.parse(raw);
        store.set(key, value);
        return json(res, 200, { key, value, stored: true });
      } catch {
        return json(res, 400, { error: 'invalid json, expected {"value": ...}' });
      }
    }

    if (req.method === 'DELETE') {
      const existed = store.delete(key);
      return json(res, 200, { key, deleted: existed });
    }
  }

  if (parts[0] === 'keys' && req.method === 'GET') {
    return json(res, 200, { keys: [...store.keys()], count: store.size });
  }

  if (parts[0] === 'stats' && req.method === 'GET') {
    return json(res, 200, {
      keys: store.size,
      memory: process.memoryUsage().heapUsed,
      uptime: process.uptime() | 0,
    });
  }

  json(res, 404, { error: 'not found', routes: ['GET/PUT/DELETE /k/:key', 'GET /keys', 'GET /stats'] });
});

server.listen(PORT, () => console.log(`KV store listening on :${PORT}`));
