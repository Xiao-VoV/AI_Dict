import { BookOpen } from "lucide-react";
import type { Messages } from "../i18n/messages";
import type { LookupResult } from "../types";

type ResultPanelProps = {
  result: LookupResult | null;
  error: string;
  t: Messages;
};

export function ResultPanel({ result, error, t }: ResultPanelProps) {
  return (
    <section className="min-h-0 overflow-hidden rounded-lg border border-line bg-white/78 p-4 shadow-sm">
      <div className="mb-3 flex h-5 items-center gap-2 text-sm font-semibold text-ink">
        <BookOpen className="h-4 w-4" />
        {t.result}
      </div>
      <div className="result-content">
        {error ? (
          <div className="rounded-md border border-red-200 bg-red-50 p-3 text-sm text-red-700">
            {error}
          </div>
        ) : null}
        {!error && !result ? (
          <div className="flex h-full items-center justify-center text-sm text-moss">
            {t.emptyResult}
          </div>
        ) : null}
        {!error && result?.kind === "dictionary" ? <DictionaryResult result={result} /> : null}
        {!error && result?.kind === "dictionaryMiss" ? (
          <div className="space-y-3">
            <p className="text-lg font-semibold text-ink">{result.source}</p>
            <p className="rounded-md bg-paper p-3 text-sm text-moss">{result.message}</p>
          </div>
        ) : null}
        {!error && result?.kind === "translation" ? (
          <TranslationResult result={result} t={t} />
        ) : null}
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
            <span className="mr-2 text-xs font-semibold text-amber">
              {definition.partOfSpeech}
            </span>
            <span className="text-sm text-ink">{definition.meaning}</span>
          </div>
        ))}
      </div>
      {result.entry.examples.length > 0 ? (
        <div className="border-t border-line pt-3">
          {result.entry.examples.map((example) => (
            <p key={example} className="text-sm text-moss">
              {example}
            </p>
          ))}
        </div>
      ) : null}
    </div>
  );
}

function TranslationResult({
  result,
  t,
}: {
  result: Extract<LookupResult, { kind: "translation" }>;
  t: Messages;
}) {
  return (
    <div className="grid gap-3">
      <div className="rounded-md bg-paper p-3">
        <p className="mb-1 text-xs font-semibold text-moss">{t.source}</p>
        <p className="whitespace-pre-wrap text-sm text-ink">{result.source}</p>
      </div>
      <div className="rounded-md bg-white p-3 ring-1 ring-line">
        <p className="mb-1 text-xs font-semibold text-amber">{t.translation}</p>
        <p className="whitespace-pre-wrap text-base leading-7 text-ink">{result.translated}</p>
      </div>
    </div>
  );
}
