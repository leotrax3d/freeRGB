-- Run once in the Supabase SQL editor. No service-role key goes into freeRGB.
begin;
create table if not exists public.effect_catalog (
  id text primary key check (id ~ '^[a-zA-Z0-9][a-zA-Z0-9._-]{0,63}$')
);
insert into public.effect_catalog(id) values ('aurora'),('comet'),('prism') on conflict do nothing;
create table if not exists public.effect_ratings (
  user_id uuid not null references auth.users(id) on delete cascade,
  effect_id text not null references public.effect_catalog(id),
  stars smallint not null check (stars between 1 and 5),
  primary key (user_id,effect_id)
);
create index if not exists effect_ratings_effect_idx on public.effect_ratings(effect_id);
alter table public.effect_catalog enable row level security;
alter table public.effect_ratings enable row level security;
revoke all on public.effect_catalog from anon,authenticated;
revoke all on public.effect_ratings from anon,authenticated;
grant select on public.effect_catalog to anon,authenticated;
grant select,delete on public.effect_ratings to authenticated;
grant insert(user_id,effect_id,stars),update(user_id,effect_id,stars) on public.effect_ratings to authenticated;
drop policy if exists effect_catalog_read on public.effect_catalog;
create policy effect_catalog_read on public.effect_catalog for select to anon,authenticated using (true);
drop policy if exists effect_ratings_own_read on public.effect_ratings;
create policy effect_ratings_own_read on public.effect_ratings for select to authenticated using ((select auth.uid())=user_id);
drop policy if exists effect_ratings_own_insert on public.effect_ratings;
create policy effect_ratings_own_insert on public.effect_ratings for insert to authenticated with check ((select auth.uid())=user_id);
drop policy if exists effect_ratings_own_update on public.effect_ratings;
create policy effect_ratings_own_update on public.effect_ratings for update to authenticated using ((select auth.uid())=user_id) with check ((select auth.uid())=user_id);
drop policy if exists effect_ratings_own_delete on public.effect_ratings;
create policy effect_ratings_own_delete on public.effect_ratings for delete to authenticated using ((select auth.uid())=user_id);
-- Return only aggregate counts, never anonymous ids or individual votes.
create or replace function public.effect_rating_summary()
returns table(effect_id text,average double precision,count bigint)
language sql stable security definer set search_path = '' as $$
  select r.effect_id,avg(r.stars)::double precision,count(*)
  from public.effect_ratings r group by r.effect_id;
$$;
revoke all on function public.effect_rating_summary() from public;
grant execute on function public.effect_rating_summary() to anon,authenticated;
commit;
