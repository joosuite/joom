import { execFileSync } from 'node:child_process';
import { createHash } from 'node:crypto';
import { chmodSync, closeSync, copyFileSync, cpSync, lstatSync, mkdirSync, openSync, readlinkSync, readSync, realpathSync, statSync, symlinkSync, writeFileSync } from 'node:fs';
import { basename, dirname, join, resolve, win32 } from 'node:path';
import { ROOT, isMain, readJSON } from './common.ts';

export type Execute = (command: string, args: string[], options: { cwd: string }) => string;
const execute: Execute = (command,args,options) => execFileSync(command,args,{ ...options,encoding:'utf8',maxBuffer:16 * 1024 * 1024 });
export function checkVersion(root = ROOT, env: NodeJS.ProcessEnv = process.env, exec: Execute = execute): string {
  const metadata = JSON.parse(exec('cargo',['+stable','metadata','--locked','--no-deps','--format-version','1'],{ cwd:root }));
  const version: string = metadata.packages[0].version;
  if (env.RELEASE_TAG !== 'v' + version) throw new Error('The tag must match the Cargo package version.');
  return version;
}
export function hashFile(path: string): string {
  const hash = createHash('sha256'), fd = openSync(path,'r'), buffer = Buffer.alloc(1024 * 1024);
  try { let length: number; while ((length = readSync(fd,buffer,0,buffer.length,null)) > 0) hash.update(buffer.subarray(0,length)); }
  finally { closeSync(fd); }
  return hash.digest('hex');
}
export function packageRelease(root = ROOT, env: NodeJS.ProcessEnv = process.env, exec: Execute = execute, platform: NodeJS.Platform = process.platform): string {
  const tag = env.RELEASE_TAG ?? '', target = env.TARGET ?? '';
  if (!/^v\d+\.\d+\.\d+(?:[-+][0-9A-Za-z.+-]+)?$/.test(tag) || !/^[0-9A-Za-z_-]+$/.test(target)) throw new Error('A version tag and Rust target are required.');
  const version = checkVersion(root,env,exec), suffix = platform === 'win32' ? '.exe' : '';
  const source = join(root,'target',target,'release'), output = join(root,'dist'), stage = join(output,`joom-${tag}-${target}`);
  mkdirSync(output,{ recursive:true }); mkdirSync(stage);
  const wrappers = readJSON(join(root,'schemas/wrappers.schema.json'))['x-wrappers'];
  for (const name of ['joom',...Object.keys(wrappers)]) {
    const invocation = resolve(source,name + suffix), executable = realpathSync(invocation);
    const stdout = exec(invocation,[name === 'joom' ? '--version' : '--help'],{ cwd:root });
    const expected = name === 'joom' ? `joom ${version}` : `Usage: ${name} ACCOUNT`;
    if (!stdout.includes(expected) || !statSync(executable).isFile()) throw new Error(`Native executable verification failed: ${name}`);
    const destination = join(stage,basename(invocation)), metadata = lstatSync(invocation);
    if (metadata.isSymbolicLink()) symlinkSync(readlinkSync(invocation),destination);
    else { copyFileSync(invocation,destination); chmodSync(destination,metadata.mode); }
  }
  for (const name of ['README.md','LICENSE','docs','schemas','examples/requests','assets/brand/logo.png','assets/brand/icon.png']) {
    const destination = join(stage,name); mkdirSync(dirname(destination),{ recursive:true }); cpSync(join(root,name),destination,{ recursive:true });
  }
  const commit = (env.GITHUB_SHA ?? exec('git',['rev-parse','HEAD'],{ cwd:root })).trim();
  writeFileSync(join(stage,'BUILD-INFO.json'),JSON.stringify({ version,tag,target,commit },null,2) + '\n');
  const archive = stage + (platform === 'win32' ? '.zip' : '.tar.gz');
  // Git Bash can put GNU tar first on PATH; Windows' native tar supports ZIP.
  const tar = platform === 'win32' ? win32.join(env.SystemRoot ?? 'C:\\Windows','System32','tar.exe') : 'tar';
  exec(tar,platform === 'win32' ? ['-a','-cf',archive,'-C',output,basename(stage)] : ['-czf',archive,'-C',output,basename(stage)],{ cwd:root });
  writeFileSync(archive + '.sha256',`${hashFile(archive)}  ${basename(archive)}\n`);
  console.log(`Packaged ${Object.keys(wrappers).length + 1} verified native executables: ${basename(archive)}`);
  return archive;
}
if (isMain(import.meta.url)) {
  if (process.argv.length !== 3) throw new Error('usage: release.ts check-version|package');
  if (process.argv[2] === 'check-version') checkVersion();
  else if (process.argv[2] === 'package') packageRelease();
  else throw new Error('usage: release.ts check-version|package');
}
