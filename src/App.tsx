import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { useEffect, useMemo, useState } from "react";
import { HomePage } from "./components/HomePage";
import { SettingsPage } from "./components/SettingsPage";
import { messages, normalizeUiLanguage } from "./i18n/messages";
import { defaultSettings } from "./settingsDefaults";
import type { AppSettings, LookupResult } from "./types";

function App() {
  const [settings, setSettings] = useState<AppSettings>(defaultSettings);
  const [page, setPage] = useState<"home" | "settings">("home");
  const [input, setInput] = useState("hello");
  const [result, setResult] = useState<LookupResult | null>(null);
  const [error, setError] = useState("");
  const [busy, setBusy] = useState(false);
  const [saved, setSaved] = useState(false);
  const t = messages[normalizeUiLanguage(settings.uiLanguage)];

  useEffect(() => {
    invoke<AppSettings>("get_settings")
      .then((loadedSettings) =>
        setSettings({
          ...defaultSettings,
          ...loadedSettings,
          uiLanguage: normalizeUiLanguage(loadedSettings.uiLanguage),
        }),
      )
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
        t={t}
      />
    );
  }

  return (
    <HomePage
      busy={busy}
      canTranslate={canTranslate}
      error={error}
      input={input}
      result={result}
      t={t}
      onCaptureSelection={captureSelection}
      onInputChange={setInput}
      onOpenSettings={() => setPage("settings")}
      onRunLookup={runLookup}
    />
  );
}

export default App;
