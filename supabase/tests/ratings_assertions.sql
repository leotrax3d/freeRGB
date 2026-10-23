\set ON_ERROR_STOP on
set role authenticated;
select set_config('request.jwt.claim.sub','11111111-1111-1111-1111-111111111111',false);
insert into public.effect_ratings(user_id,effect_id,stars) values (auth.uid(),'aurora',5);
insert into public.effect_ratings(user_id,effect_id,stars) values (auth.uid(),'aurora',3)
 on conflict(user_id,effect_id) do update set stars=excluded.stars;
do $$ begin
 if (select count(*) from public.effect_ratings)<>1 then raise exception 'upsert must keep one vote';end if;
 begin
  insert into public.effect_ratings values ('22222222-2222-2222-2222-222222222222','aurora',1);
  raise exception 'forged identity accepted';
 exception when insufficient_privilege then null;end;
 begin
  insert into public.effect_ratings values (auth.uid(),'prism',6);
  raise exception 'six stars accepted';
 exception when check_violation then null;end;
 begin
  insert into public.effect_ratings values (auth.uid(),'unpublished-effect',5);
  raise exception 'unpublished effect accepted';
 exception when foreign_key_violation then null;end;
end $$;
select set_config('request.jwt.claim.sub','22222222-2222-2222-2222-222222222222',false);
insert into public.effect_ratings values(auth.uid(),'aurora',5);
do $$ begin
 if (select count(*) from public.effect_ratings)<>1 then raise exception 'another identity rating is readable';end if;
 if (select average from public.effect_rating_summary() where effect_id='aurora')<>4 then raise exception 'wrong shared average';end if;
 if (select count from public.effect_rating_summary() where effect_id='aurora')<>2 then raise exception 'wrong shared count';end if;
 begin
  update public.effect_ratings set user_id='11111111-1111-1111-1111-111111111111' where user_id=auth.uid();
  raise exception 'identity transfer accepted';
 exception when insufficient_privilege then null;end;
 delete from public.effect_ratings where user_id='11111111-1111-1111-1111-111111111111';
 if (select count from public.effect_rating_summary() where effect_id='aurora')<>2 then raise exception 'deleted another identity vote';end if;
end $$;
set role anon;
do $$ begin
 if (select average from public.effect_rating_summary() where effect_id='aurora')<>4 then raise exception 'public shared average unavailable';end if;
 begin
  perform * from public.effect_ratings;
  raise exception 'anonymous public read exposed individual votes';
 exception when insufficient_privilege then null;end;
 begin
  insert into public.effect_ratings values ('11111111-1111-1111-1111-111111111111','prism',5);
  raise exception 'unauthenticated write accepted';
 exception when insufficient_privilege then null;end;
end $$;
reset role;
select 'PASS: rating bounds, one vote per identity, aggregate counts, identity spoofing, private rows, public summary and deletion isolation' as result;
