import { existsSync, mkdirSync, readdirSync } from 'node:fs';
import { delimiter, dirname, join, resolve } from 'node:path';
import { parseArgs } from 'node:util';
import { isMain, readJSON, type Schema } from '../../scripts/common.ts';
import { ROOT, CATALOG, WRAPPER_CATALOG, SERVICES, atomicJSON, jobKey, stableJSON, type CleanupJob } from './runner.ts';

export function aggregate(paths: string[]): Record<string, Schema> {
  const accounts: Record<string,Schema> = {};
  for (const path of paths) {
    const data = readJSON(path);
    const account = accounts[data.account] ??= { reports:[],directPassed:new Set<string>(),wrappersPassed:new Set<string>(),failures:[],pendingCleanup:[],manualCleanup:[],binaryFingerprints:new Set<string>(),resolvedCleanup:new Set<string>(),harnessFailures:[],completeRuns:[] };
    account.reports.push(resolve(path)); account.binaryFingerprints.add(data.binarySha256);
    for (const name of data.coverage.directPassed) account.directPassed.add(name);
    for (const name of data.coverage.wrappersPassed) account.wrappersPassed.add(name);
    account.failures.push(...data.checks.filter((event:Schema) => event.status === 'failed').map((event:Schema) => ({ report:resolve(path),...event })));
    account.pendingCleanup.push(...data.pendingCleanup);
    for (const event of data.checks) if (event.cleanup && ['passed','already-absent'].includes(event.status)) account.resolvedCleanup.add(jobKey(event as CleanupJob));
    if (data.pendingMailDiscovery) account.pendingCleanup.push({ report:resolve(path),mailDiscovery:data.pendingMailDiscovery });
    account.manualCleanup.push(...(data.manualCleanup ?? []));
    if (!data.coverage.directMissing.length && !data.coverage.wrappersMissing.length && !data.stopped && !data.error && !data.pendingCleanup.length && !data.pendingMailDiscovery && !data.checks.some((event:Schema) => event.status === 'failed') && SERVICES.every(service => data.services?.[service]?.status === 'completed')) account.completeRuns.push({ report:resolve(path),binarySha256:data.binarySha256,startedAt:data.startedAt,commands:data.commands });
  }
  for (const account of Object.values(accounts)) {
    const pending = new Map<string,Schema>();
    for (const item of account.pendingCleanup) pending.set('service' in item ? jobKey(item) : stableJSON(item),item);
    account.pendingCleanup = [...pending].filter(([key]) => !account.resolvedCleanup.has(key)).map(([,item]) => item); delete account.resolvedCleanup;
    account.harnessFailures = account.failures.filter((event:Schema) => String(event.error ?? '').includes('Expecting property name enclosed'));
    account.failures = account.failures.filter((event:Schema) => !account.harnessFailures.includes(event));
    account.directMissing = [...CATALOG].filter(name => !account.directPassed.has(name)).sort(); account.wrappersMissing = [...WRAPPER_CATALOG].filter(name => !account.wrappersPassed.has(name)).sort();
    for (const key of ['directPassed','wrappersPassed','binaryFingerprints']) account[key] = [...account[key]].sort();
  }
  return accounts;
}
export function main(args = process.argv.slice(2)): void {
  const { values,positionals } = parseArgs({ args,allowPositionals:true,options:{ help:{ type:'boolean',short:'h' } } });
  if (values.help) { console.log('Usage: coverage.ts [REPORT.json ...]'); return; }
  let paths = positionals.length ? positionals : (process.env.JOOM_LIVE_REPORTS ?? '').split(delimiter).filter(Boolean);
  const directory = join(ROOT,'target/microsoft-live');
  if (!paths.length && existsSync(directory)) paths = readdirSync(directory).filter(name => /^report-.*\.json$/.test(name)).sort().map(name => join(directory,name));
  const accounts = aggregate(paths);
  for (const [name,account] of Object.entries(accounts)) console.log(`${name}: ${account.directPassed.length}/${CATALOG.size} direct, ${account.wrappersPassed.length}/${WRAPPER_CATALOG.size} wrappers, ${account.failures.length} historical failures, ${account.pendingCleanup.length} pending cleanup`);
  const output = join(directory,'coverage.json'); mkdirSync(dirname(output),{ recursive:true });
  atomicJSON(output,{ note:'Successful calls across recorded runs and binary versions. Missing and failed commands are not successful coverage.',accounts }); console.log(output);
}
if (isMain(import.meta.url)) main();
