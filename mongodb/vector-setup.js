#!/usr/bin/env node
/**
 * vector-setup.js — Create Atlas vector search indexes.
 *
 * One-time setup script. Creates vector indexes on the embeddings collection
 * for $vectorSearch support. Requires MongoDB Atlas M10+ or Atlas Search.
 *
 * Usage: node mongodb/vector-setup.js
 */

const { MongoClient } = require("mongodb");
require("dotenv").config({ path: __dirname + "/.env" });

const uri = process.env.MONGODB_URI || process.env.MONGODB_CLUSTER0_ADMIN;
const dbName = process.env.MONGODB_DB || "claude_cage";

if (!uri) {
  console.error("Error: MONGODB_URI or MONGODB_CLUSTER0_ADMIN required");
  process.exit(1);
}

async function setup() {
  const client = new MongoClient(uri);

  try {
    await client.connect();
    const db = client.db(dbName);
    console.log(`Connected to ${dbName}`);

    // Ensure embeddings collection exists
    const collections = await db.listCollections({ name: "embeddings" }).toArray();
    if (collections.length === 0) {
      await db.createCollection("embeddings");
      console.log("Created 'embeddings' collection");
    }

    // Create regular indexes for non-vector queries
    const embeddings = db.collection("embeddings");

    await embeddings.createIndex({ doc_id: 1 }, { unique: true, sparse: true });
    console.log("Created index: doc_id (unique)");

    await embeddings.createIndex({ source_type: 1 });
    console.log("Created index: source_type");

    await embeddings.createIndex({ embedded_at: -1 });
    console.log("Created index: embedded_at");

    await embeddings.createIndex({ "blueprint_id": 1 }, { sparse: true });
    console.log("Created index: blueprint_id");

    // Note: Atlas vector search indexes must be created via the Atlas UI
    // or the Atlas Admin API — they cannot be created via the driver.
    console.log("");
    console.log("Regular indexes created successfully.");
    console.log("");
    console.log("IMPORTANT: Vector search index must be created via Atlas UI:");
    console.log("  1. Go to Atlas → Database → Browse Collections → embeddings");
    console.log("  2. Click 'Search Indexes' → 'Create Index'");
    console.log("  3. Use this JSON definition:");
    console.log(JSON.stringify({
      "mappings": {
        "dynamic": true,
        "fields": {
          "embedding": {
            "dimensions": 384,
            "similarity": "cosine",
            "type": "knnVector"
          }
        }
      }
    }, null, 2));
    console.log("  4. Name the index: vector_index");
    console.log("");

    // Also create text search index on artifacts for fallback
    const artifacts = db.collection("artifacts");
    try {
      await artifacts.createIndex(
        { name: "text", type: "text", content: "text" },
        { name: "artifacts_text_search" }
      );
      console.log("Created text search index on artifacts");
    } catch (e) {
      if (e.code === 85 || e.code === 86) {
        console.log("Text search index already exists on artifacts");
      } else {
        console.log(`Note: text index on artifacts: ${e.message}`);
      }
    }

    // Create indexes on blueprints collection
    const blueprints = db.collection("blueprints");
    await blueprints.createIndex({ "id": 1 }, { unique: true, sparse: true });
    await blueprints.createIndex({ "metadata.status": 1 });
    await blueprints.createIndex({ "metadata.content_hash": 1 });
    console.log("Created indexes on blueprints collection");

    console.log("\nVector setup complete.");
  } catch (err) {
    console.error("Setup error:", err.message);
    process.exit(1);
  } finally {
    await client.close();
  }
}

setup();
