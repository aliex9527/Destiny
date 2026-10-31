/* jsdom 冒烟测试：真实执行 app.js，断言渲染结果 */
const fs = require('fs');
const path = require('path');
const { JSDOM } = require('jsdom');

const DIR = __dirname;
const html = fs.readFileSync(path.join(DIR, 'index.html'), 'utf8');

let fails = 0;
function ok(cond, label, extra) {
  if (cond) { console.log('  PASS  ' + label + (extra ? '  ' + extra : '')); }
  else { console.log('  FAIL  ' + label + (extra ? '  ' + extra : '')); fails++; }
}

const errors = [];
const dom = new JSDOM(html, {
  runScripts: 'dangerously',
  pretendToBeVisual: true,
  url: 'file:///C:/temp/dlttest/index.html',
  beforeParse(win) {
    win.addEventListener('error', e => errors.push(String(e.message || e)));
    // jsdom 无 canvas 实现，返回 null 让应用走降级分支
    win.HTMLCanvasElement.prototype.getContext = function () { return null; };
  }
});

// 手动注入脚本（避免资源加载顺序问题）
['data.js', 'ticket.js', 'engine.js', 'app.js'].forEach(f => {
  const code = fs.readFileSync(path.join(DIR, f), 'utf8');
  const s = dom.window.document.createElement('script');
  s.textContent = code;
  dom.window.document.body.appendChild(s);
});

const win = dom.window, doc = win.document;
const $ = id => doc.getElementById(id);
const click = el => el.dispatchEvent(new win.MouseEvent('click', { bubbles: true }));

function tick(ms) { return new Promise(r => setTimeout(r, ms)); }

(async () => {
  await tick(400);   // 等 init + 首次推演（120ms 延时）

  console.log('\n[0] 界面结构');
  ok(!!$('btnSync') && !$('btnAdd'), '右上角是同步按钮，没有录入按钮');
  ok(!doc.querySelector('#syncBar .sb-btn'), '状态条不再带独立同步按钮');
  ok($('tabbar').querySelectorAll('button').length === 4, '底部 4 个页签', '' + $('tabbar').querySelectorAll('button').length);
  ok(!$('v-trend') && !$('v-backtest'), '走势与回测页已移除');
  ok(+$('winTxt').textContent > 100, '统计范围为近一年', $('winTxt').textContent + ' 期');

  console.log('\n[1] 启动与首屏');
  ok(errors.length === 0, '无 JS 运行时错误', errors.length ? JSON.stringify(errors.slice(0, 2)) : '');
  ok(/共 2941 期/.test($('hdSub').textContent), '头部期数统计', $('hdSub').textContent.trim());
  ok($('lastIssue').textContent === '26123 期', '上期期号', $('lastIssue').textContent);
  ok($('lastBalls').querySelectorAll('.ball.f').length === 5, '上期前区 5 个球');
  ok($('lastBalls').querySelectorAll('.ball.b').length === 2, '上期后区 2 个球');
  ok($('lastBalls').textContent.indexOf('13') >= 0, '上期号码内容正确');

  console.log('\n[2] 推演结果');
  const picks = $('predOut').querySelectorAll('.pick');
  ok(picks.length === 5, '默认生成 5 注', '实际 ' + picks.length);
  const p0 = picks[0];
  ok(p0.querySelectorAll('.ball.f').length === 5, '每注前区 5 个');
  ok(p0.querySelectorAll('.ball.b').length === 2, '每注后区 2 个');
  const front = [...p0.querySelectorAll('.ball.f')].map(b => +b.textContent);
  ok(new Set(front).size === 5, '前区号码不重复', front.join(','));
  ok(front.every(v => v >= 1 && v <= 35), '前区号码在 1-35');
  const back = [...p0.querySelectorAll('.ball.b')].map(b => +b.textContent);
  ok(back.every(v => v >= 1 && v <= 12), '后区号码在 1-12', back.join(','));
  ok(p0.querySelectorAll('.tag').length > 0, '有特征标签');
  ok(/\d/.test(p0.querySelector('.sc').textContent), '有得分');

  console.log('\n[3] 切换策略与注数');
  const hotChip = [...$('stratChips').querySelectorAll('.chip')].find(c => c.textContent === '热号追踪');
  click(hotChip);
  await tick(300);
  ok($('stratChips').querySelector('.chip[data-strat="hot"]').classList.contains('on'),
     '策略 chip 选中态切换');
  ok($('stratDesc').textContent.length > 4, '策略说明更新', $('stratDesc').textContent.slice(0, 18) + '…');
  const tenBtn = [...$('cntSeg').querySelectorAll('button')].find(b => b.textContent.indexOf('10') >= 0);
  click(tenBtn);
  click($('btnRun'));
  await tick(400);
  ok($('predOut').querySelectorAll('.pick').length === 10, '改成 10 注后生成 10 注');

  console.log('\n[4] 分析页');
  click([...$('tabbar').querySelectorAll('button')].find(b => b.dataset.tab === 'analysis'));
  await tick(200);
  ok($('v-analysis').classList.contains('on'), '分析页显示');
  ok($('heatF').querySelectorAll('.hc').length === 35, '前区热力 35 格', '实际 ' + $('heatF').querySelectorAll('.hc').length);
  ok($('heatB').querySelectorAll('.hc').length === 12, '后区热力 12 格');
  ok($('omitList').querySelectorAll('.omit-it').length === 10, '遗漏榜 10 条');
  ok($('oddBars').querySelectorAll('.bar-row').length === 6, '奇偶 6 档');
  ok($('zoneBars').querySelectorAll('.bar-row').length === 5, '分区 5 档');
  ok($('sumTxt').textContent.indexOf('均值') >= 0, '和值摘要', $('sumTxt').textContent);

  // 点号码看详情
  click($('heatF').querySelector('.hc'));
  await tick(100);
  ok($('sheet').classList.contains('on'), '号码详情弹层打开');
  ok($('sheetBody').textContent.indexOf('当前遗漏') >= 0, '详情含遗漏信息');
  click(doc.querySelector('[data-close]'));
  await tick(100);
  ok(!$('sheet').classList.contains('on'), '弹层可关闭');

  console.log('\n[5] 历史页');
  click([...$('tabbar').querySelectorAll('button')].find(b => b.dataset.tab === 'history'));
  await tick(200);
  ok($('histList').querySelectorAll('.hist-it').length === 40, '首屏 40 条', '实际 ' + $('histList').querySelectorAll('.hist-it').length);
  click($('btnMore'));
  await tick(100);
  ok($('histList').querySelectorAll('.hist-it').length === 80, '加载更多 → 80 条');
  const qi = $('qInput');
  qi.value = '26123';
  qi.dispatchEvent(new win.Event('input', { bubbles: true }));
  await tick(300);
  ok($('histList').querySelectorAll('.hist-it').length === 1, '搜索期号命中 1 条', $('histCount').textContent);

  console.log('\n[6] 收藏与我的方案');
  click([...$('tabbar').querySelectorAll('button')].find(b => b.dataset.tab === 'predict'));
  await tick(100);
  const saveBtn = $('predOut').querySelector('[data-save]');
  click(saveBtn);
  await tick(150);
  ok($('savedBox').querySelectorAll('.sv').length === 1, '收藏后出现 1 条方案');
  ok(/已收藏/.test($('toast').textContent), '收藏提示', $('toast').textContent);
  click($('savedBox').querySelector('[data-del]'));
  await tick(150);
  ok($('savedBox').querySelectorAll('.sv').length === 0, '删除后方案清空');

  console.log('\n[9] 录入开奖');
  click([...$('tabbar').querySelectorAll('button')].find(b => b.dataset.tab === 'predict'));
  await tick(100);
  click($('syncInfo'));
  await tick(120);
  ok(!!$('btnManualAdd'), '数据面板有手动录入入口');
  click($('btnManualAdd'));
  await tick(420);
  ok($('sheet').classList.contains('on'), '录入弹层打开');
  ok($('fIssue').value === '26124', '默认下一期期号', $('fIssue').value);
  $('fFront').value = '3 11 22 28 33';
  $('fBack').value = '5 9';
  click($('btnSaveDraw'));
  await tick(200);
  ok($('lastIssue').textContent === '26124 期', '保存后上期更新', $('lastIssue').textContent);
  ok(/共 2942 期/.test($('hdSub').textContent), '总期数 +1', $('hdSub').textContent.trim());

  console.log('\n[10] 异常输入校验');
  click($('syncInfo'));
  await tick(120);
  click($('btnManualAdd'));
  await tick(420);
  $('fIssue').value = '26125';
  $('fFront').value = '3 11 22 28';       // 只有 4 个
  $('fBack').value = '5 9';
  click($('btnSaveDraw'));
  await tick(100);
  ok(/前区需要 5 个号码/.test($('toast').textContent), '拦截数量不足', $('toast').textContent);
  $('fFront').value = '3 11 22 28 36';    // 超出 35
  click($('btnSaveDraw'));
  await tick(100);
  ok(/1–35/.test($('toast').textContent), '拦截越界号码', $('toast').textContent);

  console.log('\n[11] 批量导入解析');
  $('fBulk').value = '2026-10-31 03 11 22 28 33 05 09\n26126,2026-11-02,07 14 19 26 31,03 08';
  click($('btnBulk'));
  await tick(200);
  ok(/导入 2 期/.test($('toast').textContent), '批量导入 2 期', $('toast').textContent);
  ok(/共 2944 期/.test($('hdSub').textContent), '总期数 2944', $('hdSub').textContent.trim());

  console.log('\n' + (fails === 0 ? '全部通过 ✓' : fails + ' 项失败 ✗'));
  process.exit(fails === 0 ? 0 : 1);
})();
