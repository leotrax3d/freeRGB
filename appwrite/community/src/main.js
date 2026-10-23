import { createHash } from 'node:crypto';
import {security} from './security.js';
import {adminOperations} from './admin.js';
import {readFileSync} from 'node:fs';
const catalog=JSON.parse(readFileSync(new URL('../catalog.json',import.meta.url),'utf8'));
const allowed=new Set(catalog.effects.map(e=>e.id));
const summaryCache=new Map();

const id = (...parts) => createHash('sha256').update(JSON.stringify(parts)).digest('hex').slice(0, 36);
const query = (method, attribute, values) => JSON.stringify({ method, ...(attribute ? {attribute} : {}), values });
const valid = v => typeof v === 'string' && /^[a-zA-Z0-9][a-zA-Z0-9_.-]{0,127}$/.test(v) && !v.includes('..');
const fail = (status, message) => { throw Object.assign(new Error(message), {status}); };

// Only aggregates leave the server. Never trust an identity supplied in the body.
export default async function main({req, res}) {
  const endpoint = process.env.APPWRITE_FUNCTION_API_ENDPOINT;
  const project = process.env.APPWRITE_FUNCTION_PROJECT_ID;
  const api = async (path, method = 'GET', body, jwt) => {
    const headers = {'content-type':'application/json', 'x-appwrite-project':project};
    if (jwt) headers['x-appwrite-jwt'] = jwt;
    else headers['x-appwrite-key'] = req.headers['x-appwrite-key'];
    const reply = await fetch(`${endpoint}${path}`, {method, headers, body:body === undefined ? undefined : JSON.stringify(body), signal:AbortSignal.timeout(10000)});
    if (!reply.ok) fail(reply.status, 'Community request failed');
    return reply.status === 204 ? null : reply.json();
  };
  const rowsPath = table => `/tablesdb/community/tables/${table}/rows`;
  const list = (table, queries) => api(`${rowsPath(table)}?${new URLSearchParams(queries.map(q => ['queries[]',q]))}`);
  const find = async (table, row) => {try {return await api(`${rowsPath(table)}/${row}`);} catch(e) {if(e.status === 404) return null; throw e;}};
  const create = (table, row, data) => api(rowsPath(table),'POST',{rowId:row,data,permissions:[]});
  const {quota}=security({id,create,find,fail});
  const administrative=adminOperations({api,find,list,create,rowsPath,id,query,fail,quota});
  try {
    const body = req.bodyJson ?? JSON.parse(req.bodyText || '{}');
    if (!body || typeof body !== 'object' || Array.isArray(body)) fail(400,'Invalid request');
    if(Buffer.byteLength(JSON.stringify(body))>40000)fail(413,'Request too large');
    const {action, effect_id:effect} = body;
    if(effect && !allowed.has(effect))fail(400,'Unknown effect');
    if (action === 'summary') {
      if (!Array.isArray(body.effects) || body.effects.length > 64 || !body.effects.every(valid)) fail(400,'Provide up to 64 effect IDs');
      const by_effect = {};
      for (const effect of new Set(body.effects.filter(e=>allowed.has(e)))) {
        const cached=summaryCache.get(effect);
        if(cached && cached.expires>Date.now()){by_effect[effect]=cached.value;continue;}
        let cursor, sum=0, count=0;
        do {
          const queries = [query('equal','effect_id',[effect]),query('limit',null,[100]),query('orderAsc','$id',[])];
          if(cursor) queries.push(query('cursorAfter',null,[cursor]));
          const page = await list('ratings',queries);
          for (const row of page.rows) {sum += row.stars; count++;}
          cursor = page.rows.length === 100 ? page.rows.at(-1).$id : null;
        } while(cursor);
        const downloads = await list('downloads',[query('equal','effect_id',[effect]),query('limit',null,[1])]);
        by_effect[effect] = {average:count ? sum/count : 0,count,downloads:downloads.total};
        summaryCache.set(effect,{expires:Date.now()+30000,value:by_effect[effect]});
      }
      return res.json({by_effect});
    }
    const jwt = req.headers['x-freergb-jwt'] || req.headers['x-appwrite-user-jwt'];
    if (!jwt) fail(401,'Anonymous session required');
    // Verify the JWT against Appwrite. User-ID headers and body fields are not proof of identity.
    const user = await api('/account','GET',undefined,jwt);
    const administrativeResult=await administrative(body,user,jwt);
    if(administrativeResult!==undefined)return res.json(administrativeResult);
    if (action === 'mine') {
      if (!Array.isArray(body.effects) || body.effects.length > 64 || !body.effects.every(valid)) fail(400,'Invalid effects');
      const mine = {};
      for (const effect of new Set(body.effects)) {
        const row = await find('ratings', id(user.$id,effect));
        if(row) mine[effect] = row.stars;
      }
      return res.json({mine});
    }
    if (action === 'rate') {
      if (!valid(effect) || (body.stars !== null && (!Number.isInteger(body.stars) || body.stars < 1 || body.stars > 5))) fail(400,'Choose one to five stars');
      const row = id(user.$id,effect), path = `${rowsPath('ratings')}/${row}`;
      if(body.stars === null) {try {await api(path,'DELETE');} catch(e) {if(e.status!==404) throw e;}}
      else {
        await quota(user.$id,'rate',10,60);
        const data = {effect_id:effect,user_id:user.$id,stars:body.stars};
        try {await create('ratings',row,data);} catch(e) {if(e.status!==409) throw e; await api(path,'PATCH',{data});}
      }
      summaryCache.delete(effect);
      return res.json({saved:true});
    }
    if (action === 'download') {
      if(!valid(effect) || !valid(body.version) || body.success !== true) fail(400,'A completed download and version are required');
      // One successful download per anonymous identity/effect/version, including retries and reinstallations.
      if(!catalog.effects.some(e=>e.id===effect&&e.version===body.version))fail(400,'Unknown effect version');
      const key=id(user.$id,effect,body.version);
      if(await find('downloads',key))return res.json({counted:false});
      await quota(user.$id,'download',20,3600);
      try {await create('downloads',key,{effect_id:effect,user_id:user.$id,version:body.version});}
      catch(e) {if(e.status!==409) throw e; return res.json({counted:false});}
      summaryCache.delete(effect);
      return res.json({counted:true});
    }
    if (action === 'submit') {
      if(!['feedback','hardware','error'].includes(body.kind) || body.reviewed !== true || !valid(body.request_id)) fail(400,'Preview and explicit confirmation required');
      if(!body.payload || typeof body.payload !== 'object' || Array.isArray(body.payload)) fail(400,'Invalid payload');
      const payload = JSON.stringify(body.payload);
      if(Buffer.byteLength(payload) > 30000) fail(400,'Report exceeds 30 KB');
      const row = id(user.$id,body.kind,body.request_id);
      if(await find('submissions',row))return res.json({submitted:true,id:row});
      await quota(user.$id,'submit',5,3600);
      try {await create('submissions',row,{kind:body.kind,user_id:user.$id,payload});} catch(e) {if(e.status!==409) throw e;}
      return res.json({submitted:true,id:row});
    }
    fail(400,'Unknown action');
  } catch(e) {
    const status = e.status >= 400 && e.status < 500 ? e.status : 503;
    return res.json({error:status === 503 ? 'Community service temporarily unavailable' : e.message}, status);
  }
}
