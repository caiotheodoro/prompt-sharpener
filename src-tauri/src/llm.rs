use serde::Deserialize;
use serde_json::json;

#[derive(Deserialize)]
struct AnthropicResponse {
    content: Vec<ContentBlock>,
}

#[derive(Deserialize)]
struct ContentBlock {
    #[serde(rename = "type")]
    block_type: String,
    text: Option<String>,
}

#[derive(Deserialize)]
struct OpenAiResponse {
    output_text: Option<String>,
    output: Option<Vec<OpenAiOutput>>,
}

#[derive(Deserialize)]
struct OpenAiOutput {
    content: Option<Vec<OpenAiContent>>,
}

#[derive(Deserialize)]
struct OpenAiContent {
    text: Option<String>,
}

#[derive(Deserialize)]
struct ChatCompletionResponse {
    choices: Vec<ChatCompletionChoice>,
}

#[derive(Deserialize)]
struct ChatCompletionChoice {
    message: ChatCompletionMessage,
}

#[derive(Deserialize)]
struct ChatCompletionMessage {
    content: Option<String>,
}

#[derive(Deserialize)]
struct GeminiResponse {
    candidates: Vec<GeminiCandidate>,
}

#[derive(Deserialize)]
struct GeminiCandidate {
    content: Option<GeminiContent>,
}

#[derive(Deserialize)]
struct GeminiContent {
    parts: Vec<GeminiPart>,
}

#[derive(Deserialize)]
struct GeminiPart {
    text: Option<String>,
}

pub async fn call(
    text: &str,
    mode: &str,
    provider: &str,
    api_key: &str,
    model: &str,
) -> Result<String, String> {
    let client = reqwest::Client::new();
    let system = system_prompt(mode);

    match provider {
        "anthropic" => call_anthropic(&client, text, &system, api_key, model).await,
        "openai" => call_openai(&client, text, &system, api_key, model).await,
        "gemini" => call_gemini(&client, text, &system, api_key, model).await,
        "ollama" => call_ollama(&client, text, &system, model).await,
        _ => Err(format!("Unsupported provider: {provider}")),
    }
}

pub async fn check_ollama_status() -> Result<(), String> {
    let client = reqwest::Client::new();
    let resp = client
        .get("http://localhost:11434/api/tags")
        .send()
        .await
        .map_err(|e| format!("Ollama is not reachable: {e}"))?;

    if resp.status().is_success() {
        Ok(())
    } else {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        Err(format!("Ollama status check failed {status}: {body}"))
    }
}

async fn call_anthropic(
    client: &reqwest::Client,
    text: &str,
    system: &str,
    api_key: &str,
    model: &str,
) -> Result<String, String> {
    let body = json!({
        "model": model,
        "max_tokens": 1024,
        "system": system,
        "messages": [{ "role": "user", "content": text }]
    });

    let resp = client
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", api_key)
        .header("anthropic-version", "2023-06-01")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Anthropic request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Anthropic API error {status}: {body}"));
    }

    let data: AnthropicResponse = resp
        .json()
        .await
        .map_err(|e| format!("Anthropic parse error: {e}"))?;

    data.content
        .into_iter()
        .find(|b| b.block_type == "text")
        .and_then(|b| b.text)
        .ok_or_else(|| "No text in Anthropic response".to_string())
}

async fn call_openai(
    client: &reqwest::Client,
    text: &str,
    system: &str,
    api_key: &str,
    model: &str,
) -> Result<String, String> {
    let body = json!({
        "model": model,
        "instructions": system,
        "input": text,
        "reasoning": { "effort": "low" },
        "max_output_tokens": 1024
    });

    let resp = client
        .post("https://api.openai.com/v1/responses")
        .bearer_auth(api_key)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("OpenAI request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("OpenAI API error {status}: {body}"));
    }

    let data: OpenAiResponse = resp
        .json()
        .await
        .map_err(|e| format!("OpenAI parse error: {e}"))?;

    if let Some(text) = data.output_text.filter(|text| !text.trim().is_empty()) {
        return Ok(text);
    }

    data.output
        .into_iter()
        .flatten()
        .filter_map(|output| output.content)
        .flatten()
        .filter_map(|content| content.text)
        .find(|text| !text.trim().is_empty())
        .ok_or_else(|| "No text in OpenAI response".to_string())
}

async fn call_gemini(
    client: &reqwest::Client,
    text: &str,
    system: &str,
    api_key: &str,
    model: &str,
) -> Result<String, String> {
    let body = json!({
        "systemInstruction": {
            "parts": [{ "text": system }]
        },
        "contents": [{
            "role": "user",
            "parts": [{ "text": text }]
        }],
        "generationConfig": {
            "maxOutputTokens": 1024
        }
    });
    let url =
        format!("https://generativelanguage.googleapis.com/v1beta/models/{model}:generateContent");

    let resp = client
        .post(url)
        .header("x-goog-api-key", api_key)
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Gemini request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Gemini API error {status}: {body}"));
    }

    let data: GeminiResponse = resp
        .json()
        .await
        .map_err(|e| format!("Gemini parse error: {e}"))?;

    data.candidates
        .into_iter()
        .filter_map(|candidate| candidate.content)
        .flat_map(|content| content.parts)
        .filter_map(|part| part.text)
        .find(|text| !text.trim().is_empty())
        .ok_or_else(|| "No text in Gemini response".to_string())
}

async fn call_ollama(
    client: &reqwest::Client,
    text: &str,
    system: &str,
    model: &str,
) -> Result<String, String> {
    let body = json!({
        "model": model,
        "messages": [
            { "role": "system", "content": system },
            { "role": "user", "content": text }
        ],
        "stream": false,
        "reasoning": { "effort": "none" },
        "temperature": 0.2,
        "max_tokens": 1024
    });

    let resp = client
        .post("http://localhost:11434/v1/chat/completions")
        .header("content-type", "application/json")
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Ollama request failed: {e}"))?;

    if !resp.status().is_success() {
        let status = resp.status();
        let body = resp.text().await.unwrap_or_default();
        return Err(format!("Ollama API error {status}: {body}"));
    }

    let data: ChatCompletionResponse = resp
        .json()
        .await
        .map_err(|e| format!("Ollama parse error: {e}"))?;

    data.choices
        .into_iter()
        .filter_map(|choice| choice.message.content)
        .find(|text| !text.trim().is_empty())
        .ok_or_else(|| "No text in Ollama response".to_string())
}

fn system_prompt(mode: &str) -> String {
    let persona = "You are a senior prompt engineer specializing in coding AI assistants \
        (Claude Code, Cursor, GitHub Copilot, Codex). You understand exactly how these models \
        parse developer intent and where vague prompts produce wrong output.";

    let rules = "Process:
1. Assess the prompt for missing task, constraints, output format, context, examples, and edge cases.
2. Rewrite it as precise instruction/spec language using concrete verbs and no filler.
3. Validate that every gap you identified is addressed without inventing facts.

Rules:
1. Return ONLY the improved prompt. No explanation, no preamble, no quotes.
2. Preserve the user's original intent exactly.
3. Use imperative voice (\"Refactor X\", not \"Could you refactor X\").
4. Plain text only — no markdown, no bullet points, no code blocks.
5. Never wrap output in quotes.
6. Preserve every pasted URL, file path, command, code symbol, citation, dataset name, paper title, and line reference exactly as written.
7. If the user pasted a link or path as the object of the task, keep it in the improved prompt and build the task around it; never summarize it away.
8. Keep the result between 0.75x and 1.5x the original length unless the original is too vague to work safely.
9. Specify exactly one output format when the original prompt implies or requests a format.
10. If critical information is missing and cannot be inferred from the original prompt, ask a concise clarification question instead of guessing.

Do NOT add: polite phrasing, \"best practices\" without specifics, chain-of-thought requests, role framing, examples, or requirements the user didn't imply.";

    match mode {
        "clarify" => format!(
            "{persona}\n\n\
            Task: Add missing context that a coding AI would need to avoid ambiguity. \
            Specify the scope, constraints, expected output, and success criteria. \
            Ask a short clarification question if the missing context is essential and absent.\n\n\
            Examples:\n\
            Input: \"add tests\"\n\
            Output: \"Add unit tests for the UserService class in src/services/user.ts \
            using Jest. Cover createUser, updateEmail, and deleteUser. Mock the database calls.\"\n\n\
            Input: \"handle errors\"\n\
            Output: \"Add error handling to fetchUserData(): catch network timeouts, \
            invalid JSON, and 4xx/5xx status codes. Return typed error objects, do not throw.\"\n\n\
            {rules}"
        ),

        "shorten" => format!(
            "{persona}\n\n\
            Task: Remove filler, redundancy, and vague qualifiers. \
            Keep all technical specifics, constraints, output requirements, URLs, paths, and pasted references.\n\n\
            Examples:\n\
            Input: \"Could you please take a look at the code and maybe refactor it to be a bit cleaner?\"\n\
            Output: \"Refactor this code for readability.\"\n\n\
            Input: \"I need you to write some documentation for this function so developers understand it\"\n\
            Output: \"Write JSDoc for this function.\"\n\n\
            {rules}"
        ),

        "formalize" => format!(
            "{persona}\n\n\
            Task: Rewrite as a compact technical specification with explicit task, constraints, inputs, \
            output format, and success criteria. Use the simplest structure that captures the requirement.\n\n\
            Examples:\n\
            Input: \"add pagination\"\n\
            Output: \"Implement cursor-based pagination for /api/posts. Accept 'cursor' and 'limit' \
            query params (default limit: 20, max: 100). Return 'nextCursor' in the response. \
            Use the post ID as the cursor value.\"\n\n\
            {rules}"
        ),

        _ => format!(
            // default: "sharpen"
            "{persona}\n\n\
            Task: Sharpen the prompt to be more specific and actionable. \
            Add only missing details implied by the original prompt or conversation context. \
            Remove filler, replace vague verbs with measurable actions, and preserve all pasted references.\n\n\
            Before writing, silently ask: What is the user actually trying to accomplish? \
            What constraints and output format would prevent guessing? \
            Is anything critical missing that requires a clarification question?\n\n\
            Examples:\n\
            Input: \"fix the login bug\"\n\
            Output: \"Fix the bug in auth/login.ts where validateToken() returns false \
            for valid JWTs when token expiry is within 60 seconds.\"\n\n\
            Input: \"make this faster\"\n\
            Output: \"Profile processQueue() in worker.ts and optimize the bottleneck. \
            Likely cause: synchronous file reads in the inner loop.\"\n\n\
            {rules}"
        ),
    }
}
