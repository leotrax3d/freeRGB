// Opt-in integration test against the configured public project. Synthetic data only.
import assert from 'node:assert/strict';
const endpoint='https://fra.cloud.appwrite.io/v1', project='6adb5259001299138e7b';
const effect='integration-test-20261023';
async function api(path,body,cookie) {
  const reply=await fetch(endpoint+path,{method:body===undefined?'GET':'POST',headers:{'x-appwrite-project':project,'content-type':'application/json',...(cookie?{cookie}:{})},body:body===undefined?undefined:JSON.stringify(body)});
  const value=await reply.json();
  assert.ok(reply.ok,`Appwrite HTTP ${reply.status} at ${path}`);
  return {reply,value};
}
async function identity() {
  const {reply,value}=await api('/account/sessions/anonymous',{});
  const cookie=reply.headers.getSetCookie().map(s=>s.split(';')[0]).find(s=>s.startsWith(`a_session_${project}=`));
  assert.ok(cookie,'Native cookie is present');
  const jwt=(await api('/account/jwts',{},cookie)).value.jwt;
  return {cookie,jwt,user:value.userId};
}
async function call(body,user) {
  const {value}=await api('/functions/community-api/executions',{body:JSON.stringify(body),async:false,headers:user?{'x-freergb-jwt':user.jwt}:{}},user?.cookie);
  return {status:value.responseStatusCode,body:JSON.parse(value.responseBody||'{}')};
}
const alice=await identity(),bob=await identity();
// Print only disposable IDs for cleanup, never session secrets or JWTs.
console.log('TEST_USERS',JSON.stringify([alice.user,bob.user]));
assert.equal((await call({action:'rate',effect_id:effect,stars:5})).status,401);
assert.equal((await call({action:'rate',effect_id:effect,stars:5},alice)).status,200);
assert.equal((await call({action:'rate',effect_id:effect,stars:3,user_id:alice.user},bob)).status,200);
assert.deepEqual((await call({action:'mine',effects:[effect]},alice)).body,{mine:{[effect]:5}});
const download={action:'download',effect_id:effect,version:'test-1',success:true};
assert.equal((await call(download,alice)).body.counted,true);
assert.equal((await call(download,alice)).body.counted,false);
assert.deepEqual((await call({action:'summary',effects:[effect]})).body,{by_effect:{[effect]:{average:4,count:2,downloads:1}}});
for(const table of ['ratings','downloads','submissions']) {
  const response=await fetch(`${endpoint}/tablesdb/community/tables/${table}/rows`,{headers:{'x-appwrite-project':project,cookie:alice.cookie}});
  assert.ok([401,403,404].includes(response.status),`${table} rejects direct client reads`);
}
const submitted=await call({action:'submit',kind:'feedback',request_id:'test-request-1',reviewed:true,payload:{message:'Synthetic integration test; no personal or hardware data.'}},alice);
assert.equal(submitted.status,200);
console.log('TEST_SUBMISSION',submitted.body.id);
await call({action:'rate',effect_id:effect,stars:null},alice);
await call({action:'rate',effect_id:effect,stars:null},bob);
console.log('PASS: anonymous auth, isolated votes, public aggregates, duplicate download, private submissions.');
