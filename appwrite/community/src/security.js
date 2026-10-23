export function security({id,create,find,fail}) {
  return {
    async quota(user,action,limit,seconds) {
      if(await find('blocked_users',user)) fail(403,'This identity is blocked');
      const bucket=Math.floor(Date.now()/1000/seconds);
      for(let slot=0;slot<limit;slot++) {
        try {await create('quota_slots',id(user,action,bucket,slot),{expires:(bucket+1)*seconds}); return;}
        catch(e) {if(e.status!==409) throw e;}
      }
      fail(429,'Too many requests. Please try again later.');
    }
  };
}
