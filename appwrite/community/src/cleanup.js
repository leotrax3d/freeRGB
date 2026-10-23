// Dedicated scheduled Function with no client execute permission.
export default async function cleanup({req,res}) {
 const base=process.env.APPWRITE_FUNCTION_API_ENDPOINT+'/tablesdb/community/tables/quota_slots/rows';
 const headers={'x-appwrite-project':process.env.APPWRITE_FUNCTION_PROJECT_ID,'x-appwrite-key':req.headers['x-appwrite-key']};
 let removed=0;
 for(let batch=0;batch<20;batch++) {
  const queries=[JSON.stringify({method:'lessThan',attribute:'expires',values:[Math.floor(Date.now()/1000)-86400]}),JSON.stringify({method:'limit',values:[100]})];
  const response=await fetch(base+'?'+new URLSearchParams(queries.map(q=>['queries[]',q])),{headers,signal:AbortSignal.timeout(10000)});
  if(!response.ok)return res.json({error:'Cleanup failed'},503);
  const {rows}=await response.json();
  for(const row of rows) {
   const reply=await fetch(base+'/'+row.$id,{method:'DELETE',headers,signal:AbortSignal.timeout(10000)});
   if(!reply.ok&&reply.status!==404)return res.json({error:'Cleanup failed'},503);
   removed++;
  }
  if(rows.length<100)break;
 }
 return res.json({removed});
}
