import { readFileSync, readdirSync } from 'node:fs';
import { dirname, resolve } from 'node:path';
import { fileURLToPath, pathToFileURL } from 'node:url';

// The pinned schemas contain arbitrary JSON Schema extension properties.
export type Schema = Record<string, any>;
export const ROOT = resolve(dirname(fileURLToPath(import.meta.url)), '..');
export const readJSON = (path: string): Schema => JSON.parse(readFileSync(path, 'utf8'));
export const clone = <T>(value: T): T => structuredClone(value);
export const sortedEntries = <T>(value: Record<string, T>): [string, T][] =>
  Object.entries(value).sort(([a], [b]) => a < b ? -1 : a > b ? 1 : 0);
export const jsonFiles = (directory: string): string[] =>
  readdirSync(directory).filter(name => name.endsWith('.json')).sort();
export const isMain = (url: string): boolean => Boolean(process.argv[1]) && pathToFileURL(resolve(process.argv[1])).href === url;
export function checkFlag(args = process.argv.slice(2)): boolean {
  if (args.includes('--help') || args.includes('-h')) {
    console.log('Usage: ' + (process.argv[1] ?? 'generator.ts') + ' [--check]');
    process.exit(0);
  }
  if (args.some(arg => arg !== '--check')) throw new Error('usage: [--check]');
  return args.includes('--check');
}
