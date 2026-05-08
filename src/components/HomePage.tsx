import { Clipboard, Languages, Loader2, Search, Settings } from "lucide-react";
import type { Messages } from "../i18n/messages";
import type { LookupResult } from "../types";
import { ResultPanel } from "./ResultPanel";

type HomePageProps = {
    busy: boolean;
    canTranslate: boolean;
    error: string;
    input: string;
    result: LookupResult | null;
    t: Messages;
    onCaptureSelection: () => void;
    onInputChange: (input: string) => void;
    onOpenSettings: () => void;
    onRunLookup: () => void;
};

export function HomePage({
    busy,
    canTranslate,
    error,
    input,
    result,
    t,
    onCaptureSelection,
    onInputChange,
    onOpenSettings,
    onRunLookup,
}: HomePageProps) {
    return (
        <main className="app-shell">
            <header className="flex h-20 shrink-0 items-center justify-between gap-4 border-b border-line px-5">
                <div className="min-w-0">
                    <h1 className="truncate text-2xl font-semibold tracking-normal text-ink">
                        {t.appName}
                    </h1>
                    <p className="mt-1 truncate text-sm text-moss">
                        {t.homeSubtitle}
                    </p>
                </div>
                <div className="flex shrink-0 items-center gap-2">
                    <button
                        aria-label={t.openSettings}
                        className="icon-button"
                        onClick={onOpenSettings}
                        title={t.settings}
                    >
                        <Settings className="h-4 w-4" />
                    </button>
                    <button
                        onClick={onCaptureSelection}
                        className="inline-flex h-10 items-center gap-2 rounded-md bg-moss px-4 text-sm font-medium text-white shadow-sm transition hover:bg-ink disabled:cursor-not-allowed disabled:opacity-60"
                        disabled={busy}
                    >
                        {busy ? (
                            <Loader2 className="h-4 w-4 animate-spin" />
                        ) : (
                            <Clipboard className="h-4 w-4" />
                        )}
                        {t.readSelection}
                    </button>
                </div>
            </header>

            <section className="grid min-h-0 flex-1 grid-rows-[auto_minmax(0,1fr)] gap-4 p-5">
                <div className="rounded-lg border border-line bg-white/78 p-4 shadow-sm">
                    <div className="flex justify-between p-3">
                        <div className="mb-3 flex items-center gap-2 text-sm font-semibold text-ink">
                            <Search className="h-4 w-4" />
                            {t.manualTest}
                        </div>

                        <button
                            onClick={onRunLookup}
                            disabled={!canTranslate || busy}
                            className="inline-flex h-10 items-center gap-2 rounded-md bg-amber px-4 text-sm font-medium text-white transition hover:bg-moss disabled:cursor-not-allowed disabled:opacity-60"
                        >
                            {busy ? (
                                <Loader2 className="h-4 w-4 animate-spin" />
                            ) : (
                                <Languages className="h-4 w-4" />
                            )}
                            {t.lookup}
                        </button>
                    </div>
                    <textarea
                        value={input}
                        onChange={(event) => onInputChange(event.target.value)}
                        className="h-28 w-full resize-none rounded-md border border-line bg-white p-3 text-sm outline-none transition focus:border-moss focus:ring-2 focus:ring-moss/15"
                        placeholder={t.inputPlaceholder}
                    />
                </div>

                <ResultPanel result={result} error={error} t={t} />
            </section>
        </main>
    );
}
