import Link from 'next/link';

export default function HomePage() {
  return (
    <main className="flex flex-col items-center justify-center flex-1 px-6 py-16 text-center">
      <div className="max-w-3xl">
        <h1 className="mb-4 text-5xl font-bold tracking-tight">Tivana</h1>
        <p className="mb-2 text-xl text-fd-muted-foreground">
          Perception-first browser protocol for AI agents.
        </p>
        <p className="mb-8 text-lg text-fd-muted-foreground">
          Perceive the page. Reason about it. Act on it.
        </p>

        <div className="mb-12 flex flex-col items-center gap-4 sm:flex-row sm:justify-center">
          <Link
            href="/docs"
            className="rounded-lg bg-fd-primary px-6 py-3 text-sm font-medium text-fd-primary-foreground transition-colors hover:bg-fd-primary/90"
          >
            Get Started
          </Link>
          <a
            href="https://github.com/Mostrom-LLC/tivana"
            target="_blank"
            rel="noopener noreferrer"
            className="rounded-lg border border-fd-border px-6 py-3 text-sm font-medium transition-colors hover:bg-fd-accent"
          >
            GitHub →
          </a>
        </div>

        <div className="mb-12 rounded-lg border border-fd-border bg-fd-card p-6 text-left">
          <pre className="overflow-x-auto text-sm">
            <code>{`import { TivanaClient } from "tivana";

const client = new TivanaClient();
await client.connect();
await client.createSession();

// Perceive
const elements = await client.elements();
const button = elements.find(e => e.role === "button");

// Act
if (button) await client.click(button.id);`}</code>
          </pre>
        </div>

        <div className="grid gap-6 text-left sm:grid-cols-3">
          <div className="rounded-lg border border-fd-border p-5">
            <h3 className="mb-2 font-semibold">👁️ Perceive</h3>
            <p className="text-sm text-fd-muted-foreground">
              Semantic elements, roles, labels, bounds, visibility — streaming in real-time.
            </p>
          </div>
          <div className="rounded-lg border border-fd-border p-5">
            <h3 className="mb-2 font-semibold">🧠 Reason</h3>
            <p className="text-sm text-fd-muted-foreground">
              Your agent gets structured page state and decides what to do. No selectors, no scripts.
            </p>
          </div>
          <div className="rounded-lg border border-fd-border p-5">
            <h3 className="mb-2 font-semibold">🤲 Act</h3>
            <p className="text-sm text-fd-muted-foreground">
              Click, type, scroll, navigate — by semantic element ID. Human-observable.
            </p>
          </div>
        </div>

        <p className="mt-12 text-xs text-fd-muted-foreground">
          MIT Licensed · Built by{' '}
          <a href="https://github.com/Mostrom-LLC" className="underline">
            Mostrom LLC
          </a>
        </p>
      </div>
    </main>
  );
}
