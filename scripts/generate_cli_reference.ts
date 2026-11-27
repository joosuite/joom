import { execFile } from 'node:child_process';
import { mkdirSync, writeFileSync } from 'node:fs';
import { basename, join } from 'node:path';
import { promisify } from 'node:util';
import { ROOT, isMain, jsonFiles, readJSON, type Schema } from './common.ts';
import { TITLES, commandNotice } from './microsoft_reference.ts';

const execute = promisify(execFile);
export async function generate(): Promise<void> {
  const binary = join(ROOT, 'target/debug', process.platform === 'win32' ? 'joom.exe' : 'joom');
  const sources = Object.fromEntries(jsonFiles(join(ROOT, 'api')).map(file => [basename(file, '.json'), readJSON(join(ROOT, 'api', file))]));
  const pairs = Object.entries(sources).flatMap(([service, source]) => Object.keys(source.$defs).sort().map(name => [service, name] as const));
  const results = new Map<string, string>();
  let cursor = 0;
  // Limit help subprocess concurrency; large help trees use memory.
  await Promise.all(Array.from({ length: 2 }, async () => {
    while (cursor < pairs.length) {
      const [service, name] = pairs[cursor++];
      const { stdout } = await execute(binary, ['help', service, name, '--verbose'], { encoding: 'utf8', maxBuffer: 16 * 1024 * 1024 });
      results.set(`${service}.${name}`, stdout.split('\nMicrosoft support: ')[0].split('\nReference: ')[0].trimEnd());
    }
  }));
  mkdirSync(join(ROOT, 'docs/services'), { recursive: true });
  for (const [service, source] of Object.entries<Schema>(sources)) {
    let text = `# ${TITLES[service]} methods\n\nSee the [command guide](../grammar.md) and [request-file format](../file-schemas.md).\n\n| Method | Description |\n| --- | --- |\n`;
    for (const name of Object.keys(source.$defs).sort()) {
      const definition = source.$defs[name], method = source['x-api-methods'][definition['x-api-method-id']];
      const description = 'x-batch-operation' in definition ? definition.properties.body.description : method.description;
      const about = description.split('\n')[0].split('. ')[0].replaceAll('|', '\\|');
      text += `| \`${name}\` | ${about} |\n`;
    }
    text += '\n';
    for (const name of Object.keys(source.$defs).sort()) text += `## ${name}\n\n${commandNotice(source, name)}\n\n\`\`\`text\n${results.get(`${service}.${name}`)}\n\`\`\`\n\n`;
    text += 'Command parameters retain the compatibility input contract. See [Microsoft support](../microsoft.md) for Graph behavior and limits.\n';
    writeFileSync(join(ROOT, 'docs/services', service + '.md'), text);
    console.log(service);
  }
}
if (isMain(import.meta.url)) await generate();
