import { join } from 'node:path';
import { ROOT, readJSON, type Schema } from './common.ts';

export const TITLES: Record<string, string> = { mail: 'Outlook Mail', drive: 'OneDrive', calendar: 'Outlook Calendar', sheets: 'Excel', tasks: 'Microsoft To Do', contacts: 'Outlook Contacts' };
const ROUTES = new Set((readJSON(join(ROOT, 'tests/data/microsoft-routes.json')) as unknown as Schema[]).map(entry => entry.id));
const WRAPPERS = readJSON(join(ROOT, 'tests/data/microsoft-wrapper-support.json'));

export function commandNotice(source: Schema, name: string): string {
  const method = source.$defs[name]['x-api-method-id'];
  if (!ROUTES.has(method)) throw new Error(`No Microsoft route for ${method}`);
  return 'Microsoft mapping available. See [Microsoft support](../microsoft.md) for supported fields and provider limits.';
}
export function wrapperDescription(wrapper: string, original: string): string {
  return ({ jmdocs: 'List, share, and export existing Word files through OneDrive.', jmslides: 'List and export existing PowerPoint files through OneDrive.' } as Record<string,string>)[wrapper] ?? original;
}
export function wrapperNotice(wrapper: string, command: string): string {
  if (WRAPPERS[`${wrapper}.${command}`] !== 'mapped') throw new Error(`No Microsoft route for ${wrapper}.${command}`);
  if (command === 'export') return wrapper === 'jmsheets' ? 'Microsoft export formats are CSV and PDF.' : 'Microsoft export supports PDF only.';
  if (wrapper === 'jmdrive' && command === 'get') return 'Downloads the existing OneDrive file.';
  if (wrapper === 'jmdrive' && ['copy','put'].includes(command)) return 'File uploads are limited to 250 MiB. Folder copying is unavailable.';
  return '';
}
