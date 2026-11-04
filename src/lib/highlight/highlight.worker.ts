// The highlight worker (TECH.md §7.4): tokenizes files off the UI thread.
import type { LanguageId } from './languages';
import { tokenize, type Tokens } from './tokenize';

export type Request = { id: number; text: string; lang: LanguageId; theme: string };
export type Reply = { id: number; tokens: Tokens } | { id: number; error: string };

self.onmessage = async ({ data }: MessageEvent<Request>) => {
  let reply: Reply;
  try {
    reply = { id: data.id, tokens: await tokenize(data.text, data.lang, data.theme) };
  } catch (err) {
    reply = { id: data.id, error: String(err) };
  }
  self.postMessage(reply);
};
