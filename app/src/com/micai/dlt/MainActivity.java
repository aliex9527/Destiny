package com.micai.dlt;

import android.app.Activity;
import android.app.NotificationManager;
import android.content.Context;
import android.content.Intent;
import android.content.SharedPreferences;
import android.content.pm.PackageManager;
import android.content.res.Configuration;
import android.graphics.Bitmap;
import android.graphics.BitmapFactory;
import android.graphics.Color;
import android.net.Uri;
import android.provider.MediaStore;
import android.os.Build;
import android.os.Bundle;
import android.view.View;
import android.view.Window;
import android.view.WindowManager;
import android.webkit.JavascriptInterface;
import android.webkit.WebResourceRequest;
import android.webkit.WebResourceResponse;
import android.webkit.WebSettings;
import android.webkit.WebView;
import android.webkit.WebViewClient;

import org.json.JSONObject;

import java.io.ByteArrayInputStream;
import java.io.File;
import java.io.FileInputStream;
import java.io.FileOutputStream;
import java.io.IOException;
import java.io.InputStream;

/**
 * Destiny —— WebView 容器
 *
 * 页面资源全部来自 assets，通过 https://dlt.local/ 虚拟域名拦截加载，
 * 使 localStorage 拥有稳定的 origin（file:// 下部分机型会丢数据）。
 *
 * 外网请求统一走原生层（WebView 里 fetch 会被跨域挡掉），
 * 抓到的数据通过 DLT_NET.onData 回递给页面。
 */
public class MainActivity extends Activity {

    private static final String HOST = "dlt.local";
    private static final String PREFS = "dlt";
    private static final int REQ_NOTIFY = 7;
    private static final int REQ_PHOTO = 11;

    private WebView web;
    private boolean sheetOpen = false;
    private boolean usingFallback = false;
    private File pendingPhoto = null;

    /** 虚拟域名加载失败时，退回 file:// 方式，保证页面一定能打开 */
    private void fallback() {
        if (usingFallback || web == null) return;
        usingFallback = true;
        runOnUiThread(new Runnable() {
            public void run() {
                if (web != null) web.loadUrl("file:///android_asset/index.html");
            }
        });
    }

    @Override
    protected void onCreate(Bundle savedInstanceState) {
        super.onCreate(savedInstanceState);

        web = new WebView(this);
        web.setBackgroundColor(Color.parseColor("#F4F5F7"));
        web.setOverScrollMode(View.OVER_SCROLL_NEVER);

        WebSettings s = web.getSettings();
        s.setJavaScriptEnabled(true);
        s.setDomStorageEnabled(true);
        s.setAllowFileAccess(true);
        s.setSupportZoom(false);
        s.setBuiltInZoomControls(false);
        s.setDisplayZoomControls(false);
        s.setTextZoom(100);
        s.setCacheMode(WebSettings.LOAD_NO_CACHE);

        web.addJavascriptInterface(new Host(), "AndroidHost");
        web.setWebViewClient(new WebViewClient() {
            @Override
            public WebResourceResponse shouldInterceptRequest(WebView view, WebResourceRequest req) {
                Uri u = req.getUrl();
                if (u != null && HOST.equals(u.getHost())) {
                    String path = u.getPath();
                    if (path != null && path.startsWith("/shot/")) {
                        return photoResponse(path.substring(6));
                    }
                    return assetResponse(path);
                }
                return null;
            }

            @Override
            public void onReceivedError(WebView view, int errorCode, String description, String failingUrl) {
                fallback();
            }

            @Override
            public void onReceivedError(WebView view, WebResourceRequest request,
                                        android.webkit.WebResourceError error) {
                if (request != null && request.isForMainFrame()) fallback();
            }
        });

        setContentView(web);
        setupSystemBars();
        web.loadUrl("https://" + HOST + "/index.html");
    }

    private WebResourceResponse assetResponse(String path) {
        if (path == null || path.equals("/") || path.isEmpty()) path = "/index.html";
        String name = path.startsWith("/") ? path.substring(1) : path;
        String mime = "text/plain";
        if (name.endsWith(".html")) mime = "text/html";
        else if (name.endsWith(".css")) mime = "text/css";
        else if (name.endsWith(".js")) mime = "application/javascript";
        else if (name.endsWith(".json")) mime = "application/json";
        else if (name.endsWith(".png")) mime = "image/png";
        try {
            InputStream in = getAssets().open(name);
            return new WebResourceResponse(mime, "UTF-8", in);
        } catch (IOException e) {
            return new WebResourceResponse("text/plain", "UTF-8",
                    new ByteArrayInputStream("404".getBytes()));
        }
    }

    /** 把缓存目录里的票面照片喂给 WebView */
    private WebResourceResponse photoResponse(String name) {
        try {
            if (name.indexOf('/') >= 0 || name.contains("..")) throw new IOException("bad");
            File f = new File(new File(getCacheDir(), "shots"), name);
            if (!f.exists()) throw new IOException("missing");
            return new WebResourceResponse("image/jpeg", null, new FileInputStream(f));
        } catch (Exception e) {
            return new WebResourceResponse("text/plain", "UTF-8",
                    new ByteArrayInputStream("404".getBytes()));
        }
    }

    /** 相机原图动辄几 MB，缩到 1600px 以内再交给页面，省内存也够看清号码 */
    private void shrinkPhoto(File f) {
        try {
            BitmapFactory.Options probe = new BitmapFactory.Options();
            probe.inJustDecodeBounds = true;
            BitmapFactory.decodeFile(f.getAbsolutePath(), probe);
            int scale = 1;
            while (probe.outWidth > 0 && probe.outWidth / scale > 1600) scale *= 2;
            BitmapFactory.Options opt = new BitmapFactory.Options();
            opt.inSampleSize = scale;
            Bitmap bmp = BitmapFactory.decodeFile(f.getAbsolutePath(), opt);
            if (bmp == null) return;
            FileOutputStream out = new FileOutputStream(f);
            bmp.compress(Bitmap.CompressFormat.JPEG, 86, out);
            out.close();
            bmp.recycle();
        } catch (Throwable ignore) {
            // 压缩失败就用原图，不影响使用
        }
    }

    private void capturePhoto() {
        final File dir = new File(getCacheDir(), "shots");
        if (!dir.exists()) dir.mkdirs();
        pendingPhoto = new File(dir, "p" + System.currentTimeMillis() + ".jpg");
        Uri uri = Uri.parse("content://" + PhotoProvider.AUTHORITY + "/" + pendingPhoto.getName());
        Intent it = new Intent(MediaStore.ACTION_IMAGE_CAPTURE);
        it.putExtra(MediaStore.EXTRA_OUTPUT, uri);
        it.addFlags(Intent.FLAG_GRANT_WRITE_URI_PERMISSION | Intent.FLAG_GRANT_READ_URI_PERMISSION);
        if (it.resolveActivity(getPackageManager()) == null) {
            post("window.DLT_toast&&window.DLT_toast('没找到相机应用，请在系统里装一个相机')");
            return;
        }
        try {
            startActivityForResult(it, REQ_PHOTO);
        } catch (Throwable e) {
            post("window.DLT_toast&&window.DLT_toast('相机启动失败')");
        }
    }

    @Override
    protected void onActivityResult(int req, int res, Intent data) {
        if (req != REQ_PHOTO) {
            super.onActivityResult(req, res, data);
            return;
        }
        // 少数相机在写入成功后仍返回 CANCELED，所以以文件是否真的落盘为准
        if (pendingPhoto == null || !pendingPhoto.exists() || pendingPhoto.length() == 0) {
            post("window.DLT_photo&&window.DLT_photo('')");
            return;
        }
        final File f = pendingPhoto;
        final String name = f.getName();
        new Thread(new Runnable() {
            public void run() {
                shrinkPhoto(f);
                post("window.DLT_photo&&window.DLT_photo(" + JSONObject.quote(name) + ")");
            }
        }, "dlt-photo").start();
    }

    private void setupSystemBars() {
        Window w = getWindow();
        w.setStatusBarColor(Color.parseColor("#FFFFFF"));
        w.setNavigationBarColor(Color.parseColor("#FFFFFF"));
        View decor = w.getDecorView();
        int flags = View.SYSTEM_UI_FLAG_LIGHT_STATUS_BAR;
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.O) {
            flags |= View.SYSTEM_UI_FLAG_LIGHT_NAVIGATION_BAR;
        }
        decor.setSystemUiVisibility(flags);
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.P) {
            w.getAttributes().layoutInDisplayCutoutMode =
                    WindowManager.LayoutParams.LAYOUT_IN_DISPLAY_CUTOUT_MODE_DEFAULT;
        }
    }

    private void post(final String js) {
        final WebView v = web;
        if (v == null) return;
        v.post(new Runnable() {
            public void run() {
                try {
                    v.evaluateJavascript(js, null);
                } catch (Throwable ignore) {
                }
            }
        });
    }

    private SharedPreferences sp() {
        return getSharedPreferences(PREFS, Context.MODE_PRIVATE);
    }

    private boolean canNotify() {
        if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.N) {
            NotificationManager nm = (NotificationManager) getSystemService(NOTIFICATION_SERVICE);
            return nm != null && nm.areNotificationsEnabled();
        }
        return true;
    }

    private void requestNotifyPermission() {
        if (Build.VERSION.SDK_INT >= 33) {
            runOnUiThread(new Runnable() {
                public void run() {
                    if (checkSelfPermission("android.permission.POST_NOTIFICATIONS")
                            != PackageManager.PERMISSION_GRANTED) {
                        requestPermissions(new String[]{"android.permission.POST_NOTIFICATIONS"}, REQ_NOTIFY);
                    }
                }
            });
        }
    }

    private class Host {

        /** 通用 HTTPS GET。结果通过 window.DLT_NET.onData(token, code, body) 回到页面 */
        @JavascriptInterface
        public void get(final String url, final String token, final String headers) {
            if (url == null || token == null) return;
            new Thread(new Runnable() {
                public void run() {
                    int code = -1;
                    String body;
                    try {
                        body = Net.fetch(url, headers, 12000);
                        code = 200;
                    } catch (Throwable e) {
                        body = e.getMessage() == null ? e.getClass().getSimpleName() : e.getMessage();
                    }
                    post("window.DLT_NET&&window.DLT_NET.onData("
                            + JSONObject.quote(token) + "," + code + ","
                            + JSONObject.quote(body == null ? "" : body) + ")");
                }
            }, "dlt-net").start();
        }

        /** 提醒开关状态等宿主信息 */
        @JavascriptInterface
        public String prefs() {
            SharedPreferences p = sp();
            JSONObject o = new JSONObject();
            try {
                o.put("first", !p.contains(DrawCheckReceiver.K_REMIND));
                o.put("reminder", p.getBoolean(DrawCheckReceiver.K_REMIND, false));
                o.put("seen", p.getString(DrawCheckReceiver.K_SEEN, ""));
                o.put("canNotify", canNotify());
            } catch (Exception ignore) {
            }
            return o.toString();
        }

        @JavascriptInterface
        public boolean setReminder(boolean on) {
            sp().edit().putBoolean(DrawCheckReceiver.K_REMIND, on).apply();
            Remind.sync(MainActivity.this, on);
            if (on) requestNotifyPermission();
            return on;
        }

        /** 页面已展示到的期号，后台检查据此判断"有没有新开奖" */
        @JavascriptInterface
        public void markSeen(String issue) {
            if (issue == null || issue.length() != 5) return;
            sp().edit().putString(DrawCheckReceiver.K_SEEN, issue).apply();
        }

        /** 取出后台已抓到但界面还没合并的开奖，取完即清空 */
        @JavascriptInterface
        public String takePending() {
            SharedPreferences p = sp();
            String v = p.getString(DrawCheckReceiver.K_PENDING, "");
            if (v != null && v.length() > 0) p.edit().putString(DrawCheckReceiver.K_PENDING, "").apply();
            return v == null ? "" : v;
        }

        @JavascriptInterface
        public void capture() {
            runOnUiThread(new Runnable() {
                public void run() {
                    capturePhoto();
                }
            });
        }

        @JavascriptInterface
        public void setSheet(final boolean open) {
            runOnUiThread(new Runnable() {
                public void run() {
                    sheetOpen = open;
                }
            });
        }

        @JavascriptInterface
        public void exit() {
            runOnUiThread(new Runnable() {
                public void run() {
                    finish();
                }
            });
        }
    }

    @Override
    protected void onResume() {
        super.onResume();
        post("window.DLT_resume&&window.DLT_resume()");
    }

    @Override
    public void onBackPressed() {
        if (sheetOpen) {
            web.evaluateJavascript("window.DLT_closeSheet&&window.DLT_closeSheet()", null);
        } else {
            super.onBackPressed();
        }
    }

    @Override
    public void onConfigurationChanged(Configuration cfg) {
        super.onConfigurationChanged(cfg);
    }

    @Override
    protected void onDestroy() {
        if (web != null) {
            web.removeJavascriptInterface("AndroidHost");
            web.destroy();
            web = null;
        }
        super.onDestroy();
    }
}
