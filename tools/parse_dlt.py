import re, json

h = open('data/raw_dlt.html', encoding='utf-8', errors='ignore').read()
# 去掉 HTML 注释，避免注释里的 <td>2</td> 被当成数据
h = re.sub(r'<!--.*?-->', '', h, flags=re.S)
body = h[h.find('<tbody id="tdata"'):]
rows = re.findall(r'<tr[^>]*>(.*?)</tr>', body, re.S)
out = []
for r in rows:
    tds = re.findall(r'<td[^>]*>(.*?)</td>', r, re.S)
    if len(tds) < 9:
        continue
    issue = re.sub(r'<[^>]+>', '', tds[0]).strip()
    if not re.fullmatch(r'\d{5}', issue):
        continue
    nums = [int(re.sub(r'<[^>]+>', '', tds[i]).strip()) for i in range(1, 8)]
    date = re.sub(r'<[^>]+>', '', tds[-1]).strip()
    out.append({'i': issue, 'f': sorted(nums[:5]), 'b': sorted(nums[5:7]), 'd': date})

out.sort(key=lambda x: int(x['i']))
print('rows:', len(out))
print('first:', out[0])
print('last:', out[-1])
json.dump(out, open('data/dlt.json', 'w'), ensure_ascii=False)
