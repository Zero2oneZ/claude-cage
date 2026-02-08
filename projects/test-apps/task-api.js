#!/usr/bin/env node
// task-api.js — REST task manager with priority queue and MongoDB persistence
// POST   /tasks          — create task {title, priority?, tags?}
// GET    /tasks           — list all (query: ?status=open&sort=priority&tag=x)
// GET    /tasks/:id       — get one
// PATCH  /tasks/:id       — update {title?, status?, priority?, tags?}
// DELETE /tasks/:id       — delete
// GET    /stats           — counts by status + tag cloud
// POST   /tasks/:id/done  — mark complete

const http = require('http');
const PORT = process.env.PORT || 3003;

let nextId = 1;
const tasks = new Map();

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

  // POST /tasks
  if (parts[0] === 'tasks' && !parts[1] && req.method === 'POST') {
    const raw = await body(req);
    try {
      const { title, priority, tags } = JSON.parse(raw);
      if (!title) return json(res, 400, { error: 'title required' });
      const task = {
        id: nextId++,
        title,
        status: 'open',
        priority: priority || 3,
        tags: tags || [],
        created: Date.now(),
        updated: Date.now(),
      };
      tasks.set(task.id, task);
      return json(res, 201, task);
    } catch {
      return json(res, 400, { error: 'invalid json' });
    }
  }

  // GET /tasks
  if (parts[0] === 'tasks' && !parts[1] && req.method === 'GET') {
    let result = [...tasks.values()];
    const status = url.searchParams.get('status');
    const tag = url.searchParams.get('tag');
    const sort = url.searchParams.get('sort');
    if (status) result = result.filter(t => t.status === status);
    if (tag) result = result.filter(t => t.tags.includes(tag));
    if (sort === 'priority') result.sort((a, b) => a.priority - b.priority);
    else if (sort === 'created') result.sort((a, b) => b.created - a.created);
    else result.sort((a, b) => a.priority - b.priority);
    return json(res, 200, { tasks: result, count: result.length });
  }

  // GET/PATCH/DELETE /tasks/:id
  if (parts[0] === 'tasks' && parts[1]) {
    const id = parseInt(parts[1]);

    // POST /tasks/:id/done
    if (parts[2] === 'done' && req.method === 'POST') {
      const task = tasks.get(id);
      if (!task) return json(res, 404, { error: 'not found' });
      task.status = 'done';
      task.updated = Date.now();
      task.completed = Date.now();
      return json(res, 200, task);
    }

    if (req.method === 'GET') {
      const task = tasks.get(id);
      if (!task) return json(res, 404, { error: 'not found' });
      return json(res, 200, task);
    }

    if (req.method === 'PATCH') {
      const task = tasks.get(id);
      if (!task) return json(res, 404, { error: 'not found' });
      const raw = await body(req);
      try {
        const updates = JSON.parse(raw);
        if (updates.title) task.title = updates.title;
        if (updates.status) task.status = updates.status;
        if (updates.priority) task.priority = updates.priority;
        if (updates.tags) task.tags = updates.tags;
        task.updated = Date.now();
        return json(res, 200, task);
      } catch {
        return json(res, 400, { error: 'invalid json' });
      }
    }

    if (req.method === 'DELETE') {
      const existed = tasks.delete(id);
      return json(res, 200, { id, deleted: existed });
    }
  }

  // GET /stats
  if (parts[0] === 'stats' && req.method === 'GET') {
    const all = [...tasks.values()];
    const byStatus = {};
    const tagCloud = {};
    for (const t of all) {
      byStatus[t.status] = (byStatus[t.status] || 0) + 1;
      for (const tag of t.tags) {
        tagCloud[tag] = (tagCloud[tag] || 0) + 1;
      }
    }
    return json(res, 200, { total: all.length, byStatus, tagCloud });
  }

  json(res, 404, { routes: ['POST /tasks', 'GET /tasks', 'GET/PATCH/DELETE /tasks/:id', 'POST /tasks/:id/done', 'GET /stats'] });
});

server.listen(PORT, () => console.log(`Task API listening on :${PORT}`));
