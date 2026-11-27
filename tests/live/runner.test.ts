import assert from 'node:assert/strict';
import { mkdtempSync, readFileSync, rmSync, writeFileSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { test } from 'node:test';
import { Run, Failure, decode, discoverMail, recover, atomicJSON, main, type Execute, type ProcessResult } from './runner.ts';

const result = (status = 0, stdout = 'null', stderr = ''): ProcessResult => ({ status,stdout,stderr });
function fixture(fn: (run: Run, root: string) => void, execute: Execute = () => result()): void {
  const root = mkdtempSync(join(tmpdir(),'joom-policy-'));
  try { const binary = join(root,'joom'); writeFileSync(binary,'fixture executable'); fn(new Run('test@example.com',binary,root,join(root,'report.json'),['mail'],{ execute,sleep:()=>{} }),root); }
  finally { rmSync(root,{ recursive:true,force:true }); }
}
test('mail discovery is bounded, deduplicated and fixture-only', () => fixture(run => {
  const own = { id:'owned-message',payload:{ headers:[{ name:'Subject',value:run.marker + '-direct-send' }] } }, foreign = { id:'other-message',payload:{ headers:[{ name:'Subject',value:'another message' }] } };
  const responses = [{ messages:[own,foreign] },{ messages:[own] },{ messages:[] }], calls: unknown[] = [];
  run.api = (service,command,parameters) => { calls.push([service,command,parameters]); return responses.shift(); };
  assert.deepEqual([...discoverMail(run).keys()],['owned-message']);
  assert.deepEqual(calls,['','in:inbox ','in:sent '].map(scope => ['mail','messages.list',{ q:scope + 'subject:' + run.marker,max:20 }]));
  const first = run.register('mail-delivery','mail','messages.delete',{ id:'owned-message' });
  assert.equal(run.register('mail-delivery','mail','messages.delete',{ id:'owned-message' }),first); assert.equal(run.state.pendingCleanup.length,1);
}));
test('fixture provider failure stops without retry', () => fixture((run,root) => {
  atomicJSON(join(root,'fixture-result-1.json'),{ error:'fixture operation HTTP 401',result:null,requests:[{ status:401,code:'InvalidAuthenticationToken' }] });
  let count = 0; run.execute = () => { count++; return result(1,'native test failed'); };
  assert.throws(()=>run.fixture('create-recurring-event'),Failure); assert.throws(()=>run.fixture('create-recurring-event'),Failure);
  assert.equal(count,1); assert.equal(run.state.checks[0].providerError.status,401);
}));
test('pretty and paginated JSON retains pages, quoting and primitive values', () => {
  assert.deepEqual(decode('{\n"emailAddress":"test@example.com"\n}'),{ emailAddress:'test@example.com' });
  assert.deepEqual(decode('{"items":[1]}\n{\n"items":[2]}'),{ items:[1,2] });
  assert.deepEqual(decode('{"items":["brace } and quote \\\"",{"nested":[1]}]}'),{ items:['brace } and quote "',{ nested:[1] }] });
  for (const value of ['null','false','0','"fixture"','[]']) assert.deepEqual(decode(value),JSON.parse(value));
  for (const invalid of ['','{','{"items":','{} trailing']) assert.throws(()=>decode(invalid));
});
test('authentication and throttling stop without retry or cleanup requests', () => {
  for (const code of [401,403,429]) fixture(run => {
    run.register('mail','mail','drafts.delete',{ id:'synthetic-draft' });
    let count = 0; run.execute = (_argv,options) => { count++; assert.equal(options.env?.JOOM_LIVE_SESSION_ONLY,'1'); return result(1,'',JSON.stringify({ error:`HTTP ${code}: rejected` })); };
    assert.throws(()=>run.api('mail','profile'),Failure); assert.throws(()=>run.api('mail','profile'),Failure); run.cleanup('mail');
    assert.equal(count,1); assert.equal(run.stopped,true); assert.equal(run.state.pendingCleanup.length,1);
  });
});
test('cleanup recovery clears duplicate ledgers with one deletion', () => fixture((run,root) => {
  const job = { scope:'office',service:'drive',command:'files.delete',parameters:{ fileId:'synthetic-file' } };
  const source = { account:run.account,run:'joom-test-fixture',pendingCleanup:[job] };
  const original = join(root,'report-original.json'), previous = join(root,'report-child.json'); atomicJSON(original,source); atomicJSON(previous,source);
  let count = 0; run.execute = () => { count++; return result(); }; recover(run,original);
  assert.equal(count,1); for (const path of [original,previous]) assert.deepEqual(JSON.parse(readFileSync(path,'utf8')).pendingCleanup,[]);
}));
test('locked fixtures are not retried and other cleanup still runs', () => fixture(run => {
  for (const id of ['unlocked-file','locked-file']) run.register('office','drive','files.delete',{ fileId:id });
  let count = 0; run.execute = () => ++count === 1 ? result(1,'','{"error":"HTTP 423: locked"}') : result(); run.cleanup('office');
  assert.equal(count,2); assert.deepEqual(run.state.pendingCleanup.map(job=>job.parameters.fileId),['locked-file']); assert.equal(run.stopped,true);
}));
test('deadline and command budget stop before any further requests', () => fixture(run => {
  let count = 0; run.execute = () => { count++; return { status:null,stdout:'',stderr:'',error:Object.assign(new Error('deadline'),{ code:'ETIMEDOUT' }) }; };
  assert.throws(()=>run.api('mail','profile'),/40-second command deadline/); assert.throws(()=>run.api('mail','profile'),/run stopped/); assert.equal(count,1);
  run.stopped = false; run.commands = 300; assert.throws(()=>run.api('mail','profile'),/budget exhausted/); assert.equal(count,1);
}));
test('empty or directory cleanup reports never fall through to a full live run', () => fixture((_run,root) => {
  for (const path of ['',root]) assert.throws(()=>main(['--accounts','test@example.com','--cleanup-report',path]),/cleanup report must be an existing JSON file/);
}));
test('cleanup recovery rejects other accounts and non-cleanup operations', () => fixture((run,root) => {
  const path = join(root,'source.json'); let count = 0; run.execute = () => { count++; return result(); };
  for (const [account,command] of [['other@example.com','files.delete'],[run.account,'files.update']]) {
    atomicJSON(path,{ account,run:'joom-test-fixture',pendingCleanup:[{ scope:'office',service:'drive',command,parameters:{ fileId:'fixture' } }] });
    assert.throws(()=>recover(run,path),Failure);
  }
  assert.equal(count,0);
}));
