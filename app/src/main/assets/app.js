/* ================= Destiny · 交互 ================= */
(function () {
  'use strict';

  var E = window.DLT;
  var $ = function (id) { return document.getElementById(id); };
  var FM = 35, BM = 12;

  /* ---------------- 本地存储 ---------------- */
  var Store = (function () {
    var mem = {};
    var ok = false;
    try { localStorage.setItem('__t', '1'); localStorage.removeItem('__t'); ok = true; } catch (e) { ok = false; }
    function get(k, dflt) {
      try {
        var v = ok ? localStorage.getItem(k) : mem[k];
        return v ? JSON.parse(v) : dflt;
      } catch (e) { return dflt; }
    }
    function set(k, v) {
      var s = JSON.stringify(v);
      try { if (ok) localStorage.setItem(k, s); else mem[k] = s; } catch (e) { mem[k] = s; }
    }
    return { get: get, set: set };
  })();

  /* ---------------- 数据 ---------------- */
  function parseRaw(raw) {
    var out = [];
    var arr = raw.split(';');
    for (var i = 0; i < arr.length; i++) {
      var p = arr[i].split(',');
      if (p.length < 8) continue;
      out.push({
        i: p[0],
        f: [+p[1], +p[2], +p[3], +p[4], +p[5]],
        b: [+p[6], +p[7]],
        d: p[8] || ''
      });
    }
    return out;
  }

  var BASE = parseRaw(window.DLT_RAW || '');
  var DRAW = BASE.slice();

  function loadCustom() {
    var c = Store.get('dlt.custom', []);
    if (!c || !c.length) return;
    var map = {};
    for (var i = 0; i < DRAW.length; i++) map[DRAW[i].i] = i;
    for (var j = 0; j < c.length; j++) {
      if (map[c[j].i] !== undefined) DRAW[map[c[j].i]] = c[j];
      else DRAW.push(c[j]);
    }
    DRAW.sort(function (a, b) { return a.i < b.i ? -1 : 1; });
  }
  loadCustom();

  function last() { return DRAW[DRAW.length - 1]; }

  /* ---------------- 通用工具 ---------------- */
  function pad2(n) { return n < 10 ? '0' + n : '' + n; }

  function ballsHTML(f, b, cls) {
    cls = cls || '';
    var h = '';
    for (var i = 0; i < f.length; i++) h += '<span class="ball f ' + cls + '">' + pad2(f[i]) + '</span>';
    if (b && b.length) {
      h += '<span class="sep"></span>';
      for (var j = 0; j < b.length; j++) h += '<span class="ball b ' + cls + '">' + pad2(b[j]) + '</span>';
    }
    return h;
  }

  var toastTimer = null;
  function toast(msg) {
    var t = $('toast');
    t.textContent = msg;
    t.classList.add('on');
    clearTimeout(toastTimer);
    toastTimer = setTimeout(function () { t.classList.remove('on'); }, 1700);
  }

  function nextIssueInfo() {
    var L = last();
    if (!L) return { issue: '—', date: '—' };
    var d = new Date(L.d.replace(/-/g, '/') + ' 00:00:00');
    for (var k = 0; k < 8; k++) {
      d.setDate(d.getDate() + 1);
      var wd = d.getDay();
      if (wd === 1 || wd === 3 || wd === 6) break;   // 周一/三/六
    }
    var y = d.getFullYear(), m = d.getMonth() + 1, day = d.getDate();
    var dateStr = y + '-' + pad2(m) + '-' + pad2(day);
    var lastY = L.i.slice(0, 2), seq = parseInt(L.i.slice(2), 10);
    var issue;
    var sameYear = String(y % 100) === lastY;
    if (sameYear) issue = lastY + ('00' + (seq + 1)).slice(-3);
    else issue = ('0' + (y % 100)).slice(-2) + '001';
    return { issue: issue, date: dateStr };
  }

  /* ---------------- 状态 ---------------- */
  var S = {
    tab: 'predict',
    strat: Store.get('dlt.strat', 'balanced'),
    count: Store.get('dlt.count', 5),
    histLimit: 40,
    histQuery: '',
    result: null
  };

  /* ---------------- 导航 ---------------- */
  var VIEWS = { predict: 'v-predict', check: 'v-check', analysis: 'v-analysis', history: 'v-history' };

  function goTab(tab) {
    S.tab = tab;
    var btns = document.querySelectorAll('#tabbar button');
    for (var i = 0; i < btns.length; i++) btns[i].classList.toggle('on', btns[i].getAttribute('data-tab') === tab);
    for (var k in VIEWS) $(VIEWS[k]).classList.toggle('on', k === tab);
    $('main').scrollTop = 0;
    if (tab === 'check') { if (!CK.issue) CK.issue = last() ? last().i : ''; renderCheck(); renderTickets(); }
    if (tab === 'analysis') renderAnalysis();
    if (tab === 'history') renderHistory(true);
  }

  /* ================= 预测 ================= */
  function renderHeader() {
    var L = last();
    if (!L) return;
    $('hdSub').textContent = '共 ' + DRAW.length + ' 期 · ' + L.d.slice(0, 4) + ' 年至今';
    $('winTxt').textContent = yearCount();
    $('lastIssue').textContent = L.i + ' 期';
    $('lastDate').textContent = L.d + ' ' + weekday(L.d);
    $('lastBalls').innerHTML = ballsHTML(L.f, L.b);
    var nx = nextIssueInfo();
    $('nextIssue').textContent = nx.issue + ' 期';
    $('nextDate').textContent = nx.date + ' ' + weekday(nx.date) + ' 开奖';
    renderSaved();
    renderSyncBar();
  }

  function renderSaved() {
    var saved = Store.get('dlt.saved', []);
    var box = $('savedBox');
    if (!saved || !saved.length) { box.innerHTML = ''; return; }
    var h = '<div class="card"><div class="card-hd"><h3>我的方案</h3><span class="sub">'
      + saved.length + ' 注 · 本机保存</span></div>';
    for (var i = 0; i < saved.length; i++) {
      var s = saved[i];
      h += '<div class="sv"><span class="sv-issue">' + (s.issue || '') + '</span>'
        + '<span class="sv-balls">' + ballsHTML(s.f, s.b, 'tiny') + '</span>'
        + '<button class="sv-del" data-del="' + i + '" aria-label="删除">×</button></div>';
    }
    h += '</div>';
    box.innerHTML = h;
    box.querySelectorAll('[data-del]').forEach(function (b) {
      b.onclick = function () {
        var idx = +b.getAttribute('data-del');
        var arr = Store.get('dlt.saved', []);
        arr.splice(idx, 1);
        Store.set('dlt.saved', arr);
        renderSaved();
        toast('已删除');
      };
    });
  }

  function weekday(ds) {    try {
      var d = new Date(ds.replace(/-/g, '/') + ' 00:00:00');
      return '周' + '日一二三四五六'.charAt(d.getDay());
    } catch (e) { return ''; }
  }

  function renderStrategyChips() {
    var h = '';
    for (var k in E.STRATEGIES) {
      var s = E.STRATEGIES[k];
      h += '<button class="chip' + (k === S.strat ? ' on' : '') + '" data-strat="' + k + '">' + s.name + '</button>';
    }
    var box = $('stratChips');
    box.innerHTML = h;
    box.querySelectorAll('.chip').forEach(function (b) {
      b.onclick = function () {
        S.strat = b.getAttribute('data-strat');
        Store.set('dlt.strat', S.strat);
        renderStrategyChips();
        if (S.result) runPredict();
      };
    });
    $('stratDesc').textContent = (E.STRATEGIES[S.strat] || {}).desc || '';
  }

  /** 统计口径固定为「近一年」：从最新一期往前 365 天，按实际开奖期数取窗口 */
  var yearN = 0;
  function yearCount() {
    if (yearN) return yearN;
    if (!DRAW.length) return 1;
    var lastT = 0;
    try { lastT = new Date(DRAW[DRAW.length - 1].d + 'T00:00:00').getTime(); } catch (e) { lastT = 0; }
    if (!lastT) return Math.min(DRAW.length, 156);
    var cut = lastT - 365 * 86400000;
    var n = 0;
    for (var i = DRAW.length - 1; i >= 0; i--) {
      var t = 0;
      try { t = new Date(DRAW[i].d + 'T00:00:00').getTime(); } catch (e2) { t = 0; }
      if (t && t >= cut) n++; else break;
    }
    yearN = Math.max(1, Math.min(n, DRAW.length));
    return yearN;
  }

  function renderSegs() {
    $('winTxt').textContent = yearCount();
    var cs = [3, 5, 10];
    var ch = '';
    for (var j = 0; j < cs.length; j++) ch += '<button data-c="' + cs[j] + '" class="' + (S.count === cs[j] ? 'on' : '') + '">' + cs[j] + ' 注</button>';
    $('cntSeg').innerHTML = ch;
    $('cntSeg').querySelectorAll('button').forEach(function (b) {
      b.onclick = function () { S.count = +b.getAttribute('data-c'); Store.set('dlt.count', S.count); renderSegs(); };
    });
  }

  function runPredict() {
    var btn = $('btnRun');
    btn.disabled = true;
    btn.innerHTML = '推演中…';
    setTimeout(function () {
      var t0 = Date.now();
      var res = E.predict(DRAW, {
        strategy: S.strat, window: yearCount(), count: S.count
      });
      S.result = res;
      renderPredict(res, Date.now() - t0);
      btn.disabled = false;
      btn.innerHTML = '<svg viewBox="0 0 24 24"><path d="M13 2 4.5 13.5H11l-1 8.5L19.5 10H13l0-8Z" fill="currentColor"/></svg>重新推演';
    }, 40);
  }

  function renderPredict(res, ms) {
    var box = $('predOut');
    if (!res || !res.sets.length) { box.innerHTML = '<p class="hint">没有生成符合条件的号码，请放宽参数。</p>'; return; }
    var h = '<div class="list-hd"><span>第 ' + nextIssueInfo().issue + ' 期 · '
      + E.STRATEGIES[res.strategy].name
      + ' · 近一年 ' + res.window + ' 期'
      + ' · 共 ' + res.sets.length + ' 注</span><span>' + ms + 'ms</span></div>';

    for (var i = 0; i < res.sets.length; i++) {
      var s = res.sets[i];
      h += '<div class="pick" style="animation-delay:' + (i * 40) + 'ms">'
        + '<div class="pick-hd"><span class="rk">' + (i + 1) + '</span>'
        + '<div class="pick-balls">' + ballsHTML(s.f, s.b, 'sm') + '</div>'
        + '<span class="sc">' + (s.score * 100).toFixed(0) + '</span></div>'
        + '<div class="pick-ft"><span>和值 <b>' + s.sum + '</b></span>'
        + '<span>跨度 <b>' + s.span + '</b></span>'
        + '<span>奇偶 <b>' + s.oddF + ':' + (5 - s.oddF) + '</b></span>'
        + '<span>大小 <b>' + s.bigF + ':' + (5 - s.bigF) + '</b></span>'
        + '<span>连号 <b>' + s.consec + '</b></span></div>'
        + '<div class="tags">';
      for (var t = 0; t < s.tags.length; t++) {
        var warn = /高|低|连/.test(s.tags[t]);
        h += '<span class="tag' + (warn ? ' warn' : '') + '">' + s.tags[t] + '</span>';
      }
      h += '</div><div class="pick-acts">'
        + '<button data-copy="' + i + '">复制号码</button>'
        + '<button data-save="' + i + '">收藏</button></div></div>';
    }
    h += '<p class="hint">得分 = 该组合与历史号码结构（和值/跨度/奇偶/分区）的贴合度，满分为近似 100，仅表示"像不像往期"，不代表中奖概率。</p>';
    box.innerHTML = h;

    box.querySelectorAll('[data-copy]').forEach(function (b) {
      b.onclick = function () {
        var s = res.sets[+b.getAttribute('data-copy')];
        var txt = s.f.map(pad2).join(' ') + ' + ' + s.b.map(pad2).join(' ');
        copyText(txt);
      };
    });
    box.querySelectorAll('[data-save]').forEach(function (b) {
      b.onclick = function () {
        var s = res.sets[+b.getAttribute('data-save')];
        var saved = Store.get('dlt.saved', []);
        var key = s.f.join(',') + '|' + s.b.join(',');
        for (var q = 0; q < saved.length; q++) {
          if (saved[q].key === key) { toast('已经收藏过了'); return; }
        }
        saved.unshift({ key: key, f: s.f, b: s.b, issue: nextIssueInfo().issue, ts: Date.now() });
        Store.set('dlt.saved', saved.slice(0, 60));
        renderSaved();
        toast('已收藏，共 ' + saved.length + ' 注');
      };
    });
  }

  function copyText(txt) {
    try {
      var ta = document.createElement('textarea');
      ta.value = txt;
      ta.style.position = 'fixed';
      ta.style.opacity = '0';
      document.body.appendChild(ta);
      ta.select();
      document.execCommand('copy');
      document.body.removeChild(ta);
      toast('已复制 ' + txt);
    } catch (e) { toast(txt); }
  }

  /* ================= 分析 ================= */
  function renderAnalysis() {
    var st = E.buildStats(DRAW, yearCount());
    $('anaWinTxt').textContent = '近一年 · ' + st.n + ' 期';

    // 前区热力
    var maxF = 1, minF = 999, v;
    for (v = 1; v <= FM; v++) { if (st.freqF[v] > maxF) maxF = st.freqF[v]; if (st.freqF[v] < minF) minF = st.freqF[v]; }
    var hf = '';
    for (v = 1; v <= FM; v++) {
      var t = (st.freqF[v] - minF) / Math.max(1, maxF - minF);
      var bg = heatColor(t);
      var light = t > 0.70;
      hf += '<div class="hc" data-num="' + v + '" style="background:' + bg + ';color:' + (light ? '#fff' : '#3A4150') + '">'
        + pad2(v) + '<i style="color:' + (light ? 'rgba(255,255,255,.82)' : 'rgba(58,65,80,.42)') + '">' + st.omitF[v] + '</i></div>';
    }
    $('heatF').innerHTML = hf;
    $('heatF').querySelectorAll('.hc').forEach(function (c) {
      c.onclick = function () { showNumberDetail(+c.getAttribute('data-num'), st); };
    });

    var maxB = 1, minB = 999;
    for (v = 1; v <= BM; v++) { if (st.freqB[v] > maxB) maxB = st.freqB[v]; if (st.freqB[v] < minB) minB = st.freqB[v]; }
    var hb = '';
    for (v = 1; v <= BM; v++) {
      var tb = (st.freqB[v] - minB) / Math.max(1, maxB - minB);
      var bg2 = heatBlue(tb);
      var lightB = tb > 0.68;
      hb += '<div class="hc" data-numb="' + v + '" style="background:' + bg2 + ';color:' + (lightB ? '#fff' : '#28406F') + '">'
        + pad2(v) + '<i style="color:' + (lightB ? 'rgba(255,255,255,.82)' : 'rgba(40,64,111,.42)') + '">' + st.omitB[v] + '</i></div>';
    }
    $('heatB').innerHTML = hb;

    // 和值分布
    $('sumTxt').textContent = '均值 ' + st.sumMean.toFixed(1) + ' · 80%区间 ' + Math.round(st.sumQ10) + '–' + Math.round(st.sumQ90);
    drawHistogram($('sumChart'), st.sumsSorted, 40, 160, 10);

    // 奇偶
    $('oddBars').innerHTML = barRows([
      ['5:0', st.oddDist[5]], ['4:1', st.oddDist[4]], ['3:2', st.oddDist[3]],
      ['2:3', st.oddDist[2]], ['1:4', st.oddDist[1]], ['0:5', st.oddDist[0]]
    ], st.n, false);

    $('bigBars').innerHTML = barRows([
      ['5:0', st.bigDist[5]], ['4:1', st.bigDist[4]], ['3:2', st.bigDist[3]],
      ['2:3', st.bigDist[2]], ['1:4', st.bigDist[1]], ['0:5', st.bigDist[0]]
    ], st.n, false);

    var zones = ['01-07', '08-14', '15-21', '22-28', '29-35'];
    var zrows = [];
    for (var z = 0; z < 5; z++) zrows.push([zones[z], st.zoneTotal[z]]);
    $('zoneBars').innerHTML = barRows(zrows, st.n * 5, true);

    // 遗漏榜
    var list = [];
    for (v = 1; v <= FM; v++) list.push({ n: v, o: st.omitF[v], m: st.maxOmitF[v] });
    list.sort(function (a, b) { return b.o - a.o; });
    var maxO = list[0].o || 1;
    var oh = '';
    for (var i = 0; i < 10 && i < list.length; i++) {
      var it = list[i];
      oh += '<div class="omit-it"><span class="n">' + pad2(it.n) + '</span>'
        + '<span class="om">当前遗漏 <b>' + it.o + '</b> 期 · 历史最大 ' + it.m + '</span>'
        + '<span class="bar"><i style="width:' + Math.max(4, it.o / maxO * 100) + '%"></i></span></div>';
    }
    $('omitList').innerHTML = oh;
  }

  function heatColor(t) {
    // 浅粉 -> 深红，不透明色阶保证文字对比度稳定
    return 'hsl(4,72%,' + (96 - t * 52).toFixed(1) + '%)';
  }
  function heatBlue(t) {
    return 'hsl(222,78%,' + (95 - t * 48).toFixed(1) + '%)';
  }

  function barRows(rows, total, isCount) {
    var max = 1;
    for (var i = 0; i < rows.length; i++) if (rows[i][1] > max) max = rows[i][1];
    var h = '';
    for (var j = 0; j < rows.length; j++) {
      var pct = total ? rows[j][1] / total * 100 : 0;
      h += '<div class="bar-row"><span class="bar-lb">' + rows[j][0] + '</span>'
        + '<span class="bar-tr"><i class="bar-fl' + (isCount ? ' b' : '') + '" style="width:' + (rows[j][1] / max * 100) + '%"></i></span>'
        + '<span class="bar-vl">' + pct.toFixed(1) + '%</span></div>';
    }
    return h;
  }

  function drawHistogram(cv, sorted, lo, hi, step) {
    if (!cv || !cv.parentElement) return;
    var W = cv.parentElement.clientWidth - 28;
    if (W <= 0) W = 300;
    var H = 130;
    var dpr = window.devicePixelRatio || 1;
    cv.width = W * dpr; cv.height = H * dpr;
    cv.style.width = W + 'px'; cv.style.height = H + 'px';
    var ctx = cv.getContext('2d');
    if (!ctx) return;   // 环境不支持 canvas 时静默跳过，不阻塞页面
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, W, H);

    var nb = Math.ceil((hi - lo) / step);
    var bins = new Array(nb).fill(0);
    for (var i = 0; i < sorted.length; i++) {
      var b = Math.floor((sorted[i] - lo) / step);
      if (b < 0) b = 0;
      if (b >= nb) b = nb - 1;
      bins[b]++;
    }
    var maxV = Math.max.apply(null, bins);
    var padL = 22, padB = 18, gw = (W - padL) / nb;
    var bw = Math.max(3, gw - 3);

    // 轴线
    ctx.strokeStyle = '#EDEFF3';
    ctx.lineWidth = 1;
    ctx.beginPath();
    ctx.moveTo(padL, 2); ctx.lineTo(padL, H - padB); ctx.lineTo(W, H - padB);
    ctx.stroke();

    for (var k = 0; k < nb; k++) {
      var hgt = (bins[k] / maxV) * (H - padB - 8);
      var x = padL + k * gw + 1.5;
      var y = H - padB - hgt;
      var grad = ctx.createLinearGradient(0, y, 0, H - padB);
      grad.addColorStop(0, '#F0705F');
      grad.addColorStop(1, 'rgba(222,51,43,.55)');
      ctx.fillStyle = grad;
      var r = Math.min(3, bw / 2);
      roundRect(ctx, x, y, bw, hgt, r);
      ctx.fill();
    }
    // 均值线
    var meanV = E.mean(sorted);
    var mx = padL + ((meanV - lo) / step) * gw;
    ctx.strokeStyle = 'rgba(37,99,235,.55)';
    ctx.setLineDash([3, 3]);
    ctx.beginPath(); ctx.moveTo(mx, 2); ctx.lineTo(mx, H - padB); ctx.stroke();
    ctx.setLineDash([]);

    ctx.fillStyle = '#9AA1AC';
    ctx.font = '9px -apple-system,sans-serif';
    ctx.textAlign = 'center';
    for (var l = 0; l < nb; l += 2) {
      ctx.fillText(String(lo + l * step), padL + l * gw + gw / 2, H - 5);
    }
    ctx.textAlign = 'right';
    ctx.fillStyle = '#A8AEB9';
    ctx.font = '700 9px -apple-system,sans-serif';
    ctx.fillText('均值 ' + meanV.toFixed(0), W - 2, 11);
  }

  function roundRect(ctx, x, y, w, h, r) {
    if (h <= 0) return;
    r = Math.min(r, h);
    ctx.beginPath();
    ctx.moveTo(x + r, y);
    ctx.arcTo(x + w, y, x + w, y + h, r);
    ctx.arcTo(x + w, y + h, x, y + h, 0);
    ctx.arcTo(x, y + h, x, y, 0);
    ctx.arcTo(x, y, x + w, y, r);
    ctx.closePath();
  }

  function showNumberDetail(num, st) {
    var p = E.numberProfile(DRAW, st, num);
    // 共现 top5
    var mx = E.coMatrix(DRAW, yearCount());
    var co = [];
    for (var v = 1; v <= FM; v++) {
      if (v === num || !mx.m[num][v]) continue;
      co.push([v, mx.m[num][v] / Math.max(1, Math.min(mx.cnt[num], mx.cnt[v]))]);
    }
    co.sort(function (a, b) { return b[1] - a[1]; });
    co = co.slice(0, 5);

    var h = '<h3>前区 ' + pad2(num) + '</h3>'
      + '<p class="sh-sub">近一年 ' + st.n + ' 期，累计全量 ' + DRAW.length + ' 期</p>'
      + '<div class="kv-list">'
      + '<div class="kv"><span>窗口内出现次数</span><b>' + st.freqF[num] + ' / ' + st.n + ' 次</b></div>'
      + '<div class="kv"><span>历史总出现次数</span><b>' + p.total + ' 次（' + (p.rate * 100).toFixed(2) + '%）</b></div>'
      + '<div class="kv"><span>当前遗漏</span><b style="color:#DE332B">' + st.omitF[num] + ' 期</b></div>'
      + '<div class="kv"><span>历史最大遗漏</span><b>' + st.maxOmitF[num] + ' 期</b></div>'
      + '<div class="kv"><span>平均间隔</span><b>' + st.avgGapF[num].toFixed(1) + ' 期</b></div>'
      + '<div class="kv"><span>理论出现概率</span><b>14.29%</b></div>'
      + '</div>';

    if (co.length) {
      h += '<div style="margin-top:14px"><div class="sec-title">最常同现</div><div class="co-list">';
      for (var i = 0; i < co.length; i++) {
        h += '<span class="tag">' + pad2(co[i][0]) + ' · ' + (co[i][1] * 100).toFixed(0) + '%</span>';
      }
      h += '</div></div>';
    }
    if (p.recent.length) {
      h += '<div style="margin-top:14px"><div class="sec-title">最近出现</div><div class="co-list">';
      for (var j = 0; j < p.recent.length; j++) h += '<span class="tag">' + p.recent[j] + ' 期</span>';
      h += '</div></div>';
    }
    h += '<p class="disclaimer">以上均为历史统计，遗漏长短不会改变下一期的理论概率。</p>';
    openSheet(h);
  }

  /* ================= 历史 ================= */
  function renderHistory(reset) {
    if (reset) { S.histLimit = 40; }
    var q = S.histQuery.trim();
    var list = DRAW.slice().reverse();
    if (q) {
      var nums = q.split(/[\s,，]+/).filter(function (x) { return x; });
      list = list.filter(function (d) {
        if (d.i.indexOf(q) >= 0) return true;
        for (var i = 0; i < nums.length; i++) {
          var v = parseInt(nums[i], 10);
          if (!isNaN(v) && (d.f.indexOf(v) >= 0 || d.b.indexOf(v) >= 0)) return true;
        }
        return false;
      });
    }
    $('histCount').textContent = (q ? '匹配 ' : '共 ') + list.length + ' 期';
    var show = list.slice(0, S.histLimit);
    var h = '';
    for (var i = 0; i < show.length; i++) {
      var d = show[i];
      h += '<div class="hist-it" data-issue="' + d.i + '"><div class="l"><b>' + d.i + '</b><span>' + d.d + '</span></div>'
        + '<div class="bl">' + ballsHTML(d.f, d.b, 'tiny') + '</div></div>';
    }
    $('histList').innerHTML = h || '<p class="hint">没有匹配的记录</p>';
    $('btnMore').style.display = list.length > S.histLimit ? 'block' : 'none';
    $('histList').querySelectorAll('.hist-it').forEach(function (it) {
      it.onclick = function () { showDrawDetail(it.getAttribute('data-issue')); };
    });
  }

  function showDrawDetail(issue) {
    var d = null;
    for (var i = 0; i < DRAW.length; i++) if (DRAW[i].i === issue) { d = DRAW[i]; break; }
    if (!d) return;
    var ft = E.drawFeatures(d);
    var zones = ft.zones.join(' / ');
    var h = '<h3>' + d.i + ' 期</h3><p class="sh-sub">' + d.d + ' ' + weekday(d.d) + '</p>'
      + '<div class="balls-row" style="margin-bottom:14px">' + ballsHTML(d.f, d.b) + '</div>'
      + '<div class="kv-list">'
      + '<div class="kv"><span>前区和值</span><b>' + ft.sumF + '</b></div>'
      + '<div class="kv"><span>跨度</span><b>' + ft.span + '</b></div>'
      + '<div class="kv"><span>奇偶比</span><b>' + ft.oddF + ':' + (5 - ft.oddF) + '</b></div>'
      + '<div class="kv"><span>大小比</span><b>' + ft.bigF + ':' + (5 - ft.bigF) + '</b></div>'
      + '<div class="kv"><span>五分区</span><b>' + zones + '</b></div>'
      + '<div class="kv"><span>连号对数</span><b>' + ft.consecPairs + '</b></div>'
      + '<div class="kv"><span>AC 值</span><b>' + ft.ac + '</b></div>'
      + '<div class="kv"><span>后区和值 / 奇偶</span><b>' + ft.sumB + ' / ' + ft.oddB + ':' + (2 - ft.oddB) + '</b></div>'
      + '</div>';
    openSheet(h);
  }

  /* ================= 弹层 & 录入 ================= */
  function notifyHost(open) {
    try {
      if (window.AndroidHost && window.AndroidHost.setSheet) window.AndroidHost.setSheet(open);
    } catch (e) { /* 浏览器预览环境无宿主 */ }
  }

  function openSheet(html) {
    $('sheetBody').innerHTML = html;
    $('sheet').classList.add('on');
    notifyHost(true);
  }
  function closeSheet() {
    $('sheet').classList.remove('on');
    notifyHost(false);
  }
  window.DLT_closeSheet = closeSheet;
  window.DLT_toast = toast;

  function openAddSheet() {
    var L = last();
    var nx = nextIssueInfo();
    var h = '<h3>录入开奖</h3><p class="sh-sub">一般不用手动填 —— 联网同步会自动补上。开奖当晚查不到时再来这里补录</p>'
      + '<div class="form-row"><label>期号</label><input id="fIssue" inputmode="numeric" placeholder="如 ' + nx.issue + '" value="' + nx.issue + '"></div>'
      + '<div class="form-row"><label>日期</label><input id="fDate" placeholder="2026-10-31" value="' + nx.date + '"></div>'
      + '<div class="form-row"><label>前区</label><input id="fFront" inputmode="numeric" placeholder="5 个号码，空格分隔"></div>'
      + '<div class="form-row"><label>后区</label><input id="fBack" inputmode="numeric" placeholder="2 个号码，空格分隔"></div>'
      + '<button class="primary sm" id="btnSaveDraw">保存这一期</button>'
      + '<div class="sec-title" style="margin-top:16px">批量导入</div>'
      + '<div class="form-row"><textarea id="fBulk" placeholder="每行一期，格式不限，如：&#10;2026-10-31 03 11 22 28 33 05 09&#10;26125,2026-11-02,07 14 19 26 31,03 08"></textarea></div>'
      + '<button class="ghost-btn wide" id="btnBulk">解析并导入</button>'
      + '<p class="disclaimer">期号 5 位数字（如 26125），日期 YYYY-MM-DD 可省略。前区 5 个 1–35、后区 2 个 1–12。</p>';
    openSheet(h);

    $('btnSaveDraw').onclick = function () {
      var issue = $('fIssue').value.trim();
      var date = $('fDate').value.trim() || new Date().toISOString().slice(0, 10);
      var f = parseNums($('fFront').value);
      var b = parseNums($('fBack').value);
      var err = validate(issue, f, b);
      if (err) { toast(err); return; }
      saveDraws([{ i: issue, f: f.sort(function (x, y) { return x - y; }), b: b.sort(function (x, y) { return x - y; }), d: date }]);
      closeSheet(); toast('已保存 ' + issue + ' 期');
    };

    $('btnBulk').onclick = function () {
      var lines = $('fBulk').value.split(/\n+/);
      var ok = 0, bad = 0, arr = [];
      for (var i = 0; i < lines.length; i++) {
        var line = lines[i].trim();
        if (!line) continue;
        var parsed = parseLine(line);
        if (!parsed) { bad++; continue; }
        arr.push(parsed); ok++;
      }
      if (!ok) { toast('没有识别到有效数据'); return; }
      saveDraws(arr);
      closeSheet();
      toast('导入 ' + ok + ' 期' + (bad ? '，忽略 ' + bad + ' 行' : ''));
    };
  }

  function parseNums(s) {
    return (s.match(/\d+/g) || []).map(Number);
  }

  function validate(issue, f, b) {
    if (!/^\d{5}$/.test(issue)) return '期号应为 5 位数字';
    if (f.length !== 5) return '前区需要 5 个号码';
    if (b.length !== 2) return '后区需要 2 个号码';
    var s = {}, i;
    for (i = 0; i < 5; i++) {
      if (f[i] < 1 || f[i] > 35) return '前区号码需在 1–35 之间';
      if (s['f' + f[i]]) return '前区号码不能重复';
      s['f' + f[i]] = 1;
    }
    for (i = 0; i < 2; i++) {
      if (b[i] < 1 || b[i] > 12) return '后区号码需在 1–12 之间';
      if (s['b' + b[i]]) return '后区号码不能重复';
      s['b' + b[i]] = 1;
    }
    return null;
  }

  function parseLine(line) {
    var issue = null, date = '';
    var dm = line.match(/(\d{4})-(\d{1,2})-(\d{1,2})/);
    if (dm) {
      date = dm[1] + '-' + pad2(+dm[2]) + '-' + pad2(+dm[3]);
      line = line.replace(dm[0], ' ');
    }
    var im = line.match(/(?:^|[^\d])(\d{5})(?:[^\d]|$)/);
    var nums;
    if (im) {
      issue = im[1];
      nums = parseNums(line.replace(issue, ' '));
    } else {
      nums = parseNums(line);
    }
    if (nums.length < 7) return null;
    var f = nums.slice(0, 5), b = nums.slice(5, 7);
    if (!issue) {
      var L = last();
      issue = L ? (L.i.slice(0, 2) + ('00' + (parseInt(L.i.slice(2), 10) + 1)).slice(-3)) : '26001';
    }
    if (validate(issue, f, b)) return null;
    return {
      i: issue,
      f: f.sort(function (x, y) { return x - y; }),
      b: b.sort(function (x, y) { return x - y; }),
      d: date || new Date().toISOString().slice(0, 10)
    };
  }

  /**
   * 把新期次写进本机存储（基线 data.js 是只读的，增量单独存一层）
   * keepOut=true 时保留当前推演结果，便于联网同步后原地刷新而不闪屏
   */
  function saveDraws(arr, keepOut) {
    var custom = Store.get('dlt.custom', []);
    var map = {};
    for (var i = 0; i < custom.length; i++) map[custom[i].i] = i;
    for (var j = 0; j < arr.length; j++) {
      if (map[arr[j].i] !== undefined) custom[map[arr[j].i]] = arr[j];
      else custom.push(arr[j]);
    }
    Store.set('dlt.custom', custom);
    // 重建数据
    DRAW = BASE.slice();
    loadCustom();
    yearN = 0;                       // 数据变了，窗口期数要重算
    if (!CK.picked && DRAW.length) CK.issue = DRAW[DRAW.length - 1].i;
    renderHeader();
    renderHistory(true);
    if (!keepOut) {
      if (S.result) S.result = null;
      $('predOut').innerHTML = '';
    }
  }

  /* ================= 联网同步 ================= */

  var OFFICIAL_URL = 'https://webapi.sporttery.cn/gateway/lottery/getHistoryPageListV1.qry'
    + '?gameNo=85&provinceId=0&pageSize=30&isVerify=1&pageNo=1';
  var OFFICIAL_HDR = 'Referer: https://static.sporttery.cn/\nAccept: application/json, text/plain, */*';
  var BACKUP_HDR = 'Referer: https://datachart.500.com/dlt/history/history.shtml';

  /** 调用宿主（Android）方法；浏览器预览环境下静默返回 undefined */
  function host(name) {
    var a = Array.prototype.slice.call(arguments, 1);
    try {
      if (window.AndroidHost && typeof window.AndroidHost[name] === 'function') {
        return window.AndroidHost[name].apply(window.AndroidHost, a);
      }
    } catch (e) { /* 宿主不可用 */ }
    return undefined;
  }

  /**
   * 原生层代理的 HTTP GET。
   * WebView 里直接 fetch 会被跨域拦掉，所以请求交给 Java 发起，
   * 结果通过 DLT_NET.onData(token, code, body) 异步回来。
   */
  var NET = (function () {
    var seq = 0, waiting = {}, alive = false;

    window.DLT_NET = {
      onData: function (token, code, body) {
        var cb = waiting[token];
        if (!cb) return;
        delete waiting[token];
        if (code >= 200 && code < 300 && body) cb(null, body);
        else cb(new Error(code < 0 ? (body || '连不上服务器') : ('HTTP ' + code)));
      }
    };

    function ready() {
      return !!(window.AndroidHost && typeof window.AndroidHost.get === 'function');
    }

    function get(url, headers, cb) {
      if (!ready()) { cb(new Error('当前环境不支持联网')); return; }
      alive = true;
      var token = 'n' + (++seq) + '_' + Date.now();
      var done = false, timer;

      function finish(err, body) {
        if (done) return;
        done = true;
        clearTimeout(timer);
        delete waiting[token];
        cb(err, body);
      }

      waiting[token] = finish;
      timer = setTimeout(function () { finish(new Error('请求超时')); }, 14000);
      try {
        window.AndroidHost.get(url, token, headers || '');
      } catch (e) {
        finish(e);
      }
    }

    return { get: get, ready: ready, ran: function () { return alive; } };
  })();

  /** 体彩官网 JSON */
  function parseOfficial(body) {
    var j = JSON.parse(body);
    var list = (j && j.value && j.value.list) || [];
    var out = [];
    for (var i = 0; i < list.length; i++) {
      var r = list[i] || {};
      var nums = String(r.lotteryDrawResult || '').match(/\d+/g) || [];
      if (nums.length < 7) continue;
      out.push({
        i: String(r.lotteryDrawNum || ''),
        f: nums.slice(0, 5).map(Number),
        b: nums.slice(5, 7).map(Number),
        d: String(r.lotteryDrawTime || '').slice(0, 10),
        pz: parsePrize(r.prizeLevelList)
      });
    }
    return out;
  }

  /** 官方奖级金额 -> { b:[各奖级基本单注], x:[一/二等奖追加单注] } */
  function parsePrize(levels) {
    if (!levels || !levels.length) return null;
    var pz = { b: [], x: [] };
    for (var i = 0; i < levels.length; i++) {
      var name = String(levels[i].prizeLevel || '');
      var amt = parseInt(String(levels[i].stakeAmount || '').replace(/[^\d]/g, ''), 10);
      if (isNaN(amt)) continue;
      if (name.indexOf('追加') >= 0) {
        if (pz.b.length) pz.x[pz.b.length - 1] = amt;
      } else {
        pz.b.push(amt);
      }
    }
    return pz.b.length ? pz : null;
  }

  /** 500 彩票网历史表格（备用源） */
  function parse500(body) {
    var rows = String(body).match(/<tr class="t_tr1">[\s\S]*?<\/tr>/g) || [];
    var out = [];
    for (var i = 0; i < rows.length; i++) {
      var cells = rows[i].match(/<td[^>]*>[\s\S]*?<\/td>/g) || [];
      var txt = cells.map(function (c) { return c.replace(/<[^>]+>/g, '').trim(); });
      var issue = null, pos = -1, k;
      for (k = 0; k < txt.length; k++) {
        if (/^\d{5}$/.test(txt[k])) { issue = txt[k]; pos = k; break; }
      }
      if (!issue) continue;
      var nums = [];
      for (k = pos + 1; k < txt.length && nums.length < 7; k++) {
        if (/^\d{1,2}$/.test(txt[k])) nums.push(+txt[k]);
        else if (nums.length) break;
      }
      if (nums.length < 7) continue;
      var dm = rows[i].match(/(\d{4}-\d{2}-\d{2})/);
      out.push({ i: issue, f: nums.slice(0, 5), b: nums.slice(5, 7), d: dm ? dm[1] : '' });
    }
    return out;
  }

  function numAsc(a, b) { return a - b; }

  function validDraw(d) {
    if (!d || !/^\d{5}$/.test(d.i || '')) return false;
    var f = d.f || [], b = d.b || [];
    if (f.length !== 5 || b.length !== 2) return false;
    for (var i = 0; i < 5; i++) if (!(f[i] >= 1 && f[i] <= 35)) return false;
    for (var j = 0; j < 2; j++) if (!(b[j] >= 1 && b[j] <= 12)) return false;
    return !validate(d.i, f, b);
  }

  /** 合并网络取回的期次，返回新增条数 */
  function mergeFetched(list) {
    var have = {}, i;
    for (i = 0; i < DRAW.length; i++) have[DRAW[i].i] = 1;
    var fresh = [];
    for (i = 0; i < list.length; i++) {
      var d = list[i];
      if (!d || have[d.i] || !validDraw(d)) continue;
      have[d.i] = 1;
      d.f = d.f.slice().sort(numAsc);
      d.b = d.b.slice().sort(numAsc);
      if (!d.d) d.d = '';
      d.s = 1;                       // 标记：联网同步而来
      fresh.push(d);
    }
    if (!fresh.length) return 0;
    var hadResult = !!S.result;
    saveDraws(fresh, true);
    markSeen();
    if (hadResult) runPredict();
    return fresh.length;
  }

  function markSeen() {
    var L = last();
    if (L) host('markSeen', L.i);
  }

  var SYNCING = false;

  /** 拉取最新开奖：先官方接口，失败自动回退 500 彩票网 */
  function syncDraws(opts) {
    opts = opts || {};
    if (SYNCING) return;
    if (!NET.ready()) {
      renderSyncBar('nohost');
      if (opts.manual) toast('预览环境不支持联网，请在 App 内使用');
      return;
    }
    SYNCING = true;
    renderSyncBar('loading');

    var L = last();
    var start = L ? L.i : '';
    var sources = [
      { url: OFFICIAL_URL, hdr: OFFICIAL_HDR, parse: parseOfficial },
      {
        url: 'https://datachart.500.com/dlt/history/newinc/history.php?start=' + start + '&end=99999',
        hdr: BACKUP_HDR, parse: parse500
      }
    ];
    var idx = 0, lastErr = '';

    function step() {
      if (idx >= sources.length) {
        SYNCING = false;
        renderSyncBar('fail');
        if (opts.manual) toast(lastErr ? '更新失败：' + lastErr : '更新失败，请检查网络');
        return;
      }
      var src = sources[idx++];
      NET.get(src.url, src.hdr, function (err, body) {
        if (err) { lastErr = err.message; step(); return; }
        var list;
        try { list = src.parse(body); } catch (e) { lastErr = '数据格式异常'; step(); return; }
        if (!list || !list.length) { lastErr = '未识别到开奖数据'; step(); return; }

        SYNCING = false;
        Store.set('dlt.syncAt', Date.now());
        var added = mergeFetched(list);
        renderSyncBar(added ? 'added' : 'ok');
        if (added) {
          toast('已同步 ' + added + ' 期新开奖');
          setTimeout(function () { try { renderSyncBar(); } catch (e) { } }, 4000);   // 4 秒后回到常规文案
        } else if (opts.manual) {
          toast('已是最新，暂无新开奖');
        }
        if (S.tab === 'analysis') renderAnalysis();
        if (S.tab === 'check') { renderCheck(); renderTickets(); }
      });
    }
    step();
  }

  function agoText(ts) {
    var m = Math.round((Date.now() - ts) / 60000);
    if (m < 1) return '刚刚';
    if (m < 60) return m + ' 分钟前';
    var h = Math.round(m / 60);
    if (h < 24) return h + ' 小时前';
    return Math.round(h / 24) + ' 天前';
  }

  /** 距上期开奖过了几天（用来判断数据是否可能过期） */
  function drawGap() {
    var L = last();
    if (!L || !L.d) return 0;
    var t = new Date(L.d.replace(/-/g, '/') + ' 00:00:00');
    if (isNaN(t.getTime())) return 0;
    var n = new Date();
    var today = new Date(n.getFullYear(), n.getMonth(), n.getDate());
    return Math.round((today - t) / 86400000);
  }

  function renderSyncBar(state) {
    var bar = $('syncBar');
    if (!bar) return;
    var L = last();
    var dot = $('syncDot'), txt = $('syncText');
    var btn = $('btnSync'), btnTxt = $('syncBtnTxt');
    if (!dot || !txt) return;

    var cls = 'dot', msg;
    if (btn) btn.disabled = false;
    if (btnTxt) btnTxt.textContent = '同步';

    if (state === 'loading') {
      cls += ' busy';
      msg = '正在获取最新开奖…';
      if (btn) btn.disabled = true;
      if (btnTxt) btnTxt.textContent = '同步中';
    } else if (state === 'fail') {
      cls += ' bad';
      msg = '更新失败 · 点右上角重试';
    } else if (state === 'nohost') {
      msg = '数据截至 ' + (L ? L.i : '—') + ' 期 · 预览环境不支持联网';
    } else {
      var gap = drawGap();
      var syncAt = Store.get('dlt.syncAt', 0);
      cls += gap <= 1 ? ' ok' : (gap <= 4 ? ' warn' : ' bad');
      msg = '更新至 ' + (L ? L.i : '—') + ' 期 · ' + (gap <= 0 ? '今天开奖' : gap + ' 天前开奖');
      if (state === 'added') msg = '已更新至 ' + (L ? L.i : '—') + ' 期';
      if (!syncAt) msg += ' · 尚未同步';
    }
    dot.className = cls;
    txt.textContent = msg;
  }

  /* ---------------- 数据与提醒 ---------------- */

  function openDataSheet() {
    var pf = {};
    try {
      var r = host('prefs');
      pf = (typeof r === 'string' && r) ? JSON.parse(r) : {};
    } catch (e) { pf = {}; }
    pf = pf || {};

    var hasHost = !!(window.AndroidHost && typeof window.AndroidHost.setReminder === 'function');
    var on = hasHost ? !!pf.reminder : false;
    var syncAt = Store.get('dlt.syncAt', 0);
    var synced = 0;
    var custom = Store.get('dlt.custom', []);
    for (var i = 0; i < custom.length; i++) if (custom[i].s) synced++;

    var h = '<h3>数据与提醒</h3>'
      + '<p class="sh-sub">大乐透每周一 / 三 / 六 21:25 开奖，官方结果一般在几分钟内上线</p>'
      + '<div class="set-row"><div class="sr-t"><b>每日自动查号</b>'
      + '<i>开奖夜 21:45 查询，22:20 补查，有新号码即推送</i></div>'
      + '<button class="sw' + (on ? ' on' : '') + (hasHost ? '' : ' lock') + '" id="swRemind"'
      + ' role="switch" aria-checked="' + (on ? 'true' : 'false') + '"><span></span></button></div>'
      + (on && pf.canNotify === false
        ? '<p class="warn-line">系统通知权限未开启，提醒可能不会显示</p>' : '')
      + '<div class="set-row"><div class="sr-t"><b>打开应用自动更新</b>'
      + '<i>每次启动静默同步一次，也可以点顶部「同步」手动更新</i></div>'
      + '<span class="sw-tag">常开</span></div>'
      + '<button class="primary sm" id="btnSyncNow">立即检查更新</button>'
      + '<button class="ghost-btn wide" style="margin-top:8px" id="btnManualAdd">手动录入开奖</button>'
      + '<div class="sec-title" style="margin-top:18px">数据情况</div>'
      + '<div class="kv"><span>已内置</span><b>' + BASE.length + ' 期</b></div>'
      + '<div class="kv"><span>联网补录</span><b>' + synced + ' 期</b></div>'
      + '<div class="kv"><span>上次同步</span><b>' + (syncAt ? agoText(syncAt) : '从未') + '</b></div>'
      + '<p class="disclaimer">开奖号码优先读取中国体彩网官方接口，失败时自动回退 500 彩票网。'
      + '只获取公开的开奖数据，不上传任何本机信息。</p>';
    openSheet(h);

    var sw = $('swRemind');
    if (sw) {
      sw.onclick = function () {
        if (!hasHost) { toast('预览环境无法设置提醒'); return; }
        var now = !sw.classList.contains('on');
        sw.classList.toggle('on', now);
        sw.setAttribute('aria-checked', now ? 'true' : 'false');
        host('setReminder', now);
        toast(now ? '已开启每日开奖提醒（21:45）' : '已关闭每日开奖提醒');
        if (now) {
          setTimeout(function () {
            var p2 = {};
            try { p2 = JSON.parse(host('prefs') || '{}') || {}; } catch (e) { p2 = {}; }
            if (p2.canNotify === false) toast('请在系统设置中允许通知，否则收不到提醒');
            else closeSheet();
          }, 700);
        }
      };
    }
    $('btnSyncNow').onclick = function () {
      closeSheet();
      syncDraws({ manual: true });
    };
    $('btnManualAdd').onclick = function () {
      closeSheet();
      setTimeout(openAddSheet, 180);
    };
  }

  function bindSyncBar() {
    var info = $('syncInfo'), btn = $('btnSync');
    if (info) info.onclick = openDataSheet;
    if (btn) {
      btn.onclick = function () {
        if (SYNCING) { toast('正在同步，稍等一下'); return; }
        syncDraws({ manual: true });
      };
    }
  }

  /** 后台查号时抓到的数据，启动时静默并入 */
  function pullPending() {
    var raw = host('takePending');
    if (!raw || typeof raw !== 'string') return 0;
    var list = parseRaw(raw);
    return list.length ? mergeFetched(list) : 0;
  }

  /** 首次运行默认开启每日提醒 */
  function initRemind() {
    var pf = {};
    try { pf = JSON.parse(host('prefs') || '{}') || {}; } catch (e) { pf = {}; }
    if (!window.AndroidHost || typeof window.AndroidHost.setReminder !== 'function') return;
    if (pf.first) host('setReminder', true);
  }

  /* ================= 验票 ================= */

  var T = window.DLT_TICKET;
  var CK = { issue: '', type: 'single', extra: false, front: [], back: [], photo: '', result: null, picked: false };

  function closest(el, sel) {
    while (el && el.nodeType === 1) {
      if (el.matches && el.matches(sel)) return el;
      el = el.parentNode;
    }
    return null;
  }

  function drawByIssue(issue) {
    for (var i = DRAW.length - 1; i >= 0; i--) if (DRAW[i].i === issue) return DRAW[i];
    return null;
  }

  function prizeTable(issue) {
    var d = drawByIssue(issue);
    if (d && d.pz) return d.pz;
    var all = Store.get('dlt.prize', {});
    return all && all[issue] ? all[issue] : null;
  }

  function yuan(n) {
    var s = String(Math.round(n));
    var out = '';
    for (var i = 0; i < s.length; i++) {
      if (i && (s.length - i) % 3 === 0) out += ',';
      out += s.charAt(i);
    }
    return out;
  }

  function renderCheck() {
    var d = drawByIssue(CK.issue);
    $('ckIssue').textContent = CK.issue + ' 期';

    if (d) {
      $('ckDraw').innerHTML = ballsHTML(d.f, d.b, 'sm');
      var tbl = prizeTable(CK.issue);
      $('ckNote').textContent = d.d + ' ' + weekday(d.d) + ' 开奖'
        + (tbl ? ' · 奖金取自官方公告' : ' · 一等奖金额未同步');
    } else {
      $('ckDraw').innerHTML = '';
      $('ckNote').textContent = '该期还没有开奖数据，可能尚未开奖';
    }

    // 号码盘
    var capF = CK.type === 'single' ? 5 : 12;
    var capB = CK.type === 'single' ? 2 : 6;
    var fh = '', i;
    for (i = 1; i <= 35; i++) {
      fh += '<button class="pk-ball' + (CK.front.indexOf(i) >= 0 ? ' on' : '')
        + '" data-z="f" data-n="' + i + '">' + pad2(i) + '</button>';
    }
    $('ckFront').innerHTML = fh;
    var bh = '';
    for (i = 1; i <= 12; i++) {
      bh += '<button class="pk-ball' + (CK.back.indexOf(i) >= 0 ? ' on' : '')
        + '" data-z="b" data-n="' + i + '">' + pad2(i) + '</button>';
    }
    $('ckBack').innerHTML = bh;

    $('ckFHd').textContent = '已选 ' + CK.front.length + ' / ' + (CK.type === 'single' ? 5 : '5–12');
    $('ckBHd').textContent = '已选 ' + CK.back.length + ' / ' + (CK.type === 'single' ? 2 : '2–6');

    var bets = T.betCount(CK.front, CK.back);
    if (bets) {
      $('ckFoot').innerHTML = '共 <b>' + bets + '</b> 注 · 票面金额 <b>' + bets * (CK.extra ? 3 : 2)
        + '</b> 元　请与票面金额核对，一致说明录入无误';
    } else {
      $('ckFoot').innerHTML = CK.front.length < 5 ? '前区至少选 5 个' : '后区至少选 2 个';
    }

    // 分段控件状态
    var tb = $('ckTypeSeg').querySelectorAll('button');
    for (i = 0; i < tb.length; i++) tb[i].classList.toggle('on', tb[i].getAttribute('data-t') === CK.type);
    var eb = $('ckExtraSeg').querySelectorAll('button');
    for (i = 0; i < eb.length; i++) {
      eb[i].classList.toggle('on', (eb[i].getAttribute('data-e') === '1') === CK.extra);
    }
  }

  function ckToggle(zone, n) {
    var arr = zone === 'f' ? CK.front : CK.back;
    var cap = zone === 'f'
      ? (CK.type === 'single' ? 5 : 12)
      : (CK.type === 'single' ? 2 : 6);
    var at = arr.indexOf(n);
    if (at >= 0) {
      arr.splice(at, 1);
    } else {
      if (arr.length >= cap) {
        toast((zone === 'f' ? '前区' : '后区') + (CK.type === 'single'
          ? '只能选 ' + cap + ' 个，复式请切换投注方式'
          : '最多选 ' + cap + ' 个'));
        return;
      }
      arr.push(n);
      arr.sort(function (a, b) { return a - b; });
    }
    renderCheck();
  }

  function runCheck() {
    var d = drawByIssue(CK.issue);
    if (!d) { toast('该期还没有开奖数据'); return; }
    if (CK.front.length < 5) { toast('前区至少选 5 个号码'); return; }
    if (CK.back.length < 2) { toast('后区至少选 2 个号码'); return; }
    if (!T.isValid(CK.front, CK.back)) {
      toast('号码有重复或超出范围，最多 ' + T.MAX_BETS + ' 注');
      return;
    }
    var res = T.check(
      { issue: CK.issue, front: CK.front, back: CK.back, extra: CK.extra },
      d, prizeTable(CK.issue)
    );
    CK.result = res;
    renderCkResult(res, d);
    try { $('ckResult').scrollIntoView({ behavior: 'smooth', block: 'center' }); } catch (e) { }
  }

  function cmpRow(label, front, back, fHit, bHit, dimMiss) {
    var h = '<div class="hr"><em>' + label + '</em><div class="balls-row">';
    var i, on;
    for (i = 0; i < front.length; i++) {
      on = fHit.indexOf(front[i]) >= 0;
      h += '<span class="ball f' + (dimMiss && !on ? ' off' : '') + '">' + pad2(front[i]) + '</span>';
    }
    h += '<span class="sep"></span>';
    for (i = 0; i < back.length; i++) {
      on = bHit.indexOf(back[i]) >= 0;
      h += '<span class="ball b' + (dimMiss && !on ? ' off' : '') + '">' + pad2(back[i]) + '</span>';
    }
    return h + '</div></div>';
  }

  function renderCkResult(res, d) {
    var box = $('ckResult');
    var win = res.bestLevel > 0;
    var h = '<div class="ck-res ' + (win ? 'win' : 'lose') + '">';
    var i;

    if (win) {
      var top = res.levels[0];
      h += '<div class="tagline">中了 ' + top.name + '</div>';
      h += '<div class="big">' + (res.live ? '浮动奖' : yuan(res.total) + ' 元') + '</div>';
      h += '<div class="sub">命中 ' + res.bestHF + '+' + res.bestHB + ' · ' + res.bets + ' 注'
        + (res.extra ? ' · 已追加' : '') + '</div>';
    } else {
      h += '<div class="tagline">本期未中奖</div>';
      h += '<div class="big">' + res.bestHF + '+' + res.bestHB + '</div>';
      h += '<div class="sub">前区中 ' + res.bestHF + ' 个 · 后区中 ' + res.bestHB
        + ' 个 · 共 ' + res.bets + ' 注</div>';
    }

    h += '<div class="ck-hit">'
      + cmpRow('开奖', d.f, d.b, d.f, d.b, false)
      + cmpRow('我的', res.front, res.back, d.f, d.b, true)
      + '</div>';

    if (win) {
      h += '<div class="ck-levels">';
      for (i = 0; i < res.levels.length; i++) {
        var L = res.levels[i];
        h += '<div class="ck-lv"><span>' + L.name + '<i>' + L.count + ' 注'
          + (L.extra ? ' · 含追加' : '') + '</i></span>'
          + '<b>' + (L.live ? '浮动' : yuan(L.sum) + ' 元') + '</b></div>';
      }
      h += '<div class="ck-total"><span>合计奖金</span><b>'
        + (res.live ? '含浮动奖' : yuan(res.total) + ' 元') + '</b></div>';
      if (res.live) {
        h += '<p class="ck-note" style="margin-top:9px">浮动奖按当期官方公告确定，联网同步后会自动显示具体金额。</p>';
      }
      h += '</div>';
    }

    h += '</div>';
    h += '<button class="ghost-btn wide" style="margin-top:10px" id="btnSaveTicket">保存到我的彩票</button>';
    box.innerHTML = h;

    var sv = $('btnSaveTicket');
    if (sv) sv.onclick = function () { saveTicket(res); };
  }

  function saveTicket(res) {
    var list = Store.get('dlt.tickets', []);
    var key = res.issue + '|' + res.front.join(',') + '|' + res.back.join(',') + '|' + (res.extra ? 1 : 0);
    for (var i = 0; i < list.length; i++) {
      var k = list[i].i + '|' + list[i].f.join(',') + '|' + list[i].b.join(',') + '|' + (list[i].x ? 1 : 0);
      if (k === key) { toast('这张票已经保存过了'); return; }
    }
    list.unshift({
      i: res.issue, f: res.front.slice(), b: res.back.slice(),
      x: res.extra ? 1 : 0, ts: Date.now()
    });
    list = list.slice(0, 50);
    Store.set('dlt.tickets', list);
    renderTickets();
    toast('已保存，共 ' + list.length + ' 张');
  }

  function renderTickets() {
    var list = Store.get('dlt.tickets', []);
    var box = $('ckSaved');
    if (!list || !list.length) {
      box.innerHTML = '<p class="tk-empty">还没有保存的票，核对后点「保存到我的彩票」</p>';
      return;
    }
    var h = '<div class="card" style="padding:4px 14px">';
    for (var i = 0; i < list.length; i++) {
      var t = list[i];
      var d = drawByIssue(t.i);
      var lv = 0;
      if (d) {
        var r = T.check({ issue: t.i, front: t.f, back: t.b, extra: !!t.x }, d, prizeTable(t.i));
        lv = r.bestLevel;
      }
      var pill = '待开奖';
      if (d) pill = lv ? ('中 ' + T.rulesOf(t.i)[lv - 1].name) : '未中奖';
      h += '<div class="tk" data-idx="' + i + '">'
        + '<div class="tk-main"><div class="tk-top"><b>' + t.i + ' 期</b>'
        + (t.x ? '<span class="tk-pill">追加</span>' : '')
        + '<span class="tk-pill' + (lv ? ' win' : '') + '">' + pill + '</span>'
        + '</div><div class="tk-balls">' + ballsHTML(t.f, t.b, 'tiny') + '</div></div>'
        + '<button class="tk-del" data-del="' + i + '" aria-label="删除">×</button></div>';
    }
    box.innerHTML = h + '</div>';

    box.onclick = function (e) {
      var del = closest(e.target, '[data-del]');
      if (del) {
        var arr = Store.get('dlt.tickets', []);
        arr.splice(+del.getAttribute('data-del'), 1);
        Store.set('dlt.tickets', arr);
        renderTickets();
        toast('已删除');
        return;
      }
      var row = closest(e.target, '[data-idx]');
      if (row) {
        var all = Store.get('dlt.tickets', []);
        var t2 = all[+row.getAttribute('data-idx')];
        if (!t2) return;
        CK.issue = t2.i;
        CK.picked = true;
        CK.front = t2.f.slice();
        CK.back = t2.b.slice();
        CK.extra = !!t2.x;
        CK.type = (t2.f.length > 5 || t2.b.length > 2) ? 'multi' : 'single';
        CK.result = null;
        $('ckResult').innerHTML = '';
        renderCheck();
        $('main').scrollTop = 0;
        toast('已载入这张票');
      }
    };
  }

  function openIssuePicker() {
    var h = '<h3>选择期号</h3><p class="sh-sub">选你要核对的那一期</p><div class="iss-list">';
    var end = Math.max(0, DRAW.length - 40);
    for (var i = DRAW.length - 1; i >= end; i--) {
      var d = DRAW[i];
      h += '<button class="iss-item' + (d.i === CK.issue ? ' on' : '') + '" data-i="' + d.i + '">'
        + '<b>' + d.i + '</b><em>' + d.f.map(pad2).join(' ') + ' + ' + d.b.map(pad2).join(' ') + '</em>'
        + '<span>' + d.d + '</span></button>';
    }
    h += '</div>';
    openSheet(h);

    $('sheetBody').onclick = function (e) {
      var b = closest(e.target, '[data-i]');
      if (!b) return;
      CK.issue = b.getAttribute('data-i');
      CK.picked = true;
      CK.result = null;
      $('ckResult').innerHTML = '';
      closeSheet();
      renderCheck();
    };
  }

  function bindCheck() {
    $('ckFront').onclick = function (e) {
      var b = closest(e.target, '[data-n]');
      if (b) ckToggle('f', +b.getAttribute('data-n'));
    };
    $('ckBack').onclick = function (e) {
      var b = closest(e.target, '[data-n]');
      if (b) ckToggle('b', +b.getAttribute('data-n'));
    };
    $('ckTypeSeg').onclick = function (e) {
      var b = closest(e.target, '[data-t]');
      if (!b) return;
      CK.type = b.getAttribute('data-t');
      if (CK.type === 'single') {
        if (CK.front.length > 5) CK.front = CK.front.slice(0, 5);
        if (CK.back.length > 2) CK.back = CK.back.slice(0, 2);
      }
      renderCheck();
    };
    $('ckExtraSeg').onclick = function (e) {
      var b = closest(e.target, '[data-e]');
      if (!b) return;
      CK.extra = b.getAttribute('data-e') === '1';
      renderCheck();
    };
    $('btnCheck').onclick = runCheck;
    $('btnIssuePick').onclick = openIssuePicker;
    $('btnClear').onclick = function () {
      CK.front = [];
      CK.back = [];
      CK.result = null;
      $('ckResult').innerHTML = '';
      renderCheck();
    };
    $('btnPhoto').onclick = function () {
      if (!window.AndroidHost || typeof window.AndroidHost.capture !== 'function') {
        toast('预览环境没有相机，请在 App 内使用');
        return;
      }
      try { window.AndroidHost.capture(); } catch (e) { toast('无法调用相机'); }
    };
    $('btnPhotoDel').onclick = function () {
      CK.photo = '';
      $('photoBox').classList.remove('on');
      $('photoImg').removeAttribute('src');
    };
    $('btnPhotoZoom').onclick = function () {
      if (!CK.photo) return;
      openSheet('<h3>票面照片</h3><p class="sh-sub">上下滑动查看票面细节</p>'
        + '<img src="https://dlt.local/shot/' + CK.photo + '" style="width:100%;border-radius:12px;display:block">');
    };
  }

  /** Java 拍照完成后回调，把照片名交给页面 */
  window.DLT_photo = function (name) {
    if (!name) return;
    CK.photo = name;
    $('photoImg').src = 'https://dlt.local/shot/' + name;
    $('photoBox').classList.add('on');
    toast('照片已就位，对照着点号码即可');
  }


  function init() {
    if (!DRAW.length) {
      $('hdSub').textContent = '数据缺失';
      return;
    }
    renderHeader();
    renderStrategyChips();
    renderSegs();
    renderHistory(true);

    $('btnRun').onclick = runPredict;
    $('btnMore').onclick = function () { S.histLimit += 40; renderHistory(false); };

    bindCheck();
    if (last()) CK.issue = last().i;
    renderCheck();
    renderTickets();

    var qi = $('qInput');
    var qt = null;
    qi.oninput = function () {
      clearTimeout(qt);
      qt = setTimeout(function () { S.histQuery = qi.value; renderHistory(true); }, 200);
    };

    $('tabbar').querySelectorAll('button').forEach(function (b) {
      b.onclick = function () { goTab(b.getAttribute('data-tab')); };
    });
    $('sheet').querySelectorAll('[data-close]').forEach(function (m) { m.onclick = closeSheet; });

    // 联网同步
    bindSyncBar();
    initRemind();
    var got = pullPending();          // 后台查号抓到的，先并进来
    if (got) toast('已并入 ' + got + ' 期新开奖');
    setTimeout(function () { syncDraws({ silent: true }); }, 1200);

    // 首屏自动生成一次参考号码
    setTimeout(runPredict, 120);

    // 开发辅助：?tab=analysis 直接定位到某个页签
    var m = /[?&]tab=(\w+)/.exec(location.search);
    if (m && VIEWS[m[1]]) goTab(m[1]);
  }

  // 从后台切回前台时，若距上次同步超过 3 小时就再查一次
  window.DLT_resume = function () {
    if (SYNCING) return;
    var at = Store.get('dlt.syncAt', 0);
    if (Date.now() - at > 3 * 3600000) syncDraws({ silent: true });
  };

  if (document.readyState === 'loading') document.addEventListener('DOMContentLoaded', init);
  else init();
})();
