import {test} from 'node:test';
import assert from 'node:assert/strict';
import cleanup from '../src/cleanup.js';
import {retentionDays} from '../src/retention.js';

test('scheduled cleanup retains quota grace and never touches other tables', async () => {
  const originalFetch=globalThis.fetch;
  const originalNow=Date.now;
  const endpoint=process.env.APPWRITE_FUNCTION_API_ENDPOINT;
  const project=process.env.APPWRITE_FUNCTION_PROJECT_ID;
  process.env.APPWRITE_FUNCTION_API_ENDPOINT='https://appwrite.test/v1';
  process.env.APPWRITE_FUNCTION_PROJECT_ID='test';
  Date.now=()=>2000000000000;
  const calls=[];
  globalThis.fetch=async (url,options)=>{
    const parsed=new URL(url);
    assert.ok(parsed.pathname.startsWith('/v1/tablesdb/community/tables/quota_slots/rows'));
    calls.push({parsed,options});
    if(options.method==='DELETE')return new Response(null,{status:204});
    const queries=parsed.searchParams.getAll('queries[]').map(JSON.parse);
    assert.equal(queries[0].attribute,'expires');
    assert.equal(queries[0].method,'lessThan');
    assert.deepEqual(queries[0].values,[2000000000-86400]);
    return Response.json({rows:[{$id:'expired-slot'}]});
  };
  try {
    assert.equal(retentionDays.quotaGraceAfterExpiry,1);
    const result=await cleanup({req:{headers:{'x-appwrite-key':'test-key'}},res:{json:body=>body}});
    assert.deepEqual(result,{removed:1});
    assert.equal(calls.length,2);
    assert.equal(calls[1].options.method,'DELETE');
    assert.ok(calls[1].parsed.pathname.endsWith('/expired-slot'));
  } finally {
    globalThis.fetch=originalFetch;
    Date.now=originalNow;
    if(endpoint===undefined)delete process.env.APPWRITE_FUNCTION_API_ENDPOINT;
    else process.env.APPWRITE_FUNCTION_API_ENDPOINT=endpoint;
    if(project===undefined)delete process.env.APPWRITE_FUNCTION_PROJECT_ID;
    else process.env.APPWRITE_FUNCTION_PROJECT_ID=project;
  }
});
