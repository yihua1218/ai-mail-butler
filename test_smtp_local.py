#!/usr/bin/env python3
"""
Local SMTP reception test for AI Mail Butler.

Usage:
    python3 test_smtp_local.py [--host HOST] [--port PORT] [--to RECIPIENT]

The script sends a complete SMTP transaction to the local server and then
verifies that the .eml file appeared in data/mail_spool/.

Requirements: Python 3.7+ standard library only (no external packages).
"""

import argparse
import glob
import os
import socket
import sys
import time

# ---------------------------------------------------------------------------
# Defaults
# ---------------------------------------------------------------------------
DEFAULT_HOST = "127.0.0.1"
DEFAULT_PORT = 2525
DEFAULT_FROM = "tester@example.com"
DEFAULT_TO   = "assistant@mail.yihua.app"   # adjust to a user in your DB
SPOOL_DIR    = "data/mail_spool"

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def recv_line(sock: socket.socket, timeout: float = 5.0) -> str:
    """Read one SMTP response line (may be multi-line 250-)."""
    sock.settimeout(timeout)
    buf = b""
    while True:
        chunk = sock.recv(4096)
        if not chunk:
            break
        buf += chunk
        # A complete response ends with \r\n and the code has no '-' after it
        lines = buf.split(b"\r\n")
        for line in lines:
            if line and len(line) >= 4 and line[3:4] == b" ":
                return buf.decode(errors="replace")
        if buf.endswith(b"\r\n"):
            # Multi-line still in progress or done
            last = [l for l in lines if l]
            if last and len(last[-1]) >= 4 and last[-1][3:4] == b" ":
                return buf.decode(errors="replace")
    return buf.decode(errors="replace")


def send_cmd(sock: socket.socket, cmd: str) -> str:
    print(f"  C: {cmd}")
    sock.sendall((cmd + "\r\n").encode())
    resp = recv_line(sock)
    for line in resp.strip().splitlines():
        print(f"  S: {line}")
    return resp


def expect(resp: str, code: str, label: str):
    if not resp.startswith(code):
        print(f"\n[FAIL] {label}: expected {code}, got: {resp.strip()}")
        sys.exit(1)
    print(f"  ✓ {label}")


# ---------------------------------------------------------------------------
# Main test
# ---------------------------------------------------------------------------

def run_test(host: str, port: int, mail_from: str, rcpt_to: str):
    print(f"\n{'='*60}")
    print(f"  AI Mail Butler – SMTP reception test")
    print(f"  Server : {host}:{port}")
    print(f"  From   : {mail_from}")
    print(f"  To     : {rcpt_to}")
    print(f"{'='*60}\n")

    # ---- Snapshot spool before ----
    before = set(glob.glob(f"{SPOOL_DIR}/mail_*.eml"))

    # ---- Connect ----
    print("[1] Connecting …")
    try:
        sock = socket.create_connection((host, port), timeout=5)
    except OSError as e:
        print(f"\n[FAIL] Cannot connect to {host}:{port}: {e}")
        print("       Is the AI Mail Butler server running?")
        sys.exit(1)

    banner = recv_line(sock)
    for line in banner.strip().splitlines():
        print(f"  S: {line}")
    expect(banner, "220", "SMTP banner")

    # ---- EHLO ----
    print("\n[2] EHLO …")
    resp = send_cmd(sock, "EHLO test.local")
    expect(resp, "250", "EHLO accepted")

    # ---- MAIL FROM ----
    print("\n[3] MAIL FROM …")
    resp = send_cmd(sock, f"MAIL FROM:<{mail_from}>")
    expect(resp, "250", "MAIL FROM accepted")

    # ---- RCPT TO ----
    print("\n[4] RCPT TO …")
    resp = send_cmd(sock, f"RCPT TO:<{rcpt_to}>")
    expect(resp, "250", "RCPT TO accepted")

    # ---- DATA ----
    print("\n[5] DATA …")
    resp = send_cmd(sock, "DATA")
    expect(resp, "354", "DATA started")

    subject  = "SMTP Test – AI Mail Butler local test"
    body     = (
        "This is an automated SMTP reception test sent by test_smtp_local.py.\r\n"
        "\r\n"
        "If you can read this, the SMTP server correctly received the email\r\n"
        "and saved it to the spool directory.\r\n"
    )
    message = (
        f"From: {mail_from}\r\n"
        f"To: {rcpt_to}\r\n"
        f"Subject: {subject}\r\n"
        f"MIME-Version: 1.0\r\n"
        f"Content-Type: text/plain; charset=utf-8\r\n"
        f"\r\n"
        f"{body}"
        f".\r\n"
    )
    print("  C: <message body + end-of-data>")
    sock.sendall(message.encode())
    resp = recv_line(sock)
    for line in resp.strip().splitlines():
        print(f"  S: {line}")
    expect(resp, "250", "Message accepted")

    # ---- QUIT ----
    print("\n[6] QUIT …")
    resp = send_cmd(sock, "QUIT")
    expect(resp, "221", "QUIT accepted")
    sock.close()

    # ---- Verify spool file appeared ----
    print(f"\n[7] Checking spool dir {SPOOL_DIR!r} …")
    deadline = time.time() + 5
    new_file = None
    while time.time() < deadline:
        after = set(glob.glob(f"{SPOOL_DIR}/mail_*.eml"))
        new_files = after - before
        if new_files:
            new_file = sorted(new_files)[-1]
            break
        time.sleep(0.2)

    if new_file:
        print(f"  ✓ Email saved → {new_file}")
        size = os.path.getsize(new_file)
        print(f"    Size: {size} bytes")
    else:
        print(f"  ✗ No new .eml file appeared in {SPOOL_DIR}/ within 5 s")
        print("    Check server logs for errors.")
        sys.exit(1)

    # ---- Check session log ----
    session_logs = sorted(glob.glob(f"{SPOOL_DIR}/session_*.log"))
    if session_logs:
        print(f"\n  Session logs present ({len(session_logs)} total):")
        print(f"    Latest: {session_logs[-1]}")
    else:
        print("\n  (No session logs found yet)")

    print(f"\n{'='*60}")
    print("  ALL CHECKS PASSED – SMTP reception is working correctly.")
    print(f"{'='*60}\n")


# ---------------------------------------------------------------------------

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Test SMTP reception for AI Mail Butler")
    parser.add_argument("--host", default=DEFAULT_HOST, help="SMTP server host")
    parser.add_argument("--port", type=int, default=DEFAULT_PORT, help="SMTP server port")
    parser.add_argument("--from", dest="mail_from", default=DEFAULT_FROM, help="MAIL FROM address")
    parser.add_argument("--to", dest="rcpt_to", default=DEFAULT_TO, help="RCPT TO address")
    args = parser.parse_args()

    run_test(args.host, args.port, args.mail_from, args.rcpt_to)
