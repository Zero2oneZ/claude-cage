#!/bin/bash
# ═══════════════════════════════════════════════════════════════════════════════
# GENTLYOS API + QUEUE - paste in Zero2oneZ-DeathStar folder
# ═══════════════════════════════════════════════════════════════════════════════
set -e
echo "📡 Deploying API + Queue"

mkdir -p public/api public/queue api/queue

# ═══════════════════════════════════════════════════════════════════════════════
# QUEUE WEB FORM (paste from phone)
# ═══════════════════════════════════════════════════════════════════════════════
cat > public/queue/index.html << 'EOF'
<!DOCTYPE html>
<html><head>
<meta charset="UTF-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>GentlyOS Queue</title>
<style>
*{box-sizing:border-box}
body{font-family:monospace;background:#0a0a0a;color:#0f0;padding:20px;max-width:800px;margin:0 auto}
h1{color:#0ff;border-bottom:1px solid #0ff;padding-bottom:10px}
label{display:block;margin-top:15px;color:#0ff}
select,input,textarea{width:100%;padding:10px;margin-top:5px;background:#111;border:1px solid #0f0;color:#0f0;font-family:monospace;font-size:14px}
textarea{height:300px}
button{margin-top:20px;padding:15px;background:#0f0;color:#000;border:none;font-family:monospace;font-size:16px;font-weight:bold;cursor:pointer;width:100%}
button:hover{background:#0ff}
#output{margin-top:20px;padding:15px;background:#111;border:1px solid #0f0;white-space:pre-wrap;display:none;overflow-x:auto}
.tip{color:#888;font-size:12px;margin-top:5px}
</style>
</head><body>
<h1>📡 GentlyOS Queue</h1>
<p>Claude Desktop (phone) → paste here → Claude Code (Dell) picks up</p>

<form id="f">
<label>Type<select name="type">
<option value="research">📚 Research</option>
<option value="task">⚡ Task</option>
<option value="context">📋 Context</option>
<option value="idea">💡 Idea</option>
</select></label>

<label>Priority<select name="priority">
<option value="now">🔴 NOW</option>
<option value="soon" selected>🟡 SOON</option>
<option value="later">🟢 LATER</option>
</select></label>

<label>Title<input name="title" placeholder="Brief description" required></label>

<label>Content<textarea name="content" placeholder="Paste research/notes/markdown here..." required></textarea>
<div class="tip">Tip: Tell Claude Desktop "Format this as markdown for my dev environment" first</div></label>

<button type="submit">📤 GENERATE JSON</button>
</form>

<div id="output"></div>

<script>
document.getElementById('f').onsubmit=e=>{
e.preventDefault();
const d=new FormData(e.target);
const item={
id:Date.now().toString(36),
type:d.get('type'),
priority:d.get('priority'),
title:d.get('title'),
content:d.get('content'),
source:'claude-desktop',
created_at:new Date().toISOString(),
status:'pending'
};
const o=document.getElementById('output');
o.style.display='block';
o.textContent=JSON.stringify(item,null,2);
navigator.clipboard.writeText(JSON.stringify(item,null,2)).then(()=>alert('Copied!\\n\\nCommit to queue/pending.json'));
};
</script>
</body></html>
EOF

# Empty pending queue
cat > public/queue/pending.json << 'EOF'
{"queue":[],"updated":"2026-01-23T00:00:00Z"}
EOF

# ═══════════════════════════════════════════════════════════════════════════════
# API SPECS
# ═══════════════════════════════════════════════════════════════════════════════
cat > public/api/stack.json << 'EOF'
{
  "name": "GentlyOS",
  "proof": {"btc_block":933402,"hash":"de7b79b446e31bd487bc479eee1942ae116e07c60881a094f0fe3f9da3e13b2a"},
  "layers": {
    "user": "GOO GUI (2789 LOC) - SDF unified field",
    "query": "Alexandria (66361 LOC) - 5W + BBBCP",
    "storage": "BS-ARTISAN - toroidal foam",
    "execution": "SYNTH + Cold Exec"
  },
  "formulas": {
    "tokens_to_radius": "r = tokens / 2π",
    "convergence": "|survive| = |Ω| × 0.3^n",
    "smooth_min": "= softmax(temp)"
  }
}
EOF

cat > public/api/bs-artisan.json << 'EOF'
{
  "name": "BS-ARTISAN",
  "proof": {"btc":933402,"hash":"de7b79b446e31bd487bc479eee1942ae116e07c60881a094f0fe3f9da3e13b2a"},
  "concept": "Knowledge on toroidal surfaces. Similarity = foam traversal. No embeddings.",
  "structs": {
    "Torus": {"id":"Hash","major_radius":"f64 scope","minor_radius":"f64 tokens/2π","winding":"1-6","bs":"0-1"},
    "TorusPoint": {"theta":"f64","phi":"f64"},
    "Foam": {"tori":"HashMap","blends":"Vec<TorusBlend>","flux":"Vec<FluxLine>","genesis":"Hash"},
    "FluxLine": {"origin":"Hash","length":"tokens","threshold":"break point","result":"radius=tokens/2π"},
    "CullingZone": {"inward":"compress 70%","outward":"preserve 100%"}
  },
  "winding": {"1":"RAW","2":"STRUCTURED","3":"REFINED","4":"TESTED","5":"DOCUMENTED","6":"PRODUCTION"},
  "barf": "XOR distance + blend boost"
}
EOF

cat > public/api/alexandria.json << 'EOF'
{
  "name": "Alexandria",
  "loc": 66361,
  "5w": ["WHO","WHAT","WHERE","WHEN","WHY"],
  "tesseract": {"0":"WHAT1","1":"WHAT2","2":"WHEN","3":"WHY","4":"WHO","5":"WHERE","6":"Potential","7":"Negation"},
  "bbbcp": {"BONE":"constraint","CIRCLE":"impossible (70%)","BLOB":"search","PIN":"solution","BIZ":"goal→new BONE"},
  "convergence": "2.7% after 3, 0.24% after 5",
  "codie": ["START","STOP","WHILE","IF","ELSE","AND","OR","NOT","TRUE","FALSE","ASSIGN","COMPARE","READ","WRITE","MATH","USER"],
  "output": ["ANSWER","TABLE","CHAIN"]
}
EOF

cat > public/api/gently-goo.json << 'EOF'
{
  "name": "GOO",
  "loc": 2789,
  "insight": "G(x,y,t,θ) → pixels, attention, learning",
  "math": "smooth_min(k) = softmax(temp)",
  "modules": {"sense":687,"lib":262,"score":258,"specialist":260,"source":216,"claude":186,"field":171,"cascade":166,"attend":162,"render":160,"rhythm":139,"learn":122},
  "score": {"example":"eager TEXT hello near_user","tokens":20},
  "templates": ["TEXT_BUBBLE","CODE_BLOCK","LINE_CHART","BAR_CHART","PIE_CHART","TABLE","IMAGE_FRAME","FORM","CHOICE_GRID","THINKING","ERROR","MERMAID","CANVAS","TERMINAL","DIFF"]
}
EOF

cat > public/api/synth.json << 'EOF'
{
  "name": "SYNTH",
  "philosophy": "Utility meter, not store of value",
  "costs": {"freeze":1,"hydrate":0.1,"domain":5,"stream":0.01,"query":0.001,"contribute":-0.01},
  "dust_vacuum": "shitcoins → Jupiter → SYNTH",
  "whammy": "72 chains × grid, BTC rotation",
  "cold_exec": "vault refs only, keys never in context"
}
EOF

cat > public/api/index.json << 'EOF'
{
  "specs": ["/api/stack.json","/api/bs-artisan.json","/api/alexandria.json","/api/gently-goo.json","/api/synth.json"],
  "queue": {"form":"/queue/","pending":"/queue/pending.json"}
}
EOF

# ═══════════════════════════════════════════════════════════════════════════════
# VERCEL CONFIG
# ═══════════════════════════════════════════════════════════════════════════════
cat > vercel.json << 'EOF'
{
  "headers": [
    {"source":"/(.*).json","headers":[{"key":"Access-Control-Allow-Origin","value":"*"},{"key":"Cache-Control","value":"public, max-age=60"}]}
  ]
}
EOF

# ═══════════════════════════════════════════════════════════════════════════════
# GIT
# ═══════════════════════════════════════════════════════════════════════════════
git add -A
git commit -m "feat: API specs + Queue system

Specs: /api/stack.json /api/bs-artisan.json /api/alexandria.json /api/gently-goo.json /api/synth.json
Queue: /queue/ (web form) /queue/pending.json (poll)

Phone→Dell bridge ready"

git push

echo "✅ DEPLOYED"
echo ""
echo "SPECS: /api/stack.json"
echo "QUEUE: /queue/ (paste form)"
echo ""
echo "Workflow:"
echo "1. Phone: Claude Desktop formats → copy"
echo "2. Phone: YOUR_URL/queue/ → paste → generate JSON"  
echo "3. Phone: Commit JSON to queue/pending.json"
echo "4. Dell: git pull && cat queue/pending.json"
