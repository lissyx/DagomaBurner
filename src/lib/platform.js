"use strict";

// Browser replacement for Node's `process.platform`.
var platform = navigator.userAgent || "";

export default /Mac/.test(platform) ? "darwin"
  : /Win/.test(platform) ? "win32"
  : "linux";
