// The UI thread's side of the highlight worker. If the worker can't start or
// fails, every file is shown as plain text from then on.
import type { Reply, Request } from './highlight.worker';
import type { LanguageId } from './languages';
import type { Tokens } from './tokenize';

function startWorker(): Worker {
  return new Worker(new URL('./highlight.worker.ts', import.meta.url), { type: 'module' });
}

export class Highlighter {
  #worker: Worker | null = null;
  #broken = false;
  #next = 0;
  #pending = new Map<number, (tokens: Tokens | null) => void>();

  constructor(private start: () => Worker = startWorker) {}

  /** Whether the worker failed, so nothing more gets highlighted. */
  get broken(): boolean {
    return this.#broken;
  }

  /** The tokens of `text`, or `null` if it couldn't be highlighted. */
  highlight(text: string, lang: LanguageId, theme: string): Promise<Tokens | null> {
    const worker = this.#ensure();
    if (!worker) return Promise.resolve(null);
    const id = this.#next++;
    return new Promise((resolve) => {
      this.#pending.set(id, resolve);
      worker.postMessage({ id, text, lang, theme } satisfies Request);
    });
  }

  /** Stops the worker. */
  stop(): void {
    this.#worker?.terminate();
    this.#worker = null;
    this.#settleAll();
  }

  #ensure(): Worker | null {
    if (this.#broken) return null;
    if (this.#worker) return this.#worker;
    try {
      const worker = this.start();
      worker.onmessage = ({ data }: MessageEvent<Reply>) => {
        const resolve = this.#pending.get(data.id);
        this.#pending.delete(data.id);
        resolve?.('tokens' in data ? data.tokens : null);
      };
      worker.onerror = worker.onmessageerror = () => this.#fail();
      this.#worker = worker;
      return worker;
    } catch {
      this.#fail();
      return null;
    }
  }

  #fail(): void {
    this.#broken = true;
    this.stop();
  }

  #settleAll(): void {
    for (const resolve of this.#pending.values()) resolve(null);
    this.#pending.clear();
  }
}
