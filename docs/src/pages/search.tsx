import Layout from '@theme/Layout';
import Link from '@docusaurus/Link';
import useBaseUrl, {useBaseUrlUtils} from '@docusaurus/useBaseUrl';
import {useEffect, useMemo, useState} from 'react';
import styles from './index.module.css';

type SearchEntry = {
  title: string;
  url: string;
  locale: string;
  summary: string;
};

const normalize = (value: string) => value.toLocaleLowerCase();

export default function SearchPage() {
  const searchIndexUrl = useBaseUrl('/search-index.json');
  const {withBaseUrl} = useBaseUrlUtils();
  const [entries, setEntries] = useState<SearchEntry[]>([]);
  const [query, setQuery] = useState('');
  const [locale, setLocale] = useState('all');
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    fetch(searchIndexUrl)
      .then((response) => {
        if (!response.ok) {
          throw new Error(`search index returned ${response.status}`);
        }
        return response.json() as Promise<SearchEntry[]>;
      })
      .then((data) => {
        if (!cancelled) {
          setEntries(data);
          setError(null);
        }
      })
      .catch((reason: unknown) => {
        if (!cancelled) {
          setError(reason instanceof Error ? reason.message : 'failed to load search index');
        }
      });
    return () => {
      cancelled = true;
    };
  }, [searchIndexUrl]);

  const filteredEntries = useMemo(() => {
    const needle = normalize(query.trim());
    return entries.filter((entry) => {
      const localeMatches = locale === 'all' || entry.locale === locale;
      if (!localeMatches) return false;
      if (!needle) return true;
      return normalize(`${entry.title} ${entry.summary} ${entry.url}`).includes(needle);
    });
  }, [entries, locale, query]);

  return (
    <Layout title="Search" description="Search source-backed Tikeo documentation entrypoints.">
      <main className={styles.mainShell}>
        <section className={styles.hero}>
          <p className={styles.eyebrow}>Local docs search</p>
          <h1>Search Tikeo documentation</h1>
          <p>
            This page reads the committed <code>search-index.json</code> so the docs site has a
            deterministic search fallback before hosted DocSearch credentials are available.
          </p>
        </section>

        <section className={styles.quickstart}>
          <label>
            Search terms
            <input
              aria-label="Search terms"
              value={query}
              onChange={(event) => setQuery(event.target.value)}
              placeholder="jobs, Worker Tunnel, audit, API-Key..."
              style={{width: '100%', marginTop: 8, padding: '0.75rem 1rem', borderRadius: 12, border: '1px solid var(--ifm-color-emphasis-300)'}}
            />
          </label>
          <label style={{display: 'block', marginTop: 16}}>
            Locale
            <select
              aria-label="Locale"
              value={locale}
              onChange={(event) => setLocale(event.target.value)}
              style={{display: 'block', marginTop: 8, padding: '0.75rem 1rem', borderRadius: 12, border: '1px solid var(--ifm-color-emphasis-300)'}}
            >
              <option value="all">All locales</option>
              <option value="en">English</option>
              <option value="zh-CN">简体中文</option>
            </select>
          </label>
          {error ? <p role="alert" style={{color: 'var(--ifm-color-danger)'}}>Search index error: {error}</p> : null}
        </section>

        <section className={styles.cardGrid} aria-label="Search results">
          {filteredEntries.map((entry) => (
            <Link key={`${entry.locale}-${entry.url}`} className={styles.featureCard} to={withBaseUrl(entry.url)}>
              <span>{entry.locale}</span>
              <h2>{entry.title}</h2>
              <p>{entry.summary}</p>
            </Link>
          ))}
          {filteredEntries.length === 0 ? (
            <article className={styles.featureCard}>
              <span>No results</span>
              <h2>No indexed page matched.</h2>
              <p>Try a source-backed term such as jobs, Worker Tunnel, audit, API-Key, or workflows.</p>
            </article>
          ) : null}
        </section>
      </main>
    </Layout>
  );
}
