import { spawnSync } from 'node:child_process';
import { createHash, randomUUID } from 'node:crypto';
import { chmodSync, copyFileSync, existsSync, mkdirSync, mkdtempSync, readFileSync, readdirSync, readlinkSync, renameSync, rmSync, statSync, symlinkSync, writeFileSync } from 'node:fs';
import { homedir, tmpdir } from 'node:os';
import { basename, dirname, join, resolve } from 'node:path';
import { parseArgs } from 'node:util';
import { ROOT, isMain, readJSON, type Schema } from '../../scripts/common.ts';
import { calendar, contacts, drive, mail, mailDelivery, sheets, tasks } from './scenarios.ts';

export { ROOT };
export const SERVICES = ['calendar', 'mail', 'drive', 'sheets', 'tasks', 'contacts'];
export const SCHEMAS = Object.fromEntries(SERVICES.map(service => [service, readJSON(join(ROOT, 'schemas', `${service}.schema.json`))]));
export const WRAPPERS = readJSON(join(ROOT, 'schemas/wrappers.schema.json'));
export const CATALOG = new Set(Object.entries(SCHEMAS).flatMap(([service, schema]) => Object.keys(schema.$defs).map(command => `${service}.${command}`)));
export const WRAPPER_CATALOG = new Set(Object.keys(WRAPPERS.$defs).filter(name => name.includes('.')));
const EXE = process.platform === 'win32' ? '.exe' : '';
export class Failure extends Error {}
export interface CleanupJob { scope: string; service: string; command: string; parameters: Schema }
interface RunState extends Schema { checks: Schema[]; pendingCleanup: CleanupJob[]; services: Record<string, Schema> }
export interface ProcessResult { status: number | null; stdout: string; stderr: string; error?: NodeJS.ErrnoException }
export type Execute = (argv: string[], options: { env?: NodeJS.ProcessEnv; timeout?: number; cwd?: string }) => ProcessResult;
const executeProcess: Execute = ([command, ...args], options) => {
  const result = spawnSync(command, args, { ...options, encoding: 'utf8', maxBuffer: 64 * 1024 * 1024, windowsHide: true });
  return { status: result.status, stdout: result.stdout ?? '', stderr: result.stderr ?? '', error: result.error };
};
const sleep = (seconds: number): void => { if (seconds > 0) Atomics.wait(new Int32Array(new SharedArrayBuffer(4)), 0, 0, seconds * 1000); };
export function identifier(value: any, field = 'id'): string {
  if (!value || typeof value[field] !== 'string' || !value[field]) throw new Failure(`fixture response has no ${field}`);
  return value[field];
}
export const records = (value: any, key: string): Schema[] => Array.isArray(value) ? value : Array.isArray(value?.[key]) ? value[key] : [];
export function decode(text: string): any {
  const pages: any[] = [];
  while ((text = text.trimStart())) {
    let end = 0;
    if (text[0] === '{' || text[0] === '[' || text[0] === '"') {
      let depth = 0, quoted = false, escaped = false;
      const stringOnly = text[0] === '"';
      for (let i = 0; i < text.length; i++) {
        const ch = text[i];
        if (quoted) {
          if (escaped) escaped = false;
          else if (ch === '\\') escaped = true;
          else if (ch === '"') { quoted = false; if (stringOnly) { end = i + 1; break; } }
        } else if (ch === '"') quoted = true;
        else if (ch === '{' || ch === '[') depth++;
        else if (ch === '}' || ch === ']') { depth--; if (depth === 0) { end = i + 1; break; } }
      }
    } else end = /^(?:null|true|false|-?(?:0|[1-9]\d*)(?:\.\d+)?(?:[eE][+-]?\d+)?)/.exec(text)?.[0].length ?? 0;
    if (!end) throw new Failure('command returned invalid JSON');
    pages.push(JSON.parse(text.slice(0, end))); text = text.slice(end);
  }
  if (!pages.length) throw new Failure('successful command returned no JSON');
  const result = pages[0];
  for (const page of pages.slice(1)) {
    if (!result || typeof result !== 'object' || Array.isArray(result) || !page || typeof page !== 'object' || Array.isArray(page)) throw new Failure('inconsistent paginated JSON');
    for (const [key, value] of Object.entries(page)) {
      if (Array.isArray(value) && Array.isArray(result[key])) result[key].push(...value);
      else Object.defineProperty(result, key, { value, enumerable: true, writable: true, configurable: true });
    }
  }
  return result;
}
export function atomicJSON(path: string, value: unknown): void {
  const temporary = join(dirname(path), `.tmp-${randomUUID()}`);
  try { writeFileSync(temporary, JSON.stringify(value, null, 2) + '\n', { mode: 0o600, flag: 'wx' }); renameSync(temporary, path); }
  finally { rmSync(temporary, { force: true }); }
}
export function stableJSON(value: any): string {
  const sort = (value: any): any => Array.isArray(value) ? value.map(sort) : value && typeof value === 'object' ? Object.fromEntries(Object.keys(value).sort().map(key => [key, sort(value[key])])) : value;
  return JSON.stringify(sort(value));
}
export const jobKey = (item: Pick<CleanupJob, 'service' | 'command' | 'parameters'>): string => JSON.stringify([item.service, item.command, stableJSON(item.parameters)]);
const lines = (path: string): string[] => existsSync(path) ? readFileSync(path, 'utf8').split(/\r?\n/).filter(Boolean) : [];
export class Run {
  marker = 'joom-test-' + randomUUID().replaceAll('-', '').slice(0, 16);
  trace: string; errors: string; lastCall = 0; commands = 0; stopped = false; cleaning = false;
  context: Schema = {}; peer: string | undefined; case = 'full'; state: RunState;
  execute: Execute; pause: (seconds: number) => void;
  constructor(public account: string, public binary: string, public directory: string, public report: string, public services: string[], options: { execute?: Execute; sleep?: (seconds: number) => void } = {}) {
    this.execute = options.execute ?? executeProcess; this.pause = options.sleep ?? sleep;
    this.trace = join(directory, 'commands.jsonl'); this.errors = join(directory, 'errors.jsonl');
    this.state = { account, run: this.marker, startedAt: new Date().toISOString(), binarySha256: createHash('sha256').update(readFileSync(binary)).digest('hex'), authenticationPolicy: 'Existing session only. Token refresh, sign-in and HTTP retries disabled.', checks: [], services: {}, pendingCleanup: [] };
    this.save();
  }
  save(): void {
    const wire = lines(this.trace).map(line => JSON.parse(line)); this.state.wireCommands = wire;
    const direct = new Set(wire.filter(event => event.status === 'passed').map(event => `${event.service}.${event.command}`));
    const wrappers = new Set(this.state.checks.filter(event => event.status === 'passed' && event.interface !== 'joom').map(event => `${event.interface}.${event.command}`));
    this.state.coverage = { directPassed: [...CATALOG].filter(name => direct.has(name)).sort(), directMissing: [...CATALOG].filter(name => !direct.has(name)).sort(), wrappersPassed: [...WRAPPER_CATALOG].filter(name => wrappers.has(name)).sort(), wrappersMissing: [...WRAPPER_CATALOG].filter(name => !wrappers.has(name)).sort() };
    this.state.commands = this.commands; this.state.stopped = this.stopped; atomicJSON(this.report, this.state);
  }
  call = (interfaceName: string, command: string, params: Schema, service?: string, absenceOK = false): any => {
    if (this.stopped) throw new Failure('account run stopped; no further requests allowed');
    if (this.commands >= 300 + (this.cleaning ? 40 : 0)) { this.stopped = true; throw new Failure('command budget exhausted'); }
    this.pause(Math.max(0, 1 - (Number(process.hrtime.bigint()) / 1e9 - this.lastCall)));
    const request = join(this.directory, 'request.json'); atomicJSON(request, params);
    const fixtureResult = join(this.directory, `fixture-result-${this.commands + 1}.json`);
    const argv = interfaceName === 'fixture-setup' ? [join(dirname(this.binary), 'joom-fixtures' + EXE), '--exact', 'auth::live::graph_fixture_operation', '--ignored', '--nocapture'] : interfaceName === 'joom' ? [this.binary, '--json', '--timeout', '15', service!, this.account, command, '--file', request] : [join(dirname(this.binary), interfaceName + EXE), this.account, command, `file:${request}`, 'json:true', 'timeout:15'];
    const event: Schema = { interface: interfaceName, service: service ?? null, command, parameters: params, status: 'running', cleanup: this.cleaning };
    this.state.checks.push(event); this.commands++; this.save();
    const env: NodeJS.ProcessEnv = { ...process.env, JOOM_LIVE_SESSION_ONLY: '1', JOOM_LIVE_COMMAND_REPORT: this.trace, JOOM_LIVE_ERROR_REPORT: this.errors };
    if (this.cleaning && service === 'drive' && command === 'files.delete') env.JOOM_LIVE_CLEANUP_MARKER = this.marker;
    if (interfaceName === 'fixture-setup') Object.assign(env, { JOOM_LIVE_FIXTURE_REQUEST: request, JOOM_LIVE_FIXTURE_REPORT: fixtureResult });
    const before = lines(this.errors).length;
    try {
      const output = this.execute(argv, { timeout: 40000, env });
      if (output.error?.code === 'ETIMEDOUT') { this.stopped = true; throw new Failure('40-second command deadline exceeded; no retry'); }
      if (output.error) { this.stopped = true; throw new Failure(output.error.message); }
      const fixture = interfaceName === 'fixture-setup' && existsSync(fixtureResult) ? readJSON(fixtureResult) : undefined;
      if (output.status !== 0) {
        let error: string;
        try { error = fixture ? String(fixture.error) : String(JSON.parse(output.stderr).error); }
        catch { error = interfaceName === 'fixture-setup' ? output.stderr.trim().slice(0, 1000) : 'command failed without a structured error'; }
        const diagnostics = lines(this.errors).slice(before); if (diagnostics.length) event.providerError = JSON.parse(diagnostics.at(-1)!);
        if (fixture?.requests?.length) event.providerError = fixture.requests.at(-1);
        if (absenceOK && error.includes('HTTP 404')) { event.status = 'already-absent'; return null; }
        if (['HTTP 401','HTTP 403','HTTP 429','HTTP 5','no authentication or refresh attempted','timed out','error sending request','keychain','master key','credential missing'].some(text => error.includes(text))) this.stopped = true;
        throw new Failure(error);
      }
      const value = fixture ? fixture.result : decode(output.stdout); event.status = 'passed'; return value;
    } catch (error) {
      event.status = 'failed'; event.error = error instanceof Error ? error.message : String(error); throw new Failure(event.error);
    } finally { this.lastCall = Number(process.hrtime.bigint()) / 1e9; this.save(); }
  };
  api = (service: string, command: string, params: Schema = {}, absenceOK = false): any => {
    params = { ...params }; const properties = SCHEMAS[service].$defs[command].properties;
    if ('max' in properties) params.max ??= 2; if ('maxPages' in properties) params.maxPages ??= 1;
    return this.call('joom', command, params, service, absenceOK);
  };
  wrapper = (name: string, command: string, params: Schema = {}): any => {
    params = { ...params }; if ('limit' in WRAPPERS.$defs[`${name}.${command}`].properties) params.limit ??= 2;
    return this.call(name, command, params);
  };
  fixture = (operation: string, params: Schema = {}): any => this.call('fixture-setup', operation, { account: this.account, operation, ...params });
  register(scope: string, service: string, command: string, parameters: Schema): CleanupJob {
    const item = { scope, service, command, parameters }, previous = this.state.pendingCleanup.find(job => jobKey(job) === jobKey(item));
    if (previous) return previous;
    this.state.pendingCleanup.push(item); this.save(); return item;
  }
  discard(item: CleanupJob): void { const index = this.state.pendingCleanup.indexOf(item); if (index !== -1) { this.state.pendingCleanup.splice(index, 1); this.save(); } }
  cleanup(scope: string): void {
    this.cleaning = true; let locked = false;
    try {
      for (const item of [...this.state.pendingCleanup].reverse()) {
        if (item.scope !== scope || this.stopped) continue;
        try { this.api(item.service, item.command, item.parameters, true); this.discard(item); }
        catch (error) { if (error instanceof Error && error.message.includes('HTTP 423')) { locked = true; continue; } this.stopped = true; break; }
      }
    } finally { this.stopped ||= locked; this.cleaning = false; this.save(); }
  }
  service(name: string, fn: (run: Run) => void): void {
    if (this.stopped) { this.state.services[name] = { status: 'not-run', reason: 'account run stopped' }; this.save(); return; }
    this.state.services[name] = { status: 'running' };
    try { fn(this); this.state.services[name] = { status: 'completed' }; }
    catch (error) { if (!(error instanceof Failure)) throw error; this.state.services[name] = { status: 'stopped', reason: error.message }; }
    finally { this.cleanup(name); }
    console.log(JSON.stringify({ account: this.account, service: name, ...this.state.services[name], directPassed: this.state.coverage.directPassed.length, wrappersPassed: this.state.coverage.wrappersPassed.length, pendingCleanup: this.state.pendingCleanup.length }));
  }
}

export function discoverMail(run: Run): Map<string, Schema> {
  const messages = new Map<string, Schema>(), counts: Schema = {};
  for (const scope of ['', 'in:inbox ', 'in:sent ']) {
    const value = run.api('mail', 'messages.list', { q: scope + 'subject:' + run.marker, max: 20 });
    let count = 0;
    for (const message of records(value, 'messages')) {
      const subject = (message.payload?.headers ?? []).find((header: Schema) => String(header.name ?? '').toLowerCase() === 'subject')?.value ?? '';
      if (subject.startsWith(run.marker)) { messages.set(identifier(message), message); count++; }
    }
    counts[scope.trim() || 'all'] = count;
  }
  run.state.mailDiscoveryCounts = counts; run.save(); return messages;
}
export function recover(run: Run, sourcePath: string): void {
  const original = readJSON(sourcePath);
  if (original.account !== run.account || !original.run.startsWith('joom-test-')) throw new Failure('cleanup report does not belong to the approved account and fixture run');
  const allowed: Record<string,string[]> = { mail: ['drafts.delete','labels.delete','messages.delete'], drive: ['files.delete','permissions.delete'], calendar: ['events.delete','calendars.delete'], tasks: ['tasks.delete','tasklists.delete'], contacts: ['people.delete-contact'] };
  for (const item of original.pendingCleanup) if (!allowed[item.service]?.includes(item.command)) throw new Failure('cleanup report contains an unapproved operation');
  run.marker = original.run; run.state.run = run.marker; run.state.recoverySource = sourcePath; run.state.pendingCleanup = original.pendingCleanup;
  const discovery = original.pendingMailDiscovery;
  if (discovery) { for (const messageID of discoverMail(run).keys()) run.register('mail-delivery','mail','messages.delete',{ id: messageID }); run.state.mailCleanupSearchCompleted = true; }
  for (const scope of [...new Set(run.state.pendingCleanup.map(item => item.scope))].sort()) run.cleanup(scope);
  original.pendingCleanup = run.state.pendingCleanup;
  if (!run.stopped && !run.state.pendingCleanup.length) {
    delete original.pendingMailDiscovery; original.cleanupResolvedBy = run.report;
    if (discovery && (original.mailRecipient ?? run.account).endsWith('@gmail.com')) original.manualCleanup = [{ kind: 'self-addressed-email', subjectPrefix: run.marker, note: 'Microsoft copies removed. Up to four delivered messages may remain in the external Gmail inbox; that inbox was not accessed.' }];
  }
  atomicJSON(sourcePath, original);
  const completed = new Set(run.state.checks.filter(event => event.cleanup && ['passed','already-absent'].includes(event.status)).map(event => jobKey(event as CleanupJob)));
  for (const file of readdirSync(dirname(sourcePath)).filter(name => /^report-.*\.json$/.test(name))) {
    const path = join(dirname(sourcePath), file), previous = readJSON(path);
    if (previous.account !== run.account || previous.run !== run.marker) continue;
    previous.pendingCleanup = previous.pendingCleanup.filter((item: CleanupJob) => !completed.has(jobKey(item)));
    if (!previous.pendingCleanup.length && completed.size) previous.cleanupResolvedBy = run.report;
    atomicJSON(path, previous);
  }
  run.save();
}

function checked(argv: string[], options: { cwd?: string } = {}): ProcessResult {
  const result = executeProcess(argv, options);
  if (result.error) throw result.error;
  if (result.status !== 0) throw new Failure(`${basename(argv[0])} failed: ${result.stderr.trim()}`);
  return result;
}
export function main(args = process.argv.slice(2)): number {
  const { values } = parseArgs({ args, options: { accounts: { type: 'string', default: process.env.JOOM_LIVE_ACCOUNTS ?? process.env.JOOM_LIVE_ACCOUNT ?? '' }, case: { type: 'string', default: process.env.JOOM_LIVE_CASE ?? 'full' }, 'cleanup-report': { type: 'string' }, peer: { type: 'string' }, services: { type: 'string', default: process.env.JOOM_LIVE_SERVICES ?? SERVICES.join(',') }, help: { type: 'boolean', short: 'h' } } });
  if (values.help) { console.log('Usage: runner.ts [--accounts EMAIL,...] [--services calendar,mail,drive,sheets,tasks,contacts] [--case full|drive-files|office-files|mail-draft|drive-metadata|regressions] [--cleanup-report PATH] [--peer EMAIL]'); return 0; }
  const accounts = values.accounts.split(',').map(value => value.trim().toLowerCase()).filter(Boolean), selected = values.services.split(',').map(value => value.trim()).filter(Boolean);
  if (!accounts.length || new Set(accounts).size !== accounts.length || accounts.some(value => !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(value))) throw new Failure('set JOOM_LIVE_ACCOUNT or JOOM_LIVE_ACCOUNTS to the explicitly approved accounts');
  if (!selected.length || new Set(selected).size !== selected.length || selected.some(value => !SERVICES.includes(value))) throw new Failure('choose supported Microsoft services');
  if (!['full','drive-files','office-files','mail-draft','drive-metadata','regressions'].includes(values.case)) throw new Failure('unknown live-test case');
  const cleanupRequested = values['cleanup-report'] !== undefined;
  if (cleanupRequested && (!values['cleanup-report'] || !existsSync(values['cleanup-report']) || !statSync(values['cleanup-report']).isFile())) throw new Failure('cleanup report must be an existing JSON file');
  if (cleanupRequested && accounts.length !== 1) throw new Failure('cleanup requires exactly one approved account');
  if (values.peer && !/^[^\s@]+@[^\s@]+\.[^\s@]+$/.test(values.peer)) throw new Failure('peer must be an explicitly approved account email');
  const output = join(ROOT,'target/microsoft-live'); mkdirSync(output,{ recursive:true,mode:0o700 });
  const lock = join(output,'.run.lock.d');
  try { mkdirSync(lock, { mode: 0o700 }); } catch { throw new Failure('another live run holds target/microsoft-live/.run.lock.d; do not run concurrent account checks'); }
  const summaries: Schema[] = [];
  try {
    const fromPath = executeProcess([process.platform === 'win32' ? 'where' : 'which','cargo'],{});
    const cargo = fromPath.status === 0 ? fromPath.stdout.trim().split(/\r?\n/)[0] : join(homedir(),'.cargo/bin','cargo' + EXE);
    checked([cargo,'build','--locked','--offline','--features','live-test-diagnostics'],{ cwd:ROOT });
    const snapshot = join(ROOT,'target/live-test-bin'); mkdirSync(snapshot,{ recursive:true });
    const binary = join(snapshot,'joom' + EXE); copyFileSync(join(ROOT,'target/debug','joom' + EXE), binary); chmodSync(binary,0o755);
    const artifacts = checked([cargo,'test','--locked','--offline','--features','live-test-diagnostics','--bin','joom','--no-run','--message-format=json'],{ cwd:ROOT });
    const executables = artifacts.stdout.trim().split(/\r?\n/).map(line => JSON.parse(line)).filter(record => record.reason === 'compiler-artifact' && record.executable && record.profile?.test).map(record => record.executable);
    if (executables.length !== 1) throw new Failure('expected one native fixture-test executable');
    const fixtures = join(snapshot,'joom-fixtures' + EXE); copyFileSync(executables[0],fixtures); chmodSync(fixtures,0o755);
    for (const name of Object.keys(WRAPPERS['x-wrappers'])) {
      const target = join(snapshot,name + EXE);
      if (process.platform === 'win32') { copyFileSync(binary,target); continue; }
      try { if (readlinkSync(target) === 'joom') continue; } catch { /* only create absent paths below */ }
      symlinkSync('joom',target);
    }
    const scenarios: Record<string,(run:Run)=>void> = { calendar,mail,drive,sheets,tasks,contacts };
    for (const account of accounts) {
      const report = join(output,`report-${account.replace(/[^a-z0-9]/g,'-')}-${randomUUID().slice(0,8)}.json`), temporary = mkdtempSync(join(tmpdir(),'joom-live-'));
      const run = new Run(account,binary,temporary,report,selected); run.case = values.case; run.peer = values.peer || accounts.find(peer => peer !== account);
      try {
        const profile = run.api('mail','profile');
        if ((profile.emailAddress ?? '').toLowerCase() !== account) { run.stopped = true; throw new Failure('signed-in Microsoft identity does not match the approved account'); }
        run.state.identityVerified = true;
        if (cleanupRequested) recover(run,resolve(values['cleanup-report']!));
        else { for (const service of selected) run.service(service,scenarios[service]); run.cleanup('office'); if (run.state.pendingMailDiscovery && !run.stopped) run.service('mail-delivery',mailDelivery); }
      } catch (error) { if (!(error instanceof Failure)) throw error; run.state.error = error.message; }
      finally { run.save(); rmSync(temporary,{ recursive:true,force:true }); }
      const summary = { account,report,commands:run.commands,directPassed:run.state.coverage.directPassed.length,wrappersPassed:run.state.coverage.wrappersPassed.length,pendingCleanup:run.state.pendingCleanup.length,pendingMailDiscovery:Boolean(run.state.pendingMailDiscovery),stopped:run.stopped,failures:run.state.checks.filter(event => event.status === 'failed').length,incomplete:SERVICES.every(service => selected.includes(service)) && !cleanupRequested && Boolean(run.state.coverage.directMissing.length || run.state.coverage.wrappersMissing.length) };
      summaries.push(summary); console.log(JSON.stringify(summary));
    }
    atomicJSON(join(output,'latest.json'),summaries);
  } finally { rmSync(lock,{ recursive:true,force:true }); }
  return Number(summaries.some(summary => summary.stopped || summary.pendingCleanup || summary.pendingMailDiscovery || summary.failures || summary.incomplete));
}
if (isMain(import.meta.url)) { try { process.exitCode = main(); } catch (error) { console.error(error instanceof Error ? error.message : String(error)); process.exitCode = 1; } }
