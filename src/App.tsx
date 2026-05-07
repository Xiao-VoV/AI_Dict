import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import {
  ArrowLeft,
  BookOpen,
  Check,
  Clipboard,
  Languages,
  Loader2,
  Save,
  Search,
  Settings,
} from "lucide-react";
import { useEffect, useMemo, useState } from "react";

type AppSettings = {
  baseUrl: string;
  apiKey: string;
  model: string;
  temperature: number;
  targetLanguage: string;
};

type DictionaryEntry = {
  word: string;
  phonetic: string;
  definitions: Array<{ partOfSpeech: string; meaning: string }>;
  examples: string[];
};

type LookupResult =
  | { kind: "dictionary"; source: string; entry: DictionaryEntry }
  | { kind: "translation"; source: string; translated: string }
  | { kind: "dictionaryMiss"; source: string; message: string };

const defaultSettings: AppSettings = {
  baseUrl: "https://api.openai.com/v1",
  apiKey: "",
  model: "gpt-4o-mini",
  temperature: 0.2,
  targetLanguage: "简体中文",
};

function App() {
  const [settings, setSettings] = useState<AppSettings>(defaultSettings);
  const [page, setPage] = useState<"home" | "settings">("home");
  const [input, setInput] = useState("hello");
  const [result, setResult] = useState<LookupResult | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    invoke<AppSettings>("get_settings")
      .then(setSettings)
      .catch((message) => setError(String(message)));

    const unlistenResult = listen<LookupResult>("lookup-result", (event) => {
      setResult(event.payload);
      setError("");
      setBusy(false);
    });
    const unlistenError = listen<string>("lookup-error", (event) => {
      setError(event.payload);
      setBusy(false);
    });

    return () => {
      void unlistenResult.then((unlisten) => unlisten());
      void unlistenError.then((unlisten) => unlisten());
    };
  }, []);

  const canTranslate = useMemo(() => input.trim().length > 0, [input]);

  async function runLookup() {
    if (!canTranslate) return;
    setBusy(true);
    setError("");
    try {
      const response = await invoke<LookupResult>("lookup_text", {
        text: input,
      });
      setResult(response);
    } catch (message) {
      setError(String(message));
    } finally {
      setBusy(false);
    }
  }

  async function captureSelection() {
    setBusy(true);
    setError("");
    try {
      const response = await invoke<LookupResult>("capture_selection_and_lookup");
      setResult(response);
    } catch (message) {
      setError(String(message));
    } finally {
      setBusy(false);
    }
  }

  async function saveSettings() {
    setSaved(false);
    setError("");
    try {
      await invoke("save_settings", { settings });
      setSaved(true);
      window.setTimeout(() => setSaved(false), 1800);
    } catch (message) {
      setError(String(message));
    }
  }

  if (page === "settings") {
    return (
      <SettingsPage
        onBack={() => setPage("home")}
        onSave={saveSettings}
        saved={saved}
        settings={settings}
        setSettings={setSettings}
      />
    );
  }

  return (
    <main className="app-shell">
      <header className="flex h-20 shrink-0 items-center justify-between gap-4 border-b border-line px-5">
        <div className="min-w-0">
          <h1 className="truncate text-2xl font-semibold tracking-normal text-ink">划词翻译</h1>
          <p className="mt-1 truncate text-sm text-moss">
            快捷键：Cmd/Ctrl + Shift + E。单词查词典，句子走 AI 翻译。
          </p>
        </div>
        <div className="flex shrink-0 items-center gap-2">
          <button
            aria-label="打开设置"
            className="icon-button"
            onClick={() => setPage("settings")}
            title="设置"
          >
            <Settings className="h-4 w-4" />
          </button>
          <button
            onClick={captureSelection}
            className="inline-flex h-10 items-center gap-2 rounded-md bg-moss px-4 text-sm font-medium text-white shadow-sm transition hover:bg-ink disabled:cursor-not-allowed disabled:opacity-60"
            disabled={busy}
          >
            {busy ? (
              <Loader2 className="h-4 w-4 animate-spin" />
            ) : (
              <Clipboard className="h-4 w-4" />
            )}
            读取当前选中
          </button>
        </div>
      </header>

      <section className="grid min-h-0 flex-1 grid-rows-[auto_minmax(0,1fr)] gap-4 p-5">
        <div className="rounded-lg border border-line bg-white/78 p-4 shadow-sm">
          <div className="mb-3 flex items-center gap-2 text-sm font-semibold text-ink">
            <Search className="h-4 w-4" />
            手动测试
          </div>
          <textarea
            value={input}
            onChange={(event) => setInput(event.target.value)}
            className="h-28 w-full resize-none rounded-md border border-line bg-white p-3 text-sm outline-none transition focus:border-moss focus:ring-2 focus:ring-moss/15"
            placeholder="输入 hello 测试本地词典，或输入一句话测试 AI 翻译"
          />
          <div className="mt-3 flex justify-end">
            <button
              onClick={runLookup}
              disabled={!canTranslate || busy}
              className="inline-flex h-10 items-center gap-2 rounded-md bg-amber px-4 text-sm font-medium text-white transition hover:bg-moss disabled:cursor-not-allowed disabled:opacity-60"
            >
              {busy ? (
                <Loader2 className="h-4 w-4 animate-spin" />
              ) : (
                <Languages className="h-4 w-4" />
              )}
              查询 / 翻译
            </button>
          </div>
        </div>

        <ResultPanel result={result} error={error} />
      </section>
    </main>
  );
}

function SettingsPage({
  onBack,
  onSave,
  saved,
  settings,
  setSettings,
}: {
  onBack: () => void;
  onSave: () => void;
  saved: boolean;
  settings: AppSettings;
  setSettings: React.Dispatch<React.SetStateAction<AppSettings>>;
}) {
  return (
    <main className="app-shell">
      <header className="flex h-20 shrink-0 items-center justify-between gap-4 border-b border-line px-5">
        <div className="flex min-w-0 items-center gap-3">
          <button aria-label="返回首页" className="icon-button" onClick={onBack} title="返回">
            <ArrowLeft className="h-4 w-4" />
          </button>
          <div className="min-w-0">
            <h1 className="truncate text-xl font-semibold text-ink">设置</h1>
            <p className="mt-1 truncate text-sm text-moss">模型、目标语言和翻译参数</p>
          </div>
        </div>
        <button
          onClick={onSave}
          className="inline-flex h-10 shrink-0 items-center justify-center gap-2 rounded-md bg-ink px-4 text-sm font-medium text-white transition hover:bg-moss"
        >
          {saved ? <Check className="h-4 w-4" /> : <Save className="h-4 w-4" />}
          {saved ? "已保存" : "保存"}
        </button>
      </header>

      <section className="grid min-h-0 flex-1 place-items-center p-5">
        <div className="w-full max-w-2xl rounded-lg border border-line bg-white/78 p-5 shadow-sm">
          <div className="mb-4 flex items-center gap-2 text-sm font-semibold text-ink">
            <Settings className="h-4 w-4" />
            模型配置
          </div>
          <div className="grid gap-3">
            <Field label="Base URL">
              <input
                value={settings.baseUrl}
                onChange={(event) => setSettings({ ...settings, baseUrl: event.target.value })}
                className="field"
              />
            </Field>
            <Field label="API Key">
              <input
                value={settings.apiKey}
                onChange={(event) => setSettings({ ...settings, apiKey: event.target.value })}
                className="field"
                type="password"
                placeholder="sk-..."
              />
            </Field>
            <div className="grid gap-3 sm:grid-cols-2">
              <Field label="Model">
                <input
                  value={settings.model}
                  onChange={(event) => setSettings({ ...settings, model: event.target.value })}
                  className="field"
                />
              </Field>
              <Field label="目标语言">
                <input
                  value={settings.targetLanguage}
                  onChange={(event) =>
                    setSettings({ ...settings, targetLanguage: event.target.value })
                  }
                  className="field"
                />
              </Field>
            </div>
            <Field label={`Temperature ${settings.temperature.toFixed(1)}`}>
              <input
                value={settings.temperature}
                onChange={(event) =>
                  setSettings({ ...settings, temperature: Number(event.target.value) })
                }
                className="w-full accent-moss"
                max={1}
                min={0}
                step={0.1}
                type="range"
              />
            </Field>
          </div>
        </div>
      </section>
    </main>
  );
}

function Field({ label, children }: { label: string; children: React.ReactNode }) {
  return (
    <label className="block">
      <span className="mb-1 block text-xs font-medium text-moss">{label}</span>
      {children}
    </label>
  );
}

function ResultPanel({ result, error }: { result: LookupResult | null; error: string }) {
  return (
    <section className="min-h-0 overflow-hidden rounded-lg border border-line bg-white/78 p-4 shadow-sm">
      <div className="mb-3 flex h-5 items-center gap-2 text-sm font-semibold text-ink">
        <BookOpen className="h-4 w-4" />
        结果
      </div>
      <div className="result-content">
        {error ? (
          <div className="rounded-md border border-red-200 bg-red-50 p-3 text-sm text-red-700">
            {error}
          </div>
        ) : null}
        {!error && !result ? (
          <div className="flex h-full items-center justify-center text-sm text-moss">
            选中文本后按快捷键，或在上方输入内容测试。
          </div>
        ) : null}
        {!error && result?.kind === "dictionary" ? <DictionaryResult result={result} /> : null}
        {!error && result?.kind === "dictionaryMiss" ? (
          <div className="space-y-3">
            <p className="text-lg font-semibold text-ink">{result.source}</p>
            <p className="rounded-md bg-paper p-3 text-sm text-moss">{result.message}</p>
          </div>
        ) : null}
        {!error && result?.kind === "translation" ? <TranslationResult result={result} /> : null}
      </div>
    </section>
  );
}

function DictionaryResult({ result }: { result: Extract<LookupResult, { kind: "dictionary" }> }) {
  return (
    <div className="space-y-4">
      <div>
        <div className="flex flex-wrap items-baseline gap-2">
          <h2 className="text-2xl font-semibold text-ink">{result.entry.word}</h2>
          <span className="text-sm text-moss">{result.entry.phonetic}</span>
        </div>
      </div>
      <div className="space-y-2">
        {result.entry.definitions.map((definition, index) => (
          <div key={`${definition.partOfSpeech}-${index}`} className="rounded-md bg-paper p-3">
            <span className="mr-2 text-xs font-semibold text-amber">{definition.partOfSpeech}</span>
            <span className="text-sm text-ink">{definition.meaning}</span>
          </div>
        ))}
      </div>
      {result.entry.examples.length > 0 ? (
        <div className="border-t border-line pt-3">
          {result.entry.examples.map((example) => (
            <p key={example} className="text-sm text-moss">{example}</p>
          ))}
        </div>
      ) : null}
    </div>
  );
}

function TranslationResult({ result }: { result: Extract<LookupResult, { kind: "translation" }> }) {
  return (
    <div className="grid gap-3">
      <div className="rounded-md bg-paper p-3">
        <p className="mb-1 text-xs font-semibold text-moss">原文</p>
        <p className="whitespace-pre-wrap text-sm text-ink">{result.source}</p>
      </div>
      <div className="rounded-md bg-white p-3 ring-1 ring-line">
        <p className="mb-1 text-xs font-semibold text-amber">译文</p>
        <p className="whitespace-pre-wrap text-base leading-7 text-ink">{result.translated}</p>
      </div>
    </div>
  );
}

export default App;
