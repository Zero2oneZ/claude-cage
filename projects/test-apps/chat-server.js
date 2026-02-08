#!/usr/bin/env node
// chat-server.js — WebSocket-less chat server using SSE + POST
// POST /send {user, message} — send message
// GET  /stream — SSE stream of all messages
// GET  /history — last 100 messages
// GET  /users — active users (posted in last 5 min)

const http = require('http');
const PORT = process.env.PORT || 3002;

const messages = [];
const clients = new Set();
const MAX_HISTORY = 100;

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

function broadcast(msg) {
  const data = `data: ${JSON.stringify(msg)}\n\n`;
  for (const client of clients) {
    client.write(data);
  }
}

const server = http.createServer(async (req, res) => {
  const url = new URL(req.url, `http://localhost:${PORT}`);

  if (url.pathname === '/send' && req.method === 'POST') {
    const raw = await body(req);
    try {
      const { user, message } = JSON.parse(raw);
      if (!user || !message) return json(res, 400, { error: 'need user and message' });
      const msg = { user, message, ts: Date.now(), id: messages.length };
      messages.push(msg);
      if (messages.length > MAX_HISTORY) messages.shift();
      broadcast(msg);
      return json(res, 200, msg);
    } catch {
      return json(res, 400, { error: 'invalid json' });
    }
  }

  if (url.pathname === '/stream' && req.method === 'GET') {
    res.writeHead(200, {
      'Content-Type': 'text/event-stream',
      'Cache-Control': 'no-cache',
      Connection: 'keep-alive',
    });
    res.write(`data: ${JSON.stringify({ type: 'connected', ts: Date.now() })}\n\n`);
    clients.add(res);
    req.on('close', () => clients.delete(res));
    return;
  }

  if (url.pathname === '/history' && req.method === 'GET') {
    return json(res, 200, { messages, count: messages.length });
  }

  if (url.pathname === '/users' && req.method === 'GET') {
    const cutoff = Date.now() - 5 * 60 * 1000;
    const active = [...new Set(messages.filter(m => m.ts > cutoff).map(m => m.user))];
    return json(res, 200, { users: active, count: active.length, listeners: clients.size });
  }

  json(res, 404, { routes: ['POST /send', 'GET /stream', 'GET /history', 'GET /users'] });
});

server.listen(PORT, () => console.log(`Chat server listening on :${PORT}`));
