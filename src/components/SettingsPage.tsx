import { invoke } from "@tauri-apps/api/core";
import { ArrowLeft, Check, Globe2, Loader2, Save, Settings, Wifi } from "lucide-react";
import type { Dispatch, SetStateAction } from "react";
import { useEffect, useState } from "react";
import { normalizeUiLanguage, type Messages } from "../i18n/messages";
import type { AppSettings } from "../types";
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

  useEffect(() => {
    if (testStatus === "idle") {
      setTestMessage(t.testIdle);
    } else if (testStatus === "success") {
      setTestMessage(t.testSuccess);
    }
  }, [t, testStatus]);

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
          {/*  */}
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
        </div>
      </section>
    </main>
  );
}
