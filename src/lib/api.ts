// Typed wrappers around the generated IPC bindings in types.ts.
import { commands } from './types';

export function appVersion(): Promise<string> {
  return commands.appVersion();
}
