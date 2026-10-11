#!/bin/sh
# Applies the DagomApp udev rule to already plugged in printers, after the
# package is installed or removed.
if command -v udevadm >/dev/null 2>&1; then
	udevadm control --reload-rules || true
	udevadm trigger --subsystem-match=tty --action=change || true
fi
