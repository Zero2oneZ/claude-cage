"""ptc/embeddings.py — Vector embeddings and semantic search.

Every artifact, trace, commit, and blueprint gets embedded for semantic navigation.
Uses MongoDB Atlas Vector Search ($vectorSearch) or local fallback.

Embedding model: all-MiniLM-L6-v2 (384 dims, runs locally on the 3090s).
Graceful degradation: if no model available, falls back to text search.
"""

import hashlib
import json
import os
import subprocess
import sys
from datetime import datetime, timezone

CAGE_ROOT = os.environ.get("CAGE_ROOT", os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# Lazy-loaded model
_model = None
_model_name = None


# ── Configuration ──────────────────────────────────────────────


def _load_config():
    """Load embedding config from environment or defaults."""
    return {
        "enabled": os.environ.get("EMBEDDING_ENABLED", "false").lower() in ("true", "1", "yes"),
        "model": os.environ.get("EMBEDDING_MODEL", "all-MiniLM-L6-v2"),
        "dim": int(os.environ.get("EMBEDDING_DIM", "384")),
        "api": os.environ.get("EMBEDDING_API", "local"),
    }


# ── Model Loading ──────────────────────────────────────────────


def _get_model():
    """Lazy-load the sentence-transformers model."""
    global _model, _model_name
    config = _load_config()

    if _model is not None and _model_name == config["model"]:
        return _model

    try:
        from sentence_transformers import SentenceTransformer
        _model = SentenceTransformer(config["model"])
        _model_name = config["model"]
        return _model
    except ImportError:
        return None


def embedding_available():
    """Check if embedding generation is possible."""
    config = _load_config()
    if not config["enabled"]:
        return False
    if config["api"] == "local":
        return _get_model() is not None
    return True


# ── Embedding Generation ──────────────────────────────────────


def embed_text(text):
    """Generate embedding vector for text.

    Args:
        text: input string (will be truncated to ~512 tokens)

    Returns:
        list[float]: embedding vector (384 dims for MiniLM), or None if unavailable
    """
    config = _load_config()
    if not config["enabled"]:
        return None

    # Truncate to reasonable length for embedding
    text = text[:2000]

    if config["api"] == "local":
        model = _get_model()
        if model is None:
            return None
        embedding = model.encode(text)
        return embedding.tolist()
    else:
        return None  # API backends can be added later


def embed_and_store(collection, doc_id, text, extra_fields=None):
    """Fire-and-forget: compute embedding and update MongoDB document.

    Runs in a background subprocess to never block the CLI.

    Args:
        collection: MongoDB collection name
        doc_id: document identifier (used to find and update)
        text: text to embed
        extra_fields: dict of additional fields to store alongside embedding
    """
    config = _load_config()
    if not config["enabled"]:
        return

    # Background the embedding computation
    try:
        cmd_data = json.dumps({
            "collection": collection,
            "doc_id": doc_id,
            "text": text[:2000],
            "extra": extra_fields or {},
        })
        subprocess.Popen(
            [
                "python3", "-c",
                f"from ptc.embeddings import _bg_embed_and_store; "
                f"_bg_embed_and_store({json.dumps(cmd_data)})"
            ],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
            env={**os.environ, "CAGE_ROOT": CAGE_ROOT, "PYTHONPATH": CAGE_ROOT},
        )
    except Exception:
        pass


def _bg_embed_and_store(cmd_data_json):
    """Background worker for embed_and_store(). Not called directly."""
    data = json.loads(cmd_data_json)
    embedding = embed_text(data["text"])
    if embedding is None:
        return

    store_js = os.path.join(CAGE_ROOT, "mongodb", "store.js")
    if not os.path.exists(store_js):
        return

    doc = json.dumps({
        "doc_id": data["doc_id"],
        "collection": data["collection"],
        "embedding": embedding,
        "embedded_at": datetime.now(timezone.utc).isoformat(),
        **data.get("extra", {}),
    })

    try:
        subprocess.Popen(
            ["node", store_js, "put", "embeddings", doc],
            stdout=subprocess.DEVNULL,
            stderr=subprocess.DEVNULL,
        )
    except Exception:
        pass


# ── Semantic Search ────────────────────────────────────────────


def semantic_search(collection, query, limit=5):
    """Search a collection using vector similarity.

    Uses MongoDB Atlas $vectorSearch if available,
    falls back to text search.

    Args:
        collection: MongoDB collection to search
        query: search query text
        limit: max results

    Returns:
        list[dict]: matching documents with similarity scores
    """
    config = _load_config()
    store_js = os.path.join(CAGE_ROOT, "mongodb", "store.js")
    if not os.path.exists(store_js):
        return []

    # Try vector search first
    if config["enabled"]:
        embedding = embed_text(query)
        if embedding:
            pipeline = json.dumps([
                {
                    "$vectorSearch": {
                        "index": "vector_index",
                        "path": "embedding",
                        "queryVector": embedding,
                        "numCandidates": limit * 10,
                        "limit": limit,
                    }
                },
                {
                    "$project": {
                        "_id": 0,
                        "name": 1,
                        "type": 1,
                        "doc_id": 1,
                        "content": {"$substrBytes": ["$content", 0, 500]},
                        "score": {"$meta": "vectorSearchScore"},
                    }
                }
            ])
            try:
                result = subprocess.run(
                    ["node", store_js, "aggregate", "embeddings", pipeline],
                    capture_output=True, text=True, timeout=15,
                )
                if result.returncode == 0 and result.stdout.strip():
                    return json.loads(result.stdout)
            except (subprocess.TimeoutExpired, json.JSONDecodeError):
                pass

    # Fallback: text search on the target collection
    try:
        result = subprocess.run(
            ["node", store_js, "search", collection, query, str(limit)],
            capture_output=True, text=True, timeout=15,
        )
        if result.returncode == 0 and result.stdout.strip():
            return json.loads(result.stdout)
    except (subprocess.TimeoutExpired, json.JSONDecodeError):
        pass

    return []


# ── Specialized Embedders ─────────────────────────────────────


def embed_trace(trace):
    """Embed a PTC execution trace for semantic search.

    Extracts meaningful text: intent, departments, leaf results, status.

    Args:
        trace: PTC trace dict

    Returns:
        str: doc_id used for storage
    """
    doc_id = trace.get("run_id", f"trace-{hashlib.md5(json.dumps(trace).encode()).hexdigest()[:8]}")

    # Extract searchable text from trace
    parts = [
        f"Intent: {trace.get('intent', '')}",
        f"Tree: {trace.get('tree_title', '')}",
        f"Tasks: {trace.get('tasks_decomposed', 0)} decomposed, {trace.get('tasks_completed', 0)} completed",
    ]

    for leaf in trace.get("leaf_results", []):
        parts.append(f"Node {leaf.get('node_name', '')}: {leaf.get('output', {}).get('plan', '')}")
        for f in leaf.get("output", {}).get("files", []):
            parts.append(f"File: {f}")

    text = "\n".join(parts)
    embed_and_store("embeddings", doc_id, text, {
        "source_type": "trace",
        "intent": trace.get("intent"),
        "tree": trace.get("tree_title"),
    })
    return doc_id


def embed_commit(sha, message, diff_summary=""):
    """Embed a git commit for semantic navigation.

    Args:
        sha: commit hash
        message: commit message
        diff_summary: summary of changes (files changed, etc.)

    Returns:
        str: doc_id
    """
    doc_id = f"commit-{sha[:12]}"
    text = f"Commit {sha[:8]}: {message}\n{diff_summary}"
    embed_and_store("embeddings", doc_id, text, {
        "source_type": "commit",
        "sha": sha,
        "message": message,
    })
    return doc_id


def embed_blueprint(blueprint):
    """Embed a blueprint for discovery and cache-checking.

    Args:
        blueprint: blueprint dict (universal node format)

    Returns:
        str: doc_id
    """
    doc_id = blueprint.get("id", "blueprint-unknown")

    # Extract rich text from blueprint structure
    parts = [f"Blueprint: {blueprint.get('name', '')}"]

    for artifact in blueprint.get("artifacts", []):
        content = artifact.get("content", {})
        if isinstance(content, dict):
            parts.append(f"What: {content.get('what', '')}")
            parts.append(f"Why: {content.get('why', '')}")
            how = content.get("how", {})
            if isinstance(how, dict):
                parts.append(f"Approach: {how.get('approach', '')}")

            for task in content.get("builder_tasks", []):
                parts.append(f"Task: {task.get('intent', '')}")
                for f in task.get("files", []):
                    parts.append(f"File: {f}")

            gui = content.get("gui_spec", {})
            if isinstance(gui, dict):
                for view in gui.get("views", []):
                    parts.append(f"View: {view}")
                for flow in gui.get("flows", []):
                    parts.append(f"Flow: {flow}")

    text = "\n".join(parts)
    embed_and_store("embeddings", doc_id, text, {
        "source_type": "blueprint",
        "blueprint_id": doc_id,
        "name": blueprint.get("name"),
    })
    return doc_id


# ── Similarity Search Helpers ─────────────────────────────────


def find_similar_traces(intent, limit=5):
    """Find traces with similar intents. Used for blueprint cache-checking."""
    return semantic_search("embeddings", f"Intent: {intent}", limit)


def find_related_commits(query, limit=5):
    """Semantic search over commit history."""
    return semantic_search("embeddings", f"Commit: {query}", limit)


def find_similar_blueprints(intent, limit=5):
    """Find blueprints similar to an intent. Cache-check for architect mode."""
    return semantic_search("embeddings", f"Blueprint: {intent}", limit)


# ── CLI Entry Point ────────────────────────────────────────────


def main():
    """CLI interface for embedding operations."""
    if len(sys.argv) < 2:
        print("Usage: python -m ptc.embeddings <command> [args]")
        print("Commands: status, search <query>, embed-traces, embed-all")
        sys.exit(1)

    command = sys.argv[1]

    if command == "status":
        config = _load_config()
        available = embedding_available()
        print(f"Embeddings enabled: {config['enabled']}")
        print(f"Model:              {config['model']}")
        print(f"Dimensions:         {config['dim']}")
        print(f"API:                {config['api']}")
        print(f"Available:          {available}")

    elif command == "search":
        if len(sys.argv) < 3:
            print("Usage: python -m ptc.embeddings search <query>")
            sys.exit(1)
        query = " ".join(sys.argv[2:])
        limit = 10
        results = semantic_search("embeddings", query, limit)
        if results:
            for r in results:
                score = r.get("score", "?")
                name = r.get("name", r.get("doc_id", "?"))
                stype = r.get("source_type", r.get("type", "?"))
                print(f"  [{score:.3f}] {stype}: {name}")
        else:
            print("  No results (embeddings may not be enabled or indexed)")

    elif command == "embed-traces":
        trace_dir = os.path.join(CAGE_ROOT, "training", "traces")
        if not os.path.isdir(trace_dir):
            print("No traces directory found")
            sys.exit(1)
        count = 0
        for f in sorted(os.listdir(trace_dir)):
            if f.endswith(".json"):
                with open(os.path.join(trace_dir, f)) as fh:
                    trace = json.load(fh)
                doc_id = embed_trace(trace)
                print(f"  Embedded: {f} -> {doc_id}")
                count += 1
        print(f"Embedded {count} traces")

    elif command == "embed-all":
        print("Embedding all artifacts...")
        # Embed traces
        trace_dir = os.path.join(CAGE_ROOT, "training", "traces")
        if os.path.isdir(trace_dir):
            for f in sorted(os.listdir(trace_dir)):
                if f.endswith(".json"):
                    with open(os.path.join(trace_dir, f)) as fh:
                        trace = json.load(fh)
                    embed_trace(trace)
                    print(f"  Trace: {f}")

        # Embed blueprints (from MongoDB, if any)
        print("  (Blueprint and commit embedding runs on next PTC cycle)")

    else:
        print(f"Unknown command: {command}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
