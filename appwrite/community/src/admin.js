export function adminOperations({api,find,list,create,rowsPath,id,query,fail,quota}) {
  const safe=value=>typeof value==='string'&&/^[a-zA-Z0-9][a-zA-Z0-9_.-]{0,35}$/.test(value)&&!value.includes('..');
  const clean=row=>({id:row.$id,kind:row.kind,status:row.status||'new',payload:JSON.parse(row.payload),created_at:row.$createdAt,updated_at:row.$updatedAt});
  const page=body=>[query('limit',null,[25]),query('orderDesc','$createdAt',[]),...(body.cursor?[query('cursorAfter',null,[body.cursor])]:[])];
  const messages=async submission=>{
    const result=await list('messages',[query('equal','submission_id',[submission]),query('limit',null,[100]),query('orderAsc','$createdAt',[])]);
    return result.rows.map(m=>({id:m.$id,text:m.text,internal:m.internal,created_at:m.$createdAt,author:m.author_id}));
  };
  const audit=async(user,action,target,request)=>{
    try {await create('audit',id(user,action,target,request),{actor:user,action,target});}catch(e){if(e.status!==409)throw e;}
  };
  return async(body,user,jwt)=>{
    const {action}=body;
    if(body.cursor && !safe(body.cursor))fail(400,'Invalid cursor');
    if(body.submission_id && !safe(body.submission_id))fail(400,'Invalid submission ID');
    if(action==='inbox') {
      const result=await list('submissions',[query('equal','user_id',[user.$id]),...page(body)]);
      return {items:result.rows.map(clean),total:result.total,next:result.rows.length===25?result.rows.at(-1).$id:null};
    }
    if(action==='thread' || action==='reply') {
      if(!safe(body.submission_id))fail(400,'Submission ID required');
      const row=await find('submissions',body.submission_id);
      if(!row || row.user_id!==user.$id) fail(404,'Message not found');
      if(action==='reply') {
        if(typeof body.text!=='string'||!body.text.trim()||body.text.length>6000||!safe(body.request_id))fail(400,'Invalid reply');
        const key=id(user.$id,row.$id,body.request_id);
        if(!await find('messages',key)) {
          await quota(user.$id,'submit',5,3600);
          try {await create('messages',key,{submission_id:row.$id,user_id:user.$id,author_id:user.$id,text:body.text.trim(),internal:false});}catch(e){if(e.status!==409)throw e;}
        }
        return {saved:true};
      }
      return {submission:clean(row),messages:(await messages(row.$id)).filter(m=>!m.internal).map(({author,...m})=>({...m,from_developer:author!==user.$id}))};
    }
    if(!action?.startsWith('admin.'))return undefined;
    try {await api('/teams/community-admins','GET',undefined,jwt);}catch{fail(403,'Admin membership required');}
    if(action==='admin.list') {
      const filters=[];
      if(body.status)filters.push(query('equal','status',[body.status]));
      const result=await list('submissions',[...filters,...page(body)]);
      return {items:result.rows.map(r=>({...clean(r),user_id:r.user_id})),total:result.total,next:result.rows.length===25?result.rows.at(-1).$id:null};
    }
    if(action==='admin.stats') {
      const totals={};
      for(const table of ['submissions','ratings','downloads','blocked_users']) totals[table]=(await list(table,[query('limit',null,[1])])).total;
      const by_status={};
      for(const state of ['new','in_progress','resolved','spam']) by_status[state]=(await list('submissions',[query('equal','status',[state]),query('limit',null,[1])])).total;
      return {totals,by_status};
    }
    if(action==='admin.audit') {
      const result=await list('audit',page(body));
      return {items:result.rows.map(r=>({id:r.$id,actor:r.actor,action:r.action,target:r.target,created_at:r.$createdAt})),next:result.rows.length===25?result.rows.at(-1).$id:null};
    }
    if(action==='admin.cleanup') {
      const expired=await list('quota_slots',[query('lessThan','expires',[Math.floor(Date.now()/1000)-86400]),query('limit',null,[100])]);
      for(const row of expired.rows)await api(`${rowsPath('quota_slots')}/${row.$id}`,'DELETE');
      return {removed:expired.rows.length};
    }
    if(!safe(body.submission_id))fail(400,'Submission ID required');
    const row=await find('submissions',body.submission_id);
    if(!row)fail(404,'Message not found');
    if(action==='admin.thread')return {submission:{...clean(row),user_id:row.user_id},messages:await messages(row.$id),blocked:!!await find('blocked_users',row.user_id)};
    if(typeof body.request_id!=='string' || !/^[a-zA-Z0-9-]{1,64}$/.test(body.request_id))fail(400,'Request ID required');
    if(action==='admin.reply' || action==='admin.note') {
      if(typeof body.text!=='string'||!body.text.trim()||body.text.length>6000)fail(400,'Enter up to 6000 characters');
      const key=id(user.$id,row.$id,body.request_id);
      if(!await find('messages',key)) {
        await quota(user.$id,'admin-message',30,60);
        try {await create('messages',key,{submission_id:row.$id,user_id:row.user_id,author_id:user.$id,text:body.text.trim(),internal:action==='admin.note'});}catch(e){if(e.status!==409)throw e;}
      }
      await audit(user.$id,action,row.$id,body.request_id);
      return {saved:true};
    }
    if(action==='admin.status') {
      if(!['new','in_progress','resolved','spam'].includes(body.status))fail(400,'Invalid status');
      await audit(user.$id,'status:'+body.status,row.$id,body.request_id);
      await api(`${rowsPath('submissions')}/${row.$id}`,'PATCH',{data:{status:body.status}});
      return {saved:true};
    }
    if(action==='admin.block' || action==='admin.unblock') {
      await audit(user.$id,action,row.$id,body.request_id);
      if(action==='admin.block') {try{await create('blocked_users',row.user_id,{reason:'Moderated in admin inbox'});}catch(e){if(e.status!==409)throw e;}}
      else if(await find('blocked_users',row.user_id)) await api(`${rowsPath('blocked_users')}/${row.user_id}`,'DELETE');
      return {saved:true};
    }
    if(action==='admin.delete') {
      await audit(user.$id,action,row.$id,body.request_id);
      for(const message of await messages(row.$id))await api(`${rowsPath('messages')}/${message.id}`,'DELETE');
      await api(`${rowsPath('submissions')}/${row.$id}`,'DELETE');
      return {deleted:true};
    }
    fail(400,'Unknown admin action');
  };
}
