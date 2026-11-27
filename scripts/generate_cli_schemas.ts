import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { basename, join } from 'node:path';
import { ROOT, checkFlag, clone, isMain, jsonFiles, readJSON, sortedEntries, type Schema } from './common.ts';

const RESERVED = new Set('file params body fields max max-pages ttl upload content-type output home timeout json help'.split(' '));
const ALIASES: Record<string, Record<string, string>> = { calendar: { 'start.dateTime': 'start-time', 'end.dateTime': 'end-time' } };
export function kebab(name: string): string {
  return name.replace(/([A-Z])([A-Z][a-z])/g, '$1-$2').replace(/([a-z0-9])([A-Z])/g, '$1-$2').replace(/[_.]/g, '-').toLowerCase();
}
function camel(name: string): string {
  const [head, ...tail] = name.split('-');
  return head + tail.map(word => word.charAt(0).toUpperCase() + word.slice(1).toLowerCase()).join('');
}
function shape(api: Schema, initial: Schema = {}): Schema {
  const seen = new Set<string>();
  let value = initial;
  while (Object.keys(value).length) {
    if ('anyOf' in value) value = value.anyOf.find((part: Schema) => part.type !== 'null') ?? {};
    else if ('$ref' in value) {
      const name = value.$ref.split('/').at(-1);
      if (seen.has(name)) return {};
      seen.add(name);
      value = api['x-body-types'][name];
    } else return value;
  }
  return {};
}
function binding(schema: Schema, long: string, source: string, property: string, extra: Schema = {}): Schema {
  return { ...clone(schema), 'x-cli-option': long, 'x-cli-binding': { source, property, ...extra } };
}
function presence(path: string[], checks: Schema = {}): Schema {
  let result: Schema = { not: { type: 'null' }, ...checks };
  for (const key of [...path].reverse()) result = { required: [key], properties: { [key]: result } };
  return result;
}
function requiredFor(value: Schema, method: string): boolean {
  return (value['x-required-in-methods'] ?? []).includes(method) || (value.anyOf ?? []).some((part: Schema) => requiredFor(part, method));
}
function userDescription(text: string): string {
  if (text.includes('b/157517979')) return 'Deprecated. This field is no longer populated.';
  if (text.includes('b/33408548')) return 'MENA content rating, using the MPAA rating categories.';
  return text;
}
const prefix = (path: string[], part: string[]): boolean => part.every((key, index) => path[index] === key);
const same = (a: string[], b: string[]): boolean => a.length === b.length && prefix(a, b);
function commandRules(api: Schema, old: Schema, method: Schema, command: Schema): void {
  const props: Schema = command.properties;
  const rules: Schema[] = [];
  const targets: Record<string, string[]> = {};
  for (const [key, value] of Object.entries<Schema>(props)) {
    const mapping = value['x-cli-binding'] ?? {};
    if (mapping.source === 'raw') targets[key] = [];
    else if (mapping.source === 'body') targets[key] = [mapping.property];
    else if (mapping.source === 'nested') targets[key] = mapping.path;
  }
  const either = (conditions: Schema[]): Schema => ({ anyOf: conditions });
  const at = (path: string[]): Schema => {
    const conditions: Schema[] = [];
    for (const [key, target] of Object.entries(targets)) {
      if (prefix(path, target)) conditions.push(presence([key, ...path.slice(target.length)]));
      else if (prefix(target, path)) conditions.push({ required: [key] });
    }
    return either(conditions);
  };
  const keysAt = (path: string[]): string[] => Object.entries(targets).filter(([, target]) => target.length && prefix(target, path)).map(([key]) => key);
  const names = (keys: string[]): string => keys.map(key => props[key]['x-cli-binding']?.positional ? props[key]['x-cli-value-name'] : '--' + (props[key]['x-cli-option'] ?? key)).join(' or ');
  function rule(assertion: Schema, description: string, keys: string[] = [], kind = 'choice', groups?: string[][]): void {
    const result = { ...assertion, description, 'x-cli-kind': kind, 'x-cli-parameters': [...keys] };
    if (groups?.length) Object.assign(result, { 'x-cli-groups': groups });
    rules.push(result);
  }
  function conflict(conditions: Schema[], groups: string[][], description: string): void {
    for (let i = 0; i < conditions.length; i++) for (let j = i + 1; j < conditions.length; j++) {
      rule({ not: { allOf: [conditions[i], conditions[j]] } }, description, [...groups[i], ...groups[j]], 'conflict', [groups[i], groups[j]]);
    }
  }
  const composition = Object.entries<Schema>(props).filter(([, value]) => value['x-cli-binding']?.source === 'compose').map(([key]) => key);
  for (const key of method['x-required-cli-inputs'] ?? []) rule(presence([key]), `Provide ${names([key])}.`, [key], 'required');
  const compose = either([...composition.map(key => ({ required: [key] })), ...(composition.length ? [presence(['compose'])] : [])]);
  const composePath: string[] = old['x-cli-compose-path'] ?? [];
  const defaults: Schema = { calendar: { calendarId: 'primary' }, tasks: { tasklist: '@default' } };
  for (const [key, value] of Object.entries<Schema>(props)) {
    const mapping = value['x-cli-binding'] ?? {};
    const prop = mapping.property;
    const parameter = mapping.source === 'query' ? method.parameters?.[prop] ?? {} : {};
    let defaultValue = defaults[api['x-api-service']]?.[prop];
    if (defaultValue !== undefined && defaultValue !== null) value.default = defaultValue;
    const required = value['x-required-after-merge'];
    delete value['x-required-after-merge'];
    if ((parameter.required && defaultValue == null) || required) rule(presence([key]), `Provide ${names([key])}.`, [key], 'required');
  }
  const request = old.properties?.body;
  function visit(schema: Schema, path: string[], ancestors: Set<string>, mandatory: boolean): void {
    const reference = schema.$ref;
    if ((reference && ancestors.has(reference)) || path.length > 8) return;
    const resolved = shape(api, schema);
    ancestors = new Set([...ancestors, ...(reference ? [reference] : [])]);
    const choices: string[] | undefined = resolved['x-required-one-of']?.[method.id];
    let choiceDescription: string | undefined;
    for (const group of resolved['x-exclusive-fields'] ?? []) {
      const paths: string[][] = group.map((key: string) => [...path, key]);
      const groups = paths.map(keysAt);
      let description: string;
      if (same(group, ['date', 'dateTime'])) {
        const [date, time] = paths.map(target => names(keysAt(target)));
        description = `Use ${date} for an all-day event or ${time} for a timed event; do not supply both.`;
        if (choices?.length) {
          const joined = path.join('.');
          const beginning = mandatory ? `${joined.charAt(0).toUpperCase() + joined.slice(1).toLowerCase()} is required: ` : `When setting ${joined}, `;
          description = beginning + `use ${date} for an all-day event or ${time} for a timed event.`;
        }
        choiceDescription = description;
      } else if (same(group, ['albumId', 'filters'])) description = 'Search an album with --album-id, or search with filters. Album searches do not accept filters.';
      else description = `Choose one of ${names(groups.flat())}.`;
      conflict(paths.map(at), groups, description);
    }
    if (choices?.length) {
      let assertion = either(choices.map(key => at([...path, key])));
      if (path.length) assertion = { if: at(path), then: assertion };
      const keys = choices.flatMap(field => keysAt([...path, field]));
      rule(assertion, choiceDescription ?? `For ${path.join('.')}, provide ${names(keys)}.`, keys);
    }
    for (const [key, value] of Object.entries<Schema>(resolved.properties ?? {})) {
      if (shape(api, value).readOnly) continue;
      const target = [...path, key];
      const required = [...(resolved.required ?? []), ...(resolved['x-required-fields'] ?? [])].includes(key) || requiredFor(value, method.id);
      if (required) {
        let assertion = at(target);
        if (composition.length && prefix(composePath, target)) assertion = either([assertion, compose, ...('upload' in props ? [presence(['upload'])] : [])]);
        if (path.length) assertion = { if: at(path), then: assertion };
        const keys = keysAt(target);
        const alternatives = keys.filter(k => !same(targets[k], target));
        const isRequired = mandatory && !alternatives.length && !(composition.length && prefix(composePath, target));
        let description = `Provide ${names(keys) || target.join('.')}.`;
        let requiredChildren: string[] = [];
        if (alternatives.length) {
          const child = shape(api, value);
          const childChoices = child['x-required-one-of']?.[method.id] ?? [];
          requiredChildren = Object.entries<Schema>(child.properties ?? {}).filter(([field, prop]) => [...childChoices, ...(child.required ?? []), ...(child['x-required-fields'] ?? [])].includes(field) || requiredFor(prop, method.id)).map(([field]) => field);
          const preferred = requiredChildren.flatMap(field => keysAt([...target, field]));
          const exact = keys.filter(k => same(targets[k], target));
          description = requiredChildren.length ? `Provide ${names(preferred.length ? preferred : alternatives)} for ${target.join('.')}.` : `Provide ${names(exact)} as a JSON object, or set its named fields.`;
        }
        rule(assertion, description, keys, isRequired ? 'required' : 'choice');
        if (!mandatory || (alternatives.length && requiredChildren.length) || (composition.length && prefix(composePath, target))) rules.at(-1)!['x-cli-help'] = false;
      }
      if (shape(api, value).type === 'object') visit(value, target, ancestors, mandatory && required);
    }
  }
  if (request) {
    visit(request, [], new Set(), true);
    const bodyKeys = Object.entries(targets).filter(([, target]) => target.length).map(([key]) => key);
    if (Object.keys(shape(api, request).properties ?? {}).length && !('x-batch-operation' in old)) {
      const choices = [...bodyKeys, ...composition, ...('upload' in props ? ['upload'] : []), 'body', ...(composition.length ? ['compose'] : [])];
      rule(either(choices.map(key => [...bodyKeys, ...composition].includes(key) ? { required: [key] } : presence([key]))), 'Provide the content to write using named parameters, a JSON file, or --body.', choices, 'content');
    }
  }
  if (composition.length) {
    for (const [inline, file] of api['x-cli-types'].MailComposition['x-exclusive-fields']) {
      conflict([inline, file].map(key => either([presence([key]), presence(['compose', key])])), [[inline], [file]], `Use --${props[inline]['x-cli-option']} for inline content or --${props[file]['x-cli-option']} to read a file.`);
    }
    const rawPaths = [composePath, [...composePath.slice(0, -1), 'payload']];
    const conditions = [...rawPaths.map(at), ...('upload' in props ? [presence(['upload'])] : [])];
    const rawKeys = [...rawPaths.flatMap(keysAt), ...('upload' in props ? ['upload'] : [])];
    rule({ not: { allOf: [compose, either(conditions)] } }, 'Compose a message with recipients and content, or supply raw MIME. Composition cannot be combined with raw message content or --upload.', [...composition, ...rawKeys], 'conflict', [composition, rawKeys]);
    if ((method.id ?? '').endsWith('.send')) {
      const recipients = ['to', 'cc', 'bcc'].flatMap(key => [[key], ['compose', key]].map(path => presence(path, { type: 'array', minItems: 1 })));
      rule({ if: compose, then: either(recipients) }, 'When composing a message to send, provide at least one recipient with --to, --cc, or --bcc.', ['to', 'cc', 'bcc']);
    }
  }
  if ('output' in props) for (const key of ['max', 'maxPages', 'upload']) if (key in props) conflict([presence(['output']), presence([key])], [['output'], [key]], `Use --output for a single download; omit --${props[key]['x-cli-option']} when downloading.`);
  if ('contentType' in props) rule({ if: presence(['contentType']), then: presence(['upload']) }, '--content-type sets the MIME type of a file supplied with --upload.', ['contentType', 'upload'], 'dependency');
  for (const selected of method['x-unpaginated-when'] ?? []) if (selected in props) for (const page of ['maxResults', 'pageSize', 'pageToken', 'maxPages']) if (page in props) conflict([presence([selected]), presence([page])], [[selected], [page]], `When selecting records with ${names([selected])}, omit ${names([page])}. Use --max to limit the returned records.`);
  function arrayRules(schema: Schema, ancestors: Set<Schema>, insideArray = false, path = ''): [Schema, string[]] {
    const resolved = shape(api, schema);
    if (ancestors.has(resolved)) return [{}, []];
    ancestors = new Set([...ancestors, resolved]);
    if (resolved.type === 'array') {
      const [assertion, labels] = arrayRules(resolved.items ?? {}, ancestors, true, path + '[]');
      return [Object.keys(assertion).length ? { items: assertion } : {}, labels];
    }
    const properties: Schema = {}, required: string[] = [], labels: string[] = [], assertions: Schema[] = [];
    for (const [key, value] of Object.entries<Schema>(resolved.properties ?? {})) {
      if (shape(api, value).readOnly) continue;
      const label = (path ? path + '.' : '') + key;
      let [child, childLabels] = arrayRules(value, ancestors, insideArray, label);
      labels.push(...childLabels);
      if (insideArray && ([...(resolved.required ?? []), ...(resolved['x-required-fields'] ?? [])].includes(key) || requiredFor(value, method.id))) {
        required.push(key); labels.push(label); child = { ...child, not: { type: 'null' } };
      }
      if (Object.keys(child).length) properties[key] = child;
    }
    if (insideArray) {
      for (const group of resolved['x-exclusive-fields'] ?? []) group.forEach((key: string, index: number) => { for (const other of group.slice(index + 1)) assertions.push({ not: { allOf: [presence([key]), presence([other])] } }); });
      const choices: string[] = resolved['x-required-one-of']?.[method.id] ?? [];
      if (choices.length) assertions.push(either(choices.map(key => presence([key]))));
    }
    return [{ ...(Object.keys(properties).length ? { properties } : {}), ...(required.length ? { required } : {}), ...(assertions.length ? { allOf: assertions } : {}) }, labels];
  }
  for (const [key, value] of Object.entries<Schema>(props)) {
    if (!['raw', 'body', 'nested', 'envelope'].includes(value['x-cli-binding']?.source)) continue;
    const [assertion, labels] = arrayRules(value, new Set());
    if (Object.keys(assertion).length) {
      const label = '--' + value['x-cli-option'];
      rule({ properties: { [key]: assertion } }, labels.length ? `Records in ${label} require ${[...new Set(labels)].sort().join(', ')}.` : `Choose compatible record fields in ${label}.`, [key], 'structured');
    }
  }
  if (rules.length) command.allOf = rules;
}

export function makeService(service: string, api: Schema): Schema {
  const commands: Schema = {};
  for (const [name, old] of Object.entries<Schema>(api.$defs)) {
    const method = api['x-api-methods'][old['x-api-method-id']];
    const props: Schema = {}, used = new Set(RESERVED);
    for (const [prop, parameter] of sortedEntries<Schema>(method.parameters ?? {})) {
      if (['fields', 'access_token', 'oauth_token', 'key', 'userId'].includes(prop)) continue;
      let long = kebab(prop); if (used.has(long)) long = 'query-' + long; used.add(long);
      const schema = clone(old.properties[prop]); schema.description = parameter.description ?? '';
      props[camel(long)] = { ...binding(schema, long, 'query', prop), 'x-cli-repeated': parameter.repeated ?? false };
    }
    props.fields = binding({ type: 'string', description: 'Select response fields.' }, 'fields', 'control', 'fields');
    const request = old.properties?.body;
    if (request) {
      props.body = binding(request, 'body', 'raw', 'body');
      for (const [prop, schema] of sortedEntries<Schema>(shape(api, request).properties ?? {})) {
        const resolved = shape(api, schema); if (resolved.readOnly) continue;
        let long = kebab(prop); if (used.has(long)) long = 'body-' + long; used.add(long);
        const item = binding(schema, long, 'body', prop);
        item['x-cli-repeated'] = resolved.type === 'array' && ['string','integer','number','boolean'].includes(shape(api, resolved.items ?? {}).type);
        props[camel(long)] = item;
      }
      function children(schema: Schema, path: string[], ancestors: Set<string>): void {
        const reference = schema.$ref; if (reference && ancestors.has(reference)) return;
        const resolved = shape(api, schema); if (resolved.readOnly || path.length > 4) return;
        if (resolved.type === 'object') for (const [child, value] of sortedEntries<Schema>(resolved.properties ?? {})) children(value, [...path, child], new Set([...ancestors, ...(reference ? [reference] : [])]));
        else if (path.length > 1) {
          const long = ALIASES[service]?.[path.join('.')] ?? path.map(kebab).join('-');
          if (used.has(long)) return; used.add(long);
          const item = binding(schema, long, 'nested', camel(long), { path });
          item['x-cli-repeated'] = resolved.type === 'array' && ['string','integer','number','boolean'].includes(shape(api, resolved.items ?? {}).type);
          props[camel(long)] = item;
        }
      }
      for (const [prop, schema] of sortedEntries<Schema>(shape(api, request).properties ?? {})) children(schema, [prop], new Set());
    }
    if ('x-batch-operation' in old) for (const [prop, schema] of Object.entries<Schema>(old.properties)) {
      if (['body','fields'].includes(prop) || prop in (method.parameters ?? {})) continue;
      let long = kebab(prop); if (used.has(long)) long = 'batch-' + long; used.add(long);
      props[camel(long)] = binding(schema, long, 'envelope', prop);
    }
    if (name === 'create' && ['sheets','forms'].includes(service)) props.title = binding({ type: 'string', description: 'Title.' }, 'title', 'nested', 'title', { path: [service === 'sheets' ? 'properties' : 'info', 'title'] });
    if ('x-cli-compose-path' in old) {
      for (const [prop, schema] of Object.entries<Schema>(api['x-cli-types'].MailComposition.properties)) {
        const long = schema['x-cli-option'] ?? kebab(prop); props[camel(long)] = binding(schema, long, 'compose', prop);
      }
      props.compose = { $ref: '#/x-body-types/MailComposition', description: 'Composition object. The same parameters can be supplied at the top level.' };
    }
    const read = ['GET','HEAD'].includes(method.httpMethod) || method['x-read-only'];
    const direct = method.supportsMediaDownload && (!('response' in method) || method.id.endsWith('.media.download'));
    const controls: Schema = {};
    if (read && !method['x-fresh-response'] && !direct) controls.ttl = { type: 'integer', minimum: 0, maximum: 4294967295, description: 'Cache freshness in minutes. Zero fetches fresh data.' };
    const paginated = 'pageToken' in (method.parameters ?? {}) || 'pageToken' in (method['x-body-parameters'] ?? {});
    const listed = paginated || ['pageSize','maxResults'].some(key => key in (method.parameters ?? {}) || key in (method['x-body-parameters'] ?? {})) || ['.list','.search'].some(suffix => method.id.endsWith(suffix));
    if (read && listed) controls.max = { type: 'integer', minimum: 1, maximum: 4294967295, description: 'Maximum total results.' };
    if (read && paginated) controls['max-pages'] = { type: 'integer', minimum: 1, maximum: 10000, description: 'Maximum pages to fetch.' };
    if ('mediaUpload' in method) {
      const media = method.mediaUpload, rawLimit = String(media.maxSize ?? '');
      const limit = Math.min(/^\d+$/.test(rawLimit) ? Number(rawLimit) : 1024 ** 3, 1024 ** 3);
      const unit = ([[1024 ** 3,'GiB'], [1024 ** 2,'MiB'], [1024,'KiB']] as const).find(([size]) => limit && limit % size === 0);
      const size = unit ? `${limit / unit[0]} ${unit[1]}` : `${limit} bytes`;
      controls.upload = { type: 'string', minLength: 1, description: `Upload a local file of at most ${size}.`, 'x-file-max-bytes': limit, 'x-file-mime-types': media.accept ?? ['*/*'] };
      controls['content-type'] = { type: 'string', minLength: 1, description: 'Upload MIME type.' };
    }
    if (method.supportsMediaDownload || method.id === 'mail.messages.attachments.get') {
      controls.output = { type: 'string', minLength: 1, description: 'Write downloaded content to this new file.' };
      if (direct) controls.output['x-required-after-merge'] = true;
    }
    for (const [long, schema] of Object.entries<Schema>(controls)) props[camel(long)] = binding(schema, long, 'control', long);
    commands[name] = { type: 'object', title: `joom ${service} ACCOUNT ${name}`, description: 'x-batch-operation' in old ? request?.description ?? name : method.description, additionalProperties: false, properties: props };
    commandRules(api, old, method, commands[name]);
  }
  addNative(service, commands);
  for (const [name, command] of Object.entries<Schema>(commands)) if (!(name in api.$defs)) commandRules(api, {}, {}, command);
  const result: Schema = { $schema: 'https://json-schema.org/draft/2020-12/schema', $id: `urn:joom:file:${service}`, title: `Joom ${service} command parameters`, description: 'Parameters accepted by Joom from --file. Each command definition describes CLI inputs. Explicit command-line values override file values. Select a command under $defs.', type: 'object', anyOf: Object.keys(commands).map(name => ({ $ref: '#/$defs/' + name })), $defs: commands };
  const types = { ...api['x-body-types'], ...(api['x-cli-types'] ?? {}) }, needed: Schema = {};
  function visit(value: any): void {
    if (Array.isArray(value)) value.forEach(visit);
    else if (value && typeof value === 'object') {
      if (typeof value.description === 'string') value.description = userDescription(value.description);
      if ('$ref' in value) {
        const name = value.$ref.split('/').at(-1); value.$ref = '#/x-input-types/' + name;
        if (!(name in needed)) { needed[name] = clone(types[name]); visit(needed[name]); }
      }
      for (const [key, child] of Object.entries(value)) if (key !== '$ref') visit(child);
    }
  }
  visit(commands);
  if (Object.keys(needed).length) result['x-input-types'] = Object.fromEntries(sortedEntries(needed));
  return result;
}
function addNative(service: string, commands: Schema): void {
  const specs: Record<string, Record<string, [string,string,boolean][]>> = {
    mail: Object.fromEntries(['read','unread','star','unstar'].map(name => [name, [['id','MESSAGE_ID',true]]])),
    drive: { search: [['query','TEXT',true],['parent','ID',false],['type','MIME',false]] },
    calendar: { events: [['calendar','ID',false],['from','TIMESTAMP',false],['until','TIMESTAMP',false],['query','TEXT',false]] },
  };
  for (const [name, args] of Object.entries(specs[service] ?? {})) {
    const props: Schema = {};
    for (const [long, label, positional] of args) {
      const description = label.replaceAll('_',' ').toLowerCase();
      const field = binding({ type: 'string', minLength: 1, description: description.charAt(0).toUpperCase() + description.slice(1) + '.' }, long, 'native', long, { positional });
      field['x-cli-value-name'] = label; if (positional) field['x-required-after-merge'] = true;
      props[camel(long)] = field;
    }
    const controls = ['fields']; if (['drive.search','calendar.events'].includes(service + '.' + name)) controls.push('ttl','max','max-pages');
    for (const long of controls) props[camel(long)] = binding(long === 'fields' ? { type: 'string' } : { type: 'integer', minimum: long === 'ttl' ? 0 : 1, maximum: long === 'max-pages' ? 10000 : 4294967295 }, long, 'control', long);
    commands[name] = { type: 'object', title: `joom ${service} ACCOUNT ${name}`, additionalProperties: false, properties: props };
  }
}
export function generate(check = false): void {
  for (const source of jsonFiles(join(ROOT, 'api'))) {
    const service = basename(source, '.json'), schema = makeService(service, readJSON(join(ROOT, 'api', source)));
    const destination = join(ROOT, 'schemas', service + '.schema.json');
    const rendered = JSON.stringify(schema, null, 2) + '\n';
    if (check) { if (!existsSync(destination) || readFileSync(destination, 'utf8') !== rendered) throw new Error(`${destination}: run make schemas`); }
    else writeFileSync(destination, rendered);
    console.log(`${service}: ${Object.keys(schema.$defs).length} commands`);
  }
}
if (isMain(import.meta.url)) generate(checkFlag());
