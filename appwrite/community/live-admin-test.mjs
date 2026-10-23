// Explicit live smoke test; administrator JWT arrives through stdin, never a file.
import {createInterface} from 'node:readline';
import {createHash} from 'node:crypto';
import assert from 'node:assert/strict';
if(process.stdin.isTTY)process.stdin.setRawMode(true);
console.log('READY_FOR_HIDDEN_INPUT');
const input=createInterface({input:process.stdin,terminal:false});
const config=await new Promise(resolve=>input.once('line',line=>{input.close();resolve(JSON.parse(line));}));
const endpoint='https://fra.cloud.appwrite.io/v1',project='6adb5259001299138e7b';
const id=(...p)=>createHash('sha256').update(JSON.stringify(p)).digest('hex').slice(0,36);
async function api(path,body,cookie){
 const reply=await fetch(endpoint+path,{method:body===undefined?'GET':'POST',headers:{'content-type':'application/json','x-appwrite-project':project,...(cookie?{cookie}:{})},body:body===undefined?undefined:JSON.stringify(body)});
 const value=await reply.json();assert.ok(reply.ok,`HTTP ${reply.status} at ${path}`);return {value,reply};
}
async function identity(){const {value,reply}=await api('/account/sessions/anonymous',{});const cookie=reply.headers.getSetCookie().map(s=>s.split(';')[0]).find(s=>s.startsWith(`a_session_${project}=`));assert.ok(cookie);return {id:value.userId,cookie,jwt:(await api('/account/jwts',{},cookie)).value.jwt};}
async function call(action,body,user){const {value,reply}=await api('/functions/community-api/executions',{body:JSON.stringify({action,...body}),async:false,headers:user?{'x-freergb-jwt':user.jwt}:{}},user?.cookie);return {status:value.responseStatusCode,body:JSON.parse(value.responseBody||'{}'),waf:reply.headers.get('x-appwrite-waf-rule')};}
const alice=await identity(),bob=await identity(),admin={jwt:config.jwt};
console.log('CLEANUP_USERS',JSON.stringify([alice.id,bob.id]));
let submission;
try {
 const probe=await call('admin.stats',{},admin);
 assert.equal(probe.status,200,JSON.stringify(probe.body));
 assert.equal(probe.waf,'community-executions-ip');
 assert.equal((await call('admin.list',{},alice)).status,403);
 const sent=await call('submit',{kind:'feedback',request_id:'live-admin-test',reviewed:true,payload:{message:'Synthetic admin integration test. No personal data.'}},alice);
 assert.equal(sent.status,200,JSON.stringify(sent.body));submission=sent.body.id;console.log('CLEANUP_SUBMISSION',submission);
 assert.equal((await call('thread',{submission_id:submission},bob)).status,404);
 const reply={submission_id:submission,request_id:'live-reply-1',text:'Synthetic developer answer.'};
 assert.equal((await call('admin.reply',reply,admin)).status,200);
 assert.equal((await call('admin.reply',reply,admin)).status,200);
 assert.equal((await call('admin.note',{...reply,request_id:'live-note-1',text:'PRIVATE SYNTHETIC NOTE'},admin)).status,200);
 const own=await call('thread',{submission_id:submission},alice);
 assert.equal(own.body.messages.length,1);assert.equal(own.body.messages[0].from_developer,true);assert.ok(!JSON.stringify(own.body).includes('PRIVATE SYNTHETIC NOTE'));
 assert.equal((await call('admin.status',{submission_id:submission,request_id:'live-status-1',status:'resolved'},admin)).status,200);
 assert.ok((await call('inbox',{},alice)).body.items.some(item=>item.id===submission&&item.status==='resolved'));
 for(let i=0;i<4;i++)assert.equal((await call('submit',{kind:'feedback',request_id:`live-quota-${i}`,reviewed:true,payload:{message:'Synthetic quota test'}},alice)).status,200);
 assert.equal((await call('submit',{kind:'feedback',request_id:'live-quota-overflow',reviewed:true,payload:{message:'Must be denied'}},alice)).status,429);
 console.log('PASS: live admin authorization, WAF match, private inbox, reply deduplication, internal-note isolation and persistent quota.');
} finally {
 const inbox=await call('inbox',{},alice);
 for(const item of inbox.body.items??[])await call('admin.delete',{submission_id:item.id,request_id:`cleanup-${item.id.slice(0,12)}`},admin);
 console.log('CLEANUP_USER',alice.id);
 console.log('CLEANUP_ADMIN',config.adminId);
 // Remaining quota/audit rows are printed by ID for narrowly scoped operator cleanup.
 const bucket=Math.floor(Date.now()/1000/3600);
 console.log('CLEANUP_QUOTAS',JSON.stringify(Array.from({length:5},(_,slot)=>id(alice.id,'submit',bucket,slot))));
}
