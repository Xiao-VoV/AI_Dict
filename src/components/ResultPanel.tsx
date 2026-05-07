import { BookOpen } from "lucide-react";
import type { Messages } from "../i18n/messages";
import type { LookupResult, WordProfile } from "../types";

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
        {!error && result?.kind === "word" ? <WordResult result={result} t={t} /> : null}
        {!error && result?.kind === "translation" ? (
          <TranslationResult result={result} t={t} />
        ) : null}
      </div>
    </section>
  );
}

function WordResult({
  result,
  t,
}: {
  result: Extract<LookupResult, { kind: "word" }>;
  t: Messages;
}) {
  const profile = result.profile;
  const formEntries = Object.entries(profile.forms).filter(
    ([, value]) => typeof value === "string" && value.length > 0,
  );
  return (
    <div className="space-y-4">
      <div className="grid gap-3 sm:grid-cols-2">
        <div className="rounded-md bg-paper p-3">
          <p className="mb-1 text-xs font-semibold text-moss">{t.source}</p>
          <p className="text-xl font-semibold text-ink">{profile.source}</p>
        </div>
        <div className="rounded-md bg-white p-3 ring-1 ring-line">
          <p className="mb-1 text-xs font-semibold text-amber">{t.translation}</p>
          <p className="text-xl font-semibold text-ink">{profile.translated}</p>
        </div>
      </div>
      <div className="grid gap-3 sm:grid-cols-2">
        <div className="rounded-md bg-paper p-3">
          <p className="mb-1 text-xs font-semibold text-moss">{t.wordBaseForm}</p>
          <p className="text-base font-medium text-ink">{profile.lemma}</p>
        </div>
        <div className="rounded-md bg-paper p-3">
          <p className="mb-1 text-xs font-semibold text-moss">Phonetic</p>
          <p className="text-base font-medium text-ink">
            {profile.phonetics.uk || profile.phonetics.us || "-"}
          </p>
        </div>
      </div>

      {profile.examTags.length > 0 ? (
        <div className="flex flex-wrap gap-2">
          {profile.examTags.map((tag) => (
            <span key={tag} className="rounded-md bg-moss/10 px-2 py-1 text-xs font-medium text-moss">
              {tag}
            </span>
          ))}
        </div>
      ) : null}

      {profile.definitions.length > 0 ? (
        <WordSection title={t.dictionaryDetail}>
          <div className="space-y-2">
            {profile.definitions.map((definition, index) => (
              <div key={`${definition.source}-${index}`} className="rounded-md bg-paper p-3">
                <span className="mr-2 text-xs font-semibold text-amber">
                  {definition.partOfSpeech}
                </span>
                <span className="text-sm text-ink">{definition.meaning}</span>
                <span className="ml-2 text-xs text-moss">[{definition.source}]</span>
              </div>
            ))}
          </div>
        </WordSection>
      ) : (
        <p className="rounded-md bg-paper p-3 text-sm text-moss">{t.dictionaryMiss}</p>
      )}

      {profile.memoryHint ? (
        <WordSection title={`${t.memoryHint} · ${t.aiGenerated}`}>
          <p className="rounded-md bg-paper p-3 text-sm text-ink">{profile.memoryHint}</p>
        </WordSection>
      ) : null}

      {formEntries.length > 0 ? (
        <WordSection title={t.forms}>
          <div className="grid gap-2 sm:grid-cols-2">
            {formEntries.map(([name, value]) => (
              <div key={name} className="rounded-md bg-paper px-3 py-2 text-sm">
                <span className="mr-2 text-moss">{name}</span>
                <span className="font-medium text-ink">{value}</span>
              </div>
            ))}
          </div>
        </WordSection>
      ) : null}

      {profile.phrases.length > 0 ? (
        <WordSection title={t.phrases}>
          <div className="space-y-2">
            {profile.phrases.map((phrase) => (
              <div key={phrase.phrase} className="rounded-md bg-paper p-3 text-sm">
                <p className="font-medium text-ink">{phrase.phrase}</p>
                <p className="mt-1 text-moss">{phrase.meaning}</p>
              </div>
            ))}
          </div>
        </WordSection>
      ) : null}

      {profile.examples.length > 0 ? (
        <WordSection title={t.examples}>
          <div className="space-y-2">
            {profile.examples.map((example) => (
              <div key={`${example.en}-${example.source}`} className="rounded-md bg-paper p-3 text-sm">
                <p className="text-ink">{example.en}</p>
                {example.zh ? <p className="mt-1 text-moss">{example.zh}</p> : null}
              </div>
            ))}
          </div>
        </WordSection>
      ) : null}

      <WordList title={t.synonyms} values={profile.synonyms} />
      <WordList title={t.antonyms} values={profile.antonyms} />

      {profile.importedCards.length > 0 ? (
        <WordSection title={t.importedDictionaries}>
          <div className="space-y-2">
            {profile.importedCards.map((card) => (
              <div key={`${card.dictionaryId}-${card.headword}`} className="rounded-md bg-paper p-3">
                <p className="mb-1 text-xs font-semibold text-moss">
                  {card.dictionaryName} · {card.headword}
                </p>
                <p className="line-clamp-6 whitespace-pre-wrap text-sm text-ink">{card.plainText}</p>
              </div>
            ))}
          </div>
        </WordSection>
      ) : null}

      {profile.sources.length > 0 ? (
        <p className="border-t border-line pt-3 text-xs text-moss">
          {t.sources}: {profile.sources.join(" / ")}
        </p>
      ) : null}
    </div>
  );
}

function WordSection({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div className="border-t border-line pt-4">
      <p className="mb-3 text-sm font-semibold text-ink">{title}</p>
      {children}
    </div>
  );
}

function WordList({ title, values }: { title: string; values: WordProfile["synonyms"] }) {
  if (values.length === 0) return null;
  return (
    <WordSection title={title}>
      <div className="flex flex-wrap gap-2">
        {values.map((value) => (
          <span key={value} className="rounded-md bg-paper px-2 py-1 text-sm text-ink">
            {value}
          </span>
        ))}
      </div>
    </WordSection>
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
