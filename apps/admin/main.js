import {ID} from 'appwrite';
import {account,functions,startupPing} from './appwrite.js';
import './style.css';
import catalog from './catalog.json';
const root=document.querySelector('#app');
const labels={new:'Neu',in_progress:'In Bearbeitung',resolved:'Erledigt',spam:'Spam'};
const kinds={feedback:'Feedback',hardware:'Hardware',error:'Fehlerbericht'};
const state={user:null,otpUser:null,view:'inbox',items:[],stats:null,selected:null,thread:null,status:'',cursor:null,next:null,error:'',busy:false,draft:'',replyRequest:crypto.randomUUID(),internal:false,effects:{},audit:[],auditNext:null};
const esc=value=>String(value??'').replace(/[&<>"']/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;',"'":'&#39;'}[c]));
const date=value=>value?new Date(value).toLocaleString('de-DE',{dateStyle:'short',timeStyle:'short'}):'—';
const brand='<div class="brand"><i></i><i></i><i></i> freeRGB</div>';
async function call(action,body={}) {
  const {jwt}=await account.createJWT();
  const result=await functions.createExecution({functionId:'community-api',body:JSON.stringify({action,...body}),async:false,headers:{'x-freergb-jwt':jwt}});
  let data;try{data=JSON.parse(result.responseBody);}catch{throw new Error('Ungültige Serverantwort.');}
  if(result.responseStatusCode>=400)throw new Error(result.responseStatusCode===429?'Zu viele Anfragen. Bitte später erneut versuchen.':data.error||'Anfrage fehlgeschlagen.');
  return data;
}
async function task(work) {state.busy=true;state.error='';render();try{await work();}catch(e){state.error=e.message||'Verbindung fehlgeschlagen.';}finally{state.busy=false;render();}}
async function load() {
  if(state.view==='inbox') {
    const result=await call('admin.list',{status:state.status||undefined,cursor:state.cursor||undefined});
    state.items=result.items;state.next=result.next;
    if(state.selected)state.thread=await call('admin.thread',{submission_id:state.selected});
  }
  if(state.view==='stats')state.effects=(await call('summary',{effects:catalog.effects.map(e=>e.id)})).by_effect;
  if(state.view==='audit') {const data=await call('admin.audit',{cursor:state.cursor||undefined});state.audit=data.items;state.auditNext=data.next;}
  state.stats=await call('admin.stats');
}
function render() {
  const disabled=state.busy?'disabled':'';
  if(!state.user){
    root.innerHTML=`<div class="login">${brand}<h1>Admin-Anmeldung</h1><p>Support und Marketplace-Verwaltung</p>${state.error?`<p role="alert" class="notice">${esc(state.error)}</p>`:''}<form id="login"><label>${state.otpUser?'Einmalcode':'E-Mail-Adresse'}<input id="credential" type="${state.otpUser?'text':'email'}" autocomplete="${state.otpUser?'one-time-code':'email'}" value="${state.otpUser?'':'freergbteam@gmail.com'}" required ${disabled}></label><button class="primary" ${disabled}>${state.busy?'Bitte warten…':state.otpUser?'Anmelden':'Code anfordern'}</button>${state.otpUser?'<button type="button" id="reset">Andere Adresse</button>':''}</form></div>`;
    document.querySelector('#reset')?.addEventListener('click',()=>{state.otpUser=null;render();});
    document.querySelector('#login').onsubmit=e=>{e.preventDefault();const value=document.querySelector('#credential').value;void task(async()=>{
      if(!state.otpUser){const token=await account.createEmailToken({userId:ID.unique(),email:value});state.otpUser=token.userId;}
      else{await account.createSession({userId:state.otpUser,secret:value.trim()});const stats=await call('admin.stats');state.user=await account.get();state.stats=stats;state.otpUser=null;await load();}
    });};return;
  }
  const totals=state.stats?.totals??{};
  root.innerHTML=`<div class="layout"><aside>${brand}<span class="subtle">ADMIN WORKSPACE</span><nav>${[['inbox','Posteingang'],['stats','Statistiken'],['audit','Änderungsverlauf']].map(([id,title])=>`<button data-view="${id}" class="${state.view===id?'active':''}">${title}</button>`).join('')}</nav><div style="margin-top:50px"><p class="subtle">${esc(state.user.email)}</p><button id="logout" ${disabled}>Abmelden</button></div></aside><main><header class="topbar"><div><p class="subtle">FREERGB / COMMUNITY</p><h1>${{inbox:'Posteingang',stats:'Statistiken',audit:'Änderungsverlauf'}[state.view]}</h1></div><button id="refresh" ${disabled}>${state.busy?'Lädt…':'Aktualisieren'}</button></header>${state.error?`<p role="alert" class="notice">${esc(state.error)}</p>`:''}<div class="cards">${[['Einsendungen',totals.submissions],['Neu',state.stats?.by_status?.new],['Bewertungen',totals.ratings],['Downloads',totals.downloads]].map(([label,value])=>`<div class="card"><span class="subtle">${label}</span><strong class="metric">${value??'—'}</strong></div>`).join('')}</div><div id="content"></div></main></div>`;
  document.querySelectorAll('[data-view]').forEach(button=>button.onclick=()=>void task(async()=>{state.view=button.dataset.view;state.cursor=null;await load();}));
  document.querySelector('#refresh').onclick=()=>void task(load);
  document.querySelector('#logout').onclick=()=>void task(async()=>{await account.deleteSession({sessionId:'current'});Object.assign(state,{user:null,items:[],thread:null,selected:null,draft:'',stats:null});});
  const content=document.querySelector('#content');
  if(state.view==='stats') {
    content.innerHTML=`<div class="card"><h2>Veröffentlichte Effekte</h2><table><thead><tr><th>Effekt</th><th>Version</th><th>Bewertung</th><th>Stimmen</th><th>Downloads</th></tr></thead><tbody>${catalog.effects.map(effect=>{const s=state.effects[effect.id];return `<tr><td>${esc(effect.name)}</td><td>${esc(effect.version)}</td><td>${s?.count?s.average.toFixed(1):'—'}</td><td>${s?.count??0}</td><td>${s?.downloads??0}</td></tr>`;}).join('')}</tbody></table></div><div class="card" style="margin-top:20px"><h2>Einsendungen nach Status</h2><div class="cards">${Object.entries(labels).map(([key,label])=>`<div><span class="subtle">${label}</span><strong class="metric">${state.stats?.by_status?.[key]??0}</strong></div>`).join('')}</div><p class="subtle">${totals.blocked_users??0} gesperrte Identitäten · Ausschließlich echte Daten</p></div>`;return;
  }
  if(state.view==='audit'){
    content.innerHTML=`<div class="card"><table><thead><tr><th>Zeit</th><th>Aktion</th><th>Admin</th><th>Einsendung</th></tr></thead><tbody>${state.audit.map(r=>`<tr><td>${date(r.created_at)}</td><td>${esc(r.action)}</td><td>${esc(r.actor)}</td><td>${esc(r.target)}</td></tr>`).join('')}</tbody></table>${state.audit.length?'':'<p class="empty">Noch keine Änderungen</p>'}<div class="actions"><button id="first">Neueste</button><button id="older" ${!state.auditNext||state.busy?'disabled':''}>Ältere</button></div></div>`;
    document.querySelector('#first').onclick=()=>void task(async()=>{state.cursor=null;await load();});
    document.querySelector('#older').onclick=()=>void task(async()=>{state.cursor=state.auditNext;await load();});return;
  }
  content.innerHTML=`<div class="toolbar"><select id="filter" aria-label="Status filtern" ${disabled}><option value="">Alle Status</option>${Object.entries(labels).map(([key,label])=>`<option value="${key}" ${state.status===key?'selected':''}>${label}</option>`).join('')}</select><button id="first" ${disabled}>Neueste</button><button id="older" ${!state.next||state.busy?'disabled':''}>Ältere</button></div><div class="inbox"><div class="list">${state.items.length?state.items.map(item=>`<button class="item ${item.id===state.selected?'selected':''}" data-id="${esc(item.id)}"><div class="row"><span class="subtle">${esc(kinds[item.kind]||item.kind)}</span><span class="badge ${esc(item.status)}">${esc(labels[item.status]||item.status)}</span></div><strong>${esc(item.payload.message||'Hardware- oder Fehlerbericht')}</strong><span class="subtle">${date(item.created_at)}</span></button>`).join(''):'<div class="card empty">Keine Einsendungen</div>'}</div><section class="detail" id="detail"><p class="empty">Eine Einsendung auswählen</p></section></div>`;
  document.querySelector('#filter').onchange=e=>void task(async()=>{state.status=e.target.value;state.cursor=null;await load();});
  document.querySelector('#first').onclick=()=>void task(async()=>{state.cursor=null;await load();});
  document.querySelector('#older').onclick=()=>void task(async()=>{state.cursor=state.next;await load();});
  document.querySelectorAll('[data-id]').forEach(button=>button.onclick=()=>void task(async()=>{state.selected=button.dataset.id;state.draft='';state.thread=await call('admin.thread',{submission_id:state.selected});}));
  if(!state.thread)return;
  const {submission,messages,blocked}=state.thread;
  document.querySelector('#detail').innerHTML=`<div class="row"><h2>${esc(kinds[submission.kind]||submission.kind)}</h2><span class="subtle">${date(submission.created_at)}</span></div><p class="subtle">Identität ${esc(submission.user_id)}</p><pre class="payload">${esc(submission.payload.message||JSON.stringify(submission.payload,null,2))}</pre><label>Status<select id="status" ${disabled}>${Object.entries(labels).map(([key,label])=>`<option value="${key}" ${submission.status===key?'selected':''}>${label}</option>`).join('')}</select></label><h3 style="margin-top:28px">Verlauf</h3>${messages.map(m=>`<div class="message ${m.internal?'note':''}"><div class="subtle">${m.internal?'Interne Notiz':m.author===submission.user_id?'Nutzer':'Antwort'} · ${date(m.created_at)}</div>${esc(m.text)}</div>`).join('')}<form id="reply"><label>Nachricht<textarea id="text" rows="5" maxlength="6000" required ${disabled}>${esc(state.draft)}</textarea></label><label style="display:flex;align-items:center;margin:12px 0"><input id="internal" type="checkbox" style="width:auto" ${state.internal?'checked':''}> Interne Notiz – für Nutzer unsichtbar</label><button class="primary" ${disabled}>${state.internal?'Notiz speichern':'Antwort senden'}</button></form><div class="actions"><button id="block" ${disabled}>${blocked?'Identität entsperren':'Identität sperren'}</button><button id="delete" class="danger" ${disabled}>Einsendung löschen</button></div>`;
  document.querySelector('#text').oninput=e=>{state.draft=e.target.value;state.replyRequest=crypto.randomUUID();};
  document.querySelector('#internal').onchange=e=>{state.internal=e.target.checked;state.replyRequest=crypto.randomUUID();render();};
  const mutate=async(action,extra={})=>{await call(action,{submission_id:state.selected,request_id:crypto.randomUUID(),...extra});await load();};
  document.querySelector('#status').onchange=e=>{const status=e.target.value;void task(()=>mutate('admin.status',{status}));};
  document.querySelector('#reply').onsubmit=e=>{e.preventDefault();void task(async()=>{await mutate(state.internal?'admin.note':'admin.reply',{text:state.draft,request_id:state.replyRequest});state.draft='';state.replyRequest=crypto.randomUUID();});};
  document.querySelector('#block').onclick=()=>void task(()=>mutate(blocked?'admin.unblock':'admin.block'));
  document.querySelector('#delete').onclick=()=>{
    const dialog=document.createElement('dialog');dialog.innerHTML='<h2>Einsendung löschen?</h2><p>Die Einsendung und alle Antworten und Notizen werden endgültig gelöscht. Im Änderungsverlauf bleibt nur der Löschvorgang erhalten.</p><div class="actions"><button id="cancelDelete">Abbrechen</button><button class="danger" id="confirmDelete">Endgültig löschen</button></div>';document.body.append(dialog);dialog.showModal();
    dialog.querySelector('#cancelDelete').onclick=()=>dialog.remove();
    dialog.querySelector('#confirmDelete').onclick=()=>{dialog.remove();void task(async()=>{await call('admin.delete',{submission_id:state.selected,request_id:crypto.randomUUID()});state.selected=null;state.thread=null;await load();});};
  };
}
render();
void startupPing.then(ok=>{if(!ok){state.error='Appwrite-Verbindung fehlgeschlagen. Bitte Verbindung und Web-App-Freigabe prüfen.';render();}});
void task(async()=>{try{const user=await account.get();state.stats=await call('admin.stats');state.user=user;await load();}catch{state.user=null;}});
