# FixOnce Backend Setup

Step-by-step instructions for setting up the FixOnce Supabase backend. This
guide is designed to be followed by a human or an LLM agent. When a step
requires manual action (browser login, dashboard toggle, etc.), it is clearly
marked with **MANUAL ACTION REQUIRED**.

---

## Prerequisites

Install these tools before starting:

```bash
# Supabase CLI
# macOS:
brew install supabase/tap/supabase
# Linux / other: see https://supabase.com/docs/guides/cli/getting-started

# Deno runtime (used by edge functions)
curl -fsSL https://deno.land/install.sh | sh

# Verify both are installed
supabase --version   # expect 2.x
deno --version       # expect 2.x
```

You also need:
- A [Supabase](https://supabase.com) account
- A [GitHub OAuth app](https://github.com/settings/developers) (for user authentication)
- A [VoyageAI](https://www.voyageai.com/) API key (for embeddings)

---

## Step 1: Create a Supabase project

### MANUAL ACTION REQUIRED

1. Go to [https://supabase.com/dashboard/projects](https://supabase.com/dashboard/projects)
2. Click **New project**
3. Choose your organization
4. Set a project name (e.g. `fixonce`)
5. Set a strong database password — save it somewhere safe
6. Choose a region close to your users
7. Click **Create new project** and wait for it to provision

Once ready, note these values from your project's **Settings → API** page
(`https://supabase.com/dashboard/project/<ref>/settings/api`):

| Value | Where to find it | Used for |
|-------|-------------------|----------|
| **Project ref** | URL: `https://supabase.com/dashboard/project/<THIS>` | Linking the CLI |
| **Project URL** | Under "Project URL" | `FIXONCE_API_URL` env var |
| **Anon key** | Under "Project API keys" → `anon` `public` | Client-side auth |
| **Service role key** | Under "Project API keys" → `service_role` `secret` | Edge function server-side ops |

> **When you have these values, tell the agent and it will continue.**

---

## Step 2: Link the CLI to your project

```bash
cd supabase
supabase link --project-ref <YOUR_PROJECT_REF>
```

You will be prompted for your database password (the one you set in Step 1).

---

## Step 3: Enable required extensions

The migrations enable `pgvector`, `pg_trgm`, and `pg_cron` automatically. However,
`pg_cron` requires a manual enable on some Supabase plans.

### MANUAL ACTION REQUIRED (if `pg_cron` migration fails)

1. Go to **Database → Extensions** in your Supabase dashboard
   (`https://supabase.com/dashboard/project/<ref>/database/extensions`)
2. Search for `pg_cron`
3. Toggle it **ON**
4. Do the same for `pg_trgm` and `vector` (pgvector) if they are not already enabled

> **When all three extensions are enabled, tell the agent and it will continue.**

---

## Step 4: Run database migrations

This creates all 7 tables, RLS policies, indexes, full-text search, the hybrid
search function, triggers, and the cron job for activity log cleanup.

```bash
cd supabase
supabase db push
```

Expected output: 8 migrations applied successfully.

If any migration fails, check the error message. Common issues:
- Extension not enabled → see Step 3
- Permission denied → ensure you used the correct database password

Verify the tables were created:

```bash
supabase db lint
```

---

## Step 5: Configure GitHub OAuth

### MANUAL ACTION REQUIRED

**Create a GitHub OAuth App:**

1. Go to [https://github.com/settings/developers](https://github.com/settings/developers)
2. Click **New OAuth App**
3. Fill in:
   - **Application name**: `FixOnce`
   - **Homepage URL**: `https://<YOUR_PROJECT_REF>.supabase.co`
   - **Authorization callback URL**: `https://<YOUR_PROJECT_REF>.supabase.co/auth/v1/callback`
4. Click **Register application**
5. On the next page, click **Generate a new client secret**
6. Copy both the **Client ID** and the **Client Secret**

**Enable GitHub provider in Supabase:**

1. Go to **Authentication → Providers** in your Supabase dashboard
   (`https://supabase.com/dashboard/project/<ref>/auth/providers`)
2. Find **GitHub** in the list and click to expand
3. Toggle it **ON**
4. Paste your **Client ID** and **Client Secret**
5. Click **Save**

> **When GitHub OAuth is configured, tell the agent and it will continue.**

---

## Step 6: Set edge function secrets

Edge functions need several environment variables. Set them using the Supabase
CLI:

```bash
# Required: your Supabase connection details
supabase secrets set SUPABASE_URL=https://<YOUR_PROJECT_REF>.supabase.co
supabase secrets set SUPABASE_ANON_KEY=<your-anon-key>
supabase secrets set SUPABASE_SERVICE_ROLE_KEY=<your-service-role-key>

# Required: encryption master key for server-side secret storage
# Generate a 32-byte base64 key:
#   openssl rand -base64 32
supabase secrets set ENCRYPTION_MASTER_KEY=<your-32-byte-base64-key>

# Required: JWT signing secret (used for challenge-response auth tokens)
# Use the JWT secret from your Supabase project settings:
#   Dashboard → Settings → API → JWT Settings → JWT Secret
supabase secrets set JWT_SECRET=<your-jwt-secret>

# Optional: GitHub org for membership checks
# Only needed if you want to restrict access to a specific GitHub org
supabase secrets set GITHUB_ORG=<your-github-org-name>
```

Verify secrets are set:

```bash
supabase secrets list
```

You should see all the variables listed (values are hidden).

---

## Step 7: Deploy edge functions

```bash
cd supabase
supabase functions deploy
```

This deploys all 16 edge functions:

| Function | Purpose |
|----------|---------|
| `memory-create` | Create a new memory |
| `memory-get` | Retrieve a memory by ID |
| `memory-update` | Update a memory |
| `memory-delete` | Soft-delete a memory |
| `memory-search` | Hybrid/FTS/vector search |
| `feedback-submit` | Submit feedback on a memory |
| `auth-nonce` | Generate auth challenge nonce |
| `auth-verify` | Verify Ed25519 signature, issue JWT |
| `auth-org-check` | Check GitHub org membership |
| `keys-register` | Register a CLI public key |
| `keys-list` | List registered keys |
| `keys-revoke` | Revoke a key |
| `secret-create` | Store an encrypted secret |
| `secret-get` | Retrieve and decrypt a secret |
| `secret-rotate-master` | Re-encrypt all secrets with a new master key |
| `activity-stream` | Query recent activity log entries |

Verify deployment:

```bash
supabase functions list
```

---

## Step 8: Store the VoyageAI API key

The VoyageAI API key is stored encrypted in the database (not as an env var)
using the `secret-create` edge function. This keeps it out of plain-text config.

```bash
# Call the secret-create edge function directly
curl -X POST \
  "https://<YOUR_PROJECT_REF>.supabase.co/functions/v1/secret-create" \
  -H "Authorization: Bearer <YOUR_SERVICE_ROLE_KEY>" \
  -H "Content-Type: application/json" \
  -d '{"name": "VOYAGEAI_API_KEY", "value": "<your-voyageai-api-key>"}'
```

You should get back `{"name":"VOYAGEAI_API_KEY","created_at":"..."}`.

> **Note:** The service role key is used here because secret creation is an
> admin operation. Normal users retrieve secrets via the authenticated
> `secret-get` endpoint.

---

## Step 9: Verify the backend

Run a quick smoke test to make sure everything is working:

```bash
# Test that the search function exists and responds
curl -X POST \
  "https://<YOUR_PROJECT_REF>.supabase.co/functions/v1/memory-search" \
  -H "Authorization: Bearer <YOUR_ANON_KEY>" \
  -H "Content-Type: application/json" \
  -d '{"query_text": "test", "search_type": "fts"}'
```

Expected: either a 200 with `{"results":[],"total":0,"search_type":"fts"}`
(empty database) or a 401 if auth is working correctly (anon key alone may not
pass the auth middleware depending on your RLS setup).

---

## Step 10: Configure the CLI

On each machine that will use FixOnce, set the API URL:

```bash
export FIXONCE_API_URL=https://<YOUR_PROJECT_REF>.supabase.co
```

Add this to your shell profile (`~/.bashrc`, `~/.zshrc`, etc.) for persistence.

Then authenticate:

```bash
fixonce login
```

---

## Environment variable reference

### Edge function secrets (set via `supabase secrets set`)

| Variable | Required | Description |
|----------|----------|-------------|
| `SUPABASE_URL` | yes | Your project's API URL (e.g. `https://abc123.supabase.co`) |
| `SUPABASE_ANON_KEY` | yes | The `anon` public API key from your project settings |
| `SUPABASE_SERVICE_ROLE_KEY` | yes | The `service_role` secret key — used by edge functions for admin operations (RLS bypass) |
| `ENCRYPTION_MASTER_KEY` | yes | 32-byte base64 string for AES-256-GCM encryption of stored secrets. Generate with `openssl rand -base64 32` |
| `JWT_SECRET` | yes | Used to sign challenge-response JWTs. Use the JWT secret from your Supabase project settings |
| `GITHUB_ORG` | no | GitHub organization name for membership checks. If set, only org members can authenticate |

### CLI environment variables (set in your shell)

| Variable | Required | Description |
|----------|----------|-------------|
| `FIXONCE_API_URL` | yes | Backend URL (same as `SUPABASE_URL` above) |

---

## Database schema

The migrations create these tables:

| Table | Purpose |
|-------|---------|
| `memory` | Core knowledge store — memories with embeddings, metadata, scoring |
| `feedback` | User ratings on memories (helpful/outdated/damaging) |
| `activity_log` | Audit trail of all system events (90-day retention) |
| `secrets` | Encrypted API keys and credentials (AES-256-GCM) |
| `cli_keys` | Registered Ed25519 public keys for CLI authentication |
| `memory_lineage` | Provenance chain tracking (replaces, merges, updates) |
| `contradiction_pairs` | Conflicting memory pairs with tiebreaker voting |

All tables have Row Level Security (RLS) enabled with deny-by-default policies.
Authenticated users can read memories and submit feedback. Only memory creators
can update or delete their own memories. The `secrets` and `activity_log` tables
are accessible only via the service role (used by edge functions internally).

---

## Maintenance

### Rotating the encryption master key

If you need to rotate the master key:

1. Set the new key as a secret:
   ```bash
   supabase secrets set ENCRYPTION_MASTER_KEY=<new-key>
   ```

2. Call the rotation endpoint to re-encrypt all secrets:
   ```bash
   curl -X POST \
     "https://<YOUR_PROJECT_REF>.supabase.co/functions/v1/secret-rotate-master" \
     -H "Authorization: Bearer <YOUR_SERVICE_ROLE_KEY>" \
     -H "Content-Type: application/json" \
     -d '{"new_master_key": "<new-key-base64>"}'
   ```

### Activity log cleanup

A `pg_cron` job runs daily at 03:00 UTC to delete activity log entries older
than 90 days. This is set up automatically by the migrations. No manual action
needed.

### Adding new migrations

```bash
cd supabase
supabase migration new <description>
# Edit the generated file in supabase/migrations/
supabase db push
```

### Redeploying edge functions

After changing edge function code:

```bash
cd supabase
supabase functions deploy
```

Or deploy a single function:

```bash
supabase functions deploy memory-search
```

---

## Troubleshooting

| Problem | Solution |
|---------|----------|
| `pg_cron` migration fails | Enable the extension manually in Dashboard → Database → Extensions |
| `pgvector` not found | Same — enable `vector` in Dashboard → Database → Extensions |
| Edge function returns 500 | Check `supabase functions logs <function-name>` for errors |
| Secrets not found by edge functions | Verify with `supabase secrets list` that all required vars are set |
| Auth returns 401 | Ensure GitHub OAuth is configured in Dashboard → Authentication → Providers |
| CLI can't connect | Check `FIXONCE_API_URL` is set correctly and the project is running |
| Migrations won't apply | Run `supabase db reset` to start fresh (destroys all data) |
