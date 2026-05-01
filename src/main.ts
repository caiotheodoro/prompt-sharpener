import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { currentMonitor, getCurrentWindow, type Monitor } from "@tauri-apps/api/window";
import { PhysicalPosition, LogicalSize } from "@tauri-apps/api/dpi"; // PhysicalPosition used for drag

const win = getCurrentWindow();

const OVERLAY_WIDTH = 640;
const COMPACT_HEIGHT = 180;
const MIN_READY_HEIGHT = 420;
const SCREEN_MARGIN = 4;
const HEIGHT_PAD = 8;
const FALLBACK_MAX_HEIGHT = 900;

type AcceptBehavior = "paste" | "replace_line";
type AcceptMode = "terminal" | "editor";
type Provider = "anthropic" | "openai" | "gemini" | "ollama";

interface ModelPreset {
  value: string;
  label: string;
}

interface CapturedTextPayload {
  text: string;
  acceptBehavior: AcceptBehavior;
}

const DEFAULT_PROVIDER: Provider = "anthropic";

const MODEL_PRESETS: Record<Provider, ModelPreset[]> = {
  anthropic: [
    { value: "claude-haiku-4-5-20251001", label: "Haiku 4.5 — fast" },
    { value: "claude-sonnet-4-6", label: "Sonnet 4.6 — best" },
  ],
  openai: [
    { value: "gpt-5.4-nano", label: "GPT-5.4 Nano — cheapest" },
    { value: "gpt-5.4-mini", label: "GPT-5.4 Mini — fast" },
  ],
  gemini: [
    { value: "gemini-2.5-flash-lite", label: "Gemini 2.5 Flash-Lite — cheapest" },
    { value: "gemini-3-flash-preview", label: "Gemini 3 Flash — fast" },
  ],
  ollama: [
    { value: "qwen3:1.7b", label: "Qwen3 1.7B — default local" },
    { value: "qwen3:4b", label: "Qwen3 4B — better quality" },
    { value: "qwen3:8b", label: "Qwen3 8B — best local quality" },
  ],
};

const PROVIDER_KEY_SETTING: Record<Exclude<Provider, "ollama">, string> = {
  anthropic: "anthropic_api_key",
  openai: "openai_api_key",
  gemini: "gemini_api_key",
};

const PROVIDER_KEY_LABEL: Record<Provider, string> = {
  anthropic: "Anthropic API key",
  openai: "OpenAI API key",
  gemini: "Gemini API key",
  ollama: "Ollama local runtime",
};

const PROVIDER_KEY_PLACEHOLDER: Record<Provider, string> = {
  anthropic: "sk-ant-...",
  openai: "sk-...",
  gemini: "AIza...",
  ollama: "No API key required",
};

function isProvider(value: string): value is Provider {
  return value === "anthropic" || value === "openai" || value === "gemini" || value === "ollama";
}

async function getProvider(): Promise<Provider> {
  try {
    const provider = await invoke<string>("get_setting", { key: "provider" });
    return isProvider(provider) ? provider : DEFAULT_PROVIDER;
  } catch {
    return DEFAULT_PROVIDER;
  }
}

function defaultModel(provider: Provider): string {
  return MODEL_PRESETS[provider][0].value;
}

function modelIsValidForProvider(provider: Provider, model: string): boolean {
  return MODEL_PRESETS[provider].some((preset) => preset.value === model);
}

async function getModelForProvider(provider: Provider): Promise<string> {
  try {
    const model = await invoke<string>("get_setting", { key: "model" });
    return modelIsValidForProvider(provider, model) ? model : defaultModel(provider);
  } catch {
    return defaultModel(provider);
  }
}

async function getApiKeyForProvider(provider: Provider): Promise<string> {
  if (provider === "ollama") return "ollama";

  try {
    const key = await invoke<string>("get_setting", { key: PROVIDER_KEY_SETTING[provider] });
    if (key.trim()) return key;
  } catch {
    if (provider === "anthropic") {
      try {
        const legacyKey = await invoke<string>("get_setting", { key: "api_key" });
        if (legacyKey.trim()) return legacyKey;
      } catch { /* no legacy key */ }
    }
  }

  throw new Error(`No ${PROVIDER_KEY_LABEL[provider]} — click ⚙ to open Settings.`);
}

function populateModelSelect(select: HTMLSelectElement, provider: Provider, selectedModel: string) {
  select.replaceChildren(
    ...MODEL_PRESETS[provider].map((preset) => {
      const option = document.createElement("option");
      option.value = preset.value;
      option.textContent = preset.label;
      return option;
    }),
  );
  select.value = modelIsValidForProvider(provider, selectedModel)
    ? selectedModel
    : defaultModel(provider);
}

function nextFrame(): Promise<void> {
  return new Promise((resolve) => {
    requestAnimationFrame(() => resolve());
  });
}

function monitorMaxLogicalHeight(monitor: Monitor | null): number {
  if (!monitor) return FALLBACK_MAX_HEIGHT;

  return Math.max(
    MIN_READY_HEIGHT,
    Math.floor(monitor.workArea.size.height / monitor.scaleFactor) - SCREEN_MARGIN * 2,
  );
}

async function clampOverlayToMonitor(logicalHeight: number, monitor: Monitor | null) {
  if (!monitor) return;

  const outer = await win.outerPosition();
  const physicalHeight = Math.round(logicalHeight * monitor.scaleFactor);
  const minY = monitor.workArea.position.y + SCREEN_MARGIN;
  const maxY = monitor.workArea.position.y + monitor.workArea.size.height - physicalHeight - SCREEN_MARGIN;
  const nextY = Math.max(minY, Math.min(outer.y, maxY));

  if (nextY !== outer.y) {
    await win.setPosition(new PhysicalPosition(outer.x, nextY));
  }
}

async function resizeToContent(minHeight: number) {
  const body = document.querySelector(".body") as HTMLElement;
  const chrome = document.querySelector(".chrome") as HTMLElement;
  const resultBox = document.getElementById("sharpened-text");

  if (resultBox) {
    resultBox.style.maxHeight = "";
    resultBox.style.overflowY = "auto";
  }

  await nextFrame();

  const monitor = await currentMonitor();
  const maxHeight = monitorMaxLogicalHeight(monitor);
  const chromeHeight = Math.ceil(chrome.getBoundingClientRect().height);
  const panel = document.getElementById("state-ready")!;
  const actions = document.querySelector(".actions") as HTMLElement;
  const escHint = document.querySelector(".esc-hint") as HTMLElement;
  const bodyStyle = getComputedStyle(body);
  const panelStyle = getComputedStyle(panel);
  const bodyPadding =
    Number.parseFloat(bodyStyle.paddingTop) +
    Number.parseFloat(bodyStyle.paddingBottom);
  const panelGap = Number.parseFloat(panelStyle.rowGap || panelStyle.gap || "0");
  const textHeight = resultBox?.scrollHeight ?? 0;
  const contentHeight =
    chromeHeight +
    bodyPadding +
    textHeight +
    actions.offsetHeight +
    escHint.offsetHeight +
    panelGap * 2 +
    HEIGHT_PAD;
  const nextHeight = Math.min(Math.max(minHeight, contentHeight), maxHeight);

  await win.setSize(new LogicalSize(OVERLAY_WIDTH, nextHeight));
  await clampOverlayToMonitor(nextHeight, monitor);
}

async function resizeCompact() {
  await win.setSize(new LogicalSize(OVERLAY_WIDTH, COMPACT_HEIGHT));
}

// ── Drag helper ───────────────────────────────────────────────────────────────
function makeDraggable(
  appWin: ReturnType<typeof getCurrentWindow>,
  handle: HTMLElement,
  excludeSelector: string,
) {
  let ready = false;
  let initMouseX = 0, initMouseY = 0, initWinX = 0, initWinY = 0;

  handle.addEventListener("pointerdown", (e) => {
    if (e.button !== 0 || (e.target as HTMLElement).closest(excludeSelector)) return;
    e.preventDefault();
    handle.setPointerCapture(e.pointerId);
    ready = false;
    const mx = e.screenX, my = e.screenY;
    appWin.outerPosition().then((pos) => {
      initMouseX = mx;
      initMouseY = my;
      initWinX = pos.x;
      initWinY = pos.y;
      ready = true;
    });
  });

  handle.addEventListener("pointermove", (e) => {
    if (!ready || !handle.hasPointerCapture(e.pointerId)) return;
    const dpr = globalThis.devicePixelRatio || 1;
    appWin.setPosition(new PhysicalPosition(
      initWinX + (e.screenX - initMouseX) * dpr,
      initWinY + (e.screenY - initMouseY) * dpr,
    ));
  });

  handle.addEventListener("pointerup", (e) => {
    handle.releasePointerCapture(e.pointerId);
    ready = false;
  });
}

function makeResizable(appWin: ReturnType<typeof getCurrentWindow>, handle: HTMLElement) {
  handle.addEventListener("pointerdown", (e) => {
    if (e.button !== 0) return;
    e.preventDefault();
    e.stopPropagation();
    appWin.startResizeDragging("SouthEast");
  });
}

let sharpenedText = "";
let clearPasses = 8;
let acceptBehavior: AcceptBehavior = "replace_line";

function estimateTerminalClearPasses(text: string): number {
  const explicitLines = text.split(/\r\n|\r|\n/).length;
  const wrappedLines = Math.ceil(text.length / 80);
  return Math.max(8, explicitLines + wrappedLines + 6);
}

function showState(state: "idle" | "loading" | "error" | "ready") {
  document.querySelectorAll(".panel").forEach((el) => el.classList.add("hidden"));
  document.getElementById(`state-${state}`)?.classList.remove("hidden");
  if (state !== "ready") resizeCompact();
}

async function initOverlay() {
  document.getElementById("overlay")!.classList.remove("hidden");

  makeDraggable(win, document.querySelector(".chrome") as HTMLElement, "button");
  makeResizable(win, document.getElementById("resize-handle")!);

  document.getElementById("btn-close")?.addEventListener("click", async () => {
    await win.hide();
    showState("idle");
  });

  document.getElementById("btn-settings")?.addEventListener("click", () => invoke("open_settings"));

  document.getElementById("btn-accept")?.addEventListener("click", async () => {
    await win.hide();
    showState("idle");
    if (acceptBehavior === "replace_line") {
      await invoke("replace_line_text", { text: sharpenedText, clearPasses });
      return;
    }

    await invoke("paste_text", { text: sharpenedText });
  });

  document.getElementById("btn-reject")?.addEventListener("click", async () => {
    await win.hide();
    showState("idle");
  });

  document.getElementById("btn-dismiss")?.addEventListener("click", async () => {
    await win.hide();
    showState("idle");
  });

  globalThis.addEventListener("keydown", async (e) => {
    if (e.key === "Escape") {
      await win.hide();
      showState("idle");
    }
  });

  await listen<CapturedTextPayload>("selected-text-captured", async (event) => {
    const originalText = event.payload.text;
    acceptBehavior = event.payload.acceptBehavior;
    clearPasses = estimateTerminalClearPasses(originalText);
    showState("loading");

    const provider = await getProvider();
    let apiKey: string;
    try {
      apiKey = await getApiKeyForProvider(provider);
    } catch (err) {
      document.getElementById("error-msg")!.textContent =
        err instanceof Error ? err.message : String(err);
      showState("error");
      return;
    }

    const model = await getModelForProvider(provider);

    try {
      sharpenedText = await invoke<string>("sharpen_prompt", {
        text: originalText,
        mode: "sharpen",
        provider,
        apiKey,
        model,
      });
    } catch (err) {
      document.getElementById("error-msg")!.textContent = String(err);
      showState("error");
      return;
    }

    const textEl = document.getElementById("sharpened-text")!;
    textEl.textContent = sharpenedText;
    showState("ready");
    await resizeToContent(MIN_READY_HEIGHT);
  });
}

async function initSettings() {
  document.getElementById("settings")!.classList.remove("hidden");
  const providerSelect = document.getElementById("provider-select") as HTMLSelectElement;
  const apiKeyLabel = document.getElementById("api-key-label") as HTMLElement;
  const apiKeyInput = document.getElementById("api-key-input") as HTMLInputElement;
  const modelSelect = document.getElementById("model-select") as HTMLSelectElement;
  const acceptModeSelect = document.getElementById("accept-mode-select") as HTMLSelectElement;
  const ollamaNotice = document.getElementById("ollama-notice") as HTMLElement;
  const checkOllamaButton = document.getElementById("btn-check-ollama") as HTMLButtonElement;
  const ollamaStatusDot = document.getElementById("ollama-status-dot") as HTMLElement;
  const ollamaStatusText = document.getElementById("ollama-status-text") as HTMLElement;
  const keyCache: Partial<Record<Exclude<Provider, "ollama">, string>> = {};

  let currentProvider = await getProvider();

  async function readStoredApiKey(provider: Provider): Promise<string> {
    if (provider === "ollama") return "";
    if (keyCache[provider] !== undefined) return keyCache[provider];

    try {
      const key = await invoke<string>("get_setting", { key: PROVIDER_KEY_SETTING[provider] });
      keyCache[provider] = key;
      return key;
    } catch {
      if (provider === "anthropic") {
        try {
          const legacyKey = await invoke<string>("get_setting", { key: "api_key" });
          keyCache.anthropic = legacyKey;
          return legacyKey;
        } catch { /* no legacy key */ }
      }
    }

    keyCache[provider] = "";
    return "";
  }

  function setOllamaStatus(state: "idle" | "checking" | "ok" | "error", message: string) {
    ollamaStatusDot.classList.toggle("status-dot--ok", state === "ok");
    ollamaStatusDot.classList.toggle("status-dot--error", state === "error");
    ollamaStatusText.textContent = message;
  }

  async function renderProviderSettings(provider: Provider, selectedModel?: string) {
    providerSelect.value = provider;
    apiKeyLabel.textContent = PROVIDER_KEY_LABEL[provider];
    apiKeyInput.placeholder = PROVIDER_KEY_PLACEHOLDER[provider];
    apiKeyInput.disabled = provider === "ollama";
    apiKeyInput.closest(".field")?.classList.toggle("hidden", provider === "ollama");
    ollamaNotice.classList.toggle("hidden", provider !== "ollama");
    if (provider === "ollama") {
      setOllamaStatus("idle", "check Ollama status");
    }
    const model = selectedModel ?? await getModelForProvider(provider);
    populateModelSelect(modelSelect, provider, model);
    apiKeyInput.value = await readStoredApiKey(provider);
  }

  try {
    const acceptMode = await invoke<AcceptMode>("get_setting", { key: "accept_mode" });
    acceptModeSelect.value = acceptMode;
  } catch {
    acceptModeSelect.value = "terminal";
  }

  await renderProviderSettings(currentProvider);

  providerSelect.addEventListener("change", async () => {
    if (currentProvider !== "ollama") {
      keyCache[currentProvider] = apiKeyInput.value.trim();
    }
    const nextProvider = isProvider(providerSelect.value) ? providerSelect.value : DEFAULT_PROVIDER;
    currentProvider = nextProvider;
    await renderProviderSettings(nextProvider);
  });

  checkOllamaButton.addEventListener("click", async () => {
    setOllamaStatus("checking", "checking...");
    try {
      await invoke("check_ollama_status");
      setOllamaStatus("ok", "Ollama is running");
    } catch (err) {
      setOllamaStatus("error", err instanceof Error ? err.message : String(err));
    }
  });

  const saveBtn = document.getElementById("btn-save")!;
  saveBtn.addEventListener("click", async () => {
    const provider = isProvider(providerSelect.value) ? providerSelect.value : DEFAULT_PROVIDER;
    const apiKey = apiKeyInput.value.trim();
    const model = modelSelect.value || defaultModel(provider);
    const acceptMode = acceptModeSelect.value;
    if (provider !== "ollama") {
      keyCache[provider] = apiKey;
    }

    await invoke("set_setting", { key: "provider", value: provider });
    if (provider !== "ollama") {
      await invoke("set_setting", { key: PROVIDER_KEY_SETTING[provider], value: apiKey });
    }
    if (provider === "anthropic") {
      await invoke("set_setting", { key: "api_key", value: apiKey });
    }
    await invoke("set_setting", { key: "model", value: model });
    await invoke("set_setting", { key: "accept_mode", value: acceptMode });
    saveBtn.textContent = "✓ saved";
    setTimeout(() => { saveBtn.textContent = "save"; }, 2000);
  });

  apiKeyInput.addEventListener("keydown", (e) => {
    if (e.key === "Enter") saveBtn.click();
  });
}

if (win.label === "settings") {
  initSettings();
} else {
  initOverlay();
}
