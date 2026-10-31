const http = require('http');
const fs = require('fs');
const path = require('path');
const { chromium } = require('playwright');

const ROOT = __dirname;
const PORT = 8123;
const MIME = { '.html': 'text/html; charset=utf-8', '.css': 'text/css; charset=utf-8', '.js': 'application/javascript; charset=utf-8', '.png': 'image/png' };

const server = http.createServer((req, res) => {
  let p = req.url.split('?')[0];
  if (p === '/') p = '/index.html';
  const f = path.join(ROOT, p);
  fs.readFile(f, (e, buf) => {
    if (e) { res.writeHead(404); res.end('404'); return; }
    res.writeHead(200, { 'Content-Type': MIME[path.extname(f)] || 'application/octet-stream' });
    res.end(buf);
  });
});

(async () => {
  await new Promise(r => server.listen(PORT, r));
  const browser = await chromium.launch({ args: ['--no-sandbox', '--font-render-hinting=none'] });
  const ctx = await browser.newContext({
    viewport: { width: 390, height: 844 },
    deviceScaleFactor: 2,
    isMobile: true,
    hasTouch: true,
    userAgent: 'Mozilla/5.0 (Linux; Android 14; Pixel 7) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0 Mobile Safari/537.36'
  });
  const page = await ctx.newPage();
  page.on('console', m => { if (m.type() === 'error') console.log('  [console.error]', m.text()); });
  page.on('pageerror', e => console.log('  [pageerror]', e.message));

  // 注入假宿主：让同步状态条在浏览器里也能呈现真实状态（返回已内置的 26123 期，故不会新增数据）
  await page.addInitScript(() => {
    window.AndroidHost = {
      get(url, token) {
        const body = JSON.stringify({
          value: {
            list: [{
              lotteryDrawNum: '26123',
              lotteryDrawResult: '13 18 19 20 28 09 11',
              lotteryDrawTime: '2026-10-28',
              prizeLevelList: [
                { prizeLevel: '一等奖', stakeAmount: '5,750,462' },
                { prizeLevel: '一等奖(追加)', stakeAmount: '4,600,369' },
                { prizeLevel: '二等奖', stakeAmount: '47,994' },
                { prizeLevel: '二等奖(追加)', stakeAmount: '38,395' },
                { prizeLevel: '三等奖', stakeAmount: '5,000' },
                { prizeLevel: '四等奖', stakeAmount: '300' },
                { prizeLevel: '五等奖', stakeAmount: '150' },
                { prizeLevel: '六等奖', stakeAmount: '15' },
                { prizeLevel: '七等奖', stakeAmount: '5' }
              ]
            }]
          }
        });
        setTimeout(() => { try { window.DLT_NET.onData(token, 200, body); } catch (e) { } }, 60);
      },
      prefs: () => JSON.stringify({ first: false, reminder: true, seen: '26123', canNotify: true }),
      setReminder: v => v,
      markSeen: () => { },
      takePending: () => '',
      setSheet: () => { }
    };
  });

  await page.goto(`http://127.0.0.1:${PORT}/index.html`, { waitUntil: 'load' });
  await page.waitForTimeout(900);

  const tabs = ['predict', 'check', 'analysis', 'history'];
  for (const t of tabs) {
    if (t !== 'predict') {
      await page.click(`#tabbar button[data-tab="${t}"]`);
      await page.waitForTimeout(450);
    } else {
      // 收藏两注，检查"我的方案"区块
      await page.click('#predOut .pick:nth-child(2) [data-save]');
      await page.click('#predOut .pick:nth-child(3) [data-save]');
      await page.evaluate(() => { document.getElementById('main').scrollTop = 0; });
      await page.waitForTimeout(2000);   // 等 toast 消失
    }
    if (t === 'check') {
      // 录入一张中了 4+1 的票，展示固定奖金额
      for (const n of [13, 18, 19, 20, 33]) await page.click(`#ckFront [data-n="${n}"]`);
      for (const n of [9, 5]) await page.click(`#ckBack [data-n="${n}"]`);
      await page.waitForTimeout(300);
      await page.click('#btnCheck');
      await page.waitForTimeout(800);
      await page.evaluate(() => { document.getElementById('main').scrollTop = 0; });
      await page.waitForTimeout(300);
      await page.screenshot({ path: path.join(ROOT, 'shot-check-top.png') });
      await page.evaluate(() => {
        const m = document.getElementById('main');
        m.scrollTop = document.getElementById('ckResult').offsetTop - 80;
      });
      await page.waitForTimeout(400);
      await page.screenshot({ path: path.join(ROOT, 'shot-check-result.png') });
    }
    await page.screenshot({ path: path.join(ROOT, `shot-${t}.png`) });
    await page.screenshot({ path: path.join(ROOT, `shot-${t}-full.png`), fullPage: true });
    // 量一下页面高度，确认没有横向溢出
    const m = await page.evaluate(() => ({
      scrollW: document.documentElement.scrollWidth,
      clientW: document.documentElement.clientWidth,
      viewH: document.documentElement.clientHeight
    }));
    console.log(`  ${t}: scrollW=${m.scrollW} clientW=${m.clientW} (横向溢出=${m.scrollW > m.clientW ? '是' : '否'})`);
  }

  // 弹层截图（号码详情）
  await page.click('#tabbar button[data-tab="analysis"]');
  await page.waitForTimeout(400);
  await page.click('#heatF .hc:nth-child(26)');
  await page.waitForTimeout(400);
  await page.screenshot({ path: path.join(ROOT, 'shot-sheet.png') });

  // 票面照片状态（用一张现成图模拟拍摄结果）
  await page.evaluate(() => window.DLT_closeSheet());
  await page.waitForTimeout(250);
  await page.click('#tabbar button[data-tab="check"]');
  await page.waitForTimeout(300);
  await page.evaluate(() => {
    const img = document.getElementById('photoImg');
    img.src = 'shot-analysis.png';
    document.getElementById('photoBox').classList.add('on');
    document.getElementById('main').scrollTop = 0;
  });
  await page.waitForTimeout(600);
  await page.screenshot({ path: path.join(ROOT, 'shot-check-photo.png') });

  // 数据与提醒面板
  await page.evaluate(() => window.DLT_closeSheet());
  await page.waitForTimeout(300);
  await page.click('#syncInfo');
  await page.waitForTimeout(400);
  await page.screenshot({ path: path.join(ROOT, 'shot-sync.png') });
  const syncTxt = await page.textContent('#syncText');
  console.log('  同步条文案: ' + syncTxt.trim());

  await browser.close();
  server.close();
  console.log('截图完成');
})();
