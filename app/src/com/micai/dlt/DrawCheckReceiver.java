package com.micai.dlt;

import android.app.Notification;
import android.app.NotificationChannel;
import android.app.NotificationManager;
import android.app.PendingIntent;
import android.content.BroadcastReceiver;
import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.os.Build;

import java.util.List;

/**
 * 每日开奖查询：闹钟到点后联网拉一次，发现新期号就发通知，
 * 同时把新号码缓存到 SharedPreferences，应用下次启动时会静默合并。
 */
public class DrawCheckReceiver extends BroadcastReceiver {

    static final String ACTION_CHECK = "com.micai.dlt.CHECK";
    static final String PREFS = "dlt";
    static final String K_SEEN = "seenIssue";
    static final String K_PENDING = "pending";
    static final String K_REMIND = "reminder";
    static final String CHANNEL = "draw";

    @Override
    public void onReceive(Context ctx, Intent intent) {
        if (intent != null && Intent.ACTION_BOOT_COMPLETED.equals(intent.getAction())) {
            // 重启后闹钟会被系统清空，这里只负责恢复，不立刻联网
            Remind.sync(ctx, isRemindOn(ctx));
            return;
        }
        final Context app = ctx.getApplicationContext();
        final PendingResult pr = goAsync();
        new Thread(new Runnable() {
            public void run() {
                try {
                    check(app);
                } catch (Throwable ignore) {
                    // 后台任务失败无需打扰用户，下次开奖再试
                } finally {
                    pr.finish();
                }
            }
        }, "dlt-check").start();
    }

    static boolean isRemindOn(Context c) {
        return c.getSharedPreferences(PREFS, Context.MODE_PRIVATE).getBoolean(K_REMIND, false);
    }

    private void check(Context ctx) {
        SharedPreferences sp = ctx.getSharedPreferences(PREFS, Context.MODE_PRIVATE);
        List<Net.Draw> list;
        try {
            list = Net.parse(Net.fetch(Net.API_URL, Net.API_HEADERS, 8000));
        } catch (Throwable e) {
            return;
        }
        if (list.isEmpty()) return;

        Net.Draw newest = list.get(0);
        String seen = sp.getString(K_SEEN, "");
        if (newest.i.equals(seen)) return;   // 没有新开奖

        // 只缓存比已见期号更新的，避免把整页历史塞进本地存储
        StringBuilder sb = new StringBuilder();
        for (int i = list.size() - 1; i >= 0; i--) {
            Net.Draw d = list.get(i);
            if (seen.length() == 0 || d.i.compareTo(seen) > 0) {
                if (sb.length() > 0) sb.append(';');
                sb.append(d.toRaw());
            }
        }

        SharedPreferences.Editor ed = sp.edit().putString(K_SEEN, newest.i);
        if (sb.length() > 0) ed.putString(K_PENDING, sb.toString());
        ed.apply();

        if (isRemindOn(ctx)) notifyNew(ctx, newest);
    }

    private void notifyNew(Context ctx, Net.Draw d) {
        NotificationManager nm = (NotificationManager) ctx.getSystemService(Context.NOTIFICATION_SERVICE);
        if (nm == null) return;
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N && !nm.areNotificationsEnabled()) return;

        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            NotificationChannel ch = new NotificationChannel(CHANNEL, "开奖提醒",
                    NotificationManager.IMPORTANCE_DEFAULT);
            ch.setDescription("每期开奖后推送新号码");
            nm.createNotificationChannel(ch);
        }

        Intent it = new Intent(ctx, MainActivity.class);
        it.addFlags(Intent.FLAG_ACTIVITY_NEW_TASK | Intent.FLAG_ACTIVITY_CLEAR_TOP);
        int flags = PendingIntent.FLAG_UPDATE_CURRENT;
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.M) flags |= PendingIntent.FLAG_IMMUTABLE;
        PendingIntent pi = PendingIntent.getActivity(ctx, 100, it, flags);

        Notification.Builder b = Build.VERSION.SDK_INT >= Build.VERSION_CODES.O
                ? new Notification.Builder(ctx, CHANNEL)
                : new Notification.Builder(ctx);
        b.setSmallIcon(R.drawable.ic_stat)
                .setContentTitle("第 " + d.i + " 期开奖")
                .setContentText(d.balls())
                .setStyle(new Notification.BigTextStyle()
                        .bigText(d.balls() + "    开奖日期 " + d.d))
                .setAutoCancel(true)
                .setContentIntent(pi);
        if (Build.VERSION.SDK_INT < Build.VERSION_CODES.O) {
            b.setPriority(Notification.PRIORITY_DEFAULT);
        }
        nm.notify(1001, b.build());
    }
}
