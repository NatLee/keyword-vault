#!/usr/bin/env bash
# 對一個剛啟動、資料庫是空的 Keyword Vault 跑一輪：建立管理員、登入、寫關鍵字、組合、匯出。
# 用法：bash docs/smoke-test.sh http://127.0.0.1:8686
set -euo pipefail
base=${1:-http://127.0.0.1:8686}
jar=$(mktemp)
# 每次跑都用新的隨機密碼，不放任何固定密碼在 repo 裡
pw=$(openssl rand -hex 16)
json=(-H 'content-type: application/json')

for _ in $(seq 1 30); do
  curl -fsS "$base/api/health" >/dev/null 2>&1 && break
  sleep 1
done
curl -fsS "$base/api/meta" | grep -q '"default_slop":20'
curl -fsS "$base/" | grep -q '<div id="root">'
curl -fsS "$base/keywords/1" | grep -q '<div id="root">'

# 還沒有帳號：要先建立管理員；沒登入的 API 回 401
curl -fsS "$base/api/auth/state" | grep -q '"setup_required":true'
test "$(curl -s -o /dev/null -w '%{http_code}' "$base/api/keywords")" = 401
curl -fsS -c "$jar" -D "$jar.headers" "${json[@]}" \
  -d "{\"username\":\"admin\",\"password\":\"$pw\"}" "$base/api/auth/setup" >/dev/null
grep -qi 'set-cookie: kv_session=.*HttpOnly' "$jar.headers"
# 只能建立一次
code=$(curl -s -o /dev/null -w '%{http_code}' "${json[@]}" -d "{\"username\":\"other\",\"password\":\"$pw\"}" "$base/api/auth/setup")
test "$code" != 200

# 語法檢查、兩個關鍵字、一個組合關鍵字
curl -fsS -b "$jar" "${json[@]}" -d '{"expression":"a&&b"}' "$base/api/tools/check" | grep -q '"valid":false'
curl -fsS -b "$jar" "${json[@]}" -d '{"theme":"奶茶","expression":"珍珠奶茶|(珍珠 鮮奶 ~)","visibility":"public","tags":["飲料"]}' \
  "$base/api/keywords" | grep -q '"id":1'
curl -fsS -b "$jar" "${json[@]}" -d '{"theme":"廣告","expression":"業配|抽獎"}' "$base/api/keywords" | grep -q '"id":2'
curl -fsS -b "$jar" "${json[@]}" -d '{"theme":"奶茶（去廣告）","logic":"#1!#2"}' "$base/api/keywords" >/dev/null
curl -fsS -b "$jar" "$base/api/keywords/3" | grep -q 'NOT (\\"業配\\" OR \\"抽獎\\")'
test "$(curl -fsS -b "$jar" "$base/api/keywords/export" | wc -l)" -ge 4

# 別的網站送來的寫入被擋下
code=$(curl -s -o /dev/null -w '%{http_code}' -b "$jar" -H 'Origin: https://evil.example' "${json[@]}" -d '{"theme":"x","expression":"x"}' "$base/api/keywords")
test "$code" = 403

# 登出後 session 失效
curl -fsS -b "$jar" -X POST "${json[@]}" -d '{}' "$base/api/auth/logout" >/dev/null
test "$(curl -s -o /dev/null -w '%{http_code}' -b "$jar" "$base/api/keywords")" = 401
rm -f "$jar" "$jar.headers"
echo "smoke test ok"
