/* ============================================================
 * 大乐透分析引擎  DLT Engine
 * 纯函数式：输入历史开奖 -> 输出统计 / 选号 / 回测
 * 前区 1-35 取 5，后区 1-12 取 2
 * ============================================================ */
(function (root) {
  'use strict';

  var FM = 35, BM = 12;
  var FRONT_PICK = 5, BACK_PICK = 2;
  var P_FRONT = FRONT_PICK / FM;   // 单号每期出现概率
  var P_BACK = BACK_PICK / BM;
  var EXP_GAP_F = 1 / P_FRONT;     // 理论平均间隔 7
  var EXP_GAP_B = 1 / P_BACK;      // 理论平均间隔 6

  /* ---------------- 基础工具 ---------------- */
  function mulberry32(a) {
    return function () {
      a |= 0; a = a + 0x6D2B79F5 | 0;
      var t = Math.imul(a ^ a >>> 15, 1 | a);
      t = t + Math.imul(t ^ t >>> 7, 61 | t) ^ t;
      return ((t ^ t >>> 14) >>> 0) / 4294967296;
    };
  }
  function gauss(x, mu, sig) {
    if (!sig) sig = 1;
    var d = (x - mu) / sig;
    return Math.exp(-0.5 * d * d);
  }
  function sum(a) { var s = 0; for (var i = 0; i < a.length; i++) s += a[i]; return s; }
  function mean(a) { return a.length ? sum(a) / a.length : 0; }
  function std(a) {
    if (a.length < 2) return 0;
    var m = mean(a), s = 0;
    for (var i = 0; i < a.length; i++) s += (a[i] - m) * (a[i] - m);
    return Math.sqrt(s / (a.length - 1));
  }
  function quantile(sortedArr, q) {
    if (!sortedArr.length) return 0;
    var pos = (sortedArr.length - 1) * q, base = Math.floor(pos), rest = pos - base;
    if (sortedArr[base + 1] !== undefined) {
      return sortedArr[base] + rest * (sortedArr[base + 1] - sortedArr[base]);
    }
    return sortedArr[base];
  }
  function uniqSorted(arr) {
    var s = arr.slice().sort(function (a, b) { return a - b; });
    var out = [];
    for (var i = 0; i < s.length; i++) if (i === 0 || s[i] !== s[i - 1]) out.push(s[i]);
    return out;
  }

  /* ---------------- 单期特征 ---------------- */
  function drawFeatures(d) {
    var f = d.f.slice().sort(function (a, b) { return a - b; });
    var b = d.b.slice().sort(function (a, b) { return a - b; });
    var oddF = 0, bigF = 0, zones = [0, 0, 0, 0, 0], tails = [0,0,0,0,0,0,0,0,0,0];
    for (var i = 0; i < f.length; i++) {
      if (f[i] % 2 === 1) oddF++;
      if (f[i] >= 18) bigF++;
      zones[Math.min(4, Math.floor((f[i] - 1) / 7))]++;
      tails[f[i] % 10]++;
    }
    // 连号：连续数字的最长串
    var run = 1, maxRun = 1, consecPairs = 0;
    for (var j = 1; j < f.length; j++) {
      if (f[j] === f[j - 1] + 1) { run++; consecPairs++; if (run > maxRun) maxRun = run; }
      else run = 1;
    }
    // AC 值 = 差值种类数 - (个数-1)
    var diffs = {}, dc = 0;
    for (var p = 0; p < f.length; p++) for (var q = p + 1; q < f.length; q++) {
      var v = f[q] - f[p]; if (!diffs[v]) { diffs[v] = 1; dc++; }
    }
    var oddB = 0;
    for (var k = 0; k < b.length; k++) if (b[k] % 2 === 1) oddB++;
    return {
      sumF: sum(f), span: f[f.length - 1] - f[0],
      oddF: oddF, bigF: bigF, zones: zones, tails: tails,
      maxRun: maxRun, consecPairs: consecPairs,
      ac: dc - (f.length - 1),
      sumB: sum(b), oddB: oddB,
      bigB: b.filter(function (x) { return x >= 7; }).length
    };
  }

  /* ---------------- 统计汇总 ---------------- */
  // win: 统计窗口（期数），0 或 undefined = 全部
  function buildStats(draws, win) {
    var total = draws.length;
    var n = (!win || win >= total) ? total : win;
    var seg = draws.slice(total - n);
    var freqF = new Array(FM + 1).fill(0);
    var freqB = new Array(BM + 1).fill(0);
    var omitF = new Array(FM + 1).fill(n);   // 当前遗漏（n = 窗口内未出现）
    var omitB = new Array(BM + 1).fill(n);
    var maxOmitF = new Array(FM + 1).fill(0);
    var maxOmitB = new Array(BM + 1).fill(0);
    var lastF = new Array(FM + 1).fill(-1);
    var gapsF = [], i, v, k;

    var feats = [];
    for (i = 0; i < n; i++) {
      var d = seg[i];
      for (k = 0; k < d.f.length; k++) { freqF[d.f[k]]++; }
      for (k = 0; k < d.b.length; k++) { freqB[d.b[k]]++; }
      feats.push(drawFeatures(d));
    }

    // 当前遗漏 + 历史最大遗漏（从最近一期往前扫）
    for (i = n - 1; i >= 0; i--) {
      var dd = seg[i];
      var dist = n - 1 - i;   // 距今多少期
      for (k = 0; k < dd.f.length; k++) {
        v = dd.f[k];
        if (lastF[v] === -1) { lastF[v] = i; omitF[v] = dist; }
      }
      for (k = 0; k < dd.b.length; k++) {
        v = dd.b[k];
        if (omitB[v] === n) omitB[v] = dist;
      }
    }
    // 每个号码的历史最大遗漏
    for (v = 1; v <= FM; v++) {
      var best = 0, last = -1;
      for (i = 0; i < n; i++) {
        if (seg[i].f.indexOf(v) >= 0) { if (last >= 0) best = Math.max(best, i - last - 1); last = i; }
      }
      if (last >= 0) best = Math.max(best, n - 1 - last);
      maxOmitF[v] = best || (omitF[v] > 0 ? omitF[v] : 0);
    }
    for (v = 1; v <= BM; v++) {
      var bestB = 0, lastB = -1;
      for (i = 0; i < n; i++) {
        if (seg[i].b.indexOf(v) >= 0) { if (lastB >= 0) bestB = Math.max(bestB, i - lastB - 1); lastB = i; }
      }
      if (lastB >= 0) bestB = Math.max(bestB, n - 1 - lastB);
      maxOmitB[v] = bestB;
    }

    var sumsF = feats.map(function (x) { return x.sumF; }).sort(function (a, b) { return a - b; });
    var spans = feats.map(function (x) { return x.span; }).sort(function (a, b) { return a - b; });
    var oddDist = [0, 0, 0, 0, 0, 0];
    var bigDist = [0, 0, 0, 0, 0, 0];
    var zoneTotal = [0, 0, 0, 0, 0];
    var tailTotal = new Array(10).fill(0);
    var consecDist = [0, 0, 0, 0, 0];
    var backSumDist = {};
    for (i = 0; i < feats.length; i++) {
      oddDist[feats[i].oddF]++;
      bigDist[feats[i].bigF]++;
      for (k = 0; k < 5; k++) zoneTotal[k] += feats[i].zones[k];
      for (k = 0; k < 10; k++) tailTotal[k] += feats[i].tails[k];
      consecDist[Math.min(4, feats[i].consecPairs)]++;
      backSumDist[feats[i].sumB] = (backSumDist[feats[i].sumB] || 0) + 1;
    }

    var avgGapF = new Array(FM + 1).fill(0);
    for (v = 1; v <= FM; v++) {
      var cnt = freqF[v];
      avgGapF[v] = cnt > 0 ? n / cnt : n;
    }

    return {
      n: n, total: total,
      freqF: freqF, freqB: freqB,
      omitF: omitF, omitB: omitB,
      maxOmitF: maxOmitF, maxOmitB: maxOmitB,
      avgGapF: avgGapF,
      feats: feats,
      sumMean: mean(sumsF), sumStd: std(sumsF),
      sumQ10: quantile(sumsF, 0.10), sumQ90: quantile(sumsF, 0.90),
      sumQ25: quantile(sumsF, 0.25), sumQ75: quantile(sumsF, 0.75),
      sumMin: sumsF[0] || 0, sumMax: sumsF[sumsF.length - 1] || 0,
      spanMean: mean(spans), spanStd: std(spans),
      spanQ10: quantile(spans, 0.10), spanQ90: quantile(spans, 0.90),
      oddDist: oddDist, bigDist: bigDist,
      zoneTotal: zoneTotal, tailTotal: tailTotal, consecDist: consecDist,
      backSumDist: backSumDist,
      sumsSorted: sumsF
    };
  }

  /* ---------------- 共现矩阵（前区） ---------------- */
  function coMatrix(draws, win) {
    var n = (!win || win >= draws.length) ? draws.length : win;
    var seg = draws.slice(draws.length - n);
    var m = [];
    for (var i = 0; i <= FM; i++) m.push(new Array(FM + 1).fill(0));
    var cnt = new Array(FM + 1).fill(0);
    for (var t = 0; t < seg.length; t++) {
      var f = seg[t].f;
      for (var a = 0; a < f.length; a++) {
        cnt[f[a]]++;
        for (var b = a + 1; b < f.length; b++) { m[f[a]][f[b]]++; m[f[b]][f[a]]++; }
      }
    }
    return { m: m, cnt: cnt };
  }

  /* ---------------- 选号策略 ---------------- */
  var STRATEGIES = {
    balanced: { key: 'balanced', name: '均衡型', desc: '频率/遗漏/关联综合加权，特征最贴近历史分布',
      w: { freq: .30, omit: .25, co: .20, tail: .10, zone: .15 }, omitMode: 'gauss' },
    hot: { key: 'hot', name: '热号追踪', desc: '偏重近期高频号，追热不追冷',
      w: { freq: .55, omit: .08, co: .22, tail: .05, zone: .10 }, omitMode: 'gauss' },
    cold: { key: 'cold', name: '冷号回补', desc: '偏重长期未出与超长遗漏号',
      w: { freq: .12, omit: .50, co: .13, tail: .10, zone: .15 }, omitMode: 'inc' },
    omit: { key: 'omit', name: '遗漏回归', desc: '遗漏接近历史均值倍数的号码优先',
      w: { freq: .15, omit: .50, co: .10, tail: .10, zone: .15 }, omitMode: 'regress' },
    co: { key: 'co', name: '关联传递', desc: '偏重与上期号码历史共现率高的号',
      w: { freq: .15, omit: .15, co: .50, tail: .10, zone: .10 }, omitMode: 'gauss' },
    random: { key: 'random', name: '纯随机', desc: '等概率抽样，作为对照组基准',
      w: { freq: 0, omit: 0, co: 0, tail: 0, zone: 0 }, omitMode: 'gauss' }
  };

  /* 单位置评分：返回 [1..max] 的权重表 */
  function frontWeights(draws, stats, matrix, strat, lastDraw, rnd) {
    var cfg = STRATEGIES[strat] || STRATEGIES.balanced;
    var w = cfg.w;
    var out = new Array(FM + 1).fill(0);
    var maxFreq = 1;
    for (var i = 1; i <= FM; i++) if (stats.freqF[i] > maxFreq) maxFreq = stats.freqF[i];
    var lastFront = (lastDraw && lastDraw.f) ? lastDraw.f : [];

    for (var v = 1; v <= FM; v++) {
      var freqTerm = stats.freqF[v] / maxFreq;                       // 0..1
      var r = stats.omitF[v] / EXP_GAP_F;                            // 遗漏/理论间隔
      var omitTerm;
      if (cfg.omitMode === 'inc') omitTerm = 1 - Math.exp(-r / 1.6);          // 越冷越高
      else if (cfg.omitMode === 'regress') omitTerm = gauss(r, 1.6, 0.8);     // 围绕 1.6 倍
      else omitTerm = gauss(r, 1.0, 1.1);                                     // 围绕理论值
      // 共现：与上期号码的平均历史共现率（归一化）
      var coTerm = 0;
      if (lastFront.length) {
        var acc = 0, cntc = 0;
        for (var j = 0; j < lastFront.length; j++) {
          var o = lastFront[j];
          if (o === v) continue;
          var denom = Math.min(matrix.cnt[v], matrix.cnt[o]) || 1;
          acc += matrix.m[v][o] / denom;
          cntc++;
        }
        coTerm = cntc ? acc / cntc : 0;
      }
      // 尾数冷热（尾数近期偏少则略加分）
      var tail = v % 10;
      var tailTerm = stats.tailTotal ? 1 - (stats.tailTotal[tail] / (stats.n * FRONT_PICK / 10 + 1e-9)) : 0;
      tailTerm = Math.max(0, Math.min(1, tailTerm * 0.5 + 0.5));
      // 分区均衡：本区出现偏少则略加分
      var z = Math.min(4, Math.floor((v - 1) / 7));
      var zoneTerm = stats.zoneTotal ? 1 - (stats.zoneTotal[z] / (stats.n * FRONT_PICK / 5 + 1e-9)) : 0;
      zoneTerm = Math.max(0, Math.min(1, zoneTerm * 0.5 + 0.5));

      var raw = w.freq * freqTerm + w.omit * omitTerm + w.co * coTerm
              + w.tail * tailTerm + w.zone * zoneTerm;
      // 基础权重保证任何号码都有机会（防止策略把号码权重压成 0）
      out[v] = 0.06 + raw;
      if (strat === 'random') out[v] = 1;
    }
    // 冷/热策略下，权重差距放大，便于区分
    return out;
  }

  function backWeights(draws, stats, strat, lastDraw) {
    var cfg = STRATEGIES[strat] || STRATEGIES.balanced;
    var out = new Array(BM + 1).fill(0);
    var maxFreq = 1;
    for (var i = 1; i <= BM; i++) if (stats.freqB[i] > maxFreq) maxFreq = stats.freqB[i];
    for (var v = 1; v <= BM; v++) {
      var freqTerm = stats.freqB[v] / maxFreq;
      var r = stats.omitB[v] / EXP_GAP_B;
      var omitTerm;
      if (cfg.omitMode === 'inc') omitTerm = 1 - Math.exp(-r / 1.6);
      else if (cfg.omitMode === 'regress') omitTerm = gauss(r, 1.6, 0.9);
      else omitTerm = gauss(r, 1.0, 1.2);
      if (strat === 'random') { out[v] = 1; continue; }
      out[v] = 0.08 + (cfg.w.freq || .3) * freqTerm + (cfg.w.omit || .25) * omitTerm;
    }
    return out;
  }

  /* 按权重不放回抽样 */
  function sampleWeighted(weights, k, maxIdx, rnd) {
    var pool = [], wts = [], i;
    for (i = 1; i <= maxIdx; i++) { pool.push(i); wts.push(weights[i]); }
    var out = [];
    for (var s = 0; s < k; s++) {
      var tot = 0;
      for (i = 0; i < pool.length; i++) tot += wts[i];
      var r = rnd() * tot, acc = 0, idx = pool.length - 1;
      for (i = 0; i < pool.length; i++) { acc += wts[i]; if (r <= acc) { idx = i; break; } }
      out.push(pool[idx]);
      pool.splice(idx, 1); wts.splice(idx, 1);
    }
    return out.sort(function (a, b) { return a - b; });
  }

  /* 贴合度评分：一个组合在历史分布上有多"典型" */
  function shapeScore(f, b, stats) {
    var ft = drawFeatures({ f: f, b: b });
    var sSum = gauss(ft.sumF, stats.sumMean, Math.max(6, stats.sumStd));
    var sSpan = gauss(ft.span, stats.spanMean, Math.max(3, stats.spanStd));
    var oddRate = stats.oddDist[ft.oddF] / Math.max(1, stats.n);
    var bigRate = stats.bigDist[ft.bigF] / Math.max(1, stats.n);
    var zmax = Math.max.apply(null, ft.zones);
    var sZone = zmax >= 4 ? 0.25 : (zmax === 3 ? 0.8 : 1);
    var sRun = ft.maxRun >= 4 ? 0.3 : (ft.maxRun === 3 ? 0.85 : 1);
    var score = 0.34 * sSum + 0.20 * sSpan + 0.22 * Math.min(1, oddRate * 3)
              + 0.14 * Math.min(1, bigRate * 3) + 0.05 * sZone + 0.05 * sRun;
    return { score: score, ft: ft };
  }

  /* ---------------- 生成号码 ---------------- */
  /**
   * opts: {strategy, window, count, seed, filters:{sumLo,sumHi,oddLo,oddHi,noHist}}
   */
  function predict(draws, opts) {
    opts = opts || {};
    var strat = opts.strategy || 'balanced';
    var win = opts.window || 100;
    var count = opts.count || 5;
    var seed = (opts.seed === undefined || opts.seed === null) ? (Date.now() & 0x7fffffff) : opts.seed;
    var rnd = mulberry32(seed);
    var stats = buildStats(draws, win);
    var matrix = coMatrix(draws, win);
    var last = draws[draws.length - 1];
    var wf = frontWeights(draws, stats, matrix, strat, last, rnd);
    var wb = backWeights(draws, stats, strat, last);

    var filt = opts.filters || {};
    var sumLo = filt.sumLo || stats.sumQ10;
    var sumHi = filt.sumHi || stats.sumQ90;
    var oddLo = (filt.oddLo === undefined) ? 1 : filt.oddLo;
    var oddHi = (filt.oddHi === undefined) ? 4 : filt.oddHi;
    var noHist = filt.noHist !== false;

    var histSet = {};
    if (noHist) {
      for (var i = 0; i < draws.length; i++) histSet[draws[i].f.join(',')] = 1;
    }

    var seen = {}, selected = [];
    var guard = 0, maxGuard = count * 400 + 2000;
    while (selected.length < count && guard < maxGuard) {
      guard++;
      var f = sampleWeighted(wf, FRONT_PICK, FM, rnd);
      var b = sampleWeighted(wb, BACK_PICK, BM, rnd);
      var ft = drawFeatures({ f: f, b: b });
      if (ft.sumF < sumLo || ft.sumF > sumHi) continue;
      if (ft.oddF < oddLo || ft.oddF > oddHi) continue;
      if (Math.max.apply(null, ft.zones) >= 5) continue;
      if (ft.maxRun >= 5) continue;
      var key = f.join(',');
      if (seen[key]) continue;
      if (noHist && histSet[key]) continue;
      seen[key] = 1;
      var ss = shapeScore(f, b, stats);
      // 单号权重均值，作为"个体倾向"分量
      var ind = 0;
      for (var q = 0; q < f.length; q++) ind += wf[f[q]];
      ind = ind / f.length / (1 + Math.max.apply(null, wf));
      var indB = 0;
      for (var q2 = 0; q2 < b.length; q2++) indB += wb[b[q2]];
      indB = indB / b.length / (1 + Math.max.apply(null, wb));
      var total = 0.62 * ss.score + 0.26 * Math.min(1, ind * 2) + 0.12 * Math.min(1, indB * 2);
      selected.push({
        f: f, b: b, score: total,
        shape: ss.score, ft: ss.ft,
        sum: ft.sumF, span: ft.span, oddF: ft.oddF, bigF: ft.bigF,
        consec: ft.consecPairs, ac: ft.ac
      });
    }
    selected.sort(function (x, y) { return y.score - x.score; });

    // 给每注打标签
    for (var t = 0; t < selected.length; t++) {
      var s = selected[t], tags = [];
      if (s.oddF >= 3) tags.push('偏奇'); else tags.push('偏偶');
      if (s.bigF >= 3) tags.push('大号多'); else tags.push('小号多');
      if (s.consec > 0) tags.push('含连号');
      if (s.sum > stats.sumQ75) tags.push('和值偏高');
      else if (s.sum < stats.sumQ25) tags.push('和值偏低');
      s.tags = tags;
      s.rank = t + 1;
    }

    return {
      sets: selected, stats: stats, strategy: strat,
      window: win, seed: seed,
      meta: {
        sumLo: sumLo, sumHi: sumHi, oddLo: oddLo, oddHi: oddHi,
        sumMean: stats.sumMean, spanMean: stats.spanMean
      }
    };
  }

  /* ---------------- 复式 / 胆拖展开 ---------------- */
  function combos(arr, k) {
    var out = [], cur = [];
    (function rec(start) {
      if (cur.length === k) { out.push(cur.slice()); return; }
      for (var i = start; i < arr.length; i++) { cur.push(arr[i]); rec(i + 1); cur.pop(); }
    })(0);
    return out;
  }

  // 胆拖：胆码固定 + 拖码补足
  function danTuo(dan, tuo, backDan, backTuo) {
    var need = FRONT_PICK - dan.length;
    if (need < 0) return [];
    var fc = combos(tuo.slice().sort(function (a, b) { return a - b; }), need);
    var bn = BACK_PICK - (backDan || []).length;
    var bc = bn === 0 ? [[]] : combos((backTuo || []).slice().sort(function (a, b) { return a - b; }), bn);
    var out = [];
    for (var i = 0; i < fc.length; i++) {
      for (var j = 0; j < bc.length; j++) {
        out.push({
          f: uniqSorted((dan || []).concat(fc[i])),
          b: uniqSorted((backDan || []).concat(bc[j]))
        });
      }
    }
    return out;
  }

  // 复式展开（可选条件缩水）
  function compound(front, back, opts) {
    opts = opts || {};
    var all = [];
    var fc = combos(front.slice().sort(function (a, b) { return a - b; }), FRONT_PICK);
    var bc = combos(back.slice().sort(function (a, b) { return a - b; }), BACK_PICK);
    for (var i = 0; i < fc.length; i++) for (var j = 0; j < bc.length; j++) all.push({ f: fc[i], b: bc[j] });
    if (!opts.shrink) return { list: all, kept: all.length, origin: all.length };
    var lo = opts.sumLo || 60, hi = opts.sumHi || 120, oddLo = opts.oddLo || 2, oddHi = opts.oddHi || 3;
    var kept = all.filter(function (x) {
      var s = sum(x.f), odd = x.f.filter(function (v) { return v % 2 === 1; }).length;
      return s >= lo && s <= hi && odd >= oddLo && odd <= oddHi;
    });
    return { list: kept, kept: kept.length, origin: all.length };
  }

  /* ---------------- 回测 ---------------- */
  /**
   * 对最近 n 期逐期"盲测"：只用该期之前的数据选号
   * m: 每期生成注数
   */
  function backtest(draws, opts) {
    opts = opts || {};
    var n = opts.n || 50;
    var m = opts.m || 3;
    var win = opts.window || 100;
    var strat = opts.strategy || 'balanced';
    var start = Math.max(win + 1, draws.length - n);
    var rows = [];
    var hitF = [0, 0, 0, 0, 0, 0], hitB = [0, 0, 0];
    var bestFront = 0, bestBack = 0, anyPrize = 0;
    var randomHitF = 0, randomBest = 0;

    for (var i = start; i < draws.length; i++) {
      var hist = draws.slice(0, i);
      var real = draws[i];
      var res = predict(hist, { strategy: strat, window: win, count: m, seed: 10000 + i });
      var bf = 0, bb = 0;
      for (var j = 0; j < res.sets.length; j++) {
        var s = res.sets[j];
        var cf = 0, cb = 0;
        for (var a = 0; a < s.f.length; a++) if (real.f.indexOf(s.f[a]) >= 0) cf++;
        for (var c = 0; c < s.b.length; c++) if (real.b.indexOf(s.b[c]) >= 0) cb++;
        if (cf > bf) bf = cf;
        if (cb > bb) bb = cb;
      }
      hitF[bf]++; hitB[bb]++;
      if (bf > bestFront) bestFront = bf;
      if (bb > bestBack) bestBack = bb;
      if (bf >= 3 && bb >= 2) anyPrize++;
      // 对照组：纯随机 m 注
      var rb = 0;
      var rnd = mulberry32(20000 + i);
      for (var t = 0; t < m; t++) {
        var rf = sampleWeighted(new Array(FM + 1).fill(1), FRONT_PICK, FM, rnd);
        var cf2 = 0;
        for (var z = 0; z < rf.length; z++) if (real.f.indexOf(rf[z]) >= 0) cf2++;
        if (cf2 > rb) rb = cf2;
      }
      randomHitF += rb;
      if (rb > randomBest) randomBest = rb;
      rows.push({ issue: real.i, date: real.d, real: real.f.slice(), back: real.b.slice(), bestF: bf, bestB: bb });
    }
    var tested = rows.length;
    return {
      tested: tested, strategy: strat, m: m, window: win,
      rows: rows,
      hitF: hitF, hitB: hitB,
      bestFront: bestFront, bestBack: bestBack,
      prize: anyPrize,
      avgF: tested ? rows.reduce(function (a, b) { return a + b.bestF; }, 0) / tested : 0,
      randAvgF: tested ? randomHitF / tested : 0,
      randBest: randomBest
    };
  }

  /* ---------------- 号码属性 ---------------- */
  function numberProfile(draws, stats, num) {
    var inDraws = 0, recentArr = [];
    for (var i = 0; i < draws.length; i++) {
      if (draws[i].f.indexOf(num) >= 0) {
        inDraws++;
        recentArr.push(draws[i].i);
      }
    }
    // 只保留最近几次出现（倒序取末尾）
    var recent = recentArr.slice(-5).reverse();
    return {
      num: num, total: inDraws, rate: draws.length ? inDraws / draws.length : 0,
      omit: stats.omitF[num], maxOmit: stats.maxOmitF[num], avgGap: stats.avgGapF[num],
      threshold: stats.avgGapF[num] * 1.5,
      freq: stats.freqF[num],
      recent: recent
    };
  }

  var api = {
    FM: FM, BM: BM, FRONT_PICK: FRONT_PICK, BACK_PICK: BACK_PICK,
    STRATEGIES: STRATEGIES,
    mulberry32: mulberry32,
    drawFeatures: drawFeatures,
    buildStats: buildStats,
    coMatrix: coMatrix,
    predict: predict,
    backtest: backtest,
    combos: combos,
    danTuo: danTuo,
    compound: compound,
    numberProfile: numberProfile,
    shapeScore: shapeScore,
    quantile: quantile,
    mean: mean
  };

  if (typeof module !== 'undefined' && module.exports) module.exports = api;
  root.DLT = api;
})(typeof window !== 'undefined' ? window : globalThis);
