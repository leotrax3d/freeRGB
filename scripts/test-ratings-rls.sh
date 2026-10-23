#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
container="freergb-ratings-test-$$"
trap 'docker rm -f "$container" >/dev/null 2>&1 || true' EXIT
docker run --rm -d --network none --name "$container" -e POSTGRES_PASSWORD=local-test-only postgres:17-alpine >/dev/null
for attempt in $(seq 1 60); do
  if docker exec "$container" pg_isready -h 127.0.0.1 -U postgres >/dev/null 2>&1; then break; fi
  sleep 0.25
done
cat supabase/tests/ratings_rls.sql supabase/migrations/202610230001_effect_ratings.sql supabase/tests/ratings_assertions.sql |
  docker exec -i "$container" psql -U postgres -v ON_ERROR_STOP=1
