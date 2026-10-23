import {test} from 'node:test';
import assert from 'node:assert/strict';
import main from '../src/main.js';

test('verified identities isolate votes, retries deduplicate, summaries reveal no users', async () => {
  process.env.APPWRITE_FUNCTION_API_ENDPOINT='https://appwrite.test/v1';
  process.env.APPWRITE_FUNCTION_PROJECT_ID='test';
  const tables={ratings:new Map(),downloads:new Map(),submissions:new Map()};
  const original=global.fetch;
  global.fetch=async (url,opts) => {
    const u=new URL(url), body=opts.body && JSON.parse(opts.body);
    const answer=(status,data)=>new Response(status===204?null:JSON.stringify(data),{status});
    if(u.pathname.endsWith('/account')) {
      assert.equal(opts.headers['x-appwrite-key'],undefined);
      return ['alice','bob'].includes(opts.headers['x-appwrite-jwt']) ? answer(200,{$id:opts.headers['x-appwrite-jwt']}) : answer(401,{});
    }
    assert.equal(opts.headers['x-appwrite-jwt'],undefined);
    const [,table,row]=u.pathname.match(/tables\/(\w+)\/rows(?:\/(\w+))?$/);
    const db=tables[table];
    if(opts.method==='POST') {if(db.has(body.rowId))return answer(409,{}); db.set(body.rowId,{$id:body.rowId,...body.data}); return answer(201,{});}
    if(opts.method==='PATCH'){Object.assign(db.get(row),body.data);return answer(200,{});}
    if(opts.method==='DELETE'){db.delete(row);return answer(204);}
    if(row)return db.has(row)?answer(200,db.get(row)):answer(404,{});
    let rows=[...db.values()];
    for(const raw of u.searchParams.getAll('queries[]')) {const q=JSON.parse(raw); if(q.method==='equal') rows=rows.filter(r=>q.values.includes(r[q.attribute]));}
    return answer(200,{rows,total:rows.length});
  };
  const call=(body,user)=>main({req:{bodyJson:body,headers:{'x-appwrite-key':'runtime-only',...(user?{'x-freergb-jwt':user}:{})}},res:{json:(body,status=200)=>({body,status})}});
  try {
    assert.equal((await call({action:'rate',effect_id:'aurora',stars:5})).status,401);
    assert.equal((await call({action:'rate',effect_id:'aurora',stars:5},'forged')).status,401);
    assert.equal((await call({action:'rate',effect_id:'aurora',stars:6},'alice')).status,400);
    await call({action:'rate',effect_id:'aurora',stars:5},'alice');
    await call({action:'rate',effect_id:'aurora',stars:3,user_id:'alice'},'bob');
    await call({action:'rate',effect_id:'aurora',stars:1,user_id:'bob'},'alice');
    assert.equal(tables.ratings.size,2);
    assert.deepEqual((await call({action:'mine',effects:['aurora']},'bob')).body,{mine:{aurora:3}});
    const download={action:'download',effect_id:'aurora',version:'1.0.0',success:true};
    assert.equal((await call({...download,success:false},'alice')).status,400);
    const concurrent=await Promise.all([call(download,'alice'),call(download,'alice')]);
    assert.deepEqual(concurrent.map(r=>r.body.counted).sort(),[false,true]);
    assert.deepEqual((await call({action:'summary',effects:['aurora']})).body,{by_effect:{aurora:{average:2,count:2,downloads:1}}});
    assert.equal((await call({action:'submit',kind:'hardware',request_id:'request-1',payload:{os:'test'}},'alice')).status,400);
    const submission={action:'submit',kind:'hardware',request_id:'request-1',reviewed:true,payload:{os:'test'}};
    await call(submission,'alice'); await call(submission,'alice');
    assert.equal(tables.submissions.size,1);
    assert.equal((await call({action:'list-submissions'},'alice')).status,400);
    await call({action:'rate',effect_id:'aurora',stars:null},'alice');
    assert.equal(tables.ratings.size,1);
  } finally {global.fetch=original;}
});
