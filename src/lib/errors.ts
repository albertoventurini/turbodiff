// One-line descriptions of backend errors, for notices (PRD §7).
import type { AppError } from './types';

export function errorTitle(error: AppError): string {
  switch (error.kind) {
    case 'notARepository':
      return 'This folder is not a git repository';
    case 'refNotFound':
      return 'Invalid ref';
    case 'noRepository':
      return 'No repository is open';
    case 'fileNotFound':
      return 'File not found';
    case 'cancelled':
      return 'Cancelled';
    case 'io':
    case 'internal':
      return 'Something went wrong';
  }
}

/** What the error is about: a path, a ref name or a message. */
export function errorDetail(error: AppError): string | null {
  switch (error.kind) {
    case 'notARepository':
    case 'fileNotFound':
      return error.path;
    case 'refNotFound':
      return error.name;
    case 'io':
    case 'internal':
      return error.message;
    default:
      return null;
  }
}
