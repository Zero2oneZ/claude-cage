"""lora.py — LoRA training pipeline with stacking.

LoRAs grow on top of each other. Each one correct. Each one cumulative.
The tree topology defines the adapter hierarchy:

  base_adapter (all traces)
    ├── scale:executive (reasoning, routing, escalation)
    ├── scale:department (coordination, aggregation)
    └── scale:captain (execution, leaf work)
        ├── dept:security (security patterns)
        ├── dept:runtime (docker, containers)
        ├── dept:web (flask, dashboard)
        └── ...
            ├── capt:sandbox (sandbox-specific)
            ├── capt:docker (docker-specific)
            └── ...

Stack them: base + scale + department + captain = specialized agent.

Hardware target: 2x RTX 3090 (24GB each)
Technique: QLoRA (4-bit quantization + LoRA)
"""

import json
import os
import sys
from datetime import datetime, timezone
from pathlib import Path


CAGE_ROOT = os.environ.get("CAGE_ROOT", str(Path(__file__).parent.parent))


# ── LoRA Config Templates ─────────────────────────────────────


def base_config(model_name="mistralai/Mistral-7B-Instruct-v0.3", dataset_path=None):
    """Base LoRA config — trains on ALL PTC traces.

    This is the foundation. Every other adapter stacks on this.
    Learns: tree topology, PTC coordination, intent routing.
    """
    return {
        "name": "ptc-base",
        "description": "Base PTC adapter — universal tree coordination",
        "model_name": model_name,
        "dataset": dataset_path or "training/datasets/latest/alpaca.jsonl",
        "output_dir": "training/adapters/ptc-base",

        # QLoRA config for 2x RTX 3090
        "quantization": {
            "load_in_4bit": True,
            "bnb_4bit_compute_dtype": "float16",
            "bnb_4bit_quant_type": "nf4",
            "bnb_4bit_use_double_quant": True,
        },

        # LoRA hyperparameters
        "lora": {
            "r": 64,
            "lora_alpha": 128,
            "lora_dropout": 0.05,
            "target_modules": ["q_proj", "k_proj", "v_proj", "o_proj",
                              "gate_proj", "up_proj", "down_proj"],
            "bias": "none",
            "task_type": "CAUSAL_LM",
        },

        # Training
        "training": {
            "num_train_epochs": 3,
            "per_device_train_batch_size": 4,
            "gradient_accumulation_steps": 4,
            "learning_rate": 2e-4,
            "weight_decay": 0.01,
            "warmup_ratio": 0.03,
            "lr_scheduler_type": "cosine",
            "max_seq_length": 2048,
            "fp16": True,
            "logging_steps": 10,
            "save_strategy": "epoch",
            "evaluation_strategy": "epoch",
            "gradient_checkpointing": True,
        },

        # Hardware
        "hardware": {
            "devices": "0,1",
            "gpu_memory_utilization": 0.85,
            "note": "2x RTX 3090 24GB — QLoRA fits 7B model comfortably",
        },

        # Stacking
        "stacking": {
            "parent": None,
            "children": ["ptc-scale-executive", "ptc-scale-department", "ptc-scale-captain"],
            "level": 0,
        },
    }


def scale_config(scale, model_name="mistralai/Mistral-7B-Instruct-v0.3"):
    """Scale-specific LoRA — trained on traces filtered by node scale.

    executive: learns routing, escalation, integration
    department: learns coordination, aggregation, rule application
    captain: learns execution, file operations, leaf work
    """
    return {
        "name": f"ptc-scale-{scale}",
        "description": f"Scale adapter for {scale}-level operations",
        "model_name": model_name,
        "dataset": f"training/datasets/latest/by_scale/{scale}.jsonl",
        "output_dir": f"training/adapters/ptc-scale-{scale}",
        "base_adapter": "training/adapters/ptc-base",

        "quantization": {
            "load_in_4bit": True,
            "bnb_4bit_compute_dtype": "float16",
            "bnb_4bit_quant_type": "nf4",
            "bnb_4bit_use_double_quant": True,
        },

        "lora": {
            "r": 32,
            "lora_alpha": 64,
            "lora_dropout": 0.05,
            "target_modules": ["q_proj", "k_proj", "v_proj", "o_proj"],
            "bias": "none",
            "task_type": "CAUSAL_LM",
        },

        "training": {
            "num_train_epochs": 5,
            "per_device_train_batch_size": 4,
            "gradient_accumulation_steps": 2,
            "learning_rate": 1e-4,
            "max_seq_length": 2048,
            "fp16": True,
            "gradient_checkpointing": True,
        },

        "stacking": {
            "parent": "ptc-base",
            "children": [],  # Populated by department configs
            "level": 1,
        },
    }


def department_config(dept_id, model_name="mistralai/Mistral-7B-Instruct-v0.3"):
    """Department-specific LoRA — trained on traces for one department.

    Learns patterns specific to that department:
    - security: threat assessment, sandbox verification, rule enforcement
    - runtime: container lifecycle, image builds, compose orchestration
    - web: API design, dashboard rendering, endpoint patterns
    """
    safe_name = dept_id.replace(":", "_")
    return {
        "name": f"ptc-{safe_name}",
        "description": f"Department adapter for {dept_id}",
        "model_name": model_name,
        "dataset": f"training/datasets/latest/by_department/{safe_name}.jsonl",
        "output_dir": f"training/adapters/ptc-{safe_name}",
        "base_adapter": "training/adapters/ptc-base",
        "scale_adapter": "training/adapters/ptc-scale-department",

        "quantization": {
            "load_in_4bit": True,
            "bnb_4bit_compute_dtype": "float16",
            "bnb_4bit_quant_type": "nf4",
            "bnb_4bit_use_double_quant": True,
        },

        "lora": {
            "r": 16,
            "lora_alpha": 32,
            "lora_dropout": 0.05,
            "target_modules": ["q_proj", "v_proj"],
            "bias": "none",
            "task_type": "CAUSAL_LM",
        },

        "training": {
            "num_train_epochs": 5,
            "per_device_train_batch_size": 8,
            "gradient_accumulation_steps": 1,
            "learning_rate": 5e-5,
            "max_seq_length": 1024,
            "fp16": True,
            "gradient_checkpointing": True,
        },

        "stacking": {
            "parent": "ptc-scale-department",
            "children": [],  # Populated by captain configs
            "level": 2,
        },
    }


def captain_config(node_id, dept_id, model_name="mistralai/Mistral-7B-Instruct-v0.3"):
    """Captain-specific LoRA — trained on traces for one leaf worker.

    The most specialized adapter. Knows exactly how to:
    - Work with specific files
    - Apply specific rules
    - Execute specific operations
    """
    safe_name = node_id.replace(":", "_")
    safe_dept = dept_id.replace(":", "_")
    return {
        "name": f"ptc-{safe_name}",
        "description": f"Captain adapter for {node_id}",
        "model_name": model_name,
        "dataset": f"training/datasets/latest/by_node/{safe_name}.jsonl",
        "output_dir": f"training/adapters/ptc-{safe_name}",
        "base_adapter": "training/adapters/ptc-base",
        "scale_adapter": "training/adapters/ptc-scale-captain",
        "dept_adapter": f"training/adapters/ptc-{safe_dept}",

        "quantization": {
            "load_in_4bit": True,
            "bnb_4bit_compute_dtype": "float16",
            "bnb_4bit_quant_type": "nf4",
            "bnb_4bit_use_double_quant": True,
        },

        "lora": {
            "r": 8,
            "lora_alpha": 16,
            "lora_dropout": 0.1,
            "target_modules": ["q_proj", "v_proj"],
            "bias": "none",
            "task_type": "CAUSAL_LM",
        },

        "training": {
            "num_train_epochs": 10,
            "per_device_train_batch_size": 8,
            "gradient_accumulation_steps": 1,
            "learning_rate": 3e-5,
            "max_seq_length": 512,
            "fp16": True,
        },

        "stacking": {
            "parent": f"ptc-{safe_dept}",
            "children": [],
            "level": 3,
        },
    }


# ── Full pipeline: generate all configs from a tree ────────────


def generate_pipeline(tree_path, model_name="mistralai/Mistral-7B-Instruct-v0.3"):
    """Generate the full LoRA training pipeline from a tree.

    Reads the tree topology and creates:
    1. Base adapter config
    2. Scale adapter configs (executive, department, captain)
    3. Department adapter configs (one per department)
    4. Captain adapter configs (one per leaf)

    Returns the full pipeline as a dict.
    """
    from ptc.engine import load_tree, get_leaves

    nodes, meta, _ = load_tree(tree_path)
    tree_title = meta.get("title", "unknown")

    pipeline = {
        "_meta": {
            "tree": tree_title,
            "tree_path": tree_path,
            "model": model_name,
            "generated": datetime.now(timezone.utc).isoformat(),
            "hardware": "2x RTX 3090 24GB",
            "technique": "QLoRA (4-bit NF4 + LoRA)",
        },
        "adapters": {},
        "stacking_order": [],
        "training_order": [],
    }

    # Level 0: Base adapter
    base = base_config(model_name)
    pipeline["adapters"]["ptc-base"] = base
    pipeline["training_order"].append("ptc-base")

    # Level 1: Scale adapters
    scales = set()
    for node in nodes.values():
        scales.add(node["scale"])

    for s in sorted(scales):
        cfg = scale_config(s, model_name)
        pipeline["adapters"][cfg["name"]] = cfg
        pipeline["training_order"].append(cfg["name"])

    # Level 2: Department adapters
    depts = {nid: n for nid, n in nodes.items() if n["scale"] == "department"}
    for dept_id, dept_node in sorted(depts.items()):
        cfg = department_config(dept_id, model_name)
        pipeline["adapters"][cfg["name"]] = cfg
        pipeline["training_order"].append(cfg["name"])

    # Level 3: Captain adapters (leaves)
    leaves = get_leaves(nodes)
    for leaf_id in leaves:
        leaf = nodes[leaf_id]
        # Find parent department
        dept_id = leaf.get("parent", "")
        cfg = captain_config(leaf_id, dept_id, model_name)
        pipeline["adapters"][cfg["name"]] = cfg
        pipeline["training_order"].append(cfg["name"])

    # Build stacking order (how to compose for inference)
    # For each leaf, the stack is: base → scale → department → captain
    for leaf_id in leaves:
        leaf = nodes[leaf_id]
        dept_id = leaf.get("parent", "")
        safe_leaf = leaf_id.replace(":", "_")
        safe_dept = dept_id.replace(":", "_")

        stack = [
            "ptc-base",
            f"ptc-scale-{leaf['scale']}",
            f"ptc-{safe_dept}",
            f"ptc-{safe_leaf}",
        ]
        pipeline["stacking_order"].append({
            "node_id": leaf_id,
            "node_name": leaf["name"],
            "stack": stack,
            "description": f"Specialized agent for {leaf['name']}",
        })

    return pipeline


# ── Training script generation ─────────────────────────────────


def generate_train_script(config, output_path=None):
    """Generate a Python training script for one adapter.

    Uses transformers + peft + bitsandbytes for QLoRA.
    """
    script = f'''#!/usr/bin/env python3
"""Auto-generated LoRA training script for {config["name"]}
Generated: {datetime.now(timezone.utc).isoformat()}
"""

import torch
from datasets import load_dataset
from transformers import (
    AutoModelForCausalLM,
    AutoTokenizer,
    BitsAndBytesConfig,
    TrainingArguments,
)
from peft import LoraConfig, get_peft_model, prepare_model_for_kbit_training
from trl import SFTTrainer

# ── Config ──────────────────────────────────────────
MODEL_NAME = "{config["model_name"]}"
DATASET_PATH = "{config["dataset"]}"
OUTPUT_DIR = "{config["output_dir"]}"

# ── Quantization (QLoRA) ───────────────────────────
bnb_config = BitsAndBytesConfig(
    load_in_4bit={config["quantization"]["load_in_4bit"]},
    bnb_4bit_compute_dtype=torch.float16,
    bnb_4bit_quant_type="{config["quantization"]["bnb_4bit_quant_type"]}",
    bnb_4bit_use_double_quant={config["quantization"]["bnb_4bit_use_double_quant"]},
)

# ── Load model + tokenizer ─────────────────────────
model = AutoModelForCausalLM.from_pretrained(
    MODEL_NAME,
    quantization_config=bnb_config,
    device_map="auto",
    trust_remote_code=True,
)
model = prepare_model_for_kbit_training(model)

tokenizer = AutoTokenizer.from_pretrained(MODEL_NAME, trust_remote_code=True)
tokenizer.pad_token = tokenizer.eos_token
tokenizer.padding_side = "right"

# ── LoRA config ────────────────────────────────────
lora_config = LoraConfig(
    r={config["lora"]["r"]},
    lora_alpha={config["lora"]["lora_alpha"]},
    lora_dropout={config["lora"]["lora_dropout"]},
    target_modules={config["lora"]["target_modules"]},
    bias="{config["lora"]["bias"]}",
    task_type="{config["lora"]["task_type"]}",
)

model = get_peft_model(model, lora_config)
model.print_trainable_parameters()

# ── Dataset ────────────────────────────────────────
dataset = load_dataset("json", data_files=DATASET_PATH, split="train")

# ── Training ───────────────────────────────────────
training_args = TrainingArguments(
    output_dir=OUTPUT_DIR,
    num_train_epochs={config["training"]["num_train_epochs"]},
    per_device_train_batch_size={config["training"]["per_device_train_batch_size"]},
    gradient_accumulation_steps={config["training"].get("gradient_accumulation_steps", 1)},
    learning_rate={config["training"]["learning_rate"]},
    fp16={config["training"].get("fp16", True)},
    logging_steps={config["training"].get("logging_steps", 10)},
    save_strategy="{config["training"].get("save_strategy", "epoch")}",
    gradient_checkpointing={config["training"].get("gradient_checkpointing", False)},
    report_to="none",
)

trainer = SFTTrainer(
    model=model,
    train_dataset=dataset,
    tokenizer=tokenizer,
    args=training_args,
    max_seq_length={config["training"].get("max_seq_length", 2048)},
)

# ── Train ──────────────────────────────────────────
print(f"Training {{config['name']}}...")
print(f"  Model: {{MODEL_NAME}}")
print(f"  Dataset: {{DATASET_PATH}}")
print(f"  Output: {{OUTPUT_DIR}}")
trainer.train()

# ── Save ───────────────────────────────────────────
trainer.save_model(OUTPUT_DIR)
print(f"Adapter saved to {{OUTPUT_DIR}}")
'''

    if output_path:
        os.makedirs(os.path.dirname(output_path), exist_ok=True)
        with open(output_path, "w") as f:
            f.write(script)

    return script


# ── CLI ────────────────────────────────────────────────────────


def main():
    import argparse
    parser = argparse.ArgumentParser(description="PTC LoRA Training Pipeline")
    parser.add_argument("action", choices=["pipeline", "config", "script", "stack"],
                       help="pipeline: generate full pipeline, config: show adapter config, "
                            "script: generate training script, stack: show stacking order")
    parser.add_argument("--tree", default="tree.json", help="Path to tree.json")
    parser.add_argument("--model", default="mistralai/Mistral-7B-Instruct-v0.3",
                       help="Base model name")
    parser.add_argument("--adapter", help="Adapter name (for config/script)")
    parser.add_argument("--output", "-o", help="Output path")

    args = parser.parse_args()

    tree_path = args.tree
    if not os.path.exists(tree_path):
        tree_path = os.path.join(CAGE_ROOT, args.tree)

    if args.action == "pipeline":
        pipeline = generate_pipeline(tree_path, args.model)
        output = args.output or os.path.join(CAGE_ROOT, "training", "pipeline.json")
        os.makedirs(os.path.dirname(output), exist_ok=True)
        with open(output, "w") as f:
            json.dump(pipeline, f, indent=2)
        print(f"Pipeline generated: {output}")
        print(f"  Adapters: {len(pipeline['adapters'])}")
        print(f"  Training order:")
        for name in pipeline["training_order"]:
            adapter = pipeline["adapters"][name]
            level = adapter.get("stacking", {}).get("level", "?")
            print(f"    L{level}: {name}")

    elif args.action == "stack":
        pipeline = generate_pipeline(tree_path, args.model)
        print(f"STACKING ORDER ({len(pipeline['stacking_order'])} leaf agents)")
        print("=" * 70)
        for s in pipeline["stacking_order"]:
            print(f"\n  {s['node_name']} ({s['node_id']})")
            print(f"  {s['description']}")
            for i, adapter in enumerate(s["stack"]):
                prefix = "  └── " if i == len(s["stack"]) - 1 else "  ├── "
                print(f"  {prefix}L{i}: {adapter}")

    elif args.action == "config":
        if not args.adapter:
            print("Error: --adapter required", file=sys.stderr)
            sys.exit(1)
        pipeline = generate_pipeline(tree_path, args.model)
        cfg = pipeline["adapters"].get(args.adapter)
        if not cfg:
            print(f"Adapter '{args.adapter}' not found. Available:", file=sys.stderr)
            for name in pipeline["adapters"]:
                print(f"  {name}", file=sys.stderr)
            sys.exit(1)
        print(json.dumps(cfg, indent=2))

    elif args.action == "script":
        if not args.adapter:
            print("Error: --adapter required", file=sys.stderr)
            sys.exit(1)
        pipeline = generate_pipeline(tree_path, args.model)
        cfg = pipeline["adapters"].get(args.adapter)
        if not cfg:
            print(f"Adapter '{args.adapter}' not found.", file=sys.stderr)
            sys.exit(1)
        output = args.output or os.path.join(CAGE_ROOT, "training", "scripts",
                                              f"train_{args.adapter.replace('-', '_')}.py")
        script = generate_train_script(cfg, output)
        print(f"Training script: {output}")


if __name__ == "__main__":
    main()
