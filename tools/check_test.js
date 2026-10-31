/* 验票页 DOM 测试：真实执行 app.js，走完整交互路径 */
const fs = require('fs');
const path = require('path');
const { JSDOM } = require('jsdom');

const DIR = __dirname;
const html = fs.readFileSync(path.join(DIR, 'index.html'), 'utf8');

let fails = 0;
function ok(cond, label, extra) {
  if (cond) console.log('  PASS  ' + label + (extra ? '  ' + extra : ''));
  else { console.log('  FAIL  ' + label + (extra ? '  ' + extra : '')); fails++; }
}
const tick = ms => new Promise(r => setTimeout(r, ms));

const errors = [];
const dom = new JSDOM(html, {
  runScripts: 'dangerously',
  pretendToBeVisual: true,
  url: 'file:///C:/temp/dlttest/index.html',
  beforeParse(win) {
    win.addEventListener('error', e => errors.push(String(e.message || e)));
    win.HTMLCanvasElement.prototype.getContext = function () { return null; };
  }
});

['data.js', 'ticket.js', 'engine.js', 'app.js'].forEach(f => {
  const s = dom.window.document.createElement('script');
  s.textContent = fs.readFileSync(path.join(DIR, f), 'utf8');
  dom.window.document.body.appendChild(s);
});

const win = dom.window, doc = win.document;
const $ = id => doc.getElementById(id);
const click = el => el.dispatchEvent(new win.MouseEvent('click', { bubbles: true }));
const ball = (zone, n) => (zone === 'f' ? $('ckFront') : $('ckBack')).querySelector('[data-n="' + n + '"]');
const pick = (zone, nums) => nums.forEach(n => click(ball(zone, n)));

(async () => {
  await tick(400);

  console.log('\n[1] 验票页入口');
  const checkTab = [...$('tabbar').querySelectorAll('button')].find(b => b.dataset.tab === 'check');
  ok(!!checkTab, '底部有验票页签');
  ok($('tabbar').querySelectorAll('button').length === 4, '共 4 个页签',
     $('tabbar').querySelectorAll('button').length + ' 个');
  click(checkTab);
  await tick(200);
  ok($('v-check').classList.contains('on'), '验票页显示');
  ok($('ckIssue').textContent === '26123 期', '默认核对最新期', $('ckIssue').textContent);
  ok($('ckDraw').querySelectorAll('.ball').length === 7, '显示 7 个开奖号码');
  ok($('ckFront').querySelectorAll('.pk-ball').length === 35, '前区 35 个号位');
  ok($('ckBack').querySelectorAll('.pk-ball').length === 12, '后区 12 个号位');

  console.log('\n[2] 点选录入');
  pick('f', [13, 18, 19, 20, 28]);
  await tick(80);
  ok($('ckFront').querySelectorAll('.pk-ball.on').length === 5, '前区选中 5 个');
  ok(/已选 5 \/ 5/.test($('ckFHd').textContent), '前区计数正确', $('ckFHd').textContent);
  pick('b', [9, 11]);
  await tick(80);
  ok($('ckBack').querySelectorAll('.pk-ball.on').length === 2, '后区选中 2 个');
  ok(/共 <b>1<\/b> 注/.test($('ckFoot').innerHTML), '单式 1 注', $('ckFoot').textContent);
  ok(/票面金额 <b>2<\/b> 元/.test($('ckFoot').innerHTML), '未追加金额 2 元');

  console.log('\n[3] 超额拦截');
  click(ball('f', 33));
  await tick(80);
  ok($('ckFront').querySelectorAll('.pk-ball.on').length === 5, '单式模式不能选第 6 个');
  ok(/只能选 5 个/.test($('toast').textContent), '给出提示', $('toast').textContent);

  console.log('\n[4] 一等奖（浮动奖）');
  click($('btnCheck'));
  await tick(200);
  ok(/ck-res win/.test($('ckResult').querySelector('.ck-res').className), '结果卡为中奖样式');
  ok(/一等奖/.test($('ckResult').textContent), '标出一等奖');
  ok(/浮动奖/.test($('ckResult').textContent), '浮动奖如实显示，不编造金额');
  ok($('ckResult').querySelectorAll('.ck-hit .hr').length === 2, '有开奖/我的两行对比');

  console.log('\n[5] 固定奖金额');
  click($('btnClear'));
  pick('f', [13, 18, 19, 20, 33]);
  pick('b', [9, 5]);
  await tick(80);
  click($('btnCheck'));
  await tick(200);
  ok(/四等奖/.test($('ckResult').textContent), '4+1 → 四等奖', $('ckResult').textContent.slice(0, 40));
  ok(/300 元/.test($('ckResult').textContent), '奖金 300 元');
  ok(/合计奖金/.test($('ckResult').textContent), '有合计行');

  console.log('\n[6] 未中奖');
  click($('btnClear'));
  pick('f', [3, 11, 22, 28, 33]);
  pick('b', [5, 9]);
  await tick(80);
  click($('btnCheck'));
  await tick(200);
  ok(/ck-res lose/.test($('ckResult').querySelector('.ck-res').className), '未中奖样式');
  ok(/本期未中奖/.test($('ckResult').textContent), '文案正确');
  ok(/1\+1/.test($('ckResult').textContent), '显示命中数 1+1', $('ckResult').textContent.slice(0, 60));

  console.log('\n[7] 复式');
  click($('ckTypeSeg').querySelector('[data-t="multi"]'));
  await tick(100);
  ok($('ckTypeSeg').querySelector('[data-t="multi"]').classList.contains('on'), '切到复式');
  click($('btnClear'));
  pick('f', [13, 18, 19, 20, 28, 33]);
  pick('b', [9, 11]);
  await tick(100);
  ok(/共 <b>6<\/b> 注/.test($('ckFoot').innerHTML), '6+2 = 6 注');
  ok(/票面金额 <b>12<\/b> 元/.test($('ckFoot').innerHTML), '金额 12 元');
  click($('btnCheck'));
  await tick(200);
  ok(/一等奖/.test($('ckResult').textContent) && /三等奖/.test($('ckResult').textContent),
     '同时中一等奖和三等奖');
  ok(/5 注/.test($('ckResult').textContent), '三等奖中 5 注');

  console.log('\n[8] 追加投注');
  click($('ckExtraSeg').querySelector('[data-e="1"]'));
  await tick(100);
  ok($('ckExtraSeg').querySelector('[data-e="1"]').classList.contains('on'), '追加已开启');
  ok(/票面金额 <b>18<\/b> 元/.test($('ckFoot').innerHTML), '追加后每注 3 元 → 18 元',
     $('ckFoot').textContent);

  console.log('\n[9] 保存与我的彩票');
  click($('ckResult').querySelector('#btnSaveTicket'));
  await tick(200);
  ok($('ckSaved').querySelectorAll('.tk').length === 1, '我的彩票出现 1 张');
  ok(/26123 期/.test($('ckSaved').textContent), '期号正确');
  ok(/一等奖/.test($('ckSaved').textContent), '标注中奖奖级', $('ckSaved').textContent.slice(0, 50));
  click($('ckSaved').querySelector('.tk'));
  await tick(150);
  ok($('ckFront').querySelectorAll('.pk-ball.on').length === 6, '点条目可重新载入号码');

  console.log('\n[10] 期号选择');
  click($('btnIssuePick'));
  await tick(150);
  ok($('sheet').classList.contains('on'), '期号面板打开');
  const items = $('sheetBody').querySelectorAll('.iss-item');
  ok(items.length === 40, '列出最近 40 期', items.length + ' 条');
  ok(items[0].textContent.indexOf('26123') >= 0, '第一条是最新期');
  click(items[1]);
  await tick(200);
  ok(!$('sheet').classList.contains('on'), '选完自动收起');
  ok($('ckIssue').textContent === '26122 期', '切到上一期', $('ckIssue').textContent);
  ok(/26122|14|21/.test($('ckDraw').textContent), '开奖号码随之变化', $('ckDraw').textContent.trim());

  console.log('\n[11] 无相机环境的降级');
  click($('btnPhoto'));
  await tick(150);
  ok(/预览环境没有相机/.test($('toast').textContent), '提示无法拍照', $('toast').textContent);

  console.log('\n[12] 运行期无异常');
  ok(errors.length === 0, '无 JS 运行时错误', errors.slice(0, 2).join(' | '));

  console.log('\n' + (fails === 0 ? '验票页全部通过 ✓' : fails + ' 项失败 ✗'));
  process.exit(fails === 0 ? 0 : 1);
})();
