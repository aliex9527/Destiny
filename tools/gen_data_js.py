import json, io

draws = json.load(open('data/dlt.json', encoding='utf-8'))
parts = []
for d in draws:
    parts.append('{i},{f},{b},{d}'.format(
        i=d['i'], f=','.join(map(str, d['f'])), b=','.join(map(str, d['b'])), d=d['d']))
raw = ';'.join(parts)
js = '/* 大乐透历史开奖数据（自动生成，勿手改）\n * 共 %d 期，%s ~ %s\n * 格式: 期号,前区5码,后区2码,开奖日期\n */\nwindow.DLT_RAW="%s";\n' % (
    len(draws), draws[0]['d'], draws[-1]['d'], raw)
open('app/src/main/assets/data.js', 'w', encoding='utf-8').write(js)
print('draws:', len(draws), 'bytes:', len(js.encode('utf-8')))
print('range:', draws[0]['i'], draws[0]['d'], '->', draws[-1]['i'], draws[-1]['d'])
