package com.micai.dlt;

import android.content.ContentProvider;
import android.content.ContentValues;
import android.database.Cursor;
import android.net.Uri;
import android.os.ParcelFileDescriptor;

import java.io.File;
import java.io.FileNotFoundException;

/**
 * 给相机写照片用的极简 ContentProvider。
 *
 * Android 7 起不允许把 file:// 交给别的应用（FileUriExposedException），
 * 而 androidx 的 FileProvider 我们没依赖 —— 这里自己实现三个方法就够了。
 */
public class PhotoProvider extends ContentProvider {

    static final String AUTHORITY = "com.micai.dlt.photo";

    @Override
    public boolean onCreate() {
        return true;
    }

    @Override
    public ParcelFileDescriptor openFile(Uri uri, String mode) throws FileNotFoundException {
        String name = uri.getLastPathSegment();
        if (name == null || name.isEmpty() || name.contains("..") || name.indexOf('/') >= 0) {
            throw new FileNotFoundException("bad name");
        }
        File dir = new File(getContext().getCacheDir(), "shots");
        if (!dir.exists() && !dir.mkdirs()) throw new FileNotFoundException("no dir");
        File f = new File(dir, name);
        int flags = ParcelFileDescriptor.MODE_CREATE;
        if (mode != null && mode.contains("w")) {
            flags |= ParcelFileDescriptor.MODE_READ_WRITE | ParcelFileDescriptor.MODE_TRUNCATE;
        } else {
            flags |= ParcelFileDescriptor.MODE_READ_ONLY;
        }
        return ParcelFileDescriptor.open(f, flags);
    }

    @Override
    public String getType(Uri uri) {
        return "image/jpeg";
    }

    @Override
    public Cursor query(Uri uri, String[] projection, String selection,
                        String[] selectionArgs, String sortOrder) {
        return null;
    }

    @Override
    public Uri insert(Uri uri, ContentValues values) {
        return null;
    }

    @Override
    public int delete(Uri uri, String selection, String[] selectionArgs) {
        return 0;
    }

    @Override
    public int update(Uri uri, ContentValues values, String selection, String[] selectionArgs) {
        return 0;
    }
}
