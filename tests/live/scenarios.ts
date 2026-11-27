import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { ROOT, Failure, discoverMail, identifier, records, type Run } from './runner.ts';
import type { Schema } from '../../scripts/common.ts';

const startTomorrow = (): Date => { const date = new Date(Date.now() + 86400000); date.setUTCMinutes(0,0,0); return date; };
const later = (date: Date, hours: number): string => new Date(date.getTime() + hours * 3600000).toISOString();
const officeTypes = [
  ['document','document.docx','application/vnd.openxmlformats-officedocument.wordprocessingml.document'],
  ['workbook','workbook.xlsx','application/vnd.openxmlformats-officedocument.spreadsheetml.sheet'],
  ['presentation','presentation.pptx','application/vnd.openxmlformats-officedocument.presentationml.presentation'],
] as const;
export function tasks(run: Run): void {
  const { api,wrapper } = run;
  api('tasks','tasklists.list'); wrapper('jmtasks','lists');
  const listID = identifier(api('tasks','tasklists.insert',{ title:run.marker }));
  run.register('tasks','tasks','tasklists.delete',{ tasklist:listID });
  api('tasks','tasklists.get',{ tasklist:listID });
  api('tasks','tasklists.patch',{ tasklist:listID,title:run.marker + '-patched' });
  api('tasks','tasklists.update',{ tasklist:listID,title:run.marker + '-updated' });
  const taskID = identifier(api('tasks','tasks.insert',{ tasklist:listID,title:run.marker,notes:'Synthetic test only.' }));
  const cleanup = run.register('tasks','tasks','tasks.delete',{ tasklist:listID,task:taskID });
  api('tasks','tasks.list',{ tasklist:listID }); api('tasks','tasks.get',{ tasklist:listID,task:taskID });
  api('tasks','tasks.patch',{ tasklist:listID,task:taskID,title:run.marker + '-patched' });
  api('tasks','tasks.update',{ tasklist:listID,task:taskID,title:run.marker + '-updated' });
  wrapper('jmtasks','ls',{ list:listID });
  for (const command of ['show','done','undo']) wrapper('jmtasks',command,{ target:taskID,list:listID });
  wrapper('jmtasks','set',{ target:taskID,list:listID,notes:'Updated synthetic notes.' });
  wrapper('jmtasks','delete',{ target:taskID,list:listID }); run.discard(cleanup);
  run.register('tasks','tasks','tasks.delete',{ tasklist:listID,task:identifier(wrapper('jmtasks','add',{ title:run.marker + '-wrapper',list:listID })) });
  run.register('tasks','tasks','tasklists.delete',{ tasklist:identifier(wrapper('jmtasks','new-list',{ title:run.marker + '-wrapper' })) });
}
export function contacts(run: Run): void {
  const { api,wrapper } = run;
  api('contacts','people.connections.list',{ resourceName:'people/me' }); wrapper('jmcontacts','ls');
  const personID = identifier(api('contacts','people.create-contact',{ names:[{ givenName:run.marker,familyName:'Fixture' }],emailAddresses:[{ value:run.account }] }),'resourceName');
  const cleanup = run.register('contacts','contacts','people.delete-contact',{ resourceName:personID });
  api('contacts','people.get',{ resourceName:personID }); api('contacts','people.search-contacts',{ query:run.marker });
  api('contacts','people.update-contact',{ resourceName:personID,organizations:[{ name:'Joom synthetic fixture' }] });
  wrapper('jmcontacts','find',{ query:run.marker }); wrapper('jmcontacts','show',{ target:personID });
  wrapper('jmcontacts','set',{ target:personID,company:'Joom test update' }); wrapper('jmcontacts','delete',{ target:personID }); run.discard(cleanup);
  run.register('contacts','contacts','people.delete-contact',{ resourceName:identifier(wrapper('jmcontacts','new',{ name:run.marker + ' Wrapper',email:[run.account] })) });
}
export function calendar(run: Run): void {
  if (run.case === 'regressions') return calendarWrappers(run);
  const { api,wrapper } = run;
  api('calendar','calendars.list'); wrapper('jmcal','calendars');
  const calendarID = identifier(api('calendar','calendars.insert',{ summary:run.marker }));
  run.register('calendar','calendar','calendars.delete',{ calendarId:calendarID });
  for (const command of ['calendars.get','calendar-list.get']) api('calendar',command,{ calendarId:calendarID });
  for (const command of ['calendars.patch','calendars.update']) api('calendar',command,{ calendarId:calendarID,summary:run.marker + '-' + command.split('.').at(-1) });
  const start = startTomorrow(), startText = start.toISOString(), endText = later(start,1);
  const eventID = identifier(api('calendar','events.insert',{ calendarId:calendarID,summary:run.marker,startTime:startText,endTime:endText,startTimeZone:'UTC',endTimeZone:'UTC' }));
  const cleanup = run.register('calendar','calendar','events.delete',{ calendarId:calendarID,eventId:eventID });
  api('calendar','events.get',{ calendarId:calendarID,eventId:eventID });
  for (const command of ['events.patch','events.update']) api('calendar',command,{ calendarId:calendarID,eventId:eventID,summary:run.marker + '-' + command.split('.').at(-1) });
  api('calendar','events.list',{ calendarId:calendarID,timeMin:startText,timeMax:endText });
  api('calendar','events',{ calendar:calendarID,from:startText,until:endText });
  for (const command of ['today','week']) wrapper('jmcal',command,{ calendar:calendarID,timezone:'UTC' });
  wrapper('jmcal','find',{ calendar:calendarID,timezone:'UTC',query:run.marker }); wrapper('jmcal','show',{ calendar:calendarID,target:eventID });
  wrapper('jmcal','move',{ calendar:calendarID,target:eventID,at:later(start,2),timezone:'UTC' }); wrapper('jmcal','cancel',{ calendar:calendarID,target:eventID }); run.discard(cleanup);
  run.register('calendar','calendar','events.delete',{ calendarId:calendarID,eventId:identifier(wrapper('jmcal','add',{ calendar:calendarID,title:run.marker + '-wrapper',at:startText,timezone:'UTC' })) });
  const recurring = run.fixture('create-recurring-event',{ calendarId:calendarID,marker:run.marker,date:startText.slice(0,10) });
  const recurringID = identifier(recurring); run.register('calendar','calendar','events.delete',{ calendarId:calendarID,eventId:recurringID });
  const midnight = new Date(start); midnight.setUTCHours(0);
  const instances = api('calendar','events.instances',{ calendarId:calendarID,eventId:recurringID,timeMin:midnight.toISOString(),timeMax:later(start,72) });
  if (records(instances,'items').length !== 2) throw new Failure('recurring fixture did not produce exactly two instances');
  if (typeof recurring.mailboxAddress === 'string') run.context.mailRecipient = recurring.mailboxAddress;
}
export function mail(run: Run): void {
  const { api,wrapper } = run;
  if (['mail-draft','regressions'].includes(run.case)) {
    run.register('mail','mail','drafts.delete',{ id:identifier(api('mail','drafts.create',{ subject:run.marker,text:'Unsent synthetic fixture.' })) }); return;
  }
  api('mail','messages.list',{ q:'in:inbox',max:1 }); api('mail','drafts.list',{ max:1 }); api('mail','labels.list');
  wrapper('jmmail','inbox',{ limit:1 }); wrapper('jmmail','drafts',{ limit:1 });
  const folderID = identifier(api('mail','labels.create',{ name:run.marker })); run.register('mail','mail','labels.delete',{ id:folderID });
  api('mail','labels.get',{ id:folderID });
  for (const command of ['labels.patch','labels.update']) api('mail',command,{ id:folderID,name:run.marker + '-' + command.split('.').at(-1) });
  if (!('mailRecipient' in run.context) && run.account.endsWith('@gmail.com')) { const mailbox = run.fixture('mailbox-address'); if (typeof mailbox.address === 'string') run.context.mailRecipient = mailbox.address; }
  const recipient: string = run.context.mailRecipient ?? run.account; run.state.mailRecipient = recipient;
  const attachment = join(run.directory,'attachment.txt'); writeFileSync(attachment,'Joom synthetic live-test attachment.\n'); run.context.attachment = attachment;
  const draftID = identifier(api('mail','drafts.create',{ to:[recipient],subject:run.marker + '-direct-draft',text:'Synthetic test message addressed only to this account.',attach:[attachment] }));
  const cleanup = run.register('mail','mail','drafts.delete',{ id:draftID }); api('mail','drafts.get',{ id:draftID });
  run.register('mail','mail','drafts.delete',{ id:identifier(api('mail','drafts.create',{ subject:run.marker + '-unsent',text:'Unsent cleanup fixture.' })) });
  api('mail','drafts.send',{ id:draftID }); run.discard(cleanup);
  const copies = recipient.endsWith('@gmail.com') ? 1 : 2;
  run.state.pendingMailDiscovery = { subjectPrefix:run.marker,expectedCopies:copies,reason:'Self-addressed fixture copies; one deferred cleanup search.' }; run.save();
  api('mail','messages.send',{ to:[recipient],subject:run.marker + '-direct-send',text:'Synthetic test message addressed only to this account.',attach:[attachment] }); run.state.pendingMailDiscovery.expectedCopies += copies;
  const wrapperID = identifier(wrapper('jmmail','draft',{ to:[recipient],subject:run.marker + '-wrapper-draft',body:'Synthetic fixture.',attachments:[attachment] }));
  const wrapperCleanup = run.register('mail','mail','drafts.delete',{ id:wrapperID });
  wrapper('jmmail','send-draft',{ target:wrapperID }); run.discard(wrapperCleanup); run.state.pendingMailDiscovery.expectedCopies += copies;
  wrapper('jmmail','send',{ to:[recipient],subject:run.marker + '-wrapper-send',body:'Synthetic fixture.',attachments:[attachment] }); run.state.pendingMailDiscovery.expectedCopies += copies;
  if (recipient.endsWith('@gmail.com')) run.state.manualCleanup = [{ kind:'self-addressed-email',subjectPrefix:run.marker,note:'Up to four synthetic emails may remain in the external Gmail inbox; that inbox is not accessed by Microsoft tests.' }];
  run.save();
}
export function mailDelivery(run: Run): void {
  if (!run.state.pendingMailDiscovery) return;
  const { api,wrapper } = run, messages = discoverMail(run);
  const owned = [...messages.keys()].map(id => [id,run.register('mail-delivery','mail','messages.delete',{ id })] as const);
  if (owned.length) { delete run.state.pendingMailDiscovery; run.save(); }
  else throw new Failure('self-addressed fixtures are not searchable yet; no polling or resend was attempted');
  let [messageID] = owned[0]; const cleanup = owned[0][1];
  const detail = api('mail','messages.get',{ id:messageID });
  wrapper('jmmail','find',{ query:'subject:' + run.marker,limit:1 }); wrapper('jmmail','read',{ target:messageID });
  for (const command of ['read','unread','star','unstar']) api('mail',command,{ id:messageID });
  api('mail','messages.modify',{ id:messageID,addLabelIds:['STARRED'],removeLabelIds:['UNREAD'] }); wrapper('jmmail','mark',{ target:messageID,state:'read' });
  const attachments: Schema[] = (detail.payload?.parts ?? []).filter((part:Schema) => part.filename === 'attachment.txt');
  if (!attachments.length) throw new Failure('synthetic message attachment metadata is missing');
  const data = api('mail','messages.attachments.get',{ messageId:messageID,id:attachments[0].body.attachmentId });
  if (!Buffer.from(data.data ?? '', 'base64url').equals(readFileSync(run.context.attachment))) throw new Failure('downloaded synthetic mail attachment did not match the fixture');
  for (const command of ['messages.trash','messages.untrash']) { messageID = identifier(api('mail',command,{ id:messageID })); cleanup.parameters.id = messageID; run.save(); }
  cleanup.parameters.id = identifier(wrapper('jmmail','trash',{ target:messageID })); run.save();
}
export function drive(run: Run): void {
  if (['drive-metadata','regressions'].includes(run.case)) {
    const drives = records(run.api('drive','drives.list'),'drives');
    if (!drives.length) throw new Failure('Microsoft returned no drive metadata fixture');
    run.api('drive','drives.get',{ driveId:identifier(drives[0]) }); return;
  }
  if (run.case === 'office-files') return officeFiles(run);
  const { api,wrapper } = run, drives = records(api('drive','drives.list'),'drives');
  if (drives.length && run.case !== 'drive-files') run.context.driveType = api('drive','drives.get',{ driveId:identifier(drives[0]) }).driveType;
  api('drive','files.list',{ max:1 }); wrapper('jmdrive','ls',{ limit:1 });
  const folderID = identifier(api('drive','files.create',{ name:run.marker,mimeType:'inode/directory' })); run.register('office','drive','files.delete',{ fileId:folderID });
  const secondID = identifier(wrapper('jmdrive','mkdir',{ name:run.marker + '-nested',folder:folderID })); run.register('drive','drive','files.delete',{ fileId:secondID });
  let source = join(run.directory,'source.txt'); writeFileSync(source,'Joom synthetic OneDrive fixture.\n');
  const fileID = identifier(api('drive','files.create',{ name:run.marker + '.txt',parents:[folderID],upload:source,contentType:'text/plain' })); run.register('drive','drive','files.delete',{ fileId:fileID });
  api('drive','files.get',{ fileId:fileID });
  const destination = join(run.directory,'direct-download.txt'); api('drive','files.get',{ fileId:fileID,output:destination });
  if (!readFileSync(destination).equals(readFileSync(source))) throw new Failure('OneDrive upload/download bytes differ');
  api('drive','files.update',{ fileId:fileID,name:run.marker + '-renamed.txt' }); writeFileSync(source,'Joom synthetic OneDrive updated fixture.\n');
  api('drive','files.update',{ fileId:fileID,upload:source,contentType:'text/plain' });
  const versions = records(api('drive','revisions.list',{ fileId:fileID }),'revisions');
  if (versions.length) api('drive','revisions.get',{ fileId:fileID,revisionId:identifier(versions[0]) });
  else (run.state.notTested ??= []).push({ command:'drive.revisions.get',reason:'Microsoft returned no versions for the new fixture.' });
  api('drive','permissions.list',{ fileId:fileID }); api('drive','search',{ query:run.marker,parent:folderID });
  wrapper('jmdrive','find',{ query:run.marker,folder:folderID }); wrapper('jmdrive','info',{ target:fileID });
  wrapper('jmdrive','get',{ target:fileID,out:join(run.directory,'wrapper-download.txt') });
  const uploadedID = identifier(wrapper('jmdrive','put',{ path:source,folder:folderID,name:run.marker + '-wrapper.txt' })); run.register('drive','drive','files.delete',{ fileId:uploadedID });
  wrapper('jmdrive','move',{ target:uploadedID,to:secondID,name:run.marker + '-moved.txt' });
  const copiedID = identifier(wrapper('jmdrive','copy',{ target:fileID,to:secondID,name:run.marker + '-copy.txt' }));
  const cleanup = run.register('drive','drive','files.delete',{ fileId:copiedID }); wrapper('jmdrive','trash',{ target:copiedID }); run.discard(cleanup);
  for (const [kind,filename,mime] of officeTypes) {
    source = join(ROOT,'tests/data/live',filename);
    const id = identifier(api('drive','files.create',{ name:run.marker + '-' + filename,parents:[folderID],upload:source,contentType:mime })); run.context[kind] = id; run.register('office','drive','files.delete',{ fileId:id });
  }
  for (const name of ['jmdocs','jmsheets','jmslides']) wrapper(name,'ls',{ name:run.marker });
  api('drive','files.export',{ fileId:run.context.document,mimeType:'application/pdf',output:join(run.directory,'direct-word.pdf') });
  exportOfficePDFs(run);
  run.state.driveCleanupDisposition = 'Only owned fixtures are moved to the OneDrive recycle bin; the account recycle bin is never emptied.';
  if (!run.peer) { (run.state.notTested ??= []).push({ command:'sharing',reason:'No second approved account supplied for notification-free fixture sharing.' }); return; }
  shareFixture(run,fileID,'drive');
  for (const [name,target] of [['jmdrive',fileID],['jmdocs',run.context.document]]) {
    const granted = wrapper(name,'share',{ target,to:run.peer,role:'reader',notify:false });
    for (const permission of records(granted,'permissions')) run.register('drive','drive','permissions.delete',{ fileId:target,permissionId:identifier(permission) });
  }
}
function exportOfficePDFs(run: Run): void {
  for (const [name,kind] of [['jmdocs','document'],['jmslides','presentation'],['jmsheets','workbook']]) {
    const output = join(run.directory,kind + '.pdf'); run.wrapper(name,'export',{ target:run.context[kind],format:'pdf',out:output });
    if (!readFileSync(output).subarray(0,5).equals(Buffer.from('%PDF-'))) throw new Failure(`${name} did not export a PDF`);
  }
}
function shareFixture(run: Run, fileID: string, scope: string): void {
  const granted = run.api('drive','permissions.create',{ fileId:fileID,type:'user',role:'reader',emailAddress:run.peer,sendNotificationEmail:false });
  for (const permission of records(granted,'permissions')) {
    const permissionID = identifier(permission), cleanup = run.register(scope,'drive','permissions.delete',{ fileId:fileID,permissionId:permissionID });
    run.api('drive','permissions.get',{ fileId:fileID,permissionId:permissionID }); run.api('drive','permissions.delete',{ fileId:fileID,permissionId:permissionID }); run.discard(cleanup);
  }
}
export function officeFiles(run: Run): void {
  const { api,wrapper } = run;
  for (const [kind,filename,mime] of officeTypes) {
    const id = identifier(api('drive','files.create',{ name:run.marker + '-' + filename,upload:join(ROOT,'tests/data/live',filename),contentType:mime }));
    run.context[kind] = id; run.register('office','drive','files.delete',{ fileId:id });
  }
  for (const name of ['jmdocs','jmsheets','jmslides']) wrapper(name,'ls',{ limit:1 });
  api('drive','files.export',{ fileId:run.context.document,mimeType:'application/pdf',output:join(run.directory,'direct-word.pdf') }); exportOfficePDFs(run);
  if (run.peer) {
    const fileID = run.context.document; shareFixture(run,fileID,'office');
    for (const name of ['jmdrive','jmdocs']) {
      const granted = wrapper(name,'share',{ target:fileID,to:run.peer,role:'reader',notify:false });
      for (const permission of records(granted,'permissions')) {
        const item = { fileId:fileID,permissionId:identifier(permission) };
        if (!run.state.pendingCleanup.some(job => job.service === 'drive' && job.command === 'permissions.delete' && job.parameters.fileId === item.fileId && job.parameters.permissionId === item.permissionId)) run.register('office','drive','permissions.delete',item);
      }
    }
  }
}
export function sheets(run: Run): void {
  const workbook = run.context.workbook;
  if (!workbook) { (run.state.notTested ??= []).push({ command:'sheets',reason:'OneDrive workbook fixture unavailable; no guessed IDs or repeated probes.' }); return; }
  const { api,wrapper } = run;
  api('sheets','get',{ spreadsheetId:workbook }); api('sheets','values.get',{ spreadsheetId:workbook,range:'Fixture!A1:B2' });
  api('sheets','values.update',{ spreadsheetId:workbook,range:'Fixture!A1:B2',values:[['Name','Count'],['Joom',43]] }); api('sheets','values.clear',{ spreadsheetId:workbook,range:'Fixture!A3:B3' });
  wrapper('jmsheets','read',{ target:workbook,tab:'Fixture',range:'A1:B2' }); wrapper('jmsheets','set',{ target:workbook,tab:'Fixture',cell:'B2',value:'44' });
  const csv = join(run.directory,'data.csv'); writeFileSync(csv,'Name,Count\nJoom,45\n');
  wrapper('jmsheets','load',{ target:workbook,tab:'Fixture',csv,at:'A1' }); wrapper('jmsheets','clear',{ target:workbook,tab:'Fixture',range:'A3:B3' });
  wrapper('jmsheets','export',{ target:workbook,tab:'Fixture',format:'csv',out:join(run.directory,'export.csv') });
}
export function calendarWrappers(run: Run): void {
  const { api,wrapper } = run, calendarID = identifier(api('calendar','calendars.insert',{ summary:run.marker })); run.register('calendar','calendar','calendars.delete',{ calendarId:calendarID });
  const start = startTomorrow();
  const eventID = identifier(api('calendar','events.insert',{ calendarId:calendarID,summary:run.marker,startTime:start.toISOString(),endTime:later(start,1),startTimeZone:'UTC',endTimeZone:'UTC' }));
  const cleanup = run.register('calendar','calendar','events.delete',{ calendarId:calendarID,eventId:eventID });
  api('calendar','events',{ calendar:calendarID,from:start.toISOString(),until:later(start,1) }); wrapper('jmcal','calendars',{ limit:1 });
  for (const command of ['today','week']) wrapper('jmcal',command,{ calendar:calendarID,timezone:'UTC' });
  wrapper('jmcal','find',{ calendar:calendarID,timezone:'UTC',query:run.marker }); wrapper('jmcal','show',{ calendar:calendarID,target:eventID });
  wrapper('jmcal','move',{ calendar:calendarID,target:eventID,at:later(start,2),timezone:'UTC' }); wrapper('jmcal','cancel',{ calendar:calendarID,target:eventID }); run.discard(cleanup);
  run.register('calendar','calendar','events.delete',{ calendarId:calendarID,eventId:identifier(wrapper('jmcal','add',{ calendar:calendarID,title:run.marker,at:start.toISOString(),timezone:'UTC' })) });
}
