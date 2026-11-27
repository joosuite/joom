import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { ROOT, checkFlag, isMain, readJSON, type Schema } from './common.ts';
import { wrapperDescription, wrapperNotice } from './microsoft_reference.ts';

export function expand(value: any, schema: Schema): any {
  if (Array.isArray(value)) return value.map(item => expand(item, schema));
  if (!value || typeof value !== 'object') return value;
  const result = value.$ref ? expand(schema.$defs[value.$ref.split('/').at(-1)], schema) : {};
  for (const [key, child] of Object.entries(value)) if (key !== '$ref') result[key] = expand(child, schema);
  for (const rule of result.allOf ?? []) if (!('description' in rule) && !('if' in rule)) Object.assign(result.properties ??= {}, rule.properties ?? {});
  return result;
}
export function render(schema = readJSON(join(ROOT, 'schemas/wrappers.schema.json'))): string {
  let text = HEADER;
  for (const [wrapper, config] of Object.entries<Schema>(schema['x-wrappers'])) {
    const commands: [string, Schema][] = Object.entries<Schema>(schema.$defs).filter(([key]) => key.startsWith(wrapper + '.')).map(([key, value]) => [key.split('.', 2)[1], expand(value, schema)]);
    text += `## ${wrapper}\n\n${wrapperDescription(wrapper, config.description)} The default command is \`${config.default}\`.\n\n| Command | Description |\n| --- | --- |\n`;
    for (const [command, value] of commands) { wrapperNotice(wrapper, command); text += `| \`${command}\` | ${value.description} |\n`; }
    text += '\n';
    for (const [command, value] of commands) {
      const notice = wrapperNotice(wrapper, command);
      const positions = Object.entries<Schema>(value.properties).filter(([, field]) => 'x-position' in field).sort(([, a], [, b]) => a['x-position'] - b['x-position']);
      let usage = `${wrapper} ACCOUNT ${command}`;
      for (const [name, field] of positions) { const label = field['x-label'] ?? name.toUpperCase(); usage += ' ' + ((value.required ?? []).includes(name) ? label : `[${label}]`); }
      const detail = value.description + (notice ? '\n\n' + notice : '');
      text += `### ${command}\n\n${detail}\n\n\`\`\`text\n${usage} [key:value ...]\n\`\`\`\n\n| Parameter | Required | Description |\n| --- | --- | --- |\n`;
      for (const [name, field] of Object.entries<Schema>(value.properties)) {
        let description = field.description ?? (field['x-label'] ?? name) + '.';
        if (field['x-repeat']) description += ' Repeat for multiple values.';
        if ('enum' in field) description += ' Parser choices: ' + field.enum.map((choice: unknown) => `\`${typeof choice === 'boolean' ? (choice ? 'True' : 'False') : String(choice)}\``).join(', ') + '.';
        if ('default' in field) description += ` Default: \`${JSON.stringify(field.default)}\`.`;
        if ('maxLength' in field) description += ` Maximum ${field.maxLength} characters.`;
        if ('maxItems' in field) description += ` Maximum ${field.maxItems} values.`;
        if ('minimum' in field && 'maximum' in field) description += ` Allowed range: ${field.minimum} to ${field.maximum}.`;
        if (field['x-advanced']) description += ' Advanced parameter.';
        const required = (value.required ?? []).includes(name) ? 'yes' : 'no';
        const option = field['x-key'] ?? name.replace(/([a-z0-9])([A-Z])/g, '$1-$2').toLowerCase();
        text += `| \`${option}:\` | ${required} | ${description.replaceAll('|', '\\|')} |\n`;
      }
      text += '\n';
      const explanations = new Set<string>();
      for (const rule of value.allOf ?? []) if (rule.description && !explanations.has(rule.description)) { explanations.add(rule.description); text += rule.description + '\n\n'; }
    }
  }
  return text;
}
export function generate(check = false): void {
  const destination = join(ROOT, 'docs/wrappers.md'), rendered = render();
  if (check) { if (!existsSync(destination) || readFileSync(destination, 'utf8') !== rendered) throw new Error('run npm exec -- tsx scripts/generate_wrapper_reference.ts'); }
  else writeFileSync(destination, rendered);
}
const HEADER = `# Short command wrappers

Run \`make build\` to create the wrappers in \`bin/\`. \`make install\` installs them
beside \`joom\`. Add that directory to your \`PATH\`.

\`joom\` and its wrappers are native executables. Installing and running them
requires no Node.js. Building from source requires Rust and Cargo; \`make build\`
and \`make install\` also create the wrapper entry points with a native installer.

\`\`\`sh
jmmail ACCOUNT to:lara@example.com body:"Hello\\nHow are you?"
jmcal ACCOUNT add "Lunch" at:"tomorrow 12:30" for:1h with:lara@example.com
jmdocs ACCOUNT ls limit:10
\`\`\`

The short commands and \`key:value\` parameters belong to the wrappers. Existing
\`joom\` commands keep their syntax. For example, \`joom mail ACCOUNT messages.send
--to lara@example.com --text Hello\` remains available.

Quote values containing spaces. Repeat parameters to supply several recipients,
attachments or rows. Inline text accepts \`\\n\`, \`\\t\`, and \`\\\\\`.
Use \`--\` before positional text that looks like a named parameter.

Use \`file:PATH\` to read parameters from a JSON object. File property names use
camelCase, such as \`bodyFile\` and \`matchCase\`. Explicit parameters replace file
values, including an entire repeated list. Explicit inline text replaces its
corresponding file input. Required values are checked after combining the inputs.
Select the command definition under \`$defs\` in
[wrappers.schema.json](../schemas/wrappers.schema.json) to validate complete input.

Lists default to 20 results unless the command describes a date window. Use
\`limit:all\` to fetch all results. Downloads and exports require \`out:PATH\` and
never overwrite existing files. CSV imports write literal values, including
text beginning with \`=\`. Task due dates use the computer's local date. Calendar
dates use the mailbox timezone, or the computer timezone when the mailbox has
none. An explicit \`timezone:\` value overrides both. Ambiguous times at clock changes
require a timestamp with an explicit offset.

Use \`WRAPPER --help\` to list its commands, \`WRAPPER ACCOUNT COMMAND --help\` for
common parameters, and \`--help --verbose\` for advanced parameters. \`json:true\`
changes only the output format. Both formats contain the same records and fields,
including message IDs and thread IDs in mail results. Advanced \`home:PATH\` and
\`timeout:SECONDS\` select the credential
directory and request timeout.

The wrappers use your existing Joom sign-in. Document, spreadsheet, and presentation
listing use OneDrive. Document sharing and native file exports also use
OneDrive. Authorize it with the service when needed, for example
\`joom auth ACCOUNT login docs,drive\`. See
[Microsoft support](microsoft.md) for each command group's implemented operations.

Compound commands can perform several requests. If a later request fails, the
error identifies an already-created resource where available. Check that
resource before retrying a creation command.

`;
if (isMain(import.meta.url)) generate(checkFlag());
