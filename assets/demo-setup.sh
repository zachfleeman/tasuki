#!/usr/bin/env bash
# Seed demo data for VHS recordings.
# Creates local tasks + a fake Obsidian vault with tasks.
# Dates are relative to today so recordings always look current.

set -euo pipefail

today=$(date +%Y-%m-%d)
yesterday=$(date -d "-1 day" +%Y-%m-%d)
three_days_ago=$(date -d "-3 days" +%Y-%m-%d)
tomorrow=$(date -d "+1 day" +%Y-%m-%d)
in_3_days=$(date -d "+3 days" +%Y-%m-%d)
in_5_days=$(date -d "+5 days" +%Y-%m-%d)

VAULT=~/.tasuki/demo-vault

mkdir -p ~/.tasuki
mkdir -p ~/.config/tasuki
mkdir -p "$VAULT/.obsidian"
mkdir -p "$VAULT/Projects"
mkdir -p "$VAULT/Daily"

# Config: both backends enabled
cat > ~/.config/tasuki/config.toml << CONF
[general]
theme = "dark"

[backends.local]
enabled = true

[backends.obsidian]
enabled = true
vault_path = "$VAULT"
inbox_file = "Inbox.md"
CONF

# Local file tasks
cat > ~/.tasuki/todo.txt << EOF
(p1) Fix login redirect bug due:${three_days_ago} #backend
(p2) Update API rate limits due:${yesterday} #backend
(p1) Deploy hotfix to production due:${today} #devops
(p3) Update dependencies due:${today}
(p2) Write changelog for v2.1 due:${tomorrow} #docs
(p3) Refactor auth middleware due:${in_3_days} #backend
Clean up old feature branches
x ${today} Archive Q4 metrics dashboard #analytics
EOF

# Obsidian vault tasks
cat > "$VAULT/Projects/Launch.md" << EOF
# Product Launch

- [ ] Review design mockups #frontend 📅 ${today}
- [ ] Prepare demo for standup 📅 ${tomorrow}
- [ ] Set up staging environment #devops 📅 ${in_5_days}
- [x] Draft launch email ✅ ${yesterday}
EOF

cat > "$VAULT/Daily/Notes.md" << EOF
# Notes

- [ ] Read through RFC proposal #docs
- [ ] Schedule 1:1 with new hire #team 📅 ${in_3_days}
EOF

echo "Demo data written (local + obsidian vault)"
