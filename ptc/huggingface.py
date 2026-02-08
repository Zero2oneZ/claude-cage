"""ptc/huggingface.py — Hugging Face Hub integration.

Full HF Hub operations: model download, embedding, chat, repo management, cache.
MCP tools handle discovery — this handles operations.

Config: HF_TOKEN, HF_ENABLED, HF_CACHE_DIR, HF_DEFAULT_EMBEDDING_MODEL, HF_INFERENCE_PROVIDER
Dependency: huggingface_hub>=1.0.0 (THE accepted Python dependency)
"""

import json
import os
import subprocess
import sys
from datetime import datetime, timezone

CAGE_ROOT = os.environ.get("CAGE_ROOT", os.path.dirname(os.path.dirname(os.path.abspath(__file__))))

# Lazy imports — huggingface_hub loaded on first use
_hf_api = None
_hf_available_cache = None


# ── Configuration ──────────────────────────────────────────────


def _load_config():
    """Load HF config from environment."""
    return {
        "enabled": os.environ.get("HF_ENABLED", "false").lower() in ("true", "1", "yes"),
        "token": os.environ.get("HF_TOKEN", ""),
        "cache_dir": os.environ.get("HF_CACHE_DIR", ""),
        "default_embedding_model": os.environ.get("HF_DEFAULT_EMBEDDING_MODEL", "sentence-transformers/all-MiniLM-L6-v2"),
        "inference_provider": os.environ.get("HF_INFERENCE_PROVIDER", "hf-inference"),
    }


def _mongo_log(event_type, key, value=None):
    """Fire-and-forget MongoDB event log."""
    store_js = os.path.join(CAGE_ROOT, "mongodb", "store.js")
    if not os.path.exists(store_js):
        return
    doc = json.dumps({
        "type": event_type, "key": key, "value": value,
        "_ts": datetime.now(timezone.utc).isoformat(), "_source": "huggingface",
    })
    try:
        subprocess.Popen(
            ["node", store_js, "log", event_type, key, doc],
            stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL,
        )
    except Exception:
        pass


def _get_api():
    """Get or create HfApi instance."""
    global _hf_api
    if _hf_api is None:
        try:
            from huggingface_hub import HfApi
            config = _load_config()
            _hf_api = HfApi(token=config["token"] or None)
        except ImportError:
            return None
    return _hf_api


def _get_client():
    """Get InferenceClient for API calls."""
    try:
        from huggingface_hub import InferenceClient
        config = _load_config()
        return InferenceClient(
            token=config["token"] or None,
            provider=config["inference_provider"],
        )
    except ImportError:
        return None


# ── Availability ───────────────────────────────────────────────


def _hf_available():
    """Check if HF Hub is available and configured."""
    global _hf_available_cache
    if _hf_available_cache is not None:
        return _hf_available_cache

    config = _load_config()
    if not config["enabled"]:
        _hf_available_cache = (False, "HF_ENABLED is not set")
        return _hf_available_cache

    try:
        import huggingface_hub  # noqa: F401
    except ImportError:
        _hf_available_cache = (False, "huggingface_hub not installed (pip install huggingface_hub)")
        return _hf_available_cache

    if not config["token"]:
        _hf_available_cache = (False, "HF_TOKEN not set")
        return _hf_available_cache

    _hf_available_cache = (True, "ok")
    return _hf_available_cache


def hf_init():
    """Validate HF setup, set ready state, log identity.

    Returns:
        dict: {ready, username, message}
    """
    available, msg = _hf_available()
    if not available:
        return {"ready": False, "username": None, "message": msg}

    api = _get_api()
    if not api:
        return {"ready": False, "username": None, "message": "Could not create HfApi"}

    try:
        info = api.whoami()
        username = info.get("name", "unknown")
        _mongo_log("hf:init", username)
        return {"ready": True, "username": username, "message": "ok"}
    except Exception as e:
        return {"ready": False, "username": None, "message": str(e)}


# ── Model & Dataset Operations ────────────────────────────────


def download_model(repo_id, files=None, revision=None):
    """Download model files from HF Hub.

    Args:
        repo_id: e.g., "meta-llama/Llama-2-7b"
        files: optional list of specific files
        revision: branch/tag/commit

    Returns:
        dict: {path, repo_id, files}
    """
    api = _get_api()
    if not api:
        return {"error": "HF Hub not available"}

    config = _load_config()
    try:
        from huggingface_hub import snapshot_download, hf_hub_download
        kwargs = {"repo_id": repo_id}
        if config["cache_dir"]:
            kwargs["cache_dir"] = config["cache_dir"]
        if revision:
            kwargs["revision"] = revision

        if files:
            paths = []
            for f in files:
                path = hf_hub_download(repo_id=repo_id, filename=f,
                                       revision=revision,
                                       cache_dir=config["cache_dir"] or None)
                paths.append(path)
            _mongo_log("hf:download", repo_id, json.dumps(files))
            return {"paths": paths, "repo_id": repo_id, "files": files}
        else:
            path = snapshot_download(**kwargs)
            _mongo_log("hf:download", repo_id, "full")
            return {"path": path, "repo_id": repo_id}
    except Exception as e:
        return {"error": str(e)}


def download_dataset(repo_id, files=None):
    """Download dataset from HF Hub.

    Args:
        repo_id: e.g., "squad" or "user/my-dataset"
        files: optional specific files

    Returns:
        dict: {path, repo_id}
    """
    api = _get_api()
    if not api:
        return {"error": "HF Hub not available"}

    config = _load_config()
    try:
        from huggingface_hub import snapshot_download
        path = snapshot_download(
            repo_id=repo_id, repo_type="dataset",
            cache_dir=config["cache_dir"] or None,
        )
        _mongo_log("hf:download-dataset", repo_id)
        return {"path": path, "repo_id": repo_id}
    except Exception as e:
        return {"error": str(e)}


# ── Inference API ──────────────────────────────────────────────


def embed_text(text, model=None):
    """Get embedding via HF Inference API.

    Args:
        text: text to embed
        model: model name (defaults to config)

    Returns:
        dict: {embedding, model, dim}
    """
    client = _get_client()
    if not client:
        return {"error": "HF Inference not available"}

    config = _load_config()
    model = model or config["default_embedding_model"]

    try:
        result = client.feature_extraction(text, model=model)
        embedding = result[0] if isinstance(result, list) and isinstance(result[0], list) else result
        if hasattr(embedding, "tolist"):
            embedding = embedding.tolist()
        return {"embedding": embedding, "model": model, "dim": len(embedding)}
    except Exception as e:
        return {"error": str(e)}


def embed_batch(texts, model=None):
    """Batch embedding via HF Inference API.

    Args:
        texts: list of strings
        model: model name

    Returns:
        dict: {embeddings, model, count}
    """
    results = []
    for text in texts:
        result = embed_text(text, model)
        if "error" in result:
            return result
        results.append(result["embedding"])

    config = _load_config()
    return {"embeddings": results, "model": model or config["default_embedding_model"], "count": len(results)}


def chat(messages, model="meta-llama/Llama-3.1-8B-Instruct", stream=False):
    """Chat completion via HF Inference API (OpenAI-compatible).

    Args:
        messages: list of {role, content} dicts
        model: model name
        stream: whether to stream response

    Returns:
        dict: {response, model, usage}
    """
    client = _get_client()
    if not client:
        return {"error": "HF Inference not available"}

    try:
        response = client.chat_completion(messages=messages, model=model, stream=stream)
        if stream:
            parts = []
            for chunk in response:
                delta = chunk.choices[0].delta.content
                if delta:
                    parts.append(delta)
            return {"response": "".join(parts), "model": model}
        else:
            content = response.choices[0].message.content
            usage = None
            if hasattr(response, "usage") and response.usage:
                usage = {
                    "prompt_tokens": response.usage.prompt_tokens,
                    "completion_tokens": response.usage.completion_tokens,
                }
            _mongo_log("hf:chat", model)
            return {"response": content, "model": model, "usage": usage}
    except Exception as e:
        return {"error": str(e)}


def generate(prompt, model="meta-llama/Llama-3.1-8B-Instruct", max_tokens=512):
    """Text generation via HF Inference API.

    Args:
        prompt: text prompt
        model: model name
        max_tokens: max new tokens

    Returns:
        dict: {text, model}
    """
    client = _get_client()
    if not client:
        return {"error": "HF Inference not available"}

    try:
        result = client.text_generation(prompt, model=model, max_new_tokens=max_tokens)
        _mongo_log("hf:generate", model)
        return {"text": result, "model": model}
    except Exception as e:
        return {"error": str(e)}


def classify(text, labels):
    """Zero-shot classification.

    Args:
        text: text to classify
        labels: list of candidate labels

    Returns:
        dict: {labels, scores}
    """
    client = _get_client()
    if not client:
        return {"error": "HF Inference not available"}

    try:
        result = client.zero_shot_classification(text, labels)
        return {"labels": result.labels, "scores": result.scores}
    except Exception as e:
        return {"error": str(e)}


def summarize(text, model="facebook/bart-large-cnn"):
    """Text summarization.

    Args:
        text: text to summarize
        model: model name

    Returns:
        dict: {summary, model}
    """
    client = _get_client()
    if not client:
        return {"error": "HF Inference not available"}

    try:
        result = client.summarization(text, model=model)
        return {"summary": result.summary_text, "model": model}
    except Exception as e:
        return {"error": str(e)}


# ── Repository Management ─────────────────────────────────────


def create_repo(name, repo_type="model", private=True):
    """Create a Hub repository.

    Args:
        name: repo name (e.g., "my-model")
        repo_type: "model", "dataset", or "space"
        private: whether repo is private

    Returns:
        dict: {url, repo_id}
    """
    api = _get_api()
    if not api:
        return {"error": "HF Hub not available"}

    try:
        result = api.create_repo(name, repo_type=repo_type, private=private)
        _mongo_log("hf:create-repo", str(result))
        return {"url": str(result), "repo_id": name}
    except Exception as e:
        return {"error": str(e)}


def upload_file(repo_id, local_path, repo_path, repo_type="model"):
    """Upload a single file to a Hub repo.

    Args:
        repo_id: target repo
        local_path: local file path
        repo_path: path within the repo
        repo_type: "model", "dataset", or "space"

    Returns:
        dict: {url, path}
    """
    api = _get_api()
    if not api:
        return {"error": "HF Hub not available"}

    try:
        result = api.upload_file(
            path_or_fileobj=local_path,
            path_in_repo=repo_path,
            repo_id=repo_id,
            repo_type=repo_type,
        )
        _mongo_log("hf:upload", f"{repo_id}/{repo_path}")
        return {"url": str(result), "path": repo_path}
    except Exception as e:
        return {"error": str(e)}


def upload_folder(repo_id, local_dir, repo_type="model"):
    """Upload an entire directory to a Hub repo (handles LFS).

    Args:
        repo_id: target repo
        local_dir: local directory
        repo_type: "model", "dataset", or "space"

    Returns:
        dict: {url, repo_id}
    """
    api = _get_api()
    if not api:
        return {"error": "HF Hub not available"}

    try:
        result = api.upload_folder(
            folder_path=local_dir,
            repo_id=repo_id,
            repo_type=repo_type,
        )
        _mongo_log("hf:upload-folder", repo_id)
        return {"url": str(result), "repo_id": repo_id}
    except Exception as e:
        return {"error": str(e)}


def repo_info(repo_id, repo_type="model"):
    """Get repository metadata.

    Args:
        repo_id: repo identifier
        repo_type: "model", "dataset", or "space"

    Returns:
        dict: repo metadata
    """
    api = _get_api()
    if not api:
        return {"error": "HF Hub not available"}

    try:
        if repo_type == "model":
            info = api.model_info(repo_id)
        elif repo_type == "dataset":
            info = api.dataset_info(repo_id)
        elif repo_type == "space":
            info = api.space_info(repo_id)
        else:
            return {"error": f"Unknown repo_type: {repo_type}"}

        return {
            "id": info.id,
            "author": info.author,
            "downloads": getattr(info, "downloads", None),
            "likes": getattr(info, "likes", None),
            "tags": getattr(info, "tags", []),
            "pipeline_tag": getattr(info, "pipeline_tag", None),
            "last_modified": str(getattr(info, "last_modified", "")),
        }
    except Exception as e:
        return {"error": str(e)}


# ── Search ─────────────────────────────────────────────────────


def list_models(query=None, author=None, task=None, limit=20):
    """Search models on HF Hub.

    Args:
        query: search term
        author: filter by author/org
        task: filter by task (e.g., "text-generation")
        limit: max results

    Returns:
        dict: {models, count}
    """
    api = _get_api()
    if not api:
        return {"error": "HF Hub not available"}

    try:
        kwargs = {"limit": limit}
        if query:
            kwargs["search"] = query
        if author:
            kwargs["author"] = author
        if task:
            kwargs["task"] = task

        models = list(api.list_models(**kwargs))
        results = []
        for m in models:
            results.append({
                "id": m.id,
                "downloads": getattr(m, "downloads", 0),
                "likes": getattr(m, "likes", 0),
                "pipeline_tag": getattr(m, "pipeline_tag", None),
                "last_modified": str(getattr(m, "last_modified", "")),
            })
        return {"models": results, "count": len(results)}
    except Exception as e:
        return {"error": str(e)}


def list_datasets(query=None, author=None, limit=20):
    """Search datasets on HF Hub.

    Args:
        query: search term
        author: filter by author/org
        limit: max results

    Returns:
        dict: {datasets, count}
    """
    api = _get_api()
    if not api:
        return {"error": "HF Hub not available"}

    try:
        kwargs = {"limit": limit}
        if query:
            kwargs["search"] = query
        if author:
            kwargs["author"] = author

        datasets = list(api.list_datasets(**kwargs))
        results = []
        for d in datasets:
            results.append({
                "id": d.id,
                "downloads": getattr(d, "downloads", 0),
                "likes": getattr(d, "likes", 0),
                "last_modified": str(getattr(d, "last_modified", "")),
            })
        return {"datasets": results, "count": len(results)}
    except Exception as e:
        return {"error": str(e)}


# ── Cache Management ──────────────────────────────────────────


def cache_status():
    """Get cached models/datasets and disk usage.

    Returns:
        dict: {repos, total_size}
    """
    try:
        from huggingface_hub import scan_cache_dir
        config = _load_config()
        cache_dir = config["cache_dir"] or None
        cache_info = scan_cache_dir(cache_dir)

        repos = []
        total_size = 0
        for repo in cache_info.repos:
            size = repo.size_on_disk
            total_size += size
            repos.append({
                "repo_id": repo.repo_id,
                "repo_type": repo.repo_type,
                "size_bytes": size,
                "size_mb": round(size / (1024 * 1024), 1),
                "revisions": len(repo.revisions),
            })

        return {
            "repos": repos,
            "total_size_bytes": total_size,
            "total_size_gb": round(total_size / (1024 ** 3), 2),
            "count": len(repos),
        }
    except ImportError:
        return {"error": "huggingface_hub not installed"}
    except Exception as e:
        return {"error": str(e)}


def cache_clean(older_than_days=30):
    """Clean old cache entries.

    Args:
        older_than_days: remove entries older than this

    Returns:
        dict: {cleaned, freed_bytes}
    """
    try:
        from huggingface_hub import scan_cache_dir
        config = _load_config()
        cache_info = scan_cache_dir(config["cache_dir"] or None)

        now = datetime.now(timezone.utc)
        to_delete = []
        freed = 0

        for repo in cache_info.repos:
            for revision in repo.revisions:
                age = (now - revision.last_modified).days if hasattr(revision, "last_modified") else 0
                if age > older_than_days:
                    to_delete.append(revision.commit_hash)
                    freed += revision.size_on_disk

        if to_delete:
            strategy = cache_info.delete_revisions(*to_delete)
            strategy.execute()

        _mongo_log("hf:cache-clean", str(len(to_delete)), str(freed))
        return {"cleaned": len(to_delete), "freed_bytes": freed, "freed_mb": round(freed / (1024 * 1024), 1)}
    except Exception as e:
        return {"error": str(e)}


# ── CLI Entry Point ────────────────────────────────────────────


def main():
    """CLI interface for Hugging Face operations."""
    if len(sys.argv) < 2:
        print("Usage: python -m ptc.huggingface <command> [args]")
        print("Commands: status, download <repo> [--files f1,f2],")
        print("          embed <text> [--model m], chat <message> [--model m],")
        print("          generate <prompt> [--model m], search <query>,")
        print("          datasets <query>, upload <repo> <path>,")
        print("          repo-create <name> [--type model|dataset|space],")
        print("          repo-info <repo>, cache, cache-clean [--days N]")
        sys.exit(1)

    command = sys.argv[1]

    if command == "status":
        result = hf_init()
        print(json.dumps(result, indent=2))
        if result["ready"]:
            cache = cache_status()
            if "error" not in cache:
                print(f"\nCache: {cache['count']} repos, {cache['total_size_gb']} GB")

    elif command == "download":
        if len(sys.argv) < 3:
            print("Usage: python -m ptc.huggingface download <repo> [--files f1,f2]", file=sys.stderr)
            sys.exit(1)
        repo_id = sys.argv[2]
        files = None
        revision = None
        for i, arg in enumerate(sys.argv[3:], 3):
            if arg == "--files" and i + 1 < len(sys.argv):
                files = sys.argv[i + 1].split(",")
            elif arg == "--revision" and i + 1 < len(sys.argv):
                revision = sys.argv[i + 1]
        result = download_model(repo_id, files, revision)
        print(json.dumps(result, indent=2))

    elif command == "embed":
        if len(sys.argv) < 3:
            print("Usage: python -m ptc.huggingface embed <text> [--model m]", file=sys.stderr)
            sys.exit(1)
        text = sys.argv[2]
        model = None
        for i, arg in enumerate(sys.argv[3:], 3):
            if arg == "--model" and i + 1 < len(sys.argv):
                model = sys.argv[i + 1]
        result = embed_text(text, model)
        if "error" in result:
            print(json.dumps(result, indent=2))
        else:
            print(f"Model: {result['model']}")
            print(f"Dim:   {result['dim']}")
            print(f"Vec:   [{result['embedding'][0]:.6f}, ..., {result['embedding'][-1]:.6f}]")

    elif command == "chat":
        if len(sys.argv) < 3:
            print("Usage: python -m ptc.huggingface chat <message> [--model m]", file=sys.stderr)
            sys.exit(1)
        message = sys.argv[2]
        model = "meta-llama/Llama-3.1-8B-Instruct"
        for i, arg in enumerate(sys.argv[3:], 3):
            if arg == "--model" and i + 1 < len(sys.argv):
                model = sys.argv[i + 1]
        messages = [{"role": "user", "content": message}]
        result = chat(messages, model)
        if "error" in result:
            print(json.dumps(result, indent=2))
        else:
            print(result["response"])

    elif command == "generate":
        if len(sys.argv) < 3:
            print("Usage: python -m ptc.huggingface generate <prompt> [--model m]", file=sys.stderr)
            sys.exit(1)
        prompt = sys.argv[2]
        model = "meta-llama/Llama-3.1-8B-Instruct"
        for i, arg in enumerate(sys.argv[3:], 3):
            if arg == "--model" and i + 1 < len(sys.argv):
                model = sys.argv[i + 1]
        result = generate(prompt, model)
        if "error" in result:
            print(json.dumps(result, indent=2))
        else:
            print(result["text"])

    elif command == "search":
        if len(sys.argv) < 3:
            print("Usage: python -m ptc.huggingface search <query>", file=sys.stderr)
            sys.exit(1)
        result = list_models(sys.argv[2])
        print(json.dumps(result, indent=2))

    elif command == "datasets":
        if len(sys.argv) < 3:
            print("Usage: python -m ptc.huggingface datasets <query>", file=sys.stderr)
            sys.exit(1)
        result = list_datasets(sys.argv[2])
        print(json.dumps(result, indent=2))

    elif command == "upload":
        if len(sys.argv) < 4:
            print("Usage: python -m ptc.huggingface upload <repo> <path>", file=sys.stderr)
            sys.exit(1)
        repo_id = sys.argv[2]
        path = sys.argv[3]
        if os.path.isdir(path):
            result = upload_folder(repo_id, path)
        else:
            repo_path = os.path.basename(path)
            result = upload_file(repo_id, path, repo_path)
        print(json.dumps(result, indent=2))

    elif command == "repo-create":
        if len(sys.argv) < 3:
            print("Usage: python -m ptc.huggingface repo-create <name> [--type model|dataset|space]", file=sys.stderr)
            sys.exit(1)
        name = sys.argv[2]
        repo_type = "model"
        for i, arg in enumerate(sys.argv[3:], 3):
            if arg == "--type" and i + 1 < len(sys.argv):
                repo_type = sys.argv[i + 1]
        result = create_repo(name, repo_type)
        print(json.dumps(result, indent=2))

    elif command == "repo-info":
        if len(sys.argv) < 3:
            print("Usage: python -m ptc.huggingface repo-info <repo>", file=sys.stderr)
            sys.exit(1)
        result = repo_info(sys.argv[2])
        print(json.dumps(result, indent=2))

    elif command == "cache":
        result = cache_status()
        print(json.dumps(result, indent=2))

    elif command == "cache-clean":
        days = 30
        for i, arg in enumerate(sys.argv[2:], 2):
            if arg == "--days" and i + 1 < len(sys.argv):
                days = int(sys.argv[i + 1])
        result = cache_clean(days)
        print(json.dumps(result, indent=2))

    else:
        print(f"Unknown command: {command}", file=sys.stderr)
        sys.exit(1)


if __name__ == "__main__":
    main()
