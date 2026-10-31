package com.micai.dlt;

import java.io.ByteArrayOutputStream;
import java.io.IOException;
import java.io.InputStream;
import java.net.HttpURLConnection;
import java.net.URL;
import java.util.ArrayList;
import java.util.Arrays;
import java.util.List;
import java.util.zip.GZIPInputStream;

/**
 * 极简 HTTP 客户端 + 体彩官网 JSON 解析。
 *
 * WebView 里直接 fetch 会撞上跨域限制，所以所有外网请求都走原生层，
 * 解析成紧凑文本再交回 JS。
 */
final class Net {

    /** 体彩官方接口：注意必须带 Referer，否则会被风控拦截 */
    static final String API_URL =
            "https://webapi.sporttery.cn/gateway/lottery/getHistoryPageListV1.qry"
                    + "?gameNo=85&provinceId=0&pageSize=20&isVerify=1&pageNo=1";
    static final String API_HEADERS =
            "Referer: https://static.sporttery.cn/\nAccept: application/json, text/plain, */*";

    private static final String UA = "Mozilla/5.0 (Linux; Android 13; Pixel 7) "
            + "AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Mobile Safari/537.36";

    private Net() {
    }

    static String fetch(String url, String headers, int timeoutMs) throws IOException {
        HttpURLConnection c = null;
        try {
            c = (HttpURLConnection) new URL(url).openConnection();
            c.setConnectTimeout(timeoutMs);
            c.setReadTimeout(timeoutMs);
            c.setInstanceFollowRedirects(true);
            c.setRequestProperty("User-Agent", UA);
            c.setRequestProperty("Accept-Language", "zh-CN,zh;q=0.9");
            c.setRequestProperty("Accept-Encoding", "gzip");
            if (headers != null) {
                for (String line : headers.split("\n")) {
                    int k = line.indexOf(':');
                    if (k > 0) {
                        c.setRequestProperty(line.substring(0, k).trim(), line.substring(k + 1).trim());
                    }
                }
            }
            int code = c.getResponseCode();
            InputStream in = code >= 400 ? c.getErrorStream() : c.getInputStream();
            if (in == null) throw new IOException("HTTP " + code);
            String enc = c.getContentEncoding();
            if (enc != null && enc.toLowerCase().contains("gzip")) in = new GZIPInputStream(in);

            ByteArrayOutputStream bo = new ByteArrayOutputStream();
            byte[] buf = new byte[8192];
            int n;
            while ((n = in.read(buf)) > 0) {
                bo.write(buf, 0, n);
                if (bo.size() > 2 * 1024 * 1024) break;   // 防御：异常大的响应直接截断
            }
            in.close();
            if (code != 200) throw new IOException("HTTP " + code);
            return bo.toString("UTF-8");
        } finally {
            if (c != null) c.disconnect();
        }
    }

    /** 从官网 JSON 中抽取最近开奖，最新的一期在最前 */
    static List<Draw> parse(String body) {
        List<Draw> out = new ArrayList<Draw>();
        if (body == null) return out;
        int p = 0;
        while (true) {
            int k = body.indexOf("\"lotteryDrawNum\":\"", p);
            if (k < 0) break;
            int next = body.indexOf("\"lotteryDrawNum\":\"", k + 18);
            int end = next < 0 ? body.length() : next;
            String chunk = body.substring(k, end);
            Draw d = Draw.of(field(chunk, "\"lotteryDrawNum\":\""),
                    field(chunk, "\"lotteryDrawResult\":\""),
                    field(chunk, "\"lotteryDrawTime\":\""));
            if (d != null) out.add(d);
            p = end;
        }
        return out;
    }

    private static String field(String s, String key) {
        int i = s.indexOf(key);
        if (i < 0) return null;
        int a = i + key.length();
        int b = s.indexOf('"', a);
        return b < 0 ? null : s.substring(a, b);
    }

    static final class Draw {
        final String i;      // 期号 5 位
        final String d;      // 日期 YYYY-MM-DD
        final int[] f = new int[5];
        final int[] b = new int[2];

        private Draw(String issue, String day) {
            this.i = issue;
            this.d = day;
        }

        static Draw of(String issue, String result, String day) {
            if (issue == null || result == null) return null;
            if (!issue.matches("\\d{5}")) return null;
            String[] p = result.trim().split("\\s+");
            if (p.length < 7) return null;
            Draw d = new Draw(issue, day == null ? "" : day);
            try {
                for (int j = 0; j < 5; j++) d.f[j] = Integer.parseInt(p[j]);
                for (int j = 0; j < 2; j++) d.b[j] = Integer.parseInt(p[5 + j]);
            } catch (NumberFormatException e) {
                return null;
            }
            if (!d.valid()) return null;
            Arrays.sort(d.f);
            Arrays.sort(d.b);
            return d;
        }

        private boolean valid() {
            boolean[] u = new boolean[36];
            for (int j = 0; j < 5; j++) {
                if (f[j] < 1 || f[j] > 35 || u[f[j]]) return false;
                u[f[j]] = true;
            }
            boolean[] v = new boolean[13];
            for (int j = 0; j < 2; j++) {
                if (b[j] < 1 || b[j] > 12 || v[b[j]]) return false;
                v[b[j]] = true;
            }
            return true;
        }

        /** 与前端 data.js 一致的紧凑格式：期号,f1,f2,f3,f4,f5,b1,b2,日期 */
        String toRaw() {
            StringBuilder sb = new StringBuilder(i);
            for (int j = 0; j < 5; j++) sb.append(',').append(f[j]);
            for (int j = 0; j < 2; j++) sb.append(',').append(b[j]);
            return sb.append(',').append(d).toString();
        }

        /** 通知里展示用：03 11 22 28 33 + 05 09 */
        String balls() {
            StringBuilder sb = new StringBuilder();
            for (int j = 0; j < 5; j++) sb.append(pad(f[j])).append(' ');
            sb.append("+ ");
            for (int j = 0; j < 2; j++) {
                sb.append(pad(b[j]));
                if (j == 0) sb.append(' ');
            }
            return sb.toString();
        }

        private static String pad(int n) {
            return n < 10 ? "0" + n : String.valueOf(n);
        }
    }
}
