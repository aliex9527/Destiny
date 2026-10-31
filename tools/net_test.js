/* 联网同步专项测试：真实执行 app.js，用假的 AndroidHost 模拟原生 HTTP 通道 */
const fs = require('fs');
const path = require('path');
const { JSDOM } = require('jsdom');

const DIR = __dirname;
const HTML = fs.readFileSync(path.join(DIR, 'index.html'), 'utf8');
const FIX = {
  official: fs.readFileSync(path.join(DIR, 'fixtures', 'official.json'), 'utf8'),
  backup: fs.readFileSync(path.join(DIR, 'fixtures', 'backup.html'), 'utf8')
};
const SCRIPTS = ['data.js', 'ticket.js', 'engine.js', 'app.js'].map(f => fs.readFileSync(path.join(DIR, f), 'utf8'));

let fails = 0;
function ok(cond, label, extra) {
  if (cond) console.log('  PASS  ' + label + (extra ? '  ' + extra : ''));
  else { console.log('  FAIL  ' + label + (extra ? '  ' + extra : '')); fails++; }
}
const tick = ms => new Promise(r => setTimeout(r, ms));

/**
 * 起一个应用实例。
 * mode: ok      两个源都成功（官方优先）
 *       backup  官方失败，回退 500 彩票网
 *       fail    两个源都失败
 *       junk    返回 200 但不是有效数据
 */
function boot(opts) {
  opts = opts || {};
  const errors = [];
  const dom = new JSDOM(HTML, {
    runScripts: 'dangerously',
    pretendToBeVisual: true,
    url: 'file:///C:/temp/dlttest/index.html',
    beforeParse(win) {
      win.addEventListener('error', e => errors.push(String(e.message || e)));
      win.HTMLCanvasElement.prototype.getContext = function () { return null; };
    }
  });
  const win = dom.window;

  const state = {
    mode: opts.mode || 'ok',
    pending: opts.pending || '',
    prefs: { first: true, reminder: false, seen: '', canNotify: false },
    remindCalls: []
  };
  const calls = [];

  win.AndroidHost = {
    get(url, token, headers) {
      calls.push({ url, headers });
      const official = url.indexOf('sporttery') !== -1;
      let code = 200, body = '';
      if (official) {
        if (state.mode === 'ok') body = FIX.official;
        else if (state.mode === 'junk') body = '<html>not json</html>';
        else code = 500;
      } else {
        if (state.mode === 'ok' || state.mode === 'backup') body = FIX.backup;
        else if (state.mode === 'junk') body = '<html><table></table></html>';
        else code = 500;
      }
      setTimeout(() => {
        try { win.DLT_NET.onData(token, code, code === 200 ? body : 'connection reset'); } catch (e) { }
      }, 10);
    },
    prefs() { return JSON.stringify(state.prefs); },
    setReminder(on) {
      state.remindCalls.push(on);
      state.prefs.reminder = !!on;
      state.prefs.first = false;
      return !!on;
    },
    markSeen(v) { state.prefs.seen = v; },
    takePending() { const v = state.pending; state.pending = ''; return v; },
    setSheet() { }
  };

  SCRIPTS.forEach(code => {
    const s = win.document.createElement('script');
    s.textContent = code;
    win.document.body.appendChild(s);
  });

  const doc = win.document;
  const $ = id => doc.getElementById(id);
  const click = el => el.dispatchEvent(new win.MouseEvent('click', { bubbles: true }));
  return { dom, win, $, click, calls, state, errors, close: () => win.close() };
}

(async () => {

  console.log('\n[1] 启动自动同步（官方源）');
  {
    const a = boot({ mode: 'ok' });
    await tick(1700);      // init 后 1200ms 触发同步，再等网络回调
    ok(a.errors.length === 0, '无 JS 运行时错误', a.errors[0] || '');
    ok(a.calls.length >= 1 && /sporttery/.test(a.calls[0].url), '首发请求走体彩官方接口');
    ok(/Referer: https:\/\/static\.sporttery\.cn/.test(a.calls[0].headers), '带上官方 Referer 头');
    ok(/共 2943 期/.test(a.$('hdSub').textContent), '自动并入 2 期', a.$('hdSub').textContent.trim());
    ok(a.$('lastIssue').textContent === '26125 期', '上期更新到新期号', a.$('lastIssue').textContent);
    ok(/更新至 26125 期/.test(a.$('syncText').textContent), '状态条显示最新期号', a.$('syncText').textContent);
    ok(/dot ok/.test(a.$('syncDot').className), '状态点为正常色', a.$('syncDot').className);
    ok(a.state.prefs.seen === '26125', '已向宿主登记已见期号', a.state.prefs.seen);

    console.log('\n[2] 重复检查不重复写入');
    const before = a.$('hdSub').textContent;
    a.click(a.$('btnSync'));
    await tick(200);
    ok(a.$('hdSub').textContent === before, '期数不变', a.$('hdSub').textContent.trim());
    ok(/已是最新/.test(a.$('toast').textContent), '提示已是最新', a.$('toast').textContent);
    a.close();
  }

  console.log('\n[3] 官方源故障时回退备用源');
  {
    const b = boot({ mode: 'backup' });
    await tick(1700);
    ok(b.calls.length >= 2, '发起了两次请求', b.calls.length + ' 次');
    ok(/500\.com/.test(b.calls[1].url), '第二次打到 500 彩票网', b.calls[1].url.slice(0, 46) + '…');
    ok(/共 2942 期/.test(b.$('hdSub').textContent), '并入备用源的 1 期', b.$('hdSub').textContent.trim());
    ok(b.$('lastIssue').textContent === '26124 期', '期号正确解析', b.$('lastIssue').textContent);
    b.close();
  }

  console.log('\n[4] 全部失败时的降级表现');
  {
    const c = boot({ mode: 'fail' });
    await tick(1700);
    ok(/共 2941 期/.test(c.$('hdSub').textContent), '数据保持原样', c.$('hdSub').textContent.trim());
    ok(/更新失败/.test(c.$('syncText').textContent), '状态条提示失败', c.$('syncText').textContent);
    ok(/dot bad/.test(c.$('syncDot').className), '状态点转为警示色');
    c.click(c.$('btnSync'));
    await tick(200);
    ok(/更新失败/.test(c.$('toast').textContent), '手动重试给出失败提示', c.$('toast').textContent);
    c.close();
  }

  console.log('\n[5] 返回异常内容时不写入脏数据');
  {
    const d = boot({ mode: 'junk' });
    await tick(1700);
    ok(/共 2941 期/.test(d.$('hdSub').textContent), '脏数据被丢弃', d.$('hdSub').textContent.trim());
    d.close();
  }

  console.log('\n[6] 后台查号缓存静默并入');
  {
    const e = boot({ mode: 'fail', pending: '26124,3,11,22,28,33,5,9,2026-10-31' });
    await tick(1700);
    ok(/共 2942 期/.test(e.$('hdSub').textContent), '启动时并入后台抓到的 1 期', e.$('hdSub').textContent.trim());
    ok(e.$('lastIssue').textContent === '26124 期', '期号来自缓存', e.$('lastIssue').textContent);
    ok(/已并入 1 期/.test(e.$('toast').textContent), '给出并入提示', e.$('toast').textContent);
    e.close();
  }

  console.log('\n[7] 数据与提醒面板');
  {
    const f = boot({ mode: 'ok' });
    await tick(1700);
    ok(f.state.remindCalls.length === 1 && f.state.remindCalls[0] === true, '首次运行默认开启提醒');
    f.click(f.$('syncInfo'));
    await tick(120);
    ok(f.$('sheet').classList.contains('on'), '点状态条打开面板');
    ok(!!f.$('swRemind'), '面板里有提醒开关');
    ok(f.$('swRemind').classList.contains('on'), '开关处于开启态');
    ok(/2941/.test(f.$('sheetBody').textContent), '展示内置期数');
    f.click(f.$('swRemind'));
    await tick(120);
    ok(f.state.remindCalls[f.state.remindCalls.length - 1] === false, '关闭开关会写回宿主', f.state.remindCalls.join(','));
    ok(!f.$('swRemind').classList.contains('on'), '开关视觉同步关闭');
    ok(/已关闭每日开奖提醒/.test(f.$('toast').textContent), '给出关闭提示', f.$('toast').textContent);
    f.close();
  }

  console.log('\n[8] 同步到的奖金能用于验票');
  {
    const g = boot({ mode: 'ok' });
    await tick(1700);
    g.click([...g.$('tabbar').querySelectorAll('button')].find(b => b.dataset.tab === 'check'));
    await tick(200);
    ok(g.$('ckIssue').textContent === '26125 期', '验票期号跟到最新', g.$('ckIssue').textContent);
    ok(/奖金取自官方公告/.test(g.$('ckNote').textContent), '注明奖金来源', g.$('ckNote').textContent);
    // 样例里 26125 的号码是真实 26123 那组：13 18 19 20 28 + 09 11
    for (const n of [13, 18, 19, 20, 28]) g.click(g.$('ckFront').querySelector('[data-n="' + n + '"]'));
    for (const n of [9, 11]) g.click(g.$('ckBack').querySelector('[data-n="' + n + '"]'));
    await tick(150);
    g.click(g.$('btnCheck'));
    await tick(250);
    const txt = g.$('ckResult').textContent;
    ok(/一等奖/.test(txt), '判为一等奖');
    ok(/5,750,462 元/.test(txt), '金额取自官方公告而非估算', txt.slice(0, 60));
    ok(!/浮动奖/.test(txt), '有官方数据时不再显示浮动');
    g.close();
  }

  console.log('\n' + (fails === 0 ? '同步模块全部通过 ✓' : fails + ' 项失败 ✗'));
  process.exit(fails === 0 ? 0 : 1);
})();
