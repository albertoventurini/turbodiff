import { describe, expect, it, vi } from 'vitest';
import { Highlighter } from './client';
import type { Reply, Request } from './highlight.worker';

/** A stand-in worker that answers each request with `answer`. */
class FakeWorker {
  onmessage: ((e: MessageEvent<Reply>) => void) | null = null;
  onerror: (() => void) | null = null;
  onmessageerror: (() => void) | null = null;
  sent: Request[] = [];
  terminate = vi.fn();

  postMessage(request: Request) {
    this.sent.push(request);
  }

  reply(data: Reply) {
    this.onmessage?.({ data } as MessageEvent<Reply>);
  }
}

const tokens = { styles: [], lines: [[[1, -1]]] as [number, number][][] };

describe('Highlighter', () => {
  it('starts the worker on first use and matches replies to requests', async () => {
    const worker = new FakeWorker();
    const start = vi.fn(() => worker as unknown as Worker);
    const highlighter = new Highlighter(start);
    expect(start).not.toHaveBeenCalled();

    const first = highlighter.highlight('a', 'typescript', 'github-light');
    const second = highlighter.highlight('b', 'rust', 'github-dark');
    expect(start).toHaveBeenCalledTimes(1);
    expect(worker.sent.map((r) => r.text)).toEqual(['a', 'b']);
    worker.reply({ id: worker.sent[1].id, error: 'no' });
    worker.reply({ id: worker.sent[0].id, tokens });
    expect(await first).toEqual(tokens);
    expect(await second).toBeNull();
    expect(highlighter.broken).toBe(false);
  });

  it('gives up for good when the worker fails', async () => {
    const worker = new FakeWorker();
    const highlighter = new Highlighter(() => worker as unknown as Worker);
    const pending = highlighter.highlight('a', 'typescript', 'github-light');
    worker.onerror!();
    expect(await pending).toBeNull();
    expect(highlighter.broken).toBe(true);
    expect(worker.terminate).toHaveBeenCalled();
    expect(await highlighter.highlight('b', 'typescript', 'github-light')).toBeNull();
    expect(worker.sent).toHaveLength(1);
  });

  it("gives up when the worker can't start", async () => {
    const highlighter = new Highlighter(() => {
      throw new Error('no workers here');
    });
    expect(await highlighter.highlight('a', 'typescript', 'github-light')).toBeNull();
    expect(highlighter.broken).toBe(true);
  });
});
