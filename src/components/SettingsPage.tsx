import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { open } from "@tauri-apps/plugin-dialog";
import {
  ArrowLeft,
  Check,
  Database,
  FileInput,
  Globe2,
  Loader2,
  Save,
  Settings,
  Trash2,
  Wifi,
} from "lucide-react";
import type { Dispatch, SetStateAction } from "react";
import { useEffect, useState } from "react";
import { normalizeUiLanguage, type Messages } from "../i18n/messages";
import type {
  AppSettings,
  DictionaryIndexProgress,
  DictionaryMetadata,
  ImportSummary,
} from "../types";
import { Field } from "./Field";

type TestStatus = "idle" | "testing" | "success" | "error";

type SettingsPageProps = {
  onBack: () => void;
  onSave: () => void;
  saved: boolean;
  settings: AppSettings;
  setSettings: Dispatch<SetStateAction<AppSettings>>;
  t: Messages;
};

export function SettingsPage({
  onBack,
  onSave,
  saved,
  settings,
  setSettings,
  t,
}: SettingsPageProps) {
  const [testStatus, setTestStatus] = useState<TestStatus>("idle");
  const [testMessage, setTestMessage] = useState(t.testIdle);
  const [dictionaries, setDictionaries] = useState<DictionaryMetadata[]>([]);
  const [dictionaryBusy, setDictionaryBusy] = useState(false);
  const [dictionaryMessage, setDictionaryMessage] = useState("");
  const [indexProgress, setIndexProgress] = useState<DictionaryIndexProgress | null>(null);

  useEffect(() => {
    if (testStatus === "idle") {
      setTestMessage(t.testIdle);
    } else if (testStatus === "success") {
      setTestMessage(t.testSuccess);
    }
  }, [t, testStatus]);

  useEffect(() => {
    void refreshDictionaries();
  }, []);

  useEffect(() => {
    const unlistenProgress = listen<DictionaryIndexProgress>(
      "dictionary-index-progress",
      (event) => {
        setIndexProgress(event.payload);
      },
    );

    return () => {
      void unlistenProgress.then((unlisten) => unlisten());
    };
  }, []);

  async function testConnection() {
    setTestStatus("testing");
    setTestMessage("");
    try {
      await invoke("test_ai_connection", { settings });
      setTestStatus("success");
      setTestMessage(t.testSuccess);
    } catch (message) {
      setTestStatus("error");
      setTestMessage(String(message));
    }
  }

  async function refreshDictionaries() {
    try {
      const response = await invoke<DictionaryMetadata[]>("list_dictionaries");
      setDictionaries(response);
    } catch (message) {
      setDictionaryMessage(String(message));
    }
  }

  async function importMdx() {
    const selected = await open({
      multiple: false,
      filters: [{ name: "MDX Dictionary", extensions: ["mdx"] }],
    });
    if (typeof selected !== "string") return;
    const name = selected.split(/[\\/]/).pop()?.replace(/\.mdx$/i, "") || t.importMdx;
    await runDictionaryImport("import_mdict", { mdxPath: selected }, name);
  }

  async function reindexBuiltinDictionary() {
    await runDictionaryImport("reindex_builtin_dictionary", {}, "ECDICT");
  }

  async function runDictionaryImport(
    command: string,
    payload: Record<string, unknown>,
    name: string,
  ) {
    setDictionaryBusy(true);
    setDictionaryMessage(t.dictionaryImporting);
    setIndexProgress({
      name,
      kind: command === "reindex_builtin_dictionary" ? "builtin_mdx" : "user_mdx",
      phase: "opening",
      processedEntries: 0,
      totalEntries: 0,
      importedEntries: 0,
      skippedEntries: 0,
      done: false,
    });
    try {
      const summary = await invoke<ImportSummary>(command, payload);
      setDictionaryMessage(
        `${t.dictionaryImportSuccess}: ${summary.name} (${summary.importedEntries})`,
      );
      await refreshDictionaries();
    } catch (message) {
      setDictionaryMessage(String(message));
    } finally {
      setDictionaryBusy(false);
    }
  }

  async function deleteDictionary(dictionaryId: number) {
    setDictionaryBusy(true);
    setIndexProgress(null);
    try {
      await invoke("delete_dictionary", { dictionaryId });
      await refreshDictionaries();
    } catch (message) {
      setDictionaryMessage(String(message));
    } finally {
      setDictionaryBusy(false);
    }
  }

  const progressPercent =
    indexProgress && indexProgress.totalEntries > 0
      ? Math.round((indexProgress.processedEntries / indexProgress.totalEntries) * 100)
      : 0;
  const progressLabel = indexProgress ? dictionaryProgressLabel(indexProgress.phase, t) : "";
  const progressWidth = indexProgress?.totalEntries ? `${progressPercent}%` : "100%";

  return (
    <main className="app-shell">
      <header className="flex h-20 shrink-0 items-center justify-between gap-4 border-b border-line px-5">
        <div className="flex min-w-0 items-center gap-3">
          <button aria-label={t.back} className="icon-button" onClick={onBack} title={t.back}>
            <ArrowLeft className="h-4 w-4" />
          </button>
          <div className="min-w-0">
            <h1 className="truncate text-xl font-semibold text-ink">{t.settings}</h1>
            <p className="mt-1 truncate text-sm text-moss">{t.settingsSubtitle}</p>
          </div>
        </div>
        <button
          onClick={onSave}
          className="inline-flex h-10 shrink-0 items-center justify-center gap-2 rounded-md bg-ink px-4 text-sm font-medium text-white transition hover:bg-moss"
        >
          {saved ? <Check className="h-4 w-4" /> : <Save className="h-4 w-4" />}
          {saved ? t.saved : t.save}
        </button>
      </header>

      <section className="settings-viewport">
        <div className="settings-stack">


          <section className="settings-panel rounded-lg border border-line bg-white/78 p-5 shadow-sm">
            <div className="mb-4 flex items-center gap-2 text-sm font-semibold text-ink">
              <Globe2 className="h-4 w-4" />
              {t.interfaceConfig}
            </div>
            <Field label={t.uiLanguage}>
              <select
                value={normalizeUiLanguage(settings.uiLanguage)}
                onChange={(event) =>
                  setSettings({
                    ...settings,
                    uiLanguage: normalizeUiLanguage(event.target.value),
                  })
                }
                className="field"
              >
                <option value="zh-CN">{t.simplifiedChinese}</option>
                <option value="en-US">{t.english}</option>
              </select>
            </Field>
          </section>

          <section className="settings-panel rounded-lg border border-line bg-white/78 p-5 shadow-sm">
            <div className="mb-4 flex items-center gap-2 text-sm font-semibold text-ink">
              <Settings className="h-4 w-4" />
              {t.modelConfig}
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
              <div className="grid gap-3 settings-grid-two">
                <Field label="Model">
                  <input
                    value={settings.model}
                    onChange={(event) => setSettings({ ...settings, model: event.target.value })}
                    className="field"
                  />
                </Field>
                <Field label={t.targetLanguage}>
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
              <div className="flex flex-wrap items-center gap-3 border-t border-line pt-3">
                <p
                  className={`min-w-0 flex-1 text-sm text-right ${
                    testStatus === "success"
                      ? "text-moss"
                      : testStatus === "error"
                        ? "text-red-700"
                        : "text-moss"
                  }`}
                >
                  {testMessage || t.testIdle}
                </p>
                <button
                  onClick={testConnection}
                  disabled={testStatus === "testing"}
                  className="inline-flex h-10 items-center justify-center gap-2 rounded-md bg-moss px-4 text-sm font-medium text-white transition hover:bg-ink disabled:cursor-not-allowed disabled:opacity-60"
                >
                  {testStatus === "testing" ? (
                    <Loader2 className="h-4 w-4 animate-spin" />
                  ) : (
                    <Wifi className="h-4 w-4" />
                  )}
                  {testStatus === "testing" ? t.testing : t.testConnection}
                </button>
              </div>
            </div>
          </section>

          <section className="settings-panel rounded-lg border border-line bg-white/78 p-5 shadow-sm">
            <div className="mb-4 flex items-center gap-2 text-sm font-semibold text-ink">
              <Database className="h-4 w-4" />
              {t.dictionaryConfig}
            </div>
            <div className="flex flex-wrap gap-2">
              <button
                onClick={importMdx}
                disabled={dictionaryBusy}
                className="inline-flex h-10 items-center justify-center gap-2 rounded-md bg-moss px-4 text-sm font-medium text-white transition hover:bg-ink disabled:cursor-not-allowed disabled:opacity-60"
              >
                {dictionaryBusy ? (
                  <Loader2 className="h-4 w-4 animate-spin" />
                ) : (
                  <FileInput className="h-4 w-4" />
                )}
                {t.importMdx}
              </button>
              <button
                onClick={reindexBuiltinDictionary}
                disabled={dictionaryBusy}
                className="inline-flex h-10 items-center justify-center gap-2 rounded-md bg-amber px-4 text-sm font-medium text-white transition hover:bg-moss disabled:cursor-not-allowed disabled:opacity-60"
              >
                <Database className="h-4 w-4" />
                {t.reindexBuiltinDictionary}
              </button>
            </div>
            {dictionaryMessage ? (
              <p className="mt-3 text-sm text-moss">{dictionaryMessage}</p>
            ) : null}
            {indexProgress ? (
              <div className="mt-3 rounded-md bg-paper p-3">
                <div className="mb-2 flex items-center justify-between gap-3 text-xs text-moss">
                  <span className="min-w-0 truncate">
                    {t.dictionaryIndexProgress}: {indexProgress.name} · {progressLabel}
                  </span>
                  <span className="shrink-0 tabular-nums">
                    {indexProgress.totalEntries > 0
                      ? `${indexProgress.processedEntries} / ${indexProgress.totalEntries}`
                      : progressLabel}
                  </span>
                </div>
                <div className="h-2 overflow-hidden rounded-full bg-line">
                  <div
                    className={`h-full rounded-full bg-moss transition-all ${
                      indexProgress.totalEntries === 0 && !indexProgress.done
                        ? "animate-pulse"
                        : ""
                    }`}
                    style={{ width: progressWidth }}
                  />
                </div>
              </div>
            ) : null}
            <div className="mt-4 grid gap-2">
              {dictionaries.length === 0 ? (
                <p className="rounded-md bg-paper p-3 text-sm text-moss">
                  {t.dictionariesEmpty}
                </p>
              ) : (
                dictionaries.map((dictionary) => (
                  <div
                    key={`${dictionary.kind}-${dictionary.id}`}
                    className="flex items-center justify-between gap-3 rounded-md bg-paper p-3"
                  >
                    <div className="min-w-0">
                      <p className="truncate text-sm font-medium text-ink">
                        {dictionary.name}
                      </p>
                      <p className="text-xs text-moss">
                        {dictionary.kind === "builtin_mdx"
                          ? t.builtinDictionary
                          : t.userDictionary}{" "}
                        · {dictionary.entryCount}
                      </p>
                    </div>
                    {dictionary.kind === "user_mdx" ? (
                      <button
                        aria-label={t.deleteDictionary}
                        className="icon-button"
                        disabled={dictionaryBusy}
                        onClick={() => deleteDictionary(dictionary.id)}
                        title={t.deleteDictionary}
                      >
                        <Trash2 className="h-4 w-4" />
                      </button>
                    ) : null}
                  </div>
                ))
              )}
            </div>
          </section>


        </div>
      </section>
    </main>
  );
}

function dictionaryProgressLabel(
  phase: DictionaryIndexProgress["phase"],
  t: Messages,
) {
  if (phase === "done") return t.dictionaryPhaseDone;
  if (phase === "error") return t.dictionaryPhaseError;
  if (phase === "indexing") return t.dictionaryPhaseIndexing;
  return t.dictionaryPhaseOpening;
}
