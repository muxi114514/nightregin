"""在 dump_text.ps1 导出的 text.json 里搜文本（只读）。
用法：python -I text_search.py <text.json> [-l zhocn|engus] [-f FMG子串] [-n 每条截断长度] [-i id] 关键词...
关键词之间是「或」；-i 按 ID 精确查（可多次）。
"""
import argparse
import json
import sys

ap = argparse.ArgumentParser()
ap.add_argument('path')
ap.add_argument('words', nargs='*')
ap.add_argument('-l', default='zhocn')
ap.add_argument('-f', default='')
ap.add_argument('-n', type=int, default=80)
ap.add_argument('-i', action='append', default=[])
ap.add_argument('-m', type=int, default=200, help='最多输出条数')
a = ap.parse_intermixed_args()
sys.stdout.reconfigure(encoding='utf-8')
data = json.load(open(a.path, encoding='utf-8'))
src = data['src']
count = 0
for key, entries in data[a.l].items():
    if a.f and a.f.lower() not in key.lower():
        continue
    for id_, text in entries.items():
        hit = (id_ in a.i) or any(w in text for w in a.words)
        if not hit:
            continue
        s = src.get(key, {}).get(id_, 0) if a.l == 'zhocn' else '-'
        print(f'{key}\t{id_}\t[{s}]\t' + text.replace('\n', '\\n')[:a.n])
        count += 1
        if count >= a.m:
            sys.exit(0)
