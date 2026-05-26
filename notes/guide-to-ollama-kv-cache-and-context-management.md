# Ollama KV Cache Mechanism & `/api/generate` Context Management Guide
## Document Version: V1.0
## Scope
This guide covers the built-in KV Cache mechanism of Ollama, standard usage of the `context` handle via the `/api/generate` endpoint, session reset rules, context window overload assessment, KV Cache hit ratio calculation, and methods to identify inference hardware. It applies to all models running on Ollama.

---

## 1. Ollama KV Cache Core Mechanism
### 1.1 Basic Overview
KV (Key-Value) Cache is a native inference acceleration feature of Ollama. It stores key-value tensors generated during model inference for historical tokens, avoiding repeated computation over full conversation history in subsequent requests. This effectively reduces computation overhead and inference latency.

### 1.2 Working Principle
1. **Initial Inference**
The model computes key-value tensors for all tokens in the input prompt. These KV data are cached, and a unique `context` handle (a numeric array) is returned to represent the snapshot of the current KV Cache.

2. **Cache Reuse & Prefix Matching**
When a new request carries the previous `context` handle and a continuously extended prompt, Ollama performs **longest common prefix matching**:
- Prefix matched: Reuse existing KV Cache directly; only compute tensors for newly added tokens (cache hit).
- Prefix mismatched: Invalidate the cache and recalculate all tokens from scratch (cache miss).

3. **Cache Lifecycle**
Each round of inference generates a brand-new `context` handle bound to the latest KV Cache snapshot. Old handles will become invalid gradually.

### 1.3 Behavior on Different Hardware
- **GPU Environment**: KV Cache is fully enabled by default. Cache data resides in video memory, delivering stable prefix matching and significant acceleration.
- **CPU Environment**: KV Cache still works with valid `context` handles, but cache is stored in system memory. The acceleration effect is weaker compared with GPU.

---

## 2. Why Prefer `/api/generate` Over `/api/chat`
### 2.1 Core Advantages
1. **Full Control Over KV Cache**
The `/api/generate` endpoint exposes the raw `context` handle, allowing complete manual management of the KV Cache lifecycle. Cache behavior is predictable and easy to debug. The `/api/chat` endpoint encapsulates conversation context and cache internally, hiding the `context` field and blocking manual intervention.

2. **Accurate Performance Metrics Collection**
Combined with token statistics and native response fields, `/api/generate` supports quantitative calculation of KV Cache hit ratio, which is essential for stress testing, performance tuning and online monitoring. Precise cache metrics cannot be obtained via `/api/chat`.

3. **Higher Compatibility & Stability**
On CPU, the cache capability of `/api/chat` is severely degraded. `/api/generate` maintains stable cache reuse via the `context` handle in both GPU and CPU scenarios. It also behaves consistently for streaming and non-streaming responses and long conversation sessions.

4. **Flexible Session Management**
You can freely create new sessions or clear conversation history according to business requirements, with isolated logic for different user sessions.

### 2.2 Interface Selection Suggestion
- Production deployment, performance testing, cache monitoring and long conversations: **Use `/api/generate` exclusively**.
- Local quick debugging and simple one-turn conversations: `/api/chat` is acceptable for temporary use.

---

## 3. Standard Usage of `context` Handle
### 3.1 Fundamental Rules
1. The `context` field is a numeric array returned by Ollama responses, acting as a handle for KV Cache snapshots. **Do not modify, splice, truncate or merge multiple historical `context` arrays manually**.
2. A new `context` is returned in every `/api/generate` response.
3. Conversation history (user inputs + model outputs) must be manually concatenated into the `prompt` by the caller. The `context` handle only binds to the underlying KV Cache.

### 3.2 Multi-turn Conversation Workflow (Streaming & Non-streaming Compatible)
#### 3.2.1 First Request (New Session, No History)
1. Send the request **without the `context` field**, and pass the initial prompt.
2. Receive the response containing model output, the first `context` handle and evaluation statistics.
3. Locally save the returned `context` array and the full conversation text.

**Sample Request**
```json
{
  "model": "your-model-name",
  "prompt": "Hello",
  "stream": false
}
```

**Key Response Fields**
```json
{
  "response": "Hello, how may I assist you?",
  "context": [114, 514, 888, ...],
  "prompt_eval_count": 4,
  "eval_count": 12
}
```

#### 3.2.2 Second & Subsequent Requests (Continue Current Session)
1. Concatenate the new prompt: combine **all historical conversation content + the new user question**. Model replies from previous turns must be included to ensure continuous text prefix.
2. Attach the **complete `context` array from the last response** to the new request without any changes.
3. Execute the request and obtain the new model output and a brand-new `context`.
4. Discard the old `context` and keep only the latest one for the next request.

**Sample Request for Second Turn**
```json
{
  "model": "your-model-name",
  "prompt": "Hello\nHello, how may I assist you?\nWhat is the weather today?",
  "context": [114, 514, 888, ...],
  "stream": false
}
```

#### 3.2.3 Forbidden Operations
- Do not accumulate, cut or customize the `context` array.
- Do not omit model outputs when concatenating the prompt (causes prefix breakage and total cache miss).
- Do not reuse one `context` handle across different user sessions (leads to conversation confusion and cache exceptions).

### 3.3 Reset Conversation & Clear Context
To start a brand-new session and discard all historical context and KV Cache:
> **Remove the `context` field from the next request entirely.**

Ollama will automatically abandon the old KV Cache and create a new empty cache snapshot. Meanwhile, clear the locally stored conversation history and use a fresh prompt.

**Sample Request for Session Reset**
```json
{
  "model": "your-model-name",
  "prompt": "Start a new conversation",
  "stream": false
}
```

---

## 4. Context Window Overload Assessment
All models running on Ollama have a fixed maximum context window. Exceeding this limit will cause text truncation, inference errors or abnormal outputs. Below is the standard detection and mitigation solution.

### 4.1 Token Statistics Endpoint
Use Ollama’s built-in tokenization endpoint to count the exact token number of input text:
- Endpoint: `POST /api/tokenize`
- Function: Returns the total token count based on the tokenizer of the loaded model.

**Sample Request**
```json
{
  "model": "your-model-name",
  "prompt": "Your full conversation text here"
}
```

**Sample Response**
```json
{
  "tokens": [123, 456, ...],
  "count": 256
}
```

### 4.2 Overload Judgment Logic
1. Record the **maximum context window size** of the target model in advance.
2. After concatenating the full prompt for each turn, call `/api/tokenize` to get the total token count.
3. Threshold rules:
   - Safe range: Current token count < 80% of maximum window (reserve space for model generation).
   - Warning range: 80% ≤ Current token count < Maximum window (trigger alert; suggest truncating early history).
   - Overload range: Current token count ≥ Maximum window (reject the request and reset or truncate the session).

### 4.3 Solutions for Overload
1. Minor overload: Adopt a sliding window strategy to retain only the latest rounds of conversation and remove early content.
2. Severe overload: Remove the `context` field to reset the entire session.
3. Automation: Embed token verification into the pre-request process for automatic risk control.

> Note: The `context` array cannot reflect context length directly. Token counting via `/api/tokenize` is the only official and reliable method.

---

## 5. KV Cache Hit Ratio Calculation
Based on manual `context` management with `/api/generate`, calculate cache performance with the following metrics and formulas.

### 5.1 Metric Definition
1. `TotalPromptTokens`: Total token count of the full prompt for the current request (obtained from `/api/tokenize`).
2. `prompt_eval_count`: Field returned in the response, representing the number of tokens that require re-computation (cache miss tokens).

### 5.2 Calculation Formulas
1. Number of cache hit tokens:
\[ HitTokens = TotalPromptTokens - prompt\_eval\_count \]

2. KV Cache Hit Ratio:
\[ CacheHitRate = \frac{HitTokens}{TotalPromptTokens} \times 100\% \]

3. Cache Miss Ratio:
\[ MissRate = \frac{prompt\_eval\_count}{TotalPromptTokens} \times 100\% \]

### 5.3 Usage Examples
1. First request (no cache):
Total tokens = 1024, `prompt_eval_count` = 1024
Hit Ratio = 0% → Total cache miss.

2. Normal multi-turn conversation (good prefix match):
Total tokens = 2048, `prompt_eval_count` = 64
Hit Ratio = 96.875% → Efficient cache hit.

3. Broken text prefix (cache failure):
Total tokens = 2048, `prompt_eval_count` = 1980
Hit Ratio = 3.32% → Cache almost invalid; check prompt concatenation and `context` delivery.

### 5.4 Monitoring Recommendations
1. Call `/api/tokenize` to get total tokens before each request.
2. Extract `prompt_eval_count` after response and calculate the hit ratio for monitoring reporting.
3. Alert rule: Investigate `context` delivery and prompt concatenation if the hit ratio stays below 30% continuously.

---

## 6. Identify Inference Hardware (GPU / CPU)
### 6.1 Key Conclusion
Standard Ollama HTTP endpoints (e.g. `/api/generate`, `/api/chat`) **do not provide any dedicated field to indicate whether inference runs on GPU or CPU**. Use the following methods for identification.

### 6.2 Available Solutions
#### Solution 1: Query Model Details (Recommended)
Call the model information endpoint: `GET /api/show?name=your-model-name`
- GPU inference: The response contains fields such as `gpu: true`, `VRAM`, `cuda`, `metal` or `rocm`.
- CPU inference: The response only shows `cpu` and `RAM` without video memory related information.

#### Solution 2: Indirect Judgment via Latency
Compare latency under the same model and prompt length:
- GPU inference: `prompt_eval_duration` and `eval_duration` are extremely low.
- CPU inference: The two duration values are several to dozens of times higher than GPU.

> Limitation: This is only qualitative judgment and not 100% accurate.

#### Solution 3: System-level Inspection (Server Local Operation)
- Linux: Run `nvidia-smi` to check if the Ollama process occupies video memory.
- Windows / macOS: Check GPU monitor for video memory and GPU load.

### 6.3 Supplement
If partial model layers run on GPU and others on CPU, `/api/show` will still mark it as GPU loaded. Ollama will automatically downgrade to CPU when video memory is insufficient, which can be detected via `/api/show`.

---

## 7. Quick Reference
1. **KV Cache**: Natively supported by Ollama; relies on the `context` handle and prefix matching for acceleration. GPU delivers better performance.
2. **Interface Selection**: Use `/api/generate` for cache control and metric statistics.
3. **`context` Usage**: Pass the latest `context` from the previous response as-is; never modify or splice it.
4. **Session Reset**: Omit the `context` field in the request to clear history and cache.
5. **Context Overload Check**: Count tokens via `/api/tokenize` and compare with the model’s maximum context window.
6. **Cache Hit Ratio**: Calculate with total prompt tokens and `prompt_eval_count`.
7. **Hardware Detection**: Use `/api/show` to confirm GPU/CPU usage; latency comparison for auxiliary judgment.