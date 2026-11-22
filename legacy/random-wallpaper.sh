#!/bin/bash
# 随机壁纸轮换：只换自然风景 / 繁华城市（用户明确要求）
# 防重复：
#   1) 候选按内容哈希去重，同一张图在候选池里只算一次
#   2) history 记的是内容哈希（不是路径），并额外记 Bing 图 ID
#      —— 同一张图可能同时存在旧版数字名缓存、新版 slug 名缓存、本地池三份，
#         路径不同但内容相同，按路径去重会漏，所以必须按内容认图
#   3) 候选全部展示过才清空 history 允许轮回
# 主题过滤（用户要求的硬规则：只上自然风景 / 繁华城市）：
#   - Bing 候选 slug 必须命中主题白名单（山/湖/海/城市/天际线…），否则进隔离区不应用
#   - wallhaven 混入数字艺术/幻想图风险高，已从自动轮换移除，只保留手动模式
# 用法：random-wallpaper [bing|random]

set -u
BG_DIR="$HOME/.cache/random-wallpapers"
QUARANTINE="$BG_DIR/quarantine"
POOL_DIR="$HOME/.config/omarchy/backgrounds/catppuccin"
HIST="$BG_DIR/history"
mkdir -p "$BG_DIR" "$QUARANTINE"
touch "$HIST"

# 主题白名单（对 Bing slug 做小写子串匹配）
is_scenic() {
  local s
  s=$(printf '%s' "$1" | tr '[:upper:]' '[:lower:]')
  case "$s" in
    *mountain*|*peak*|*ridge*|*alps*|*himalaya*|*volcano*|*glacier*|*iceberg*|*fjord*|*cliff*|*canyon*|*valley*|*dune*|*desert*|*hill*|*countryside*|*meadow*|*field*|*farm*|*forest*|*wood*|*jungle*|*tree*|*cypress*|*oak*|*blossom*|*flower*|*autumn*|*spring*|*summer*|*winter*|*snow*|*frost*|*lake*|*loch*|*river*|*creek*|*waterfall*|*pond*|*marsh*|*wetland*|*sea*|*ocean*|*coast*|*shore*|*beach*|*bay*|*cove*|*island*|*isle*|*harbor*|*harbour*|*lighthouse*|*wave*|*reef*|*sky*|*sunset*|*sunrise*|*dusk*|*dawn*|*cloud*|*storm*|*mist*|*fog*|*aurora*|*northernlights*|*star*|*milkyway*|*galaxy*|*eclipse*|*moon*|*night*|*city*|*skyline*|*skyscraper*|*downtown*|*metropolis*|*street*|*avenue*|*boulevard*|*bridge*|*tower*|*castle*|*palace*|*cathedral*|*church*|*temple*|*shrine*|*fortress*|*monastery*|*village*|*town*|*oldtown*|*harbor*|*marina*|*pier*|*walk*|*park*|*garden*|*canyon*|*gorge*|*stones*|*rocks*|*crane*|*heron*|*swan*|*flamingo*|*safari*|*savanna*|*steppe*|*tundra*|*oasis*|*waterhole*|*bayou*|*keys*|*coastline*|*seaside*|*waterfall*)
      return 0 ;;
    *alpine*|*badlands*|*prairie*|*trail*|*footpath*|*plateau*|*mesa*|*butte*|*geyser*|*geothermal*|*falls*|*cascade*|*brook*|*stream*|*lagoon*|*tarn*|*cape*|*peninsula*|*swamp*|*woodland*|*moor*|*pine*|*birch*|*maple*|*vineyard*|*wheat*|*lavender*|*olive*|*orchard*|*windmill*|*canal*|*plaza*|*square*|*viaduct*|*aqueduct*|*courtyard*|*pagoda*|*archipelago*|*atoll*|*karst*|*hoodoo*|*bison*|*elk*|*moose*|*deer*|*wolf*|*fox*|*eagle*|*owl*|*wildlife*|*panorama*)
      return 0 ;;
    *halloween*|*christmas*|*xmas*|*easter*|*valentine*|*thanksgiving*|*newyear*|*carnival*|*fest*|*party*|*library*|*museum*|*gallery*|*coffee*|*food*|*cake*|*dessert*|*laptop*|*workspace*|*desk*|*interior*|*studio*|*shop*|*market*|*car*|*bicycle*|*motorcycle*|*airplane*|*spacecraft*|*robot*|*abstract*|*pattern*|*typography*|*illustration*|*collage*|*sculpture*)
      return 1 ;;
    *) return 1 ;;  # 不认识的题材宁可不上墙（用户宁可少换不要乱换）
  esac
}


# 选一张没上过的壁纸，同时把它的内容哈希与 Bing 图 ID 记进 history。
# 具体逻辑在 pick.py，候选从 stdin 读（每行一个路径）。
PICK_PY="$HOME/.local/share/random-wallpaper/pick.py"
pick_unseen() {
  python3 "$PICK_PY" "$HIST"
}

# 清掉旧版数字名缓存里和 slug 名重复的那些（内容一样，留一份就够）
prune_legacy_dupes() {
  local keep f md5h dup
  keep=$(md5sum "$BG_DIR"/bing-th*.jpg 2>/dev/null | awk '{print $1}')
  [ -n "$keep" ] || return 0
  for f in "$BG_DIR"/bing-[0-9]*.jpg; do
    [ -f "$f" ] || continue
    md5h=$(md5sum "$f" | cut -d' ' -f1)
    dup=$(printf '%s\n' "$keep" | grep -c "^$md5h$")
    [ "$dup" -gt 0 ] && rm -f "$f"
  done
}

# 昼夜亮度偏好：白天只挑明亮的壁纸，晚上不限（缓存避免重复计算）
BR_CACHE="$BG_DIR/brightness.cache"
touch "$BR_CACHE"

is_daytime() {
  local h
  h=$(date +%H)
  [ "$h" -ge 7 ] && [ "$h" -lt 19 ]
}

brightness_of() {
  local f="$1" cached
  cached=$(grep -F "$f" "$BR_CACHE" 2>/dev/null | tail -1 | cut -d'|' -f2)
  if [ -n "$cached" ]; then printf '%s\n' "$cached"; return; fi
  local b
  b=$(magick "$f" -resize 64x64 -colorspace Gray -format "%[fx:mean]" info: 2>/dev/null)
  [ -n "$b" ] && printf '%s|%s\n' "$f" "$b" >> "$BR_CACHE"
  printf '%s\n' "${b:-0.5}"
}

filter_by_brightness() {
  # 白天只保留亮度 > 0.45 的图；无合格候选则回退全部
  if ! is_daytime; then cat; return; fi
  local all="" out="" f b
  while IFS= read -r f; do
    all="$all$f
"
    b=$(brightness_of "$f")
    awk -v b="$b" 'BEGIN{exit !(b > 0.45)}' && out="$out$f
"
  done
  if [ -n "$out" ]; then printf '%s' "$out"; else printf '%s' "$all"; fi
}

fetch_bing() {
  # Bing 的 HPImageArchive 接口无视 mkt，所有"市场"返回的是同一批图，
  # 真正能扩源的是 idx：idx 0..7 是滑动窗口，8 个窗口的并集约 15 张不重复的图。
  # 主题过滤：slug 命中白名单才进候选，否则隔离（缓存但永不应用）
  local idx url slug
  for idx in 0 1 2 3 4 5 6 7; do
    curl -s --max-time 15 "https://www.bing.com/HPImageArchive.aspx?format=js&idx=${idx}&n=8" \
      | python3 -c "
import json, sys, re
try:
    for im in json.load(sys.stdin)['images']:
        slug = re.sub(r'[^a-zA-Z0-9_-]', '', im['urlbase'].rsplit('/', 1)[-1])
        print('https://www.bing.com' + im['urlbase'] + '_UHD.jpg', slug)
except Exception:
    pass"
  done | sort -u -k2 | while read -r url slug; do
        [ -n "${url:-}" ] || continue
        if ! is_scenic "$slug"; then
          local q="$QUARANTINE/bing-${slug}.jpg"
          [ -s "$q" ] || curl -sL --max-time 60 -o "$q" "$url" 2>/dev/null
          continue
        fi
        local out="$BG_DIR/bing-${slug}.jpg"
        [ -s "$out" ] || curl -sL --max-time 60 -o "$out" "$url"
        [ -s "$out" ] && printf '%s\n' "$out"
      done
}

UA="OmarchyWallpaper/1.0 (local desktop wallpaper rotator)"

fetch_commons() {
  # Wikimedia Commons 的精选分类：自然风景 + 城市天际线，指定 2560 宽直链
  # 结果累积进 commons.all（url<TAB>标题），分类成员会变，累积起来候选才够多
  local list="$BG_DIR/commons.list" all="$BG_DIR/commons.all"
  if [ -f "$list" ] && [ "$(date -r "$list" +%Y%m%d)" = "$(date +%Y%m%d)" ]; then
    return
  fi
  local cat
  for cat in "Featured pictures of nature" "Featured pictures of landscapes" \
             "Featured pictures of skylines" "Featured pictures of cityscapes"; do
    curl -sG --max-time 25 -A "$UA" \
      --data-urlencode "action=query" \
      --data-urlencode "generator=categorymembers" \
      --data-urlencode "gcmtitle=Category:$cat" \
      --data-urlencode "gcmtype=file" \
      --data-urlencode "gcmlimit=150" \
      --data-urlencode "prop=imageinfo" \
      --data-urlencode "iiprop=url|size|mime" \
      --data-urlencode "iiurlwidth=2560" \
      --data-urlencode "format=json" \
      "https://commons.wikimedia.org/w/api.php" \
    | python3 -c "
import json, sys
try:
    for p in json.load(sys.stdin).get('query', {}).get('pages', {}).values():
        ii = (p.get('imageinfo') or [{}])[0]
        title = p.get('title', '')[5:]
        mime = ii.get('mime', '')
        w, h = ii.get('width') or 0, ii.get('height') or 0
        url = ii.get('thumburl') or ''
        if mime not in ('image/jpeg', 'image/png') or not url:
            continue
        if w < 2000 or h < 1000:
            continue
        ar = w / h
        if ar < 1.2 or ar > 2.8:
            continue
        print(url + chr(9) + title)
except Exception:
    pass"
  done > "$list"
  touch "$all"
  cat "$list" "$all" | sort -u -t$'\t' -k1,1 > "$all.new"
  mv "$all.new" "$all"
}

commons_allowed() {
  # 按标题过滤不合规的（人文建筑室内、数字艺术、人物特写等），规则随时可改
  python3 -c "
import sys
BAD = ('portrait ', 'portrait of', 'person', 'statue', 'painting', 'engraving',
       'poster', 'cover of', 'book', 'map of', 'flag of', 'coat of arms',
       'logo', 'stamp', 'coin', 'medal', 'interior', 'graffiti', 'drawing',
       'self-portrait', 'nude', 'doll',
       'cathedral', 'church', 'mosque', 'chapel', 'abbey', 'basilica',
       'altar', 'organ', 'stained glass', 'cemetery', 'tomb', 'ruins',
       'fountain', 'town hall', 'museum', 'library', 'university',
       'monastery', 'palace', 'castle',
       'monster', 'alien', 'ghost', 'myth', 'legend', 'dragon', 'angel',
       'artwork', 'render', 'concept art')
for line in sys.stdin:
    line = line.rstrip('\n')
    if not line:
        continue
    url, _, title = line.partition('\t')
    low = title.lower()
    if any(b in low for b in BAD):
        continue
    print(line)
" < "$BG_DIR/commons.all"
}

download_commons() {
  # 下载还没下过的 Commons 图，单次最多 8 张，避免跑太久
  # 文件名带上标题，方便在通知里认出来
  local limit=8 url title slug name ext expected
  expected="$BG_DIR/commons.expected"
  : > "$expected"

  while IFS=$'\t' read -r url title; do
    [ -n "${url:-}" ] || continue
    ext=".jpg"
    case "$url" in *.png) ext=".png" ;; esac
    slug=$(printf '%s' "${title:-$url}" | tr -cs 'A-Za-z0-9' '_' | cut -c1-28)
    name="$BG_DIR/commons-${slug}-$(printf '%s' "$url" | sha1sum | cut -c1-8)$ext"
    printf '%s\n' "$name" >> "$expected"
    [ -s "$name" ] && continue
    [ "$limit" -gt 0 ] || continue
    curl -sL --max-time 40 -A "$UA" "$url" -o "$name.part" 2>/dev/null || { rm -f "$name.part"; continue; }
    if [ -s "$name.part" ] && magick identify "$name.part" >/dev/null 2>&1; then
      mv "$name.part" "$name"
      limit=$((limit - 1))
    else
      rm -f "$name.part"
    fi
  done < <(commons_allowed)

  # 不在合规清单里的缓存（规则改过）直接清掉
  local f
  for f in "$BG_DIR"/commons-*.jpg "$BG_DIR"/commons-*.png; do
    [ -f "$f" ] || continue
    grep -qxF "$f" "$expected" 2>/dev/null || rm -f "$f"
  done
  rm -f "$expected"
}

case "${1:-random}" in
  bing) cands=$(fetch_bing) ;;
  *)    cands=$(fetch_bing) ;;
esac

# 本地图池永远参与（早期手工预筛过的，信任）
cands="$cands
$(find "$POOL_DIR" -maxdepth 1 -type f \( -iname '*.jpg' -o -iname '*.png' \) 2>/dev/null)"

# Commons 精选风景/天际线：先拉清单再下载，下载完的本地文件算候选
fetch_commons
download_commons
cands="$cands
$(ls "$BG_DIR"/commons-* 2>/dev/null | grep -v '\.part$')"

mapfile -t cands_arr < <(printf '%s\n' "$cands" | sed '/^$/d' | sort -u | filter_by_brightness)
img=$(printf '%s\n' "${cands_arr[@]}" | pick_unseen)

if [ -n "$img" ] && [ -s "$img" ]; then
  find "$BG_DIR" -maxdepth 1 -type f -name 'bing-*.jpg' -printf '%T@ %p\n' \
    | sort -rn | tail -n +61 | cut -d' ' -f2- | xargs -r rm -f
  find "$BG_DIR" -maxdepth 1 -type f -name 'commons-*' -printf '%T@ %p\n' \
    | sort -rn | tail -n +41 | cut -d' ' -f2- | xargs -r rm -f
  prune_legacy_dupes
  timeout 30 omarchy theme bg set "$img" >>"$BG_DIR/apply.log" 2>&1
  notify-send -u low "壁纸轮换" "$(basename "$img")" 2>/dev/null || true
fi
