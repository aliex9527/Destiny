/* ================= 大乐透验票引擎 =================
 * 纯函数，不依赖 DOM，便于单元测试。
 *
 * 规则依据：
 *  - 2026-01-31 第 26014 期起执行新规：9 个奖级合并为 7 个，
 *    三=5+0/4+2  四=4+1  五=3+2/4+0  六=2+2/3+1  七=3+0/1+2/2+1/0+2
 *    固定奖 5000/300/150/15/5（奖池≥8亿时 6666/380/200/18/7）
 *  - 26014 期之前为旧规 9 个奖级：10000/3000/300/200/100/15/5
 *  追加投注只参与浮动奖（一、二等奖），中奖时多得 80%。
 */
(function () {
  'use strict';

  var NEW_RULE_FROM = 26014;

  var NEW_RULES = [
    { lv: 1, name: '一等奖', conds: ['5+2'], fixed: 0 },
    { lv: 2, name: '二等奖', conds: ['5+1'], fixed: 0 },
    { lv: 3, name: '三等奖', conds: ['5+0', '4+2'], fixed: 5000 },
    { lv: 4, name: '四等奖', conds: ['4+1'], fixed: 300 },
    { lv: 5, name: '五等奖', conds: ['3+2', '4+0'], fixed: 150 },
    { lv: 6, name: '六等奖', conds: ['2+2', '3+1'], fixed: 15 },
    { lv: 7, name: '七等奖', conds: ['3+0', '1+2', '2+1', '0+2'], fixed: 5 }
  ];

  var OLD_RULES = [
    { lv: 1, name: '一等奖', conds: ['5+2'], fixed: 0 },
    { lv: 2, name: '二等奖', conds: ['5+1'], fixed: 0 },
    { lv: 3, name: '三等奖', conds: ['5+0'], fixed: 10000 },
    { lv: 4, name: '四等奖', conds: ['4+2'], fixed: 3000 },
    { lv: 5, name: '五等奖', conds: ['4+1'], fixed: 300 },
    { lv: 6, name: '六等奖', conds: ['3+2'], fixed: 200 },
    { lv: 7, name: '七等奖', conds: ['4+0'], fixed: 100 },
    { lv: 8, name: '八等奖', conds: ['3+1', '2+2'], fixed: 15 },
    { lv: 9, name: '九等奖', conds: ['3+0', '1+2', '2+1', '0+2'], fixed: 5 }
  ];

  // 命中组合 -> 奖级，预先展开成查表，避免每次遍历
  function buildMap(rules) {
    var m = {};
    for (var i = 0; i < rules.length; i++) {
      for (var j = 0; j < rules[i].conds.length; j++) {
        m[rules[i].conds[j]] = rules[i].lv;
      }
    }
    return m;
  }
  var NEW_MAP = buildMap(NEW_RULES);
  var OLD_MAP = buildMap(OLD_RULES);

  function isNewRule(issue) {
    return parseInt(issue, 10) >= NEW_RULE_FROM;
  }

  function rulesOf(issue) {
    return isNewRule(issue) ? NEW_RULES : OLD_RULES;
  }

  function ruleOf(issue, lv) {
    var r = rulesOf(issue);
    for (var i = 0; i < r.length; i++) if (r[i].lv === lv) return r[i];
    return null;
  }

  /** 命中数 -> 奖级，0 表示未中奖 */
  function levelOf(issue, hf, hb) {
    var map = isNewRule(issue) ? NEW_MAP : OLD_MAP;
    return map[hf + '+' + hb] || 0;
  }

  /** 单注奖金（不含追加加成）。返回 { amount, live } —— live 表示浮动奖需要官方数据 */
  function unitPrize(issue, lv, table) {
    if (!lv) return { amount: 0, live: false };
    if (table && table.b && typeof table.b[lv - 1] === 'number') {
      return { amount: table.b[lv - 1], live: false };
    }
    var r = ruleOf(issue, lv);
    if (!r) return { amount: 0, live: false };
    if (r.fixed > 0) return { amount: r.fixed, live: false };
    return { amount: 0, live: true };     // 一、二等奖，没有官方数据
  }

  /** 追加加成（仅一、二等奖） */
  function extraPrize(issue, lv, table, base) {
    if (lv !== 1 && lv !== 2) return 0;
    if (table && table.x && typeof table.x[lv - 1] === 'number') return table.x[lv - 1];
    return Math.round(base * 0.8);
  }

  /* ---------------- 组合展开 ---------------- */

  function comb(n, k) {
    if (k < 0 || k > n) return 0;
    var r = 1;
    for (var i = 1; i <= k; i++) r = r * (n - k + i) / i;
    return Math.round(r);
  }

  /** 从数组里取 k 个的全部组合 */
  function choose(arr, k) {
    var out = [];
    (function walk(start, cur) {
      if (cur.length === k) { out.push(cur.slice()); return; }
      for (var i = start; i < arr.length; i++) {
        cur.push(arr[i]);
        walk(i + 1, cur);
        cur.pop();
      }
    })(0, []);
    return out;
  }

  var MAX_BETS = 5000;

  /**
   * 票面注数。支持单式与复式（前区 n≥5、后区 m≥2）。
   * 胆拖暂不支持，isValid 会拦下来。
   */
  function betCount(front, back) {
    if (front.length < 5 || back.length < 2) return 0;
    return comb(front.length, 5) * comb(back.length, 2);
  }

  function isValid(front, back) {
    if (!front || !back) return false;
    if (front.length < 5 || front.length > 12) return false;
    if (back.length < 2 || back.length > 6) return false;
    if (betCount(front, back) > MAX_BETS) return false;
    var s = {}, i;
    for (i = 0; i < front.length; i++) {
      if (front[i] < 1 || front[i] > 35 || s['f' + front[i]]) return false;
      s['f' + front[i]] = 1;
    }
    for (i = 0; i < back.length; i++) {
      if (back[i] < 1 || back[i] > 12 || s['b' + back[i]]) return false;
      s['b' + back[i]] = 1;
    }
    return true;
  }

  function hitCount(pick, draw) {
    var n = 0;
    for (var i = 0; i < pick.length; i++) {
      if (draw.indexOf(pick[i]) >= 0) n++;
    }
    return n;
  }

  /**
   * 核对一张票。
   * ticket: { issue, extra, front:[], back:[] }
   * draw:   { i, f:[5], b:[2] }
   * table:  { b:[...], x:[...] } 官方奖级金额，可缺省
   */
  function check(ticket, draw, table) {
    var issue = ticket.issue;
    var front = ticket.front.slice().sort(function (a, b) { return a - b; });
    var back = ticket.back.slice().sort(function (a, b) { return a - b; });
    var res = {
      issue: issue,
      extra: !!ticket.extra,
      bets: betCount(front, back),
      front: front,
      back: back,
      frontHit: [],
      backHit: [],
      bestHF: 0,
      bestHB: 0,
      bestLevel: 0,
      levels: [],
      total: 0,
      live: false,          // 是否含无法确定的浮动奖金额
      ok: false
    };
    if (!draw || draw.i !== issue) return res;
    if (!isValid(front, back)) return res;

    res.ok = true;
    res.frontHit = front.filter(function (v) { return draw.f.indexOf(v) >= 0; });
    res.backHit = back.filter(function (v) { return draw.b.indexOf(v) >= 0; });

    // 展开成单注逐注判定
    var fCombos = choose(front, 5);
    var bCombos = choose(back, 2);
    var byLevel = {};

    for (var i = 0; i < fCombos.length; i++) {
      var hf = hitCount(fCombos[i], draw.f);
      for (var j = 0; j < bCombos.length; j++) {
        var hb = hitCount(bCombos[j], draw.b);
        // 命中数要独立记录 —— 没中奖时也要能告诉用户差几个
        if (hf > res.bestHF || (hf === res.bestHF && hb > res.bestHB)) {
          res.bestHF = hf;
          res.bestHB = hb;
        }
        var lv = levelOf(issue, hf, hb);
        if (!lv) continue;
        if (!res.bestLevel || lv < res.bestLevel) res.bestLevel = lv;
        byLevel[lv] = (byLevel[lv] || 0) + 1;
      }
    }

    var total = 0;
    Object.keys(byLevel).map(Number).sort(function (a, b) { return a - b; }).forEach(function (lv) {
      var n = byLevel[lv];
      var up = unitPrize(issue, lv, table);
      var unit = up.amount;
      var add = res.extra ? extraPrize(issue, lv, table, unit) : 0;
      if (up.live) res.live = true;
      var sum = (unit + add) * n;
      total += sum;
      res.levels.push({
        lv: lv,
        name: ruleOf(issue, lv).name,
        count: n,
        unit: unit,
        extra: add,
        sum: sum,
        live: up.live
      });
    });

    res.total = total;
    return res;
  }

  /** 一注的金额（用于"如果我中了一等奖"这类展示） */
  function levelAmountText(issue, lv, extra, table) {
    var up = unitPrize(issue, lv, table);
    if (up.live) return '浮动奖';
    var add = extra ? extraPrize(issue, lv, table, up.amount) : 0;
    return formatMoney(up.amount + add);
  }

  function formatMoney(n) {
    if (n >= 10000) return (n / 10000).toFixed(n % 10000 === 0 ? 0 : 1) + ' 万元';
    return n.toLocaleString ? n.toLocaleString('zh-CN') + ' 元' : n + ' 元';
  }

  window.DLT_TICKET = {
    RULE_FROM: NEW_RULE_FROM,
    MAX_BETS: MAX_BETS,
    rulesOf: rulesOf,
    isNewRule: isNewRule,
    levelOf: levelOf,
    unitPrize: unitPrize,
    betCount: betCount,
    isValid: isValid,
    check: check,
    comb: comb,
    levelAmountText: levelAmountText,
    formatMoney: formatMoney
  };
})();
