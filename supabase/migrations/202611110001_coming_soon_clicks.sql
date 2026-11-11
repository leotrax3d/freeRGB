-- Coming-soon click counter for the website. Run once in the Supabase (Lovable Cloud) SQL editor.
-- One click per device, enforced twice:
--   1. device_id: a random UUID the browser keeps in localStorage (primary key).
--   2. device_hash: a keyed SHA-256 of IP address + user agent, so clearing storage does
--      not count the same device again. The raw IP is never stored; the key lives in a
--      table nobody but the database owner can read.
-- The website can only call the two functions below; it cannot read or write the tables.
begin;
create extension if not exists pgcrypto with schema extensions;

create table if not exists public.coming_soon_secret (
  id boolean primary key default true check (id),
  salt text not null default encode(extensions.gen_random_bytes(32), 'hex')
);
insert into public.coming_soon_secret default values on conflict do nothing;

create table if not exists public.coming_soon_clicks (
  device_id uuid primary key,
  device_hash text not null unique,
  created_at timestamptz not null default now()
);

alter table public.coming_soon_secret enable row level security;
alter table public.coming_soon_clicks enable row level security;
revoke all on public.coming_soon_secret from anon, authenticated;
revoke all on public.coming_soon_clicks from anon, authenticated;

-- Returns true when this click was counted, false when the device was already counted.
create or replace function public.register_coming_soon_click(p_device uuid)
returns boolean
language plpgsql volatile security definer set search_path = '' as $$
declare
  headers json := coalesce(nullif(current_setting('request.headers', true), ''), '{}')::json;
  ip text := split_part(coalesce(headers->>'cf-connecting-ip', headers->>'x-forwarded-for', headers->>'x-real-ip', ''), ',', 1);
  ua text := left(coalesce(headers->>'user-agent', ''), 512);
  key text;
  inserted int;
begin
  if p_device is null then return false; end if;
  select salt into key from public.coming_soon_secret;
  insert into public.coming_soon_clicks(device_id, device_hash)
  values (p_device, encode(extensions.hmac(trim(ip) || '|' || ua, key, 'sha256'), 'hex'))
  on conflict do nothing;
  get diagnostics inserted = row_count;
  return inserted = 1;
end;
$$;

create or replace function public.coming_soon_click_count()
returns bigint
language sql stable security definer set search_path = '' as $$
  select count(*) from public.coming_soon_clicks;
$$;

revoke all on function public.register_coming_soon_click(uuid) from public;
revoke all on function public.coming_soon_click_count() from public;
grant execute on function public.register_coming_soon_click(uuid) to anon, authenticated;
grant execute on function public.coming_soon_click_count() to anon, authenticated;
commit;
