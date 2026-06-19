create_zip_with_python() {
    python3 - "$PUREAD_STAGE" "$PUREAD_ZIP" module.prop customize.sh service.sh post-fs-data.sh uninstall.sh action.sh scripts bin rules <<'PY'
import os
import stat
import sys
import time
import zipfile

stage = sys.argv[1]
zip_path = sys.argv[2]
entries = sys.argv[3:]
seen = set()


def zip_time(path):
    return time.localtime(os.stat(path).st_mtime)[:6]


def add_dir(zf, rel, path):
    name = rel.rstrip("/") + "/"
    if name in seen:
        return
    info = zipfile.ZipInfo(name, zip_time(path))
    mode = os.stat(path).st_mode
    info.external_attr = ((mode & 0o777) | stat.S_IFDIR) << 16
    zf.writestr(info, b"")
    seen.add(name)


def add_file(zf, rel, path):
    if rel in seen:
        return
    info = zipfile.ZipInfo(rel, zip_time(path))
    mode = os.stat(path).st_mode
    info.external_attr = (mode & 0o777) << 16
    info.compress_type = zipfile.ZIP_DEFLATED
    with open(path, "rb") as source:
        zf.writestr(info, source.read())
    seen.add(rel)


with zipfile.ZipFile(zip_path, "w") as zf:
    for entry in entries:
        source = os.path.join(stage, entry)
        if os.path.isdir(source):
            add_dir(zf, entry, source)
            for root, dirs, files in os.walk(source):
                dirs.sort()
                files.sort()
                for dirname in dirs:
                    path = os.path.join(root, dirname)
                    rel = os.path.relpath(path, stage).replace(os.sep, "/")
                    add_dir(zf, rel, path)
                for filename in files:
                    path = os.path.join(root, filename)
                    rel = os.path.relpath(path, stage).replace(os.sep, "/")
                    add_file(zf, rel, path)
        elif os.path.isfile(source):
            add_file(zf, entry, source)
        else:
            raise FileNotFoundError(source)
PY
}
