#!/bin/sh
# Copyright 2024-2024 CrabNebula Ltd.
# SPDX-License-Identifier: Apache-2.0
# SPDX-License-Identifier: MIT
#
# Runs from the extracted control directory. QPKG_ROOT (the .qpkg directory of
# the target volume) and QPKG_DIR (QPKG_ROOT/<name>) are exported by the
# self-extracting header.

set -e

[ -f ./qpkg.cfg ] || { echo "qpkg.cfg not found" >&2; exit 1; }
. ./qpkg.cfg

[ -n "$QPKG_ROOT" ] && [ -n "$QPKG_DIR" ] || { echo "QPKG_ROOT/QPKG_DIR not set" >&2; exit 1; }
[ -f ./data.tar.gz ] || { echo "data.tar.gz not found" >&2; exit 1; }

QPKG_CONF=/etc/config/qpkg.conf
SETCFG=/sbin/setcfg

# Upgrade: stop the previously installed service before replacing files.
if [ -n "$QPKG_SERVICE_PROGRAM" ] && [ -x "/etc/init.d/$QPKG_NAME.sh" ]; then
    "/etc/init.d/$QPKG_NAME.sh" stop >/dev/null 2>&1 || true
fi

/bin/mkdir -p "$QPKG_DIR"
/bin/tar -xzf ./data.tar.gz -C "$QPKG_DIR"

if [ -n "$QPKG_SERVICE_PROGRAM" ]; then
    /bin/chmod 755 "$QPKG_DIR/$QPKG_SERVICE_PROGRAM" 2>/dev/null || true
fi

# Expose a payload directory through the QTS web server so static files (e.g. a
# WebAssembly app) are served from the same origin as the QTS desktop.
if [ -n "$QPKG_WEB_ROOT" ] && [ -d "$QPKG_DIR/$QPKG_WEB_ROOT" ]; then
    if [ -d /home/httpd/cgi-bin/qpkg ]; then
        /bin/ln -sfn "$QPKG_DIR/$QPKG_WEB_ROOT" "/home/httpd/cgi-bin/qpkg/$QPKG_NAME"
    fi
fi

# Apache drop-in, e.g. Cross-Origin-Opener-Policy / Cross-Origin-Embedder-Policy
# headers required by multi-threaded WebAssembly.
if [ -n "$QPKG_APACHE_CONF" ] && [ -f "$QPKG_DIR/$QPKG_APACHE_CONF" ] && [ -d /etc/config/apache/extra ]; then
    /bin/cp -f "$QPKG_DIR/$QPKG_APACHE_CONF" "/etc/config/apache/extra/apache-$QPKG_NAME.conf"
    if [ -x /etc/init.d/Qthttpd.sh ]; then
        /etc/init.d/Qthttpd.sh restart >/dev/null 2>&1 || true
    fi
fi

if [ -f ./package_routines ]; then
    . ./package_routines
    if command -v pkg_post_install >/dev/null 2>&1; then
        pkg_post_install
    fi
fi

# Registration only happens on a real QTS system; skipped when packaging tests
# run the installer in a sandbox (no /sbin/setcfg).
if [ -x "$SETCFG" ] && [ -f "$QPKG_CONF" ]; then
    "$SETCFG" "$QPKG_NAME" Name "$QPKG_NAME" -f "$QPKG_CONF"
    "$SETCFG" "$QPKG_NAME" Version "$QPKG_VER" -f "$QPKG_CONF"
    "$SETCFG" "$QPKG_NAME" Enable TRUE -f "$QPKG_CONF"
    "$SETCFG" "$QPKG_NAME" Date "$(/bin/date +%Y%m%d)" -f "$QPKG_CONF"
    "$SETCFG" "$QPKG_NAME" Install_Path "$QPKG_DIR" -f "$QPKG_CONF"

    if [ -n "$QPKG_DISPLAY_NAME" ]; then
        "$SETCFG" "$QPKG_NAME" Display_Name "$QPKG_DISPLAY_NAME" -f "$QPKG_CONF"
    fi
    if [ -n "$QPKG_AUTHOR" ]; then
        "$SETCFG" "$QPKG_NAME" Author "$QPKG_AUTHOR" -f "$QPKG_CONF"
    fi
    if [ -n "$QPKG_RC_NUM" ]; then
        "$SETCFG" "$QPKG_NAME" RC_Number "$QPKG_RC_NUM" -f "$QPKG_CONF"
    fi
    if [ -n "$QPKG_SERVICE_PROGRAM" ]; then
        /bin/ln -sf "$QPKG_DIR/$QPKG_SERVICE_PROGRAM" "/etc/init.d/$QPKG_NAME.sh"
        "$SETCFG" "$QPKG_NAME" Shell "/etc/init.d/$QPKG_NAME.sh" -f "$QPKG_CONF"
    fi
    if [ -n "$QPKG_WEBUI" ]; then
        "$SETCFG" "$QPKG_NAME" WebUI "$QPKG_WEBUI" -f "$QPKG_CONF"
    fi
    if [ -n "$QPKG_WEB_PORT" ]; then
        "$SETCFG" "$QPKG_NAME" Web_Port "$QPKG_WEB_PORT" -f "$QPKG_CONF"
    fi
    if [ -n "$QPKG_WEB_SSL_PORT" ]; then
        "$SETCFG" "$QPKG_NAME" Web_SSL_Port "$QPKG_WEB_SSL_PORT" -f "$QPKG_CONF"
    fi
    if [ -n "$QPKG_DESKTOP_APP" ]; then
        "$SETCFG" "$QPKG_NAME" Desktop "$QPKG_DESKTOP_APP" -f "$QPKG_CONF"
    fi
    if [ -n "$QPKG_DESKTOP_APP_WIN_WIDTH" ]; then
        "$SETCFG" "$QPKG_NAME" Win_Width "$QPKG_DESKTOP_APP_WIN_WIDTH" -f "$QPKG_CONF"
    fi
    if [ -n "$QPKG_DESKTOP_APP_WIN_HEIGHT" ]; then
        "$SETCFG" "$QPKG_NAME" Win_Height "$QPKG_DESKTOP_APP_WIN_HEIGHT" -f "$QPKG_CONF"
    fi
    if [ -n "$QPKG_VISIBLE" ]; then
        "$SETCFG" "$QPKG_NAME" Visible "$QPKG_VISIBLE" -f "$QPKG_CONF"
    fi
fi

exit 0