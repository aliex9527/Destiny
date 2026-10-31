package com.micai.dlt;

import android.app.AlarmManager;
import android.app.PendingIntent;
import android.content.Context;
import android.content.Intent;
import android.os.Build;

import java.util.Calendar;

/**
 * 每日开奖查询的定时任务。
 *
 * 大乐透 21:25 开奖，官方接口通常几分钟后更新。这里排两个时间点：
 * 21:45 主查、22:20 补查 —— 万一结果发布延迟，第二次还能补上；
 * 已经通知过的期号会被 seen 拦住，所以第二次不会重复推送。
 *
 * 用不精确闹钟（setInexactRepeating）避开 Android 12+ 的精确闹钟权限，
 * 代价是触发时间可能有几分钟浮动，对"开奖后查一下"完全够用。
 */
final class Remind {

    private static final int[] AT_MINUTE = {21 * 60 + 45, 22 * 60 + 20};
    private static final int BASE_CODE = 200;

    private Remind() {
    }

    static void sync(Context c, boolean on) {
        AlarmManager am = (AlarmManager) c.getSystemService(Context.ALARM_SERVICE);
        if (am == null) return;
        for (int i = 0; i < AT_MINUTE.length; i++) {
            PendingIntent pi = pending(c, BASE_CODE + i);
            am.cancel(pi);
            if (!on) continue;
            long first = nextTrigger(AT_MINUTE[i]);
            try {
                am.setInexactRepeating(AlarmManager.RTC_WAKEUP, first, AlarmManager.INTERVAL_DAY, pi);
            } catch (Throwable e) {
                am.set(AlarmManager.RTC_WAKEUP, first, pi);
            }
        }
    }

    private static PendingIntent pending(Context c, int code) {
        Intent it = new Intent(c, DrawCheckReceiver.class);
        it.setAction(DrawCheckReceiver.ACTION_CHECK);
        int flags = PendingIntent.FLAG_UPDATE_CURRENT;
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) flags |= PendingIntent.FLAG_IMMUTABLE;
        return PendingIntent.getBroadcast(c, code, it, flags);
    }

    private static long nextTrigger(int minuteOfDay) {
        Calendar ca = Calendar.getInstance();
        ca.set(Calendar.HOUR_OF_DAY, minuteOfDay / 60);
        ca.set(Calendar.MINUTE, minuteOfDay % 60);
        ca.set(Calendar.SECOND, 0);
        ca.set(Calendar.MILLISECOND, 0);
        if (ca.getTimeInMillis() <= System.currentTimeMillis()) {
            ca.add(Calendar.DAY_OF_YEAR, 1);
        }
        return ca.getTimeInMillis();
    }
}
