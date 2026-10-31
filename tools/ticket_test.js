/* 验票引擎手算校验：每个用例的期望值都由规则表人工推导，不是"跑出来是多少就写多少" */
const fs = require('fs');
const path = require('path');

const code = fs.readFileSync(path.join(__dirname, '..', 'app', 'src', 'main', 'assets', 'ticket.js'), 'utf8');
const sandbox = { window: {} };
new Function('window', code)(sandbox.window);
const T = sandbox.window.DLT_TICKET;

let fails = 0;
function eq(got, want, label) {
  const okv = JSON.stringify(got) === JSON.stringify(want);
  if (okv) console.log('  PASS  ' + label);
  else { console.log('  FAIL  ' + label + '  got=' + JSON.stringify(got) + ' want=' + JSON.stringify(want)); fails++; }
}

// 26123 期真实开奖：前区 13 18 19 20 28，后区 09 11
const DRAW = { i: '26123', f: [13, 18, 19, 20, 28], b: [9, 11] };
// 26014 之前是旧规，用 25001 期开奖构造同样的号码验证旧规则
const OLD_DRAW = { i: '25001', f: [13, 18, 19, 20, 28], b: [9, 11] };

console.log('\n[1] 规则切换');
eq(T.isNewRule('26123'), true, '26123 走新规');
eq(T.isNewRule('26014'), true, '26014 是新规首期');
eq(T.isNewRule('26013'), false, '26013 仍走旧规');
eq(T.rulesOf('26123').length, 7, '新规 7 个奖级');
eq(T.rulesOf('26013').length, 9, '旧规 9 个奖级');

console.log('\n[2] 奖级判定（新规）');
// 开奖 13/18/19/20/28 + 09/11。以下用例的命中数由号码人工核对得出。
// 1,2,33,34,35 不在开奖号内，用来凑"未命中"的位数。
const caseNew = [
  [[13, 18, 19, 20, 28], [9, 11], 1, '5+2 一等奖'],
  [[13, 18, 19, 20, 28], [9, 5], 2, '5+1 二等奖'],
  [[13, 18, 19, 20, 28], [1, 2], 3, '5+0 三等奖'],
  [[13, 18, 19, 20, 33], [9, 11], 3, '4+2 三等奖'],
  [[13, 18, 19, 20, 33], [9, 5], 4, '4+1 四等奖'],
  [[13, 18, 19, 33, 34], [9, 11], 5, '3+2 五等奖'],
  [[13, 18, 19, 20, 33], [1, 2], 5, '4+0 五等奖'],
  [[13, 18, 19, 33, 34], [9, 5], 6, '3+1 六等奖'],
  [[13, 18, 33, 34, 35], [9, 11], 6, '2+2 六等奖'],
  [[13, 18, 19, 33, 34], [1, 2], 7, '3+0 七等奖（新规新增条件）'],
  [[13, 18, 33, 34, 35], [9, 5], 7, '2+1 七等奖'],
  [[13, 34, 35, 1, 2], [9, 11], 7, '1+2 七等奖'],
  [[33, 34, 35, 1, 2], [9, 11], 7, '0+2 七等奖'],
  [[13, 18, 33, 34, 35], [1, 2], 0, '2+0 未中奖'],
  [[13, 34, 35, 1, 2], [9, 5], 0, '1+1 未中奖'],
  [[33, 34, 35, 1, 2], [9, 5], 0, '0+1 未中奖'],
  [[13, 34, 35, 1, 2], [1, 2], 0, '1+0 未中奖']
];
caseNew.forEach(([f, b, want, label]) => {
  const hf = f.filter(v => DRAW.f.indexOf(v) >= 0).length;
  const hb = b.filter(v => DRAW.b.indexOf(v) >= 0).length;
  eq(T.levelOf('26123', hf, hb), want, label + ' (' + hf + '+' + hb + ')');
});

console.log('\n[3] 奖级判定（旧规对照）');
eq(T.levelOf('26013', 5, 0), 3, '旧规 5+0 三等奖');
eq(T.levelOf('26013', 4, 2), 4, '旧规 4+2 四等奖');
eq(T.levelOf('26013', 4, 1), 5, '旧规 4+1 五等奖');
eq(T.levelOf('26013', 3, 2), 6, '旧规 3+2 六等奖');
eq(T.levelOf('26013', 4, 0), 7, '旧规 4+0 七等奖');
eq(T.levelOf('26013', 3, 1), 8, '旧规 3+1 八等奖');
eq(T.levelOf('26013', 3, 0), 9, '旧规 3+0 九等奖');

console.log('\n[4] 单注奖金（新规基础档）');
eq(T.unitPrize('26123', 3, null).amount, 5000, '三等奖 5000');
eq(T.unitPrize('26123', 4, null).amount, 300, '四等奖 300');
eq(T.unitPrize('26123', 5, null).amount, 150, '五等奖 150');
eq(T.unitPrize('26123', 6, null).amount, 15, '六等奖 15');
eq(T.unitPrize('26123', 7, null).amount, 5, '七等奖 5');
eq(T.unitPrize('26123', 1, null).live, true, '一等奖为浮动奖');
eq(T.unitPrize('25001', 3, null).amount, 10000, '旧规三等奖 10000');
eq(T.unitPrize('25001', 4, null).amount, 3000, '旧规四等奖 3000');
eq(T.unitPrize('25001', 7, null).amount, 100, '旧规七等奖 100');

console.log('\n[5] 注数计算');
eq(T.comb(35, 5), 324632, 'C(35,5)');
eq(T.comb(12, 2), 66, 'C(12,2)');
eq(T.betCount([3, 11, 22, 28, 33], [5, 9]), 1, '单式 5+2 = 1 注');
eq(T.betCount([3, 11, 22, 28, 33, 35], [5, 9]), 6, '6+2 = 6 注');
eq(T.betCount([3, 11, 22, 28, 33, 35, 1], [5, 9, 12]), 21 * 3, '7+3 = 63 注');
eq(T.betCount([3, 11, 22, 28], [5, 9]), 0, '前区不足 5 个 → 0');

console.log('\n[6] 校验拦截');
eq(T.isValid([3, 11, 22, 28, 33], [5, 9]), true, '合法单式');
eq(T.isValid([3, 11, 22, 28, 33], [5, 9, 11]), true, '后区复式合法');
eq(T.isValid([3, 11, 22, 28, 36], [5, 9]), false, '前区越界');
eq(T.isValid([3, 3, 22, 28, 33], [5, 9]), false, '前区重复');
eq(T.isValid([3, 11, 22, 28, 33], [5, 9, 5]), false, '后区重复');
eq(T.isValid([3, 11, 22, 28, 33], [5]), false, '后区不足');

console.log('\n[7] 整票核对（单式）');
let r = T.check({ issue: '26123', front: [13, 18, 19, 20, 28], back: [9, 11] }, DRAW, null);
eq(r.ok, true, '核对执行');
eq([r.bestHF, r.bestHB, r.bestLevel], [5, 2, 1], '命中 5+2 → 一等奖');
eq(r.bets, 1, '注数 1');
eq(r.live, true, '一等奖金额未知，标记浮动');
eq(r.frontHit.length, 5, '前区 5 个全中');
eq(r.backHit.length, 2, '后区 2 个全中');

r = T.check({ issue: '26123', front: [13, 18, 19, 20, 33], back: [9, 5] }, DRAW, null);
eq([r.bestLevel, r.total], [4, 300], '4+1 → 四等奖 300 元');

r = T.check({ issue: '26123', front: [13, 18, 19, 20, 33], back: [9, 11] }, DRAW, null);
eq([r.bestLevel, r.total], [3, 5000], '4+2 → 三等奖 5000 元');

r = T.check({ issue: '26123', front: [3, 11, 22, 28, 33], back: [5, 9] }, DRAW, null);
eq([r.bestLevel, r.total], [0, 0], '未中奖');

console.log('\n[8] 追加投注');
r = T.check({ issue: '26123', front: [13, 18, 19, 20, 33], back: [9, 5], extra: true }, DRAW, null);
eq(r.total, 300, '四等奖追加不加成（只有浮动奖才有）');
// 浮动奖带官方金额：26123 一等 5,750,462 / 追加 4,600,369
const TABLE = { b: [5750462, 47994, 5000, 300, 150, 15, 5], x: [4600369, 38395] };
r = T.check({ issue: '26123', front: [13, 18, 19, 20, 28], back: [9, 11], extra: true }, DRAW, TABLE);
eq(r.total, 5750462 + 4600369, '一等奖追加 = 基本 + 官方追加金额');
eq(r.live, false, '有官方金额时不再是浮动');
r = T.check({ issue: '26123', front: [13, 18, 19, 20, 28], back: [9, 11], extra: false }, DRAW, TABLE);
eq(r.total, 5750462, '不追加只拿基本奖金');
r = T.check({ issue: '26123', front: [13, 18, 19, 20, 28], back: [9, 5], extra: true }, DRAW, TABLE);
eq(r.total, 47994 + 38395, '二等奖追加金额取自官方追加项');

console.log('\n[9] 复式票（6+2 命中 5+2 时应同时中一等和若干二等）');
// 前区 6 个：13 18 19 20 28 + 33，后区 9 11。展开 6 注
// 其中 1 注 5+2（一等奖），其余 5 注为 4+1？不 —— 每注前区都是 5 个，含 33 的那 5 注各中 4 个前区
// 所以：1 注 5+2 一等 + 5 注 4+2 三等
r = T.check({ issue: '26123', front: [13, 18, 19, 20, 28, 33], back: [9, 11] }, DRAW, TABLE);
eq(r.bets, 6, '6+2 共 6 注');
const lv3 = r.levels.find(x => x.lv === 3);
const lv1 = r.levels.find(x => x.lv === 1);
eq(!!lv1 && lv1.count, 1, '1 注一等奖');
eq(!!lv3 && lv3.count, 5, '5 注三等奖');
eq(r.total, 5750462 + 5000 * 5, '合计 = 一等 + 5×三等奖');

console.log('\n[10] 边界');
r = T.check({ issue: '26124', front: [13, 18, 19, 20, 28], back: [9, 11] }, DRAW, null);
eq(r.ok, false, '期号不匹配不核对');
r = T.check({ issue: '26123', front: [13, 18, 19], back: [9, 11] }, DRAW, null);
eq(r.ok, false, '号码不合法不核对');
eq(T.check({ issue: '26123', front: [13, 18, 19, 20, 28], back: [9, 11] }, null, null).ok, false, '没有开奖数据不核对');

console.log('\n' + (fails === 0 ? '验票引擎全部通过 ✓' : fails + ' 项失败 ✗'));
process.exit(fails === 0 ? 0 : 1);
